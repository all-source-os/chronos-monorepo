package main

import (
	"context"
	"encoding/json"
	"fmt"
	"log"
	"net/http"
	"os"
	"os/signal"
	"syscall"
	"time"

	"github.com/gin-gonic/gin"
	"github.com/go-resty/resty/v2"
	"github.com/prometheus/client_golang/prometheus/promhttp"

	"github.com/allsource/control-plane/internal"
	"github.com/allsource/control-plane/internal/domain/entities"
	"github.com/allsource/control-plane/internal/infrastructure/clients"
	"github.com/allsource/control-plane/internal/infrastructure/database"
)

// Control plane configuration constants.
const (
	// Version is the current version of the control plane.
	Version = "0.9.1"
	// DefaultPort is the default port the control plane listens on.
	DefaultPort = "3901"
	// CoreServiceURL is the URL of the core event store service.
	CoreServiceURL = "http://localhost:3900"
)

// Connection pooling configuration for HTTP connections to Rust Core.
// These settings optimize connection reuse and reduce connection overhead.
const (
	// MaxIdleConns is the maximum number of idle connections across all hosts.
	MaxIdleConns = 100
	// MaxIdleConnsPerHost is the maximum number of idle connections per host.
	MaxIdleConnsPerHost = 100
	// IdleConnTimeout is the duration an idle connection remains open.
	IdleConnTimeout = 90 * time.Second
)

// NewPooledHTTPClient creates an http.Client with connection pooling configured.
func NewPooledHTTPClient() *http.Client {
	transport := &http.Transport{
		MaxIdleConns:        MaxIdleConns,
		MaxIdleConnsPerHost: MaxIdleConnsPerHost,
		IdleConnTimeout:     IdleConnTimeout,
	}
	return &http.Client{
		Transport: transport,
	}
}

// ControlPlane is the main control plane service that manages the event store cluster.
type ControlPlane struct {
	client       *resty.Client
	router       *gin.Engine
	metrics      *ControlPlaneMetrics
	cacheMetrics *CacheMetrics
	cache        *ResponseCache
	container    *internal.Container
	authClient   *AuthClient
	auditLogger  *AuditLogger
	policyEngine *PolicyEngine
	db           *database.Postgres
	persistence  string // "postgresql" or "memory"
}

