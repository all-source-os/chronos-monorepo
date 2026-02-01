# Testcontainers Integration for Query Service Tests
#
# This test helper supports two modes:
#
# 1. Testcontainers mode (recommended):
#    Run with: mix testcontainers.test --database postgres
#    Or: mix testcontainers.test --database postgres --db-volume my_test_volume (for caching)
#    The DATABASE_URL will be set automatically by testcontainers.
#
# 2. Manual mode (for CI or local PostgreSQL):
#    Set DATABASE_URL environment variable and run: mix test
#
# The test configuration automatically picks up DATABASE_URL from the environment
# via config/runtime.exs

# Check if testcontainers is starting the container for us
testcontainers_mode = System.get_env("DATABASE_URL") != nil

if testcontainers_mode do
  # Testcontainers or external DATABASE_URL mode
  # The application will be started with the correct DATABASE_URL from runtime.exs

  # Start ExUnit with database tests included
  ExUnit.start()

  # Ensure the application is started (Repo will use DATABASE_URL from runtime.exs)
  {:ok, _} = Application.ensure_all_started(:query_service_ex)

  # Run migrations
  migrations_path = Path.join([File.cwd!(), "priv", "repo", "migrations"])

  if File.dir?(migrations_path) do
    Ecto.Migrator.run(QueryServiceEx.Repo, migrations_path, :up, all: true, log: :info)
  end

  # Configure sandbox mode for test isolation
  Ecto.Adapters.SQL.Sandbox.mode(QueryServiceEx.Repo, :manual)
else
  # No DATABASE_URL set - start testcontainers ourselves
  # This allows running `mix test` directly without the testcontainers.test task
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

  # Set the DATABASE_URL environment variable
  System.put_env("DATABASE_URL", database_url)

  # Store container info in application env for cleanup
  Application.put_env(:query_service_ex, :testcontainer_postgres, %{
    container: postgres_container,
    port: postgres_port,
    url: database_url
  })

  # Configure the Repo to use the testcontainer
  Application.put_env(:query_service_ex, QueryServiceEx.Repo,
    url: database_url,
    pool: Ecto.Adapters.SQL.Sandbox,
    pool_size: System.schedulers_online() * 2
  )

  # Start ExUnit with database tests included
  ExUnit.start()

  # Start the application (which will start the Repo with our configuration)
  {:ok, _} = Application.ensure_all_started(:query_service_ex)

  # Run migrations
  migrations_path = Path.join([File.cwd!(), "priv", "repo", "migrations"])

  if File.dir?(migrations_path) do
    Ecto.Migrator.run(QueryServiceEx.Repo, migrations_path, :up, all: true, log: :info)
  end

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
end
