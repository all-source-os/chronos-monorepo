import Config

# Configure Ecto Repo
config :query_service_ex,
  ecto_repos: [QueryServiceEx.Repo]

# Configure structured logging defaults
config :logger,
  backends: [:console]

config :logger, :console,
  metadata: [:request_id, :correlation_id, :entity_id, :event_type, :module, :function]

# Configure the Phoenix endpoint
config :query_service_ex, QueryServiceExWeb.Endpoint,
  url: [host: "localhost"],
  adapter: Bandit.PhoenixAdapter,
  render_errors: [
    formats: [json: QueryServiceExWeb.ErrorJSON],
    layout: false
  ],
  pubsub_server: QueryServiceEx.PubSub,
  live_view: [signing_salt: "query_service_secret"],
  http: [port: 3902],
  server: true

# Configure Rust Core backend URLs
config :query_service_ex,
  rust_core_url: System.get_env("RUST_CORE_URL") || "http://localhost:3900",
  core_ws_url: System.get_env("CORE_WS_URL") || "ws://localhost:3900"

# Configure Tesla HTTP client
config :tesla,
  disable_deprecated_builder_warning: true

# Configure JSON encoding
config :phoenix, :json_library, Jason

# Configure Ueberauth for OAuth
config :ueberauth, Ueberauth,
  providers: [
    google: {Ueberauth.Strategy.Google, [default_scope: "email profile"]},
    github: {Ueberauth.Strategy.Github, [default_scope: "user:email"]}
  ]

# Configure Guardian for JWT authentication
config :query_service_ex, QueryServiceEx.Accounts.Guardian,
  issuer: "query_service_ex",
  # Default TTL: 1 hour for access tokens
  ttl: {1, :hour},
  allowed_algos: ["HS512"],
  verify_issuer: true

# Configure rate limiting (requests per second)
config :query_service_ex, QueryServiceEx.RateLimiter,
  default_rate: 100,
  default_burst: 200,
  cleanup_interval: 60_000,
  tier_limits: %{
    free: %{rate: 10, burst: 20},
    starter: %{rate: 50, burst: 100},
    pro: %{rate: 200, burst: 400},
    enterprise: %{rate: 1000, burst: 2000}
  }

# Import environment specific config
import_config "#{config_env()}.exs"