// NewControlPlane creates a new control plane instance with full middleware stack.
func NewControlPlane(ctx context.Context) (*ControlPlane, error) {
	// Determine core service URL
	coreURL := os.Getenv("CORE_SERVICE_URL")
	if coreURL == "" {
		coreURL = CoreServiceURL
	}

	// Create resty client with pooled HTTP transport for connection reuse
	httpClient := NewPooledHTTPClient()
	client := resty.NewWithClient(httpClient).
		SetTimeout(10 * time.Second).
		SetBaseURL(coreURL)

	// Set up Gin router
	if os.Getenv("GIN_MODE") == "release" {
		gin.SetMode(gin.ReleaseMode)
	}
	router := gin.New()
	router.Use(gin.Recovery())

	// Initialize metrics
	metrics := NewMetrics()
	cacheMetrics := NewCacheMetrics()

	// Initialize response cache
	cache := NewResponseCache(cacheMetrics)

	// Initialize auth client
	jwtSecret := os.Getenv("JWT_SECRET")
	if jwtSecret == "" {
		log.Println("JWT_SECRET not set, using default (INSECURE for production)")
		jwtSecret = "default-secret-change-in-production"
	}
	authClient := NewAuthClient(jwtSecret)

	// Initialize audit logger
	auditLogPath := os.Getenv("AUDIT_LOG_PATH")
	if auditLogPath == "" {
		auditLogPath = "audit.log"
	}
	auditLogger, err := NewAsyncAuditLogger(auditLogPath)
	if err != nil {
		return nil, fmt.Errorf("failed to initialize audit logger: %w", err)
	}

	// Initialize policy engine
	policyEngine := NewPolicyEngine()

	// Initialize typed CoreClient for Clean Architecture use cases
	coreClient := clients.NewCoreClient()

	// Initialize database (optional — falls back to in-memory if DATABASE_URL not set)
	var db *database.Postgres
	var containerCfg internal.ContainerConfig
	persistence := "memory"

	databaseURL := os.Getenv("DATABASE_URL")
	if databaseURL != "" {
		db, err = database.New(ctx)
		if err != nil {
			return nil, fmt.Errorf("failed to connect to database: %w", err)
		}

		// Run migrations
		if err := db.RunMigrations(ctx, "migrations"); err != nil {
			db.Close()
			return nil, fmt.Errorf("failed to run migrations: %w", err)
		}
		log.Println("PostgreSQL connected, migrations applied")

		containerCfg = internal.ContainerConfig{
			DatabaseURL: databaseURL,
			Pool:        db.Pool,
			CoreClient:  coreClient,
		}
		persistence = "postgresql"
	} else {
		log.Println("DATABASE_URL not set, using in-memory repositories")
		containerCfg = internal.ContainerConfig{
			CoreClient: coreClient,
		}
	}

	// Initialize Clean Architecture container
	container := internal.NewContainerWithConfig(containerCfg)

	// Initialize tracing
	otelEndpoint := os.Getenv("OTEL_ENDPOINT")
	tracingEnabled := otelEndpoint != ""
	tracingShutdown, err := InitTracing(TracingConfig{
		Enabled:      tracingEnabled,
		OTLPEndpoint: otelEndpoint,
		SamplingRate: 1.0,
	})
	if err != nil {
		log.Printf("Failed to initialize tracing: %v", err)
	}
	// tracingShutdown will be called during graceful shutdown
	_ = tracingShutdown

	cp := &ControlPlane{
		client:       client,
		router:       router,
		metrics:      metrics,
		cacheMetrics: cacheMetrics,
		cache:        cache,
		container:    container,
		authClient:   authClient,
		auditLogger:  auditLogger,
		policyEngine: policyEngine,
		db:           db,
		persistence:  persistence,
	}

	cp.setupMiddleware()
	cp.setupRoutes()

	// Start background operation scheduler
	cp.container.Scheduler.Start(ctx)

	return cp, nil
}

func (cp *ControlPlane) setupMiddleware() {
	// CORS middleware
	cp.router.Use(func(c *gin.Context) {
		c.Writer.Header().Set("Access-Control-Allow-Origin", "*")
		c.Writer.Header().Set("Access-Control-Allow-Methods", "GET, POST, PUT, DELETE, OPTIONS")
		c.Writer.Header().Set("Access-Control-Allow-Headers", "Content-Type, Authorization")

		if c.Request.Method == "OPTIONS" {
			c.AbortWithStatus(204)
			return
		}

		c.Next()
	})

	// Prometheus metrics middleware
	cp.router.Use(PrometheusMiddleware(cp.metrics))

	// Tracing middleware (all routes)
	cp.router.Use(TracingMiddleware(serviceName))

	// Audit logging middleware (before auth so we log all attempts)
	cp.router.Use(AuditMiddleware(cp.auditLogger))

	// Auth middleware (applied globally, skips /health and /metrics)
	cp.router.Use(AuthMiddleware(cp.authClient))

	// Policy middleware (after auth, uses auth context)
	cp.router.Use(PolicyMiddleware(cp.policyEngine, cp.auditLogger))
}

