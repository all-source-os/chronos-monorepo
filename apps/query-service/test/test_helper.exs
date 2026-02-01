# Start Testcontainers for database tests
{:ok, _} = Testcontainers.start_link()

# Configure and start the PostgreSQL container
postgres_config =
  Testcontainers.PostgresContainer.new()
  |> Testcontainers.PostgresContainer.with_image("postgres:15")
  |> Testcontainers.PostgresContainer.with_user("postgres")
  |> Testcontainers.PostgresContainer.with_password("postgres")
  |> Testcontainers.PostgresContainer.with_database("query_service_test")

{:ok, postgres_container} = Testcontainers.start_container(postgres_config)

# Get the connection details from the container
postgres_port = Testcontainers.PostgresContainer.port(postgres_container)

# Build the DATABASE_URL for the container
database_url = "ecto://postgres:postgres@localhost:#{postgres_port}/query_service_test"

# Set the DATABASE_URL environment variable for other processes
System.put_env("DATABASE_URL", database_url)

# Store container info in application env for cleanup and access
Application.put_env(:query_service_ex, :testcontainer_postgres, %{
  container: postgres_container,
  port: postgres_port,
  url: database_url
})

# Configure the Repo to use the testcontainer
# This configuration will be used when the Application starts the Repo
Application.put_env(:query_service_ex, QueryServiceEx.Repo,
  url: database_url,
  pool: Ecto.Adapters.SQL.Sandbox,
  pool_size: System.schedulers_online() * 2
)

# Temporarily start the Repo to run migrations (before the Application starts)
{:ok, repo_pid} = QueryServiceEx.Repo.start_link()

# Run migrations
migrations_path = Path.join([File.cwd!(), "priv", "repo", "migrations"])

if File.dir?(migrations_path) do
  Ecto.Migrator.run(QueryServiceEx.Repo, migrations_path, :up, all: true, log: :info)
end

# Stop the temporary Repo so the Application can start it fresh
GenServer.stop(repo_pid)

# Start ExUnit - database tests are now included by default since we have testcontainers
ExUnit.start()

# Start the application (which will start the Repo with our configuration)
{:ok, _} = Application.ensure_all_started(:query_service_ex)

# Configure sandbox mode for test isolation
Ecto.Adapters.SQL.Sandbox.mode(QueryServiceEx.Repo, :manual)

# Register cleanup callback to stop the container when tests finish
ExUnit.after_suite(fn _results ->
  case Application.get_env(:query_service_ex, :testcontainer_postgres) do
    %{container: container} ->
      Testcontainers.stop_container(container.container_id)
    _ ->
      :ok
  end
end)
