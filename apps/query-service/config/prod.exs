import Config

# For production, don't forget to configure the url host
# to something meaningful, Phoenix uses this information
# when generating URLs.

config :query_service_ex, QueryServiceExWeb.Endpoint,
  url: [host: System.get_env("PHX_HOST") || "localhost", port: 443, scheme: "https"],
  http: [
    # Enable IPv6 and bind on all interfaces.
    # Set it to  {0, 0, 0, 0, 0, 0, 0, 1} for local network only access.
    ip: {0, 0, 0, 0, 0, 0, 0, 0},
    port: String.to_integer(System.get_env("PORT") || "3902")
  ],
  secret_key_base: System.fetch_env!("SECRET_KEY_BASE"),
  server: true

# Configure Rust Core backend
config :query_service_ex,
  rust_core_url: System.get_env("RUST_CORE_URL") || "http://localhost:3900"

# Do not print debug messages in production
config :logger, level: :info