func (cp *ControlPlane) setupRoutes() {
	// Public endpoints (no auth required — skipped by AuthMiddleware)
	cp.router.GET("/health", cp.healthHandler)
	cp.router.GET("/metrics", gin.WrapH(promhttp.Handler()))

	// Authentication endpoints
	auth := cp.router.Group("/api/v1/auth")
	auth.POST("/login", cp.LoginHandler)
	auth.POST("/register", cp.RegisterHandler)

	// Protected API endpoints
	api := cp.router.Group("/api/v1")

	// Cluster management
	api.GET("/cluster/status", RequirePermission(entities.PermissionRead), cp.container.OperationsHandler.GetClusterStatus)
	api.GET("/cluster/health", cp.container.OperationsHandler.ClusterHealth) // public — auth skipped by AuthMiddleware
	api.GET("/metrics/json", cp.metricsHandler)

	// Core service health check
	api.GET("/health/core", cp.coreHealthHandler)

	// Operations (Clean Architecture handlers with per-route RBAC)
	operations := api.Group("/operations")
	operations.POST("/snapshots", RequirePermission(entities.PermissionAdmin), cp.container.OperationsHandler.CreateSnapshot)
	operations.GET("/snapshots", RequirePermission(entities.PermissionRead), cp.container.OperationsHandler.ListSnapshots)
	operations.POST("/compaction", RequirePermission(entities.PermissionAdmin), cp.container.OperationsHandler.TriggerCompaction)
	operations.GET("/compaction/stats", RequirePermission(entities.PermissionRead), cp.container.OperationsHandler.GetCompactionStats)
	operations.POST("/replay", RequirePermission(entities.PermissionAdmin), cp.container.OperationsHandler.StartReplay)
	operations.GET("/replay/:id", RequirePermission(entities.PermissionRead), cp.container.OperationsHandler.GetReplayProgress)
	operations.POST("/replay/:id/cancel", RequirePermission(entities.PermissionAdmin), cp.container.OperationsHandler.CancelReplay)
	operations.GET("/history", RequirePermission(entities.PermissionRead), cp.container.OperationsHandler.ListOperations)
	operations.POST("/backup", RequirePermission(entities.PermissionAdmin), cp.backupHandler)

	// Tenant management (Clean Architecture handlers with per-route RBAC)
	tenants := api.Group("/tenants")
	tenants.GET("", RequirePermission(entities.PermissionRead), cp.container.TenantHandler.List)
	tenants.GET("/:id", RequirePermission(entities.PermissionRead), cp.container.TenantHandler.Get)
	tenants.POST("", RequirePermission(entities.PermissionManageTenants), cp.container.TenantHandler.Create)
	tenants.PUT("/:id", RequirePermission(entities.PermissionManageTenants), cp.container.TenantHandler.Update)
	tenants.POST("/:id/suspend", RequireAdmin(), cp.container.TenantHandler.Suspend)
	tenants.POST("/:id/activate", RequireAdmin(), cp.container.TenantHandler.Activate)
	tenants.DELETE("/:id", RequireAdmin(), cp.container.TenantHandler.Delete)
	tenants.GET("/:id/stats", RequirePermission(entities.PermissionRead), cp.container.TenantHandler.Stats)

	// User management (admin only)
	users := api.Group("/users")
	users.Use(RequireAdmin())
	users.GET("", cp.listUsersHandler)
	users.DELETE("/:id", cp.deleteUserHandler)

	// Policy management (Clean Architecture handlers with per-route RBAC)
	policies := api.Group("/policies")
	policies.POST("/evaluate", RequirePermission(entities.PermissionRead), cp.container.PolicyHandler.Evaluate)
	policies.GET("", RequirePermission(entities.PermissionRead), cp.container.PolicyHandler.List)
	policies.GET("/:id", RequirePermission(entities.PermissionRead), cp.container.PolicyHandler.Get)
	policies.POST("", RequireAdmin(), cp.container.PolicyHandler.Create)
	policies.PUT("/:id", RequireAdmin(), cp.container.PolicyHandler.Update)
	policies.DELETE("/:id", RequireAdmin(), cp.container.PolicyHandler.Delete)
	policies.POST("/:id/enable", RequireAdmin(), cp.container.PolicyHandler.Enable)
	policies.POST("/:id/disable", RequireAdmin(), cp.container.PolicyHandler.Disable)

	// Schema governance (Clean Architecture handlers with per-route RBAC)
	schemas := api.Group("/schemas")
	schemas.POST("", RequirePermission(entities.PermissionManageSchemas), cp.container.SchemaHandler.Register)
	schemas.GET("", RequirePermission(entities.PermissionRead), cp.container.SchemaHandler.List)
	schemas.POST("/validate", RequirePermission(entities.PermissionRead), cp.container.SchemaHandler.Validate)

	// Audit trail (Clean Architecture handlers)
	api.GET("/audit", RequirePermission(entities.PermissionRead), cp.container.AuditHandler.Query)

	// Dynamic configuration (Clean Architecture handlers with per-route RBAC)
	config := api.Group("/config")
	config.GET("", RequirePermission(entities.PermissionRead), cp.container.ConfigHandler.List)
	config.GET("/:key", RequirePermission(entities.PermissionRead), cp.container.ConfigHandler.Get)
	config.POST("", RequireAdmin(), cp.container.ConfigHandler.Create)
	config.PUT("/:key", RequireAdmin(), cp.container.ConfigHandler.Update)
	config.DELETE("/:key", RequireAdmin(), cp.container.ConfigHandler.Delete)
}

