package main

import (
	"context"
	_ "embed"
	"encoding/json"
	"fmt"
	"log"
	"net/http"
	"os"
	"os/signal"
	"syscall"
	"time"

	"github.com/dgrijalva/jwt-go"
	"github.com/gin-gonic/gin"
	"github.com/go-resty/resty/v2"
	"github.com/joho/godotenv"
	"github.com/prometheus/client_golang/prometheus/promhttp"
	"gopkg.in/yaml.v3"

	"github.com/allsource/control-plane/internal"
	"github.com/allsource/control-plane/internal/domain/entities"
	"github.com/allsource/control-plane/internal/infrastructure/clients"
	"github.com/allsource/control-plane/internal/infrastructure/x402"
	httphandlers "github.com/allsource/control-plane/internal/interfaces/http"
)

//go:embed docs/openapi.yaml
var openAPIYAML []byte

// Control plane configuration constants.
const (
	// Version is the current version of the control plane.
	Version = "0.19.2"
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
// Preserves the Authorization header on redirects (Go's default client strips it).
func NewPooledHTTPClient() *http.Client {
	transport := &http.Transport{
		MaxIdleConns:        MaxIdleConns,
		MaxIdleConnsPerHost: MaxIdleConnsPerHost,
		IdleConnTimeout:     IdleConnTimeout,
	}
	return &http.Client{
		Transport: transport,
		CheckRedirect: func(req *http.Request, via []*http.Request) error {
			if len(via) >= 10 {
				return fmt.Errorf("stopped after 10 redirects")
			}
			// Preserve Authorization header across redirects (Go strips it by default).
			// Internal service-to-service: redirect targets are constrained to
			// configured backends so header forwarding is safe here.
			if auth := via[0].Header.Get("Authorization"); auth != "" {
				req.Header.Set("Authorization", auth) //nolint:gosec // G119
			}
			return nil
		},
	}
}

// ControlPlane is the main control plane service that manages the event store cluster.
type ControlPlane struct {
	client          *resty.Client
	router          *gin.Engine
	metrics         *ControlPlaneMetrics
	cacheMetrics    *CacheMetrics
	cache           *ResponseCache
	container       *internal.Container
	authClient      *AuthClient
	auditLogger     *AuditLogger
	policyEngine    *PolicyEngine
	coreClient      clients.CoreClient
	x402Handler     *x402.Handler
	x402Pricing     *x402.PricingConfig
	x402Facilitator x402.PaymentFacilitator

	// Data-plane delegation: Control Plane is the single public entry point.
	// After auth, forward writes to Core and reads to Query Service using the
	// service JWT (backends trust CP, not the caller). Tenant is injected from
	// the authenticated caller's identity before forwarding.
	delegation *delegationClient
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
	authClient := NewAuthClient(jwtSecret, coreURL)

	// Sign a service-to-service JWT so the resty client can authenticate with Core.
	// Uses a long-lived token with admin role — Core validates with the same shared secret.
	serviceClaims := &Claims{
		UserID:   "control-plane",
		Username: "control-plane",
		TenantID: "system",
		Role:     entities.RoleAdmin,
		StandardClaims: jwt.StandardClaims{
			ExpiresAt: time.Now().Add(365 * 24 * time.Hour).Unix(),
			IssuedAt:  time.Now().Unix(),
			Issuer:    "allsource",
			Subject:   "control-plane",
		},
	}
	serviceToken, err := jwt.NewWithClaims(jwt.SigningMethodHS256, serviceClaims).SignedString([]byte(jwtSecret))
	if err != nil {
		return nil, fmt.Errorf("failed to sign service JWT: %w", err)
	}
	client.SetAuthToken(serviceToken)
	// coreURL comes from operator-controlled env config, not user input.
	log.Printf("Service JWT set for Core auth (token length: %d, core URL: %s)", len(serviceToken), coreURL) //nolint:gosec // G706

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

	// Initialize typed CoreClient for Clean Architecture use cases.
	// Uses the same admin service JWT as cp.client so it can call Core's
	// Admin-protected endpoints (config store, API key creation, etc.).
	coreClient := clients.NewCoreClientWithJWT(coreURL, serviceToken)

	// Initialize LemonSqueezy client (optional — billing features require it)
	var lsClient clients.LemonSqueezyClient
	if os.Getenv("LEMON_SQUEEZY_API_KEY") != "" {
		var lsErr error
		lsClient, lsErr = clients.NewLemonSqueezyClient()
		if lsErr != nil {
			log.Printf("LemonSqueezy client initialization failed: %v (billing endpoints will be unavailable)", lsErr)
		}
	}

	// Initialize Stripe client (optional — Stripe billing features require it)
	var stripeClient clients.StripeClient
	if os.Getenv("STRIPE_SECRET_KEY") != "" {
		var stripeErr error
		stripeClient, stripeErr = clients.NewStripeClient()
		if stripeErr != nil {
			log.Printf("Stripe client initialization failed: %v (Stripe billing endpoints will be unavailable)", stripeErr)
		}
	}

	// Initialize email client (optional — usage warning emails require it)
	var emailClient clients.EmailClient
	if os.Getenv("SMTP_HOST") != "" {
		var emailErr error
		emailClient, emailErr = clients.NewEmailClientFromEnv()
		if emailErr != nil {
			log.Printf("Email client initialization failed: %v (usage warning emails will be unavailable)", emailErr)
		}
	}

	// Initialize Coinbase CDP client (optional — enables x402 auto-pay for wallet-less agents)
	var cdpClient clients.CDPClient
	if os.Getenv("COINBASE_CDP_KEY_NAME") != "" {
		var cdpErr error
		cdpClient, cdpErr = clients.NewCoinbaseCDPClient()
		if cdpErr != nil {
			log.Printf("Coinbase CDP client init failed: %v (x402 auto-pay disabled)", cdpErr)
		}
	}

	// Initialize Clean Architecture container (Core-backed repos, no PostgreSQL)
	containerCfg := internal.ContainerConfig{
		CoreClient:   coreClient,
		LSClient:     lsClient,
		StripeClient: stripeClient,
		EmailClient:  emailClient,
		KeySigner:    authClient.SignAPIKey,
		CDPClient:    cdpClient,
	}
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

	// Initialize x402 payment facilitator (optional — requires X402_ENABLED env var)
	x402Pricing, err := x402.LoadPricingConfig()
	if err != nil {
		log.Printf("x402 pricing config load failed: %v (x402 payments will be disabled)", err)
		x402Pricing = &x402.PricingConfig{}
	}
	// Default to Coinbase's hosted facilitator — no private keys or blockchain node required.
	// Override X402_FACILITATOR_URL to point at a self-hosted facilitator.
	facilitatorURL := os.Getenv("X402_FACILITATOR_URL")
	if facilitatorURL == "" {
		facilitatorURL = x402.CoinbaseX402FacilitatorURL
	}
	var x402Facilitator x402.PaymentFacilitator = x402.NewRemoteFacilitator(facilitatorURL)
	x402Handler := x402.NewHandler(x402Facilitator, x402Pricing)

	// Data-plane delegation targets. All default to the Fly internal names
	// when unset (production wiring). Core handles event writes + query,
	// Query Service is reserved for future QS-specific reads, Prime handles
	// graph/vector/recall via /api/v1/prime/*.
	queryURL := os.Getenv("QUERY_SERVICE_URL")
	if queryURL == "" {
		queryURL = "http://allsource-query.internal:3902"
	}
	primeURL := os.Getenv("PRIME_SERVICE_URL")
	if primeURL == "" {
		primeURL = "http://allsource-prime.internal:3905"
	}
	delegation, err := newDelegationClient(coreURL, queryURL, primeURL, authClient.SignDelegationJWT, NewPooledHTTPClient())
	if err != nil {
		return nil, fmt.Errorf("init delegation client: %w", err)
	}

	cp := &ControlPlane{
		client:          client,
		router:          router,
		metrics:         metrics,
		cacheMetrics:    cacheMetrics,
		cache:           cache,
		container:       container,
		authClient:      authClient,
		auditLogger:     auditLogger,
		policyEngine:    policyEngine,
		coreClient:      coreClient,
		x402Handler:     x402Handler,
		x402Pricing:     x402Pricing,
		x402Facilitator: x402Facilitator,
		delegation:      delegation,
	}

	cp.setupMiddleware()
	cp.setupRoutes()

	// Start background operation scheduler
	cp.container.Scheduler.Start(ctx)

	// Start the event-sourced heartbeat emitter. Probes each backend every
	// heartbeatInterval and writes a service.heartbeat event to Core. The
	// /api/v1/status/services endpoint and the /status HTML page project
	// these events into the current fleet state.
	emitter := newHeartbeatEmitter(coreClient, nil, probesFromEnv(os.Getenv))
	go emitter.run(ctx, heartbeatInterval)

	return cp, nil
}

func (cp *ControlPlane) setupMiddleware() {
	// CORS middleware — echo the request origin so browsers allow credentials
	cp.router.Use(func(c *gin.Context) {
		origin := c.Request.Header.Get("Origin")
		if origin != "" {
			c.Writer.Header().Set("Access-Control-Allow-Origin", origin)
			c.Writer.Header().Set("Access-Control-Allow-Credentials", "true")
		} else {
			c.Writer.Header().Set("Access-Control-Allow-Origin", "*")
		}
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
	cp.router.GET("/docs", cp.docsHandler)
	cp.router.GET("/openapi", cp.openAPIHandler)

	// Status page + JSON feed. Both project the event-sourced heartbeats
	// written by the background emitter. No auth — this is the public
	// status data that replaces the poll-based Vigil page.
	cp.router.GET("/status", cp.statusPageHandler)
	cp.router.GET("/api/v1/status/services", cp.statusServicesHandler)

	// Authentication endpoints
	auth := cp.router.Group("/api/v1/auth")
	auth.POST("/login", cp.LoginHandler)
	auth.POST("/register", cp.RegisterHandler)
	auth.GET("/oauth/:provider", cp.OAuthAuthorize)
	auth.GET("/oauth/:provider/callback", cp.OAuthCallback)
	auth.GET("/session", cp.SessionHandler)

	// Onboarding endpoint (public, no auth required)
	onboard := cp.router.Group("/api/v1/onboard")
	onboard.POST("/start", cp.OnboardHandler)

	// Agent registration (public, no auth required)
	agents := cp.router.Group("/api/v1/agents")
	agents.POST("/register", cp.AgentRegisterHandler)

	// Agent self-service endpoints (auth required)
	agentsSelf := cp.router.Group("/api/v1/agents/me")
	agentsSelf.GET("/payments", RequirePermission(entities.PermissionRead), cp.container.AgentHandler.GetPaymentHistory)

	// x402 facilitator endpoints (public — called by payers to verify/settle payments)
	x402Routes := cp.router.Group("/x402")
	x402Routes.POST("/verify", cp.x402Handler.Verify)
	x402Routes.POST("/settle", cp.x402Handler.Settle)
	x402Routes.GET("/routes", cp.x402Handler.Routes)

	// Demo endpoint (public, no auth required)
	demo := cp.router.Group("/api/v1/demo")
	demo.POST("/start", cp.DemoStartHandler)

	// Protected API endpoints
	api := cp.router.Group("/api/v1")

	// x402 auto-pay middleware: intercepts quota-exceeded requests, auto-settles via CDP wallet
	// when the agent has one; falls back to a standard 402 when CDP is unavailable.
	if cp.x402Pricing != nil && cp.x402Pricing.Enabled {
		x402Logger := x402.NewEventLogger(cp.coreClient)
		api.Use(x402.AgentAutoPayMiddleware(
			cp.x402Facilitator,
			cp.x402Pricing,
			x402Logger,
			x402.NewCoreQuotaChecker(cp.coreClient),
			cp.container.WalletLookup,
			cp.container.CDPClient,
		))
	}

	// Reference x402 endpoint — minimal echo handler used to verify the
	// tier gate, 402 payment-required, and auto-pay flows end-to-end.
	// Sits on the `api` group so it inherits the AgentAutoPayMiddleware
	// above. Pricing lives in config/x402-pricing.json under the key
	// "POST /api/v1/agent-echo"; without that entry the route behaves
	// like any other authenticated API call.
	api.POST("/agent-echo", RequirePermission(entities.PermissionRead), cp.AgentEchoHandler)

	// Data-plane delegation. Writes delegate to Core, reads to Query Service.
	// Tenant is injected from the authenticated caller; backends see CP's
	// service JWT, never the caller's ask_ key. These sit under `api` so they
	// inherit AuthMiddleware + PolicyMiddleware + x402 auto-pay gating.
	api.POST("/events", RequirePermission(entities.PermissionWrite), cp.ProxyIngestSingle)
	api.POST("/events/batch", RequirePermission(entities.PermissionWrite), cp.ProxyIngestBatch)
	api.GET("/events/query", RequirePermission(entities.PermissionRead), cp.ProxyEventsQuery)

	// Prime catch-all: all /api/v1/prime/* methods (GET graph queries, POST
	// nodes/edges/vectors, recall, etc.) forward to the Prime service. Tenant
	// is passed as a query param; Prime's own namespace logic consumes it.
	api.Any("/prime/*path", cp.ProxyPrime)

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

	// Team management (invite flow + member management)
	// GET /api/v1/teams/invite/:token is public (no auth — used before OAuth to preview the invite).
	cp.router.GET("/api/v1/teams/invite/:token", cp.GetInviteHandler)
	teams := api.Group("/teams")
	teams.POST("/invite", cp.InviteHandler)
	teams.GET("/members", RequirePermission(entities.PermissionRead), cp.ListMembersHandler)
	teams.PUT("/members/:id", cp.UpdateMemberRoleHandler)
	teams.DELETE("/members/:id", cp.DeleteMemberHandler)
	// Agent key management — provision and revoke Core API keys for agents
	teams.POST("/agent-keys", RequirePermission(entities.PermissionWrite), cp.CreateAgentKeyHandler)
	teams.GET("/agent-keys", RequirePermission(entities.PermissionRead), cp.ListAgentKeysHandler)
	teams.DELETE("/agent-keys/:name", RequirePermission(entities.PermissionWrite), cp.RevokeAgentKeyHandler)

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

	// Billing management (Clean Architecture handlers with per-route RBAC)
	billing := api.Group("/billing")
	billing.POST("/checkout", RequirePermission(entities.PermissionManageTenants), cp.container.BillingHandler.CreateCheckout)
	billing.GET("/portal", RequirePermission(entities.PermissionManageTenants), cp.container.BillingHandler.GetPortal)
	billing.GET("/overage", RequirePermission(entities.PermissionRead), cp.container.BillingHandler.GetOverage)
	billing.POST("/overage/enable", RequirePermission(entities.PermissionManageTenants), cp.container.BillingHandler.EnableOverage)
	billing.POST("/overage/disable", RequirePermission(entities.PermissionManageTenants), cp.container.BillingHandler.DisableOverage)
	billing.GET("/projected-charges", RequirePermission(entities.PermissionRead), cp.container.BillingHandler.GetProjectedCharges)

	// Webhooks (public — no JWT auth, signature verification instead)
	api.POST("/webhooks/lemonsqueezy", cp.container.WebhookHandler.LemonSqueezy)
	api.POST("/webhooks/stripe", cp.container.WebhookHandler.Stripe)

	// Audit trail (Clean Architecture handlers)
	api.GET("/audit", RequirePermission(entities.PermissionRead), cp.container.AuditHandler.Query)

	// Dynamic configuration (Clean Architecture handlers with per-route RBAC)
	config := api.Group("/config")
	config.GET("", RequirePermission(entities.PermissionRead), cp.container.ConfigHandler.List)
	config.GET("/:key", RequirePermission(entities.PermissionRead), cp.container.ConfigHandler.Get)
	config.POST("", RequireAdmin(), cp.container.ConfigHandler.Create)
	config.PUT("/:key", RequireAdmin(), cp.container.ConfigHandler.Update)
	config.DELETE("/:key", RequireAdmin(), cp.container.ConfigHandler.Delete)

	// Admin-only route group (self-contained JWT + admin role middleware)
	admin := cp.router.Group("/api/v1/admin")
	admin.Use(httphandlers.AdminAuthMiddleware(cp.authClient.jwtSecret))
	admin.GET("/health", func(c *gin.Context) {
		c.JSON(http.StatusOK, gin.H{
			"status":  "ok",
			"service": "allsource-control-plane-admin",
			"version": Version,
		})
	})
	admin.GET("/tenants", cp.container.AdminTenantHandler.ListTenants)
	admin.POST("/tenants/bulk", cp.container.AdminTenantHandler.BulkAction)
	admin.GET("/tenants/:id", cp.container.AdminTenantHandler.GetDetail)
	admin.GET("/tenants/:id/usage", cp.container.AdminTenantHandler.GetUsage)
	admin.PUT("/tenants/:id/quotas", cp.container.AdminTenantHandler.UpdateQuotas)
	admin.POST("/tenants/:id/suspend", cp.container.AdminTenantHandler.SuspendTenant)
	admin.POST("/tenants/:id/unsuspend", cp.container.AdminTenantHandler.UnsuspendTenant)

	// Alert rules management
	admin.POST("/alerts", cp.container.AlertHandler.Create)
	admin.GET("/alerts", cp.container.AlertHandler.List)
	admin.PUT("/alerts/:id", cp.container.AlertHandler.Update)
	admin.DELETE("/alerts/:id", cp.container.AlertHandler.Delete)

	// SLO management
	admin.POST("/slos", cp.container.SLOHandler.Create)
	admin.GET("/slos", cp.container.SLOHandler.List)
	admin.DELETE("/slos/:id", cp.container.SLOHandler.Delete)

	// Security — IP allowlist/blocklist management
	admin.GET("/security/ip-rules", cp.container.IPRuleHandler.List)
	admin.POST("/security/ip-rules", cp.container.IPRuleHandler.Create)
	admin.DELETE("/security/ip-rules/:id", cp.container.IPRuleHandler.Delete)

	// Security — Token audit
	admin.GET("/security/token-audit", cp.container.TokenAuditHandler.Query)
	admin.GET("/security/token-audit/summary", cp.container.TokenAuditHandler.Summary)

	// Security — Suspicious activity detection
	admin.GET("/security/suspicious-activity", cp.container.SuspiciousActivityHandler.Detect)

	// Admin billing management
	adminBilling := admin.Group("/billing")
	adminBilling.GET("/invoices", cp.container.AdminBillingHandler.ListInvoices)
	adminBilling.GET("/revenue", cp.container.AdminBillingHandler.GetRevenue)
	adminBilling.POST("/refund", cp.container.AdminBillingHandler.ProcessRefund)
	adminBilling.GET("/dunning", cp.container.AdminBillingHandler.GetDunning)
}

// Health handler reports Core connectivity status
func (cp *ControlPlane) healthHandler(c *gin.Context) {
	coreStatus := resourceUnknown
	if cp.coreClient != nil {
		_, err := cp.coreClient.HealthCheck(c.Request.Context())
		if err != nil {
			coreStatus = "unreachable"
		} else {
			coreStatus = "healthy"
		}
	}

	health := gin.H{
		"status":      "healthy",
		"service":     "allsource-control-plane",
		"version":     Version,
		"persistence": "core",
		"timestamp":   time.Now().UTC(),
		"core_status": coreStatus,
		"features": gin.H{
			"authentication": true,
			"audit_logging":  cp.auditLogger.enabled,
			"rbac":           true,
			"tracing":        os.Getenv("OTEL_ENDPOINT") != "",
		},
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

// openAPIHandler serves the OpenAPI specification as JSON.
func (cp *ControlPlane) openAPIHandler(c *gin.Context) {
	var spec interface{}
	if err := yaml.Unmarshal(openAPIYAML, &spec); err != nil {
		c.JSON(http.StatusInternalServerError, gin.H{"error": "failed to parse OpenAPI spec"})
		return
	}
	c.JSON(http.StatusOK, spec)
}

// docsHandler serves Swagger UI for interactive API documentation.
func (cp *ControlPlane) docsHandler(c *gin.Context) {
	html := `<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>Control Plane API Documentation</title>
  <link rel="stylesheet" type="text/css" href="https://unpkg.com/swagger-ui-dist@5/swagger-ui.css">
  <style>
    html { box-sizing: border-box; overflow-y: scroll; }
    *, *:before, *:after { box-sizing: inherit; }
    body { margin: 0; background: #fafafa; }
    .swagger-ui .topbar { display: none; }
    .swagger-ui .info { margin-top: 20px; }
    .swagger-ui .info .title { font-size: 36px; }
  </style>
</head>
<body>
  <div id="swagger-ui"></div>
  <script src="https://unpkg.com/swagger-ui-dist@5/swagger-ui-bundle.js"></script>
  <script src="https://unpkg.com/swagger-ui-dist@5/swagger-ui-standalone-preset.js"></script>
  <script>
    window.onload = function() {
      window.ui = SwaggerUIBundle({
        url: "/openapi",
        dom_id: '#swagger-ui',
        deepLinking: true,
        presets: [
          SwaggerUIBundle.presets.apis,
          SwaggerUIStandalonePreset
        ],
        plugins: [
          SwaggerUIBundle.plugins.DownloadUrl
        ],
        layout: "StandaloneLayout",
        persistAuthorization: true,
        displayRequestDuration: true,
        filter: true,
        tryItOutEnabled: true
      });
    };
  </script>
</body>
</html>`

	c.Data(http.StatusOK, "text/html; charset=utf-8", []byte(html))
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
}

var startTime time.Time

func main() {
	// Load .env from repo root (two levels up from apps/control-plane/).
	// Silently ignored in production where real env vars are set.
	godotenv.Load("../../.env") //nolint:errcheck // best-effort .env load

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
		// Version is a compile-time constant, port is operator config — not user input.
		log.Printf("Control Plane v%s listening on port %s", Version, port) //nolint:gosec // G706
		log.Println("Persistence: core (no PostgreSQL)")
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
		log.Printf("Server forced to shutdown: %v", err)
	}

	// Cleanup resources
	cp.Shutdown()

	log.Println("Control Plane stopped")
}
