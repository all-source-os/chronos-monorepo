import Config

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