// Health handler reports persistence type
func (cp *ControlPlane) healthHandler(c *gin.Context) {
	health := gin.H{
		"status":      "healthy",
		"service":     "allsource-control-plane",
		"version":     Version,
		"persistence": cp.persistence,
		"timestamp":   time.Now().UTC(),
		"features": gin.H{
			"authentication": true,
			"audit_logging":  cp.auditLogger.enabled,
			"rbac":           true,
			"tracing":        os.Getenv("OTEL_ENDPOINT") != "",
		},
	}

	if cp.db != nil {
		health["database"] = cp.db.HealthCheck()
	}

	c.JSON(http.StatusOK, health)
}

func (cp *ControlPlane) coreHealthHandler(c *gin.Context) {
	start := time.Now()

	token, _ := ExtractToken(c) //nolint:errcheck

	resp, err := cp.client.R().
		SetHeader("Authorization", "Bearer "+token).
		Get("/health")

	duration := time.Since(start).Seconds()
	cp.metrics.CoreHealthCheckDuration.Observe(duration)

	if err != nil {
		cp.metrics.CoreHealthCheckTotal.WithLabelValues("error").Inc()
		c.JSON(http.StatusServiceUnavailable, gin.H{
			"status": "unhealthy",
			"error":  err.Error(),
		})
		return
	}

	var result map[string]any
	if err := json.Unmarshal(resp.Body(), &result); err != nil {
		cp.metrics.CoreHealthCheckTotal.WithLabelValues("error").Inc()
		c.JSON(http.StatusInternalServerError, gin.H{
			"status": "error",
			"error":  "failed to parse core response",
		})
		return
	}

	cp.metrics.CoreHealthCheckTotal.WithLabelValues("success").Inc()
	c.JSON(http.StatusOK, result)
}

func (cp *ControlPlane) metricsHandler(c *gin.Context) {
	// Check cache first
	if cached := cp.cache.Get(CacheKeyMetrics); cached != nil {
		c.JSON(http.StatusOK, cached.Data)
		return
	}

	token, _ := ExtractToken(c) //nolint:errcheck

	// Aggregate metrics from core
	resp, err := cp.client.R().
		SetHeader("Authorization", "Bearer "+token).
		Get("/api/v1/stats")

	if err != nil {
		c.JSON(http.StatusServiceUnavailable, gin.H{
			"error": "failed to fetch metrics from core",
		})
		return
	}

	var stats map[string]any
	_ = json.Unmarshal(resp.Body(), &stats) //nolint:errcheck // best effort parsing

	response := gin.H{
		"metrics": gin.H{
			"event_store": stats,
			"control_plane": gin.H{
				"uptime_seconds": time.Since(startTime).Seconds(),
				"version":        Version,
			},
		},
		"timestamp": time.Now().UTC(),
	}

	// Cache the response
	cp.cache.Set(CacheKeyMetrics, response, MetricsCacheTTL)

	c.JSON(http.StatusOK, response)
}

func (cp *ControlPlane) backupHandler(c *gin.Context) {
	auth, _ := GetAuthContext(c) //nolint:errcheck
	token, _ := ExtractToken(c)  //nolint:errcheck

	// Proxy to core backup endpoint
	resp, err := cp.client.R().
		SetHeader("Authorization", "Bearer "+token).
		Post("/api/v1/backup")

	if err != nil {
		c.JSON(http.StatusServiceUnavailable, gin.H{
			"error": "failed to initiate backup on core service",
		})
		return
	}

	backupID := fmt.Sprintf("backup-%d", time.Now().Unix())
	if auth != nil {
		cp.auditLogger.LogOperationEvent("backup_create", backupID, auth.UserID, "initiated")
	}

	var result map[string]any
	_ = json.Unmarshal(resp.Body(), &result) //nolint:errcheck // best effort parsing
	c.JSON(http.StatusOK, result)
}

