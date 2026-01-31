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

  # -------------------------------------------------------------------
  # Public Routes
  # -------------------------------------------------------------------

  # Health checks (no auth required)
  scope "/api", QueryServiceExWeb do
    pipe_through(:api)

    get("/health", HealthController, :show)
    get("/health/live", HealthController, :live)
    get("/health/ready", HealthController, :ready)

    # Metrics endpoint
    get("/metrics", MetricsController, :show)
  end

  # OAuth routes (public, handles provider redirects and callbacks)
  scope "/api/auth", QueryServiceExWeb do
    pipe_through(:api)

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
    pipe_through([:tenant_scoped, :events_quota])

    post("/events", EventController, :create)
    post("/events/batch", EventController, :create_batch)
  end

  # Events read endpoints (no quota for reads)
  scope "/api", QueryServiceExWeb do
    pipe_through(:tenant_scoped)

    get("/events", EventController, :index)
    get("/events/:id", EventController, :show)
    get("/events/entity/:entity_id", EventController, :by_entity)
    get("/events/type/:event_type", EventController, :by_type)
  end

  # Query execution (with usage quota enforcement)
  scope "/api", QueryServiceExWeb do
    pipe_through([:tenant_scoped, :queries_quota])

    post("/query", QueryController, :execute)
  end

  # Projections endpoints (no quota for projection operations)
  scope "/api", QueryServiceExWeb do
    pipe_through(:tenant_scoped)

    get("/projections", ProjectionController, :index)
    get("/projections/:name", ProjectionController, :show)
    post("/projections", ProjectionController, :create)
    delete("/projections/:name", ProjectionController, :delete)
    get("/projections/:name/state", ProjectionController, :get_state)
    post("/projections/:name/reset", ProjectionController, :reset)
  end

  # Schema endpoints
  scope "/api", QueryServiceExWeb do
    pipe_through(:tenant_scoped)

    get("/schemas", SchemaController, :index)
    get("/schemas/:event_type", SchemaController, :show)
    post("/schemas", SchemaController, :register)
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
  # Billing Routes
  # -------------------------------------------------------------------

  scope "/api/billing", QueryServiceExWeb do
    pipe_through(:authenticated)

    # LemonSqueezy checkout and portal
    get("/portal", BillingController, :portal)
    post("/checkout", BillingController, :checkout)
  end

  # LemonSqueezy webhooks (public, verified by signature)
  scope "/api/webhooks", QueryServiceExWeb do
    pipe_through(:api)

    post("/lemonsqueezy", WebhookController, :lemonsqueezy)
  end
end
