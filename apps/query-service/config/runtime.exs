import Config

# Runtime configuration for production
if config_env() == :prod do
  database_url =
    System.get_env("DATABASE_URL") ||
      raise """
      environment variable DATABASE_URL is missing.
      For example: ecto://USER:PASS@HOST/DATABASE
      """

  maybe_ipv6 = if System.get_env("ECTO_IPV6") in ~w(true 1), do: [:inet6], else: []

  config :query_service_ex, QueryServiceEx.Repo,
    url: database_url,
    pool_size: String.to_integer(System.get_env("POOL_SIZE") || "10"),
    socket_options: maybe_ipv6

  # The secret key base is used to sign/encrypt cookies and other secrets.
  # A default value is used in config/dev.exs and config/test.exs but you
  # want to use a different value for prod and you most likely don't want
  # to check this value into version control, so we use an environment
  # variable instead.
  secret_key_base =
    System.get_env("SECRET_KEY_BASE") ||
      raise """
      environment variable SECRET_KEY_BASE is missing.
      You can generate one by calling: mix phx.gen.secret
      """

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

  # Configure Rust Core backend URLs
  config :query_service_ex,
    rust_core_url: System.get_env("RUST_CORE_URL") || "http://localhost:3900",
    core_ws_url: System.get_env("CORE_WS_URL") || "ws://localhost:3900"

  # Google OAuth configuration for production
  google_client_id =
    System.get_env("GOOGLE_CLIENT_ID") ||
      raise """
      environment variable GOOGLE_CLIENT_ID is missing.
      Create OAuth credentials at https://console.cloud.google.com/apis/credentials
      """

  google_client_secret =
    System.get_env("GOOGLE_CLIENT_SECRET") ||
      raise """
      environment variable GOOGLE_CLIENT_SECRET is missing.
      Create OAuth credentials at https://console.cloud.google.com/apis/credentials
      """

  config :ueberauth, Ueberauth.Strategy.Google.OAuth,
    client_id: google_client_id,
    client_secret: google_client_secret

  # GitHub OAuth configuration for production
  github_client_id =
    System.get_env("GITHUB_CLIENT_ID") ||
      raise """
      environment variable GITHUB_CLIENT_ID is missing.
      Create OAuth App at https://github.com/settings/developers
      """

  github_client_secret =
    System.get_env("GITHUB_CLIENT_SECRET") ||
      raise """
      environment variable GITHUB_CLIENT_SECRET is missing.
      Create OAuth App at https://github.com/settings/developers
      """

  config :ueberauth, Ueberauth.Strategy.Github.OAuth,
    client_id: github_client_id,
    client_secret: github_client_secret

  # LemonSqueezy configuration for billing
  config :query_service_ex, :lemon_squeezy,
    api_key: System.get_env("LEMON_SQUEEZY_API_KEY"),
    store_id: System.get_env("LEMON_SQUEEZY_STORE_ID"),
    webhook_secret: System.get_env("LEMON_SQUEEZY_WEBHOOK_SECRET")

  # Guardian secret key for production (uses the same SECRET_KEY_BASE)
  config :query_service_ex, QueryServiceEx.Accounts.Guardian, secret_key: secret_key_base
end
