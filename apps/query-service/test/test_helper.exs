# Query Service Test Helper
#
# Tests use testcontainers to automatically start a PostgreSQL database.
# No external database is required - testcontainers handles everything.
#
# Integration tests (tagged with :integration) require Core to be running.
# Run with: mix test --only integration
# Run all: mix test --include integration

require Logger

# Start :inets for integration tests that use httpc
Application.ensure_all_started(:inets)

Logger.info("Starting testcontainers PostgreSQL...")

# Configure testcontainers - disable Ryuk for CI environments
# Ryuk is a cleanup container that can fail in Docker-in-Docker or restricted environments
# Setting this ensures tests work in GitHub Actions and local development
Application.put_env(:testcontainers, :ryuk_disabled, true)

# Start testcontainers supervisor
{:ok, _} = Testcontainers.start_link()

# Start PostgreSQL container
{:ok, postgres} =
  Testcontainers.PostgresContainer.new()
  |> Testcontainers.PostgresContainer.with_database("query_service_test")
  |> Testcontainers.PostgresContainer.with_user("postgres")
  |> Testcontainers.PostgresContainer.with_password("postgres")
  |> Testcontainers.start_container()

# Get the mapped port for the container (host is always localhost for testcontainers)
port = Testcontainers.Container.mapped_port(postgres, 5432)
host = "localhost"
database_url = "ecto://postgres:postgres@#{host}:#{port}/query_service_test"

Logger.info("PostgreSQL testcontainer started at #{host}:#{port}")

# Set environment variable
System.put_env("DATABASE_URL", database_url)

# Configure the Repo with testcontainer connection
Application.put_env(:query_service_ex, QueryServiceEx.Repo,
  url: database_url,
  pool: Ecto.Adapters.SQL.Sandbox,
  pool_size: System.schedulers_online() * 2
)

# Start the Repo
{:ok, _repo_pid} = QueryServiceEx.Repo.start_link()

# Run migrations
migrations_path = Path.join([:code.priv_dir(:query_service_ex), "repo", "migrations"])
Ecto.Migrator.run(QueryServiceEx.Repo, migrations_path, :up, all: true, log: false)

Logger.info("Database migrations complete")

# Configure ExUnit with better formatting and failure tracking
ExUnit.configure(
  # Store failed tests for re-running with mix test --failed
  trace: false,
  capture_log: true,
  # Increase timeout for slower CI environments
  timeout: 120_000,
  # Exclude integration tests by default (require Core to be running)
  # Run with: mix test --include integration
  exclude: [:integration]
)

ExUnit.start()

Ecto.Adapters.SQL.Sandbox.mode(QueryServiceEx.Repo, :manual)
