import Config

# Runtime configuration for test environment with testcontainers
# When DATABASE_URL is set (by testcontainers), use it
if config_env() == :test do
  if database_url = System.get_env("DATABASE_URL") do
    config :query_service_ex, QueryServiceEx.Repo,
      url: database_url,
      pool: Ecto.Adapters.SQL.Sandbox,
      pool_size: System.schedulers_online() * 2
  end
end

# Runtime configuration for production
# All environment variables are optional at startup - the app will fail gracefully
# when trying to use unconfigured services (database, OAuth, etc.)
if config_env() == :prod do
  # Database configuration - optional, will fail on first query if not set
  if database_url = System.get_env("DATABASE_URL") do
    maybe_ipv6 = if System.get_env("ECTO_IPV6") in ~w(true 1), do: [:inet6], else: []

    config :query_service_ex, QueryServiceEx.Repo,
      url: database_url,
      pool_size: String.to_integer(System.get_env("POOL_SIZE") || "10"),
      socket_options: maybe_ipv6
  end

  # Secret key base - use provided value or generate a temporary one for health checks
  secret_key_base =
    System.get_env("SECRET_KEY_BASE") ||
      Base.encode64(:crypto.strong_rand_bytes(64))

  host = System.get_env("PHX_HOST") || "localhost"
  port = String.to_integer(System.get_env("PORT") || "3902")

  config :query_service_ex, QueryServiceExWeb.Endpoint,
    url: [host: host, port: 443, scheme: "https"],
    http: [
      # Enable IPv6 and bind on all interfaces.
      ip: {0, 0, 0, 0, 0, 0, 0, 0},
      port: port
    ],
    secret_key_base: secret_key_base

  # Configure Rust Core backend URLs and authentication
  config :query_service_ex,
    rust_core_url: System.get_env("RUST_CORE_URL") || "http://localhost:3900",
    core_ws_url: System.get_env("CORE_WS_URL") || "ws://localhost:3900",
    core_api_key: System.get_env("CORE_API_KEY")

  # Google OAuth configuration - optional, OAuth will fail if not configured
  if google_client_id = System.get_env("GOOGLE_CLIENT_ID") do
    config :ueberauth, Ueberauth.Strategy.Google.OAuth,
      client_id: google_client_id,
      client_secret: System.get_env("GOOGLE_CLIENT_SECRET")
  end

  # GitHub OAuth configuration - optional, OAuth will fail if not configured
  if github_client_id = System.get_env("GITHUB_CLIENT_ID") do
    config :ueberauth, Ueberauth.Strategy.Github.OAuth,
      client_id: github_client_id,
      client_secret: System.get_env("GITHUB_CLIENT_SECRET")
  end

  # LemonSqueezy configuration for billing - optional
  config :query_service_ex, :lemon_squeezy,
    api_key: System.get_env("LEMON_SQUEEZY_API_KEY"),
    store_id: System.get_env("LEMON_SQUEEZY_STORE_ID"),
    webhook_secret: System.get_env("LEMON_SQUEEZY_WEBHOOK_SECRET")

  # Guardian secret key for production
  config :query_service_ex, QueryServiceEx.Accounts.Guardian, secret_key: secret_key_base
end
