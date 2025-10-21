package main

import (
	"context"
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
)

const (
	DefaultPort    = "8081"
	CoreServiceURL = "http://localhost:8080"
	Version        = "1.0.0"
)

type ControlPlaneV1 struct {
	client      *resty.Client
	router      *gin.Engine
	metrics     *ControlPlaneMetrics
	authClient  *AuthClient
	auditLogger *AuditLogger
}

func NewControlPlaneV1() (*ControlPlaneV1, error) {
	// Initialize auth client
	jwtSecret := os.Getenv("JWT_SECRET")
	if jwtSecret == "" {
		log.Println("⚠️  JWT_SECRET not set, using default (INSECURE for production)")
		jwtSecret = "default-secret-change-in-production"
	}
	authClient := NewAuthClient(jwtSecret)

	// Initialize audit logger
	auditLogPath := os.Getenv("AUDIT_LOG_PATH")
	if auditLogPath == "" {
		auditLogPath = "audit.log"
	}
	auditLogger, err := NewAuditLogger(auditLogPath)
	if err != nil {
		return nil, fmt.Errorf("failed to initialize audit logger: %w", err)
	}

	// Initialize HTTP client with auth token support
	client := resty.New().
		SetTimeout(10 * time.Second).
		SetBaseURL(CoreServiceURL)

	// Set up Gin router
	if os.Getenv("GIN_MODE") == "release" {
		gin.SetMode(gin.ReleaseMode)
	}
	router := gin.New()
	router.Use(gin.Recovery())

	// Initialize metrics
	metrics := NewMetrics()

	cp := &ControlPlaneV1{
		client:      client,
		router:      router,
		metrics:     metrics,
		authClient:  authClient,
		auditLogger: auditLogger,
	}

	// Setup middleware
	cp.setupMiddleware()

	// Setup routes
	cp.setupRoutes()

	return cp, nil
}

func (cp *ControlPlaneV1) setupMiddleware() {
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

	// Audit logging middleware (before auth so we log all attempts)
	cp.router.Use(AuditMiddleware(cp.auditLogger))

	// Auth middleware (applied globally, but allows health/metrics endpoints)
	cp.router.Use(AuthMiddleware(cp.authClient))
}

func (cp *ControlPlaneV1) setupRoutes() {
	// Public endpoints (no auth required)
	cp.router.GET("/health", cp.healthHandler)
	cp.router.GET("/metrics", gin.WrapH(promhttp.Handler()))

	// Authentication endpoints (public for login/register)
	auth := cp.router.Group("/api/v1/auth")
	{
		auth.POST("/login", cp.loginHandler)
		auth.POST("/register", cp.registerHandler)
		auth.GET("/me", cp.meHandler) // Requires auth (handled by middleware)
	}

	// Protected API endpoints
	api := cp.router.Group("/api/v1")
	{
		// Cluster management (read access)
		api.GET("/cluster/status", cp.clusterStatusHandler)
		api.GET("/metrics/json", cp.metricsHandler)

		// Core service health check
		api.GET("/health/core", cp.coreHealthHandler)

		// Operations (require specific permissions)
		operations := api.Group("/operations")
		{
			operations.POST("/snapshot", RequirePermission(PermissionAdmin), cp.snapshotHandler)
			operations.POST("/replay", RequirePermission(PermissionAdmin), cp.replayHandler)
			operations.POST("/backup", RequirePermission(PermissionAdmin), cp.backupHandler)
		}

		// Tenant management (admin only)
		tenants := api.Group("/tenants")
		tenants.Use(RequireAdmin())
		{
			tenants.GET("", cp.listTenantsHandler)
			tenants.GET("/:id", cp.getTenantHandler)
			tenants.POST("", cp.createTenantHandler)
			tenants.PUT("/:id", cp.updateTenantHandler)
			tenants.DELETE("/:id", cp.deleteTenantHandler)
		}

		// User management (admin only)
		users := api.Group("/users")
		users.Use(RequireAdmin())
		{
			users.GET("", cp.listUsersHandler)
			users.DELETE("/:id", cp.deleteUserHandler)
		}
	}
}

// Health handler
func (cp *ControlPlaneV1) healthHandler(c *gin.Context) {
	c.JSON(http.StatusOK, gin.H{
		"status":    "healthy",
		"service":   "allsource-control-plane",
		"version":   Version,
		"timestamp": time.Now().UTC(),
		"features": gin.H{
			"authentication": true,
			"audit_logging":  cp.auditLogger.enabled,
			"rbac":           true,
			"tracing":        false, // Will be true when OpenTelemetry is fully integrated
		},
	})
}

