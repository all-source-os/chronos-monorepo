import Config

# Skip automatic Repo startup - it will be started manually by test_helper.exs
# after testcontainers sets up the PostgreSQL database.
config :query_service_ex, skip_repo_start: true

# Configure your database for test
# Note: These settings are placeholder values. The actual connection details
# will be set by test_helper.exs after testcontainers starts PostgreSQL.
config :query_service_ex, QueryServiceEx.Repo,
  username: "postgres",
  password: "postgres",
  hostname: "localhost",
  database: "query_service_test#{System.get_env("MIX_TEST_PARTITION")}",
  pool: Ecto.Adapters.SQL.Sandbox,
  pool_size: System.schedulers_online() * 2

# We don't run a server during test
config :query_service_ex, QueryServiceExWeb.Endpoint,
  http: [ip: {127, 0, 0, 1}, port: 4002],
  secret_key_base: "test_secret_key_base_at_least_64_bytes_long_for_security_testing_purposes",
  server: false

# Disable WebSocket client during tests (Core may not be running)
config :query_service_ex, core_ws_enabled: false

# Disable EventPipeline during tests (Core may not be running)
config :query_service_ex, event_pipeline_enabled: false

# Print only warnings and errors during test
config :logger, level: :warning

# Initialize plugs at runtime for faster test compilation
config :phoenix, :plug_init_mode, :runtime

# Google OAuth configuration for testing
config :ueberauth, Ueberauth.Strategy.Google.OAuth,
  client_id: "test_client_id",
  client_secret: "test_client_secret"

# GitHub OAuth configuration for testing
config :ueberauth, Ueberauth.Strategy.Github.OAuth,
  client_id: "test_github_client_id",
  client_secret: "test_github_client_secret"

# Guardian secret key for testing
config :query_service_ex, QueryServiceEx.Accounts.Guardian,
  secret_key: "test_guardian_secret_key_at_least_64_bytes_long_for_testing_purposes_only"

# LemonSqueezy configuration for testing
config :query_service_ex, :lemon_squeezy,
  api_key: nil,
  store_id: "test_store_123",
  webhook_secret: "test_webhook_secret_for_testing",
  variant_tiers: %{
    "variant_starter" => :starter,
    "variant_pro" => :pro,
    "variant_enterprise" => :enterprise
  }
