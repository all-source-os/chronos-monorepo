defmodule QueryServiceEx.Application do
  # See https://hexdocs.pm/elixir/Application.html
  # for more information on OTP Applications
  @moduledoc false

  use Application

  alias QueryServiceEx.Application.Services.ProjectionSync

  @impl true
  def start(_type, _args) do
    # Attach telemetry handlers for structured logging
    QueryServiceEx.Telemetry.attach_handlers()

    # Initialize ETS cache for projections
    ProjectionSync.init_cache()

    children = [
      # Database repository
      QueryServiceEx.Repo,

      # PubSub for event broadcasting
      {Phoenix.PubSub, name: QueryServiceEx.PubSub},

      # Rate limiter for per-tenant request throttling
      QueryServiceEx.RateLimiter,

      # Circuit breaker for Core backend calls
      {QueryServiceEx.CircuitBreaker, name: QueryServiceEx.CircuitBreaker},

      # Registry for projection sync processes
      {Registry, keys: :unique, name: QueryServiceEx.ProjectionRegistry},

      # DynamicSupervisor for projection sync processes
      {DynamicSupervisor, strategy: :one_for_one, name: QueryServiceEx.ProjectionSyncSupervisor},

      # WebSocket client for real-time events from Core
      {QueryServiceEx.Infrastructure.Adapters.CoreWebSocketClient, []},

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
end
