defmodule QueryServiceEx.Application do
  # See https://hexdocs.pm/elixir/Application.html
  # for more information on OTP Applications
  @moduledoc false

  use Application

  alias QueryServiceEx.Application.Services.ProjectionSync
  alias QueryServiceEx.Cluster.Topology
  alias QueryServiceEx.Integrations.Kafka
  alias QueryServiceEx.Integrations.RabbitMQ

  @impl true
  def start(_type, _args) do
    # Log warning if dev mode is enabled
    QueryServiceEx.DevMode.maybe_log_warning()

    # Attach telemetry handlers for structured logging
    QueryServiceEx.Telemetry.attach_handlers()

    # Initialize ETS cache for projections
    ProjectionSync.init_cache()

    # Skip Repo start if:
    # 1. Explicitly disabled via config (test mode with testcontainers)
    # 2. In prod without DATABASE_URL (container health checks, image testing)
    is_prod? = Application.get_env(:query_service_ex, :env) == :prod
    no_database_url? = is_nil(System.get_env("DATABASE_URL"))

    skip_repo? =
      Application.get_env(:query_service_ex, :skip_repo_start, false) ||
        (is_prod? and no_database_url?)

    repo_children = if skip_repo?, do: [], else: [QueryServiceEx.Repo]

    # Get cluster topology child_spec (nil if clustering disabled)
    cluster_children = cluster_children()

    children =
      repo_children ++
        [
          # PubSub for event broadcasting (must start before cluster components)
          {Phoenix.PubSub, name: QueryServiceEx.PubSub},

          # Prometheus metrics exporter (must start before other services emit telemetry)
          QueryServiceEx.PrometheusMetrics,

          # Rate limiter for per-tenant request throttling
          QueryServiceEx.RateLimiter,

          # Circuit breaker for Core backend calls
          {QueryServiceEx.CircuitBreaker, name: QueryServiceEx.CircuitBreaker},

          # ETS cache for analytics results
          QueryServiceEx.Infrastructure.Adapters.AnalyticsCache,

          # Health checker for Core read nodes (polls /health, tracks lag in ETS)
          QueryServiceEx.Infrastructure.Adapters.CoreHealthChecker
        ] ++
        cluster_children ++
        integration_children() ++
        [
          # Registry for projection sync processes (local fallback when clustering disabled)
          {Registry, keys: :unique, name: QueryServiceEx.ProjectionRegistry},

          # DynamicSupervisor for projection sync processes (local fallback)
          {DynamicSupervisor,
           strategy: :one_for_one, name: QueryServiceEx.ProjectionSyncSupervisor},

          # WebSocket client for real-time events from Core
          {QueryServiceEx.Infrastructure.Adapters.CoreWebSocketClient, []},

          # Presence for tracking connected WebSocket clients
          QueryServiceExWeb.Presence,

          # Start the Phoenix endpoint
          QueryServiceExWeb.Endpoint
        ]

    # See https://hexdocs.pm/elixir/Supervisor.html
    # for other strategies and supported options
    opts = [strategy: :one_for_one, name: QueryServiceEx.Supervisor]
    Supervisor.start_link(children, opts)
  end

  # Tell Phoenix to update the endpoint configuration
  # whenever the application is updated.
  @impl true
  def config_change(changed, _new, removed) do
    QueryServiceExWeb.Endpoint.config_change(changed, removed)
    :ok
  end

  # Returns message queue integration children (Kafka, RabbitMQ)
  # Only started when explicitly enabled via config or env vars
  defp integration_children do
    children = []

    # Add Kafka producer if enabled
    children =
      if Kafka.Producer.enabled?() do
        [Kafka.Producer | children]
      else
        children
      end

    # Add Kafka consumer if enabled
    children =
      if Kafka.Consumer.enabled?() do
        [Kafka.Consumer | children]
      else
        children
      end

    # Add RabbitMQ producer if enabled
    children =
      if RabbitMQ.Producer.enabled?() do
        [RabbitMQ.Producer | children]
      else
        children
      end

    # Add RabbitMQ consumer if enabled
    if RabbitMQ.Consumer.enabled?() do
      [RabbitMQ.Consumer | children]
    else
      children
    end
  end

  # Returns cluster-related children based on CLUSTER_STRATEGY env var
  defp cluster_children do
    cluster_topology = Topology.child_spec()

    base_cluster = [
      # Consistent hash ring for entity-to-node mapping
      QueryServiceEx.Cluster.ConsistentHash,

      # Distributed registry for projection processes
      QueryServiceEx.Cluster.DistributedRegistry,

      # Distributed supervisor for projection processes
      QueryServiceEx.Cluster.DistributedSupervisor,

      # Cluster membership manager
      QueryServiceEx.Cluster.Membership
    ]

    if cluster_topology do
      # Add libcluster supervisor when clustering is enabled
      [cluster_topology | base_cluster]
    else
      # Just start the cluster components (they work in single-node mode too)
      base_cluster
    end
  end
end
