import Config

# Print only warnings and errors during test
config :logger, level: :warning

# Use simple console format for tests
config :logger, :console,
  format: "[$level] $message\n",
  metadata: []
