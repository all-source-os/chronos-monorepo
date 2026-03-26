import Config

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

# #10 fix: allow localhost origins for dev so direct QS calls work without
# CORS errors (e.g., when bypassing the Next.js proxy during development).
config :query_service_ex,
  dev_routes: true,
  cors_origins: ["http://localhost:3000", "http://localhost:3001", "http://localhost:3902"]

# Include metadata in development logs for debugging
config :logger, :console,
  format: "[$level] $metadata$message\n",
  metadata: [:request_id, :correlation_id, :entity_id, :event_type]

# Set a higher stacktrace during development
config :phoenix, :stacktrace_depth, 20

# Initialize plugs at runtime for faster development compilation
config :phoenix, :plug_init_mode, :runtime
