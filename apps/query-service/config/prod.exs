import Config

# Mark this as production environment (used by application.ex to skip repo if no DATABASE_URL)
config :query_service_ex, env: :prod

# For production, don't forget to configure the url host
# to something meaningful, Phoenix uses this information
# when generating URLs.
# Note: secret_key_base is set in runtime.exs, using a build-time placeholder here
config :query_service_ex, QueryServiceExWeb.Endpoint,
  url: [host: "localhost", port: 443, scheme: "https"],
  http: [
    # Enable IPv6 and bind on all interfaces.
    ip: {0, 0, 0, 0, 0, 0, 0, 0},
    port: 3902
  ],
  secret_key_base: "build_time_placeholder_will_be_replaced_at_runtime_by_runtime_exs_config",
  server: true

# Configure Rust Core backend (defaults, can be overridden in runtime.exs)
config :query_service_ex,
  rust_core_url: "http://localhost:3900",
  core_ws_url: "ws://localhost:3900/api/v1/events/stream"

# Do not print debug messages in production
config :logger, level: :info

# Configure structured JSON logging for production
config :logger, :console,
  format: {LoggerJSON.Formatters.Basic, :format},
  metadata: :all

config :logger_json, :backend,
  metadata: :all,
  json_encoder: Jason,
  formatter: LoggerJSON.Formatters.Basic

# Database configuration is handled in runtime.exs for production
# using the DATABASE_URL environment variable
