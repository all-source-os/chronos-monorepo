# Testcontainers Integration for Query Service Tests
#
# Usage:
#
# 1. Using mix testcontainers.test (recommended):
#    mix testcontainers.test --database postgres
#    Or with cached volume (faster subsequent runs):
#    mix testcontainers.test --database postgres --db-volume query_service_test_vol
#
# 2. Using the convenience aliases:
#    mix test.container                   # Runs tests with fresh PostgreSQL container
#    mix test.container.cached            # Runs tests with cached PostgreSQL container
#
# 3. Using an external database (CI environments):
#    DATABASE_URL=ecto://user:pass@host:port/db mix test
#
# How it works:
# - The testcontainers.test task starts a PostgreSQL container before loading the app
# - It sets DATABASE_URL which is picked up by config/runtime.exs
# - This ensures the Repo connects to the container instead of localhost:5432

# Start ExUnit
ExUnit.start()

# Check if the application is already started (by testcontainers.test task or mix test)
app_started? =
  Application.started_applications()
  |> Enum.any?(fn {app, _, _} -> app == :query_service_ex end)

# Check if DATABASE_URL is set (by testcontainers.test or externally)
database_url = System.get_env("DATABASE_URL")

if app_started? do
  # Application already started (mix testcontainers.test mode)
  # Run migrations and configure sandbox
  migrations_path = Path.join([File.cwd!(), "priv", "repo", "migrations"])

  if File.dir?(migrations_path) do
    Ecto.Migrator.run(QueryServiceEx.Repo, migrations_path, :up, all: true, log: :info)
  end

  Ecto.Adapters.SQL.Sandbox.mode(QueryServiceEx.Repo, :manual)
else
  if database_url do
    # DATABASE_URL set externally but app not started - start it
    {:ok, _} = Application.ensure_all_started(:query_service_ex)

    migrations_path = Path.join([File.cwd!(), "priv", "repo", "migrations"])

    if File.dir?(migrations_path) do
      Ecto.Migrator.run(QueryServiceEx.Repo, migrations_path, :up, all: true, log: :info)
    end

    Ecto.Adapters.SQL.Sandbox.mode(QueryServiceEx.Repo, :manual)
  else
    # No DATABASE_URL - start testcontainers ourselves
    # Note: This path is not recommended as the app may try to start with wrong config
    # Prefer using: mix testcontainers.test --database postgres
    IO.puts("""

    ===============================================================================
    WARNING: Running without DATABASE_URL.

    For reliable database testing, use one of these commands:
      mix test.container                         # Fresh PostgreSQL container
      mix test.container.cached                  # Cached PostgreSQL container
      mix testcontainers.test --database postgres  # Direct testcontainers command

    Attempting to start testcontainers manually...
    ===============================================================================

    """)

    {:ok, _} = Testcontainers.start_link()

    postgres_config =
      Testcontainers.PostgresContainer.new()
      |> Testcontainers.PostgresContainer.with_image("postgres:15")
      |> Testcontainers.PostgresContainer.with_user("postgres")
      |> Testcontainers.PostgresContainer.with_password("postgres")
      |> Testcontainers.PostgresContainer.with_database("query_service_test")

    {:ok, postgres_container} = Testcontainers.start_container(postgres_config)

    postgres_port = Testcontainers.PostgresContainer.port(postgres_container)
    database_url = "ecto://postgres:postgres@localhost:#{postgres_port}/query_service_test"

    System.put_env("DATABASE_URL", database_url)

    Application.put_env(:query_service_ex, :testcontainer_postgres, %{
      container: postgres_container,
      port: postgres_port,
      url: database_url
    })

    Application.put_env(:query_service_ex, QueryServiceEx.Repo,
      url: database_url,
      pool: Ecto.Adapters.SQL.Sandbox,
      pool_size: System.schedulers_online() * 2
    )

    {:ok, _} = Application.ensure_all_started(:query_service_ex)

    migrations_path = Path.join([File.cwd!(), "priv", "repo", "migrations"])

    if File.dir?(migrations_path) do
      Ecto.Migrator.run(QueryServiceEx.Repo, migrations_path, :up, all: true, log: :info)
    end

    Ecto.Adapters.SQL.Sandbox.mode(QueryServiceEx.Repo, :manual)

    ExUnit.after_suite(fn _results ->
      case Application.get_env(:query_service_ex, :testcontainer_postgres) do
        %{container: container} ->
          Testcontainers.stop_container(container.container_id)

        _ ->
          :ok
      end
    end)
  end
end
