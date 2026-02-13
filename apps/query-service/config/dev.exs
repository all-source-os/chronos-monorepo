import Config

# Configure your database
config :query_service_ex, QueryServiceEx.Repo,
  username: "postgres",
  password: "postgres",
  hostname: "localhost",
  database: "query_service_dev",
  stacktrace: true,
  show_sensitive_data_on_connection_error: true,
  pool_size: 10

# For development, we enable code reloading
config :query_service_ex, QueryServiceExWeb.Endpoint,
  # Binding to loopback ipv4 address prevents access from other machines.
  # Change to `ip: {0, 0, 0, 0}` to allow access from other machines.
  http: [ip: {127, 0, 0, 1}, port: 3902],
  check_origin: false,
  code_reloader: true,
  debug_errors: true,
  secret_key_base: "development_secret_key_base_at_least_64_bytes_long_for_security_purposes",
  watchers: []

# Enable dev routes for debugging
config :query_service_ex, dev_routes: true

# Include metadata in development logs for debugging
config :logger, :console,
  format: "[$level] $metadata$message\n",
  metadata: [:request_id, :correlation_id, :entity_id, :event_type]

# Set a higher stacktrace during development
config :phoenix, :stacktrace_depth, 20

# Initialize plugs at runtime for faster development compilation
config :phoenix, :plug_init_mode, :runtime

# Google OAuth configuration for development
# Set these environment variables: GOOGLE_CLIENT_ID, GOOGLE_CLIENT_SECRET
config :ueberauth, Ueberauth.Strategy.Google.OAuth,
  client_id: System.get_env("GOOGLE_CLIENT_ID") || "dev_client_id",
  client_secret: System.get_env("GOOGLE_CLIENT_SECRET") || "dev_client_secret"

# GitHub OAuth configuration for development
# Set these environment variables: GITHUB_CLIENT_ID, GITHUB_CLIENT_SECRET
config :ueberauth, Ueberauth.Strategy.Github.OAuth,
  client_id: System.get_env("GITHUB_CLIENT_ID") || "dev_github_client_id",
  client_secret: System.get_env("GITHUB_CLIENT_SECRET") || "dev_github_client_secret"

# LemonSqueezy configuration for development (optional)
# Set LEMON_SQUEEZY_VARIANT_TIERS="variant_id:tier,..." to map variants to tiers
config :query_service_ex, :lemon_squeezy,
  api_key: System.get_env("LEMON_SQUEEZY_API_KEY"),
  store_id: System.get_env("LEMON_SQUEEZY_STORE_ID"),
  webhook_secret: System.get_env("LEMON_SQUEEZY_WEBHOOK_SECRET"),
  variant_tiers: %{}

# Guardian secret key for development
config :query_service_ex, QueryServiceEx.Accounts.Guardian,
  secret_key: "dev_guardian_secret_key_at_least_64_bytes_long_for_development_purposes_only"
