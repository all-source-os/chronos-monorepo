# Query Service Test Helper
#
# No external database required - Query Service is stateless.
# Integration tests (tagged with :integration) require Core to be running.
# Run with: mix test --only integration
# Run all: mix test --include integration

require Logger

# Start :inets for integration tests that use httpc
Application.ensure_all_started(:inets)

# Configure ExUnit
ExUnit.configure(
  trace: false,
  capture_log: true,
  timeout: 120_000,
  exclude: [:integration]
)

ExUnit.start()
