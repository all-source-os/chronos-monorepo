import Config

# Core API URL (Rust)
config :mcp_server_elixir,
  core_url: System.get_env("ALLSOURCE_CORE_URL", "http://localhost:3900"),
  control_url: System.get_env("ALLSOURCE_CONTROL_URL", "http://localhost:3901"),
  # WebSocket client configuration
  core_ws_url: System.get_env("ALLSOURCE_CORE_WS_URL", "ws://localhost:3900/api/v1/events/stream"),
  core_ws_enabled: true,
  core_ws_max_reconnect_attempts: 10,
  core_ws_initial_backoff_ms: 1_000,
  core_ws_max_backoff_ms: 30_000,
  core_api_key: System.get_env("ALLSOURCE_CORE_API_KEY")

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
