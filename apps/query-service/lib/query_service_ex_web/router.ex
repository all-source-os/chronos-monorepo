defmodule QueryServiceExWeb.Router do
  @moduledoc """
  Phoenix router for Query Service API endpoints.

  Routes are organized into:
  - Public routes (health checks, OAuth callbacks)
  - Authenticated routes (require JWT token)
  - Tenant-scoped routes (require auth + active subscription)
  """

  use Phoenix.Router

  import Plug.Conn
  import Phoenix.Controller

  # -------------------------------------------------------------------
  # Pipelines
  # -------------------------------------------------------------------

  pipeline :api do
    plug(:accepts, ["json"])
    plug(QueryServiceExWeb.Plugs.CorrelationId)
  end

  pipeline :authenticated do
    plug(:accepts, ["json"])
    plug(QueryServiceExWeb.Plugs.CorrelationId)
    plug(QueryServiceExWeb.Plugs.AuthPipeline)
  end

  # Requires auth + tenant context + active subscription
  pipeline :tenant_scoped do
    plug(:accepts, ["json"])
    plug(QueryServiceExWeb.Plugs.CorrelationId)
    plug(QueryServiceExWeb.Plugs.AuthPipeline)
    plug(QueryServiceExWeb.Plugs.TenantContext)
  end

  # Requires tenant + enforces event usage quota
  pipeline :events_quota do
    plug(QueryServiceExWeb.Plugs.UsageEnforcement, type: :events)
  end

  # Requires tenant + enforces query usage quota
  pipeline :queries_quota do
    plug(QueryServiceExWeb.Plugs.UsageEnforcement, type: :queries)
  end

  # Rate limiting to prevent any single tenant from overwhelming the system
  pipeline :rate_limited do
    plug(QueryServiceExWeb.Plugs.RateLimiting)
  end

  # -------------------------------------------------------------------
  # Public Routes
  # -------------------------------------------------------------------

  # Root health check (MCP expects /health)
  scope "/", QueryServiceExWeb do
    pipe_through(:api)

    get("/health", HealthController, :show)
  end

  # Health checks (no auth required)
  scope "/api", QueryServiceExWeb do
    pipe_through(:api)

    get("/health", HealthController, :show)
    get("/health/live", HealthController, :live)
    get("/health/ready", HealthController, :ready)

    # Metrics endpoints
    # GET /api/metrics - JSON format (default) or Prometheus (with Accept header/format param)
    get("/metrics", MetricsController, :show)
    # GET /api/metrics/prometheus - Always returns Prometheus text format
    get("/metrics/prometheus", MetricsController, :prometheus)

    # OpenAPI documentation
    # GET /api/openapi - Returns OpenAPI 3.0 specification in JSON
    get("/openapi", OpenApiController, :spec)
    # GET /api/docs - Swagger UI for interactive API documentation
    get("/docs", OpenApiController, :docs)

    # Cluster health (public endpoint)
    get("/cluster", ClusterController, :health)
    get("/cluster/members", ClusterController, :members)
  end

  # Cluster management (authenticated)
  scope "/api/cluster", QueryServiceExWeb do
    pipe_through(:authenticated)

    get("/registry", ClusterController, :registry)
    get("/supervisor", ClusterController, :supervisor)
    get("/hash-ring", ClusterController, :hash_ring)
  end

  # OAuth routes (public, handles provider redirects and callbacks)
  scope "/api/auth", QueryServiceExWeb do
    pipe_through(:api)

    # Dev token endpoint (only works when AUTH_DISABLED=true)
    get("/dev-token", AuthController, :dev_token)

    get("/:provider", AuthController, :request)
    get("/:provider/callback", AuthController, :callback)
  end

  # -------------------------------------------------------------------
  # Authenticated Routes (user context, no tenant required)
  # -------------------------------------------------------------------

  scope "/api/auth", QueryServiceExWeb do
    pipe_through(:authenticated)

    get("/me", AuthController, :me)
    post("/logout", AuthController, :logout)
  end

  # -------------------------------------------------------------------
  # Tenant-Scoped Routes (requires active subscription)
  # -------------------------------------------------------------------

  # Events endpoints (with usage quota enforcement for writes)
  scope "/api", QueryServiceExWeb do
    pipe_through([:tenant_scoped, :rate_limited, :events_quota])

    post("/events", EventController, :create)
    post("/events/batch", EventController, :create_batch)
  end

  # Events read endpoints (no quota for reads)
  scope "/api", QueryServiceExWeb do
    pipe_through([:tenant_scoped, :rate_limited])

    get("/events", EventController, :index)
    get("/events/:id", EventController, :show)
    get("/events/entity/:entity_id", EventController, :by_entity)
    get("/events/type/:event_type", EventController, :by_type)

    # Discovery endpoints for streams and event types
    get("/streams", EventController, :streams)
    get("/event-types", EventController, :event_types)
  end

  # Query execution (with usage quota enforcement)
  scope "/api", QueryServiceExWeb do
    pipe_through([:tenant_scoped, :rate_limited, :queries_quota])

    post("/query", QueryController, :execute)
  end

  # Projections endpoints (no quota for projection operations)
  scope "/api", QueryServiceExWeb do
    pipe_through([:tenant_scoped, :rate_limited])

    get("/projections", ProjectionController, :index)
    get("/projections/:name", ProjectionController, :show)
    post("/projections", ProjectionController, :create)
    delete("/projections/:name", ProjectionController, :delete)
    get("/projections/:name/state", ProjectionController, :get_state)
    post("/projections/:name/reset", ProjectionController, :reset)
  end

  # Schema endpoints
  scope "/api", QueryServiceExWeb do
    pipe_through([:tenant_scoped, :rate_limited])

    get("/schemas", SchemaController, :index)
    get("/schemas/:event_type", SchemaController, :show)
    post("/schemas", SchemaController, :register)
  end

  # -------------------------------------------------------------------
  # Analytics Routes
  # -------------------------------------------------------------------

  # Analytics endpoints (quota enforced for complex queries)
  scope "/api/analytics", QueryServiceExWeb do
    pipe_through([:tenant_scoped, :rate_limited, :queries_quota])

    # Core analytics (proxied to Rust Core)
    get("/frequency", AnalyticsController, :frequency)
    get("/summary", AnalyticsController, :summary)
    get("/correlation", AnalyticsController, :correlation)

    # Extended analytics (computed locally)
    get("/percentiles", AnalyticsController, :percentiles)
    get("/stddev", AnalyticsController, :stddev)

    # Time window aggregations
    get("/sliding-window", AnalyticsController, :sliding_window)
    get("/session-window", AnalyticsController, :session_window)

    # Cache management
    get("/cache/stats", AnalyticsController, :cache_stats)
    post("/cache/invalidate", AnalyticsController, :invalidate_cache)
  end

  # -------------------------------------------------------------------
  # Integrations Routes (Message Queues)
  # -------------------------------------------------------------------

  # Public status endpoints
  scope "/api/integrations", QueryServiceExWeb do
    pipe_through(:api)

    get("/", IntegrationsController, :all_status)
    get("/kafka", IntegrationsController, :kafka_status)
    get("/rabbitmq", IntegrationsController, :rabbitmq_status)
  end

  # Authenticated management endpoints
  scope "/api/integrations", QueryServiceExWeb do
    pipe_through(:authenticated)

    get("/kafka/config", IntegrationsController, :kafka_config)
    post("/kafka/publish", IntegrationsController, :kafka_publish)
    get("/rabbitmq/config", IntegrationsController, :rabbitmq_config)
    post("/rabbitmq/publish", IntegrationsController, :rabbitmq_publish)
  end

  # -------------------------------------------------------------------
  # Tenant Management Routes
  # -------------------------------------------------------------------

  scope "/api", QueryServiceExWeb do
    pipe_through(:authenticated)

    # Tenant info and settings
    get("/tenant", TenantController, :show)
    put("/tenant", TenantController, :update)

    # Usage statistics
    get("/tenant/usage", TenantController, :usage)
  end

  # -------------------------------------------------------------------
  # Onboarding Routes
  # -------------------------------------------------------------------

  scope "/api/onboarding", QueryServiceExWeb do
    pipe_through(:authenticated)

    # Onboarding status and progress
    get("/", OnboardingController, :show)
    get("/steps", OnboardingController, :steps)

    # Step completion
    post("/steps/:step/complete", OnboardingController, :complete_step)

    # Skip or reset onboarding
    post("/skip", OnboardingController, :skip)
    post("/reset", OnboardingController, :reset)
  end

  # -------------------------------------------------------------------
  # API Key Management Routes
  # -------------------------------------------------------------------

  scope "/api/api-keys", QueryServiceExWeb do
    pipe_through(:authenticated)

    # List available scopes
    get("/scopes", ApiKeyController, :scopes)

    # List all API keys for current tenant
    get("/", ApiKeyController, :index)

    # Create a new API key
    post("/", ApiKeyController, :create)

    # Get a specific API key
    get("/:id", ApiKeyController, :show)

    # Update API key metadata
    put("/:id", ApiKeyController, :update)

    # Rotate API key (revoke old, create new)
    post("/:id/rotate", ApiKeyController, :rotate)

    # Revoke API key
    delete("/:id", ApiKeyController, :delete)
  end

  # -------------------------------------------------------------------
  # Billing Routes
  # -------------------------------------------------------------------

  scope "/api/billing", QueryServiceExWeb do
    pipe_through(:authenticated)

    # LemonSqueezy checkout and portal
    get("/portal", BillingController, :portal)
    post("/checkout", BillingController, :checkout)

    # Hybrid pricing / overage billing
    get("/overage", BillingController, :overage)
    post("/overage/enable", BillingController, :enable_overage)
    post("/overage/disable", BillingController, :disable_overage)
    get("/projected-charges", BillingController, :projected_charges)
  end

  # LemonSqueezy webhooks (public, verified by signature)
  scope "/api/webhooks", QueryServiceExWeb do
    pipe_through(:api)

    post("/lemonsqueezy", WebhookController, :lemonsqueezy)
  end

  # -------------------------------------------------------------------
  # MCP-Compatible v1 API Routes
  # -------------------------------------------------------------------
  # The chronos-mcp Docker container expects /api/v1/ prefixed URLs.
  # These routes use the tenant_scoped pipeline which bypasses auth
  # in dev mode (AUTH_DISABLED=true) and injects the dev tenant.

  # Health check (no auth)
  scope "/api/v1", QueryServiceExWeb do
    pipe_through(:api)

    get("/health", HealthController, :show)
  end

  # Event read endpoints (tenant-scoped for data isolation)
  scope "/api/v1", QueryServiceExWeb do
    pipe_through([:tenant_scoped, :rate_limited])

    get("/events", EventController, :index)
    get("/events/recent", EventController, :recent)
    get("/events/:id", EventController, :show)
    get("/events/entity/:entity_id", EventController, :by_entity)
    get("/events/type/:event_type", EventController, :by_type)
    get("/streams", EventController, :streams)
    get("/event-types", EventController, :event_types)
  end

  # Snapshot endpoints (tenant-scoped)
  scope "/api/v1", QueryServiceExWeb do
    pipe_through([:tenant_scoped, :rate_limited])

    get("/snapshots", SnapshotController, :index)
  end

  # Projection endpoints (tenant-scoped)
  scope "/api/v1", QueryServiceExWeb do
    pipe_through([:tenant_scoped, :rate_limited])

    get("/projections", ProjectionController, :index)
    get("/projections/rebuild-stats", ProjectionController, :rebuild_stats)
    get("/projections/:name", ProjectionController, :show)
  end
end
