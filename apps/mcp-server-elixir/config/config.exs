import Config

# Compile-time DEFAULTS only. Every env-var-driven value (core_url, control_url,
# read_only, core_ws_url, core_api_key) is read in `config/runtime.exs` so a
# `mix release` picks it up at BOOT instead of freezing the build machine's
# environment into the release (#228).
config :mcp_server_elixir,
  core_url: "http://localhost:3900",
  control_url: "http://localhost:3901",
  read_only: false,
  # WebSocket client configuration
  core_ws_url: "ws://localhost:3900/api/v1/events/stream",
  core_ws_enabled: true,
  core_ws_max_reconnect_attempts: 10,
  core_ws_initial_backoff_ms: 1_000,
  core_ws_max_backoff_ms: 30_000,
  core_api_key: nil

# Logger configuration
config :logger, level: :info

config :logger, :console,
  format: "$time $metadata[$level] $message\n",
  metadata: [
    :request_id,
    # WebSocket metadata
    :url,
    :error,
    :reason,
    :reconnect_attempts,
    :total_reconnects,
    :last_error
  ]

# Tesla configuration (suppress deprecated builder warning)
config :tesla, disable_deprecated_builder_warning: true

# Import environment specific config
import_config "#{config_env()}.exs"