// Core health handler with authenticated request
func (cp *ControlPlaneV1) coreHealthHandler(c *gin.Context) {
	start := time.Now()

	// Get auth token from current request to pass to core
	token, _ := ExtractToken(c)

	// Make authenticated request to core
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

	var result map[string]interface{}
	if err := resp.UnmarshalJson(&result); err != nil {
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

// Cluster status handler
func (cp *ControlPlaneV1) clusterStatusHandler(c *gin.Context) {
	auth, _ := GetAuthContext(c)

	// Get auth token to pass to core
	token, _ := ExtractToken(c)

	// Get core stats (authenticated)
	resp, err := cp.client.R().
		SetHeader("Authorization", "Bearer "+token).
		Get("/api/v1/stats")

	var coreStats map[string]interface{}
	if err == nil {
		resp.UnmarshalJson(&coreStats)
	}

	c.JSON(http.StatusOK, gin.H{
		"cluster_id": "allsource-v1",
		"requester": gin.H{
			"user_id":   auth.UserID,
			"tenant_id": auth.TenantID,
			"role":      auth.Role,
		},
		"nodes": []gin.H{
			{
				"id":     "core-1",
				"type":   "event-store",
				"status": "healthy",
				"url":    CoreServiceURL,
				"stats":  coreStats,
			},
		},
		"total_nodes":   1,
		"healthy_nodes": 1,
		"timestamp":     time.Now().UTC(),
	})
}

// Metrics handler
func (cp *ControlPlaneV1) metricsHandler(c *gin.Context) {
	token, _ := ExtractToken(c)

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

	var stats map[string]interface{}
	resp.UnmarshalJson(&stats)

	c.JSON(http.StatusOK, gin.H{
		"metrics": gin.H{
			"event_store": stats,
			"control_plane": gin.H{
				"uptime_seconds": time.Since(startTime).Seconds(),
				"version":        Version,
			},
		},
		"timestamp": time.Now().UTC(),
	})
}

// Snapshot handler
func (cp *ControlPlaneV1) snapshotHandler(c *gin.Context) {
	auth, _ := GetAuthContext(c)
	cp.metrics.SnapshotOperationsTotal.Inc()

	// Log audit event
	snapshotID := fmt.Sprintf("snapshot-%d", time.Now().Unix())
	cp.auditLogger.LogOperationEvent("snapshot_create", snapshotID, auth.UserID, "initiated")

	c.JSON(http.StatusOK, gin.H{
		"snapshot_id": snapshotID,
		"status":      "created",
		"timestamp":   time.Now().UTC(),
		"created_by":  auth.Username,
		"message":     "Snapshot operation initiated",
	})
}

// Replay handler
func (cp *ControlPlaneV1) replayHandler(c *gin.Context) {
	auth, _ := GetAuthContext(c)

	var req struct {
		EntityID string     `json:"entity_id"`
		AsOf     *time.Time `json:"as_of"`
	}

	if err := c.ShouldBindJSON(&req); err != nil {
		c.JSON(http.StatusBadRequest, gin.H{"error": err.Error()})
		return
	}

	cp.metrics.ReplayOperationsTotal.Inc()
	cp.auditLogger.LogOperationEvent("replay", req.EntityID, auth.UserID, "initiated")

	c.JSON(http.StatusOK, gin.H{
		"status":     "replay_initiated",
		"entity_id":  req.EntityID,
		"as_of":      req.AsOf,
		"timestamp":  time.Now().UTC(),
		"initiated_by": auth.Username,
	})
}

// Backup handler
func (cp *ControlPlaneV1) backupHandler(c *gin.Context) {
	auth, _ := GetAuthContext(c)
	token, _ := ExtractToken(c)

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
	cp.auditLogger.LogOperationEvent("backup_create", backupID, auth.UserID, "initiated")

	var result map[string]interface{}
	resp.UnmarshalJson(&result)
	c.JSON(http.StatusOK, result)
}

// Tenant handlers (proxied to core)
func (cp *ControlPlaneV1) listTenantsHandler(c *gin.Context) {
	cp.proxyToCoreAuth(c, "GET", "/api/v1/tenants")
}

func (cp *ControlPlaneV1) getTenantHandler(c *gin.Context) {
	tenantID := c.Param("id")
	cp.proxyToCoreAuth(c, "GET", "/api/v1/tenants/"+tenantID)
}

func (cp *ControlPlaneV1) createTenantHandler(c *gin.Context) {
	auth, _ := GetAuthContext(c)
	var req map[string]interface{}
	c.ShouldBindJSON(&req)

	resp, err := cp.proxyToCoreAuthWithBody(c, "POST", "/api/v1/tenants", req)
	if err != nil {
		return
	}

	if tenantID, ok := req["id"].(string); ok {
		cp.auditLogger.LogTenantEvent("create", tenantID, auth.UserID, "tenant created")
	}

	var result map[string]interface{}
	resp.UnmarshalJson(&result)
	c.JSON(resp.StatusCode(), result)
}

func (cp *ControlPlaneV1) updateTenantHandler(c *gin.Context) {
	tenantID := c.Param("id")
	auth, _ := GetAuthContext(c)

	var req map[string]interface{}
	c.ShouldBindJSON(&req)

	resp, err := cp.proxyToCoreAuthWithBody(c, "PUT", "/api/v1/tenants/"+tenantID, req)
	if err != nil {
		return
	}

	cp.auditLogger.LogTenantEvent("update", tenantID, auth.UserID, "tenant updated")

	var result map[string]interface{}
	resp.UnmarshalJson(&result)
	c.JSON(resp.StatusCode(), result)
}

func (cp *ControlPlaneV1) deleteTenantHandler(c *gin.Context) {
	tenantID := c.Param("id")
	auth, _ := GetAuthContext(c)

	resp, err := cp.proxyToCoreAuthWithBody(c, "DELETE", "/api/v1/tenants/"+tenantID, nil)
	if err != nil {
		return
	}

	cp.auditLogger.LogTenantEvent("delete", tenantID, auth.UserID, "tenant deleted")
	c.JSON(resp.StatusCode(), gin.H{"message": "tenant deleted"})
}

// User handlers (proxied to core)
func (cp *ControlPlaneV1) listUsersHandler(c *gin.Context) {
	cp.proxyToCoreAuth(c, "GET", "/api/v1/auth/users")
}

func (cp *ControlPlaneV1) deleteUserHandler(c *gin.Context) {
	userID := c.Param("id")
	auth, _ := GetAuthContext(c)

	token, _ := ExtractToken(c)
	resp, err := cp.client.R().
		SetHeader("Authorization", "Bearer "+token).
		Delete("/api/v1/auth/users/" + userID)

	if err != nil {
		c.JSON(http.StatusServiceUnavailable, gin.H{
			"error": "failed to delete user from core service",
		})
		return
	}

	cp.auditLogger.LogAuthEvent("user_delete", userID, "", "", "user deleted by "+auth.Username)
	c.JSON(resp.StatusCode(), gin.H{"message": "user deleted"})
}

// Helper: proxy request to core with auth
func (cp *ControlPlaneV1) proxyToCoreAuth(c *gin.Context, method, path string) {
	token, _ := ExtractToken(c)

	resp, err := cp.client.R().
		SetHeader("Authorization", "Bearer "+token).
		Execute(method, path)

	if err != nil {
		c.JSON(http.StatusServiceUnavailable, gin.H{
			"error": "failed to communicate with core service",
		})
		return
	}

	var result map[string]interface{}
	resp.UnmarshalJson(&result)
	c.JSON(resp.StatusCode(), result)
}

// Helper: proxy request to core with auth and body
func (cp *ControlPlaneV1) proxyToCoreAuthWithBody(c *gin.Context, method, path string, body interface{}) (*resty.Response, error) {
	token, _ := ExtractToken(c)

	resp, err := cp.client.R().
		SetHeader("Authorization", "Bearer "+token).
		SetBody(body).
		Execute(method, path)

	if err != nil {
		c.JSON(http.StatusServiceUnavailable, gin.H{
			"error": "failed to communicate with core service",
		})
		return nil, err
	}

	return resp, nil
}

func (cp *ControlPlaneV1) Start(port string) error {
	return cp.router.Run(":" + port)
}

func (cp *ControlPlaneV1) Shutdown() error {
	if cp.auditLogger != nil {
		return cp.auditLogger.Close()
	}
	return nil
}

var startTime time.Time

func main() {
	startTime = time.Now()

	log.Println("🎯 AllSource Control Plane v1.0 starting...")

	port := os.Getenv("PORT")
	if port == "" {
		port = DefaultPort
	}

	cp, err := NewControlPlaneV1()
	if err != nil {
		log.Fatalf("Failed to initialize control plane: %v", err)
	}

	// Graceful shutdown
	srv := &http.Server{
		Addr:    ":" + port,
		Handler: cp.router,
	}

	go func() {
		log.Printf("🚀 Control Plane v%s listening on port %s\n", Version, port)
		log.Println("✅ Authentication enabled")
		log.Println("✅ RBAC enabled")
		log.Println("✅ Audit logging enabled")
		if err := srv.ListenAndServe(); err != nil && err != http.ErrServerClosed {
			log.Fatalf("Server failed: %v", err)
		}
	}()

	// Wait for interrupt signal
	quit := make(chan os.Signal, 1)
	signal.Notify(quit, syscall.SIGINT, syscall.SIGTERM)
	<-quit

	log.Println("Shutting down gracefully...")
	ctx, cancel := context.WithTimeout(context.Background(), 5*time.Second)
	defer cancel()

	if err := srv.Shutdown(ctx); err != nil {
		log.Fatal("Server forced to shutdown:", err)
	}

	// Cleanup
	if err := cp.Shutdown(); err != nil {
		log.Printf("Error during shutdown: %v", err)
	}

	log.Println("Control Plane v1.0 stopped")
}
