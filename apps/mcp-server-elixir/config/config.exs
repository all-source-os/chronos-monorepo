import Config

# Core API URL (Rust)
config :mcp_server_elixir,
  core_url: System.get_env("ALLSOURCE_CORE_URL", "http://localhost:3900"),
  control_url: System.get_env("ALLSOURCE_CONTROL_URL", "http://localhost:3901")

# Logger configuration
config :logger,
  level: :info,
  format: "$time $metadata[$level] $message\n",
  metadata: [:request_id]

