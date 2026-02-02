import Config

# Development-specific configuration
config :logger, level: :debug

config :logger, :console,
  format: "$time $metadata[$level] $message\n",
  metadata: [:request_id]