// User handlers (proxied to core)
func (cp *ControlPlane) listUsersHandler(c *gin.Context) {
	cp.proxyToCoreAuth(c, "GET", "/api/v1/auth/users")
}

func (cp *ControlPlane) deleteUserHandler(c *gin.Context) {
	userID := c.Param("id")
	auth, _ := GetAuthContext(c) //nolint:errcheck

	token, _ := ExtractToken(c) //nolint:errcheck
	resp, err := cp.client.R().
		SetHeader("Authorization", "Bearer "+token).
		Delete("/api/v1/auth/users/" + userID)

	if err != nil {
		c.JSON(http.StatusServiceUnavailable, gin.H{
			"error": "failed to delete user from core service",
		})
		return
	}

	if auth != nil {
		cp.auditLogger.LogAuthEvent("user_delete", userID, "", "", "user deleted by "+auth.Username)
	}
	c.JSON(resp.StatusCode(), gin.H{"message": "user deleted"})
}

// Helper: proxy request to core with auth
func (cp *ControlPlane) proxyToCoreAuth(c *gin.Context, method, path string) {
	token, _ := ExtractToken(c) //nolint:errcheck

	resp, err := cp.client.R().
		SetHeader("Authorization", "Bearer "+token).
		Execute(method, path)

	if err != nil {
		c.JSON(http.StatusServiceUnavailable, gin.H{
			"error": "failed to communicate with core service",
		})
		return
	}

	var result map[string]any
	_ = json.Unmarshal(resp.Body(), &result) //nolint:errcheck // best effort parsing
	c.JSON(resp.StatusCode(), result)
}

// Shutdown gracefully shuts down the control plane and closes resources.
func (cp *ControlPlane) Shutdown() {
	// Stop scheduler first
	if cp.container != nil && cp.container.Scheduler != nil {
		cp.container.Scheduler.Stop()
	}
	if cp.auditLogger != nil {
		if err := cp.auditLogger.Close(); err != nil {
			log.Printf("Error closing audit logger: %v", err)
		}
	}
	if cp.db != nil {
		cp.db.Close()
		log.Println("PostgreSQL connection pool closed")
	}
}

var startTime time.Time

func main() {
	startTime = time.Now()

	log.Println("AllSource Control Plane starting...")

	port := os.Getenv("PORT")
	if port == "" {
		port = DefaultPort
	}

	ctx := context.Background()

	cp, err := NewControlPlane(ctx)
	if err != nil {
		log.Fatalf("Failed to initialize control plane: %v", err)
	}

	// Graceful shutdown
	srv := &http.Server{
		Addr:              ":" + port,
		Handler:           cp.router,
		ReadHeaderTimeout: 10 * time.Second,
	}

	go func() {
		log.Printf("Control Plane v%s listening on port %s", Version, port)
		log.Printf("Persistence: %s", cp.persistence)
		log.Println("Authentication enabled")
		log.Println("RBAC enabled")
		log.Println("Audit logging enabled")
		if err := srv.ListenAndServe(); err != nil && err != http.ErrServerClosed {
			log.Fatalf("Server failed: %v", err)
		}
	}()

	// Wait for interrupt signal
	quit := make(chan os.Signal, 1)
	signal.Notify(quit, syscall.SIGINT, syscall.SIGTERM)
	<-quit

	log.Println("Shutting down gracefully...")
	shutdownCtx, cancel := context.WithTimeout(context.Background(), 5*time.Second)
	defer cancel()

	if err := srv.Shutdown(shutdownCtx); err != nil {
		log.Fatal("Server forced to shutdown:", err)
	}

	// Cleanup resources
	cp.Shutdown()

	log.Println("Control Plane stopped")
}
