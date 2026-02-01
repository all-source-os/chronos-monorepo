# Query Service Test Helper
#
# Tests use testcontainers to automatically start a PostgreSQL database.
# No external database is required - testcontainers handles everything.

require Logger

Logger.info("Starting testcontainers PostgreSQL...")

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

ExUnit.start()

Ecto.Adapters.SQL.Sandbox.mode(QueryServiceEx.Repo, :manual)
