import Config

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

# Configure Rust Core backend URL
config :query_service_ex,
  rust_core_url: System.get_env("RUST_CORE_URL") || "http://localhost:3900"

# Configure Tesla HTTP client
config :tesla,
  disable_deprecated_builder_warning: true

# Configure JSON encoding
config :phoenix, :json_library, Jason

# Import environment specific config
import_config "#{config_env()}.exs"
