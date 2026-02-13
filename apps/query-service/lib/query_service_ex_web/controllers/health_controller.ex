defmodule QueryServiceExWeb.HealthController do
  @moduledoc """
  Health check endpoints for Fly.io instance lifecycle management.

  Provides three types of health checks:

  ## Liveness Probe (`/api/health/live`)
  Indicates whether the application process is running.
  Used by Fly.io to determine if an instance needs to be restarted.
  Only fails if the Elixir VM itself is unhealthy.

  ## Readiness Probe (`/api/health/ready`)
  Indicates whether the application is ready to accept traffic.
  Checks database connectivity and core service availability.
  Used by Fly.io to determine if an instance should receive traffic.

  ## Combined Health (`/api/health`)
  Comprehensive health check combining liveness and readiness.
  Returns HTTP 503 if any critical dependency is unavailable.

  ## Configuration

      config :query_service_ex,
        health_check_timeout_ms: 5_000,
        health_check_db_timeout_ms: 2_000,
        health_check_backend_timeout_ms: 3_000
  """

  use Phoenix.Controller, formats: [:json]
  use OpenApiSpex.ControllerSpecs

  require Logger

  alias QueryServiceEx.DevMode
  alias QueryServiceEx.Infrastructure.Adapters.CoreHealthChecker
  alias QueryServiceEx.Infrastructure.Adapters.CoreWebSocketClient
  alias QueryServiceEx.Infrastructure.Adapters.RustCoreClient
  alias QueryServiceExWeb.Schemas.Health

  @default_db_timeout_ms 2_000
  @default_backend_timeout_ms 3_000

  tags(["Health"])

  operation(:live,
    summary: "Liveness probe",
    description:
      "Returns 200 OK if the service is running. Used by orchestrators to restart unhealthy instances.",
    responses: [
      ok: {"Service is alive", "application/json", Health.LiveResponse}
    ]
  )

  @doc """
  Liveness probe - checks if the Elixir process is alive.

  This endpoint always returns 200 OK if the process can handle requests.
  Fly.io uses this to decide whether to restart an instance.
  """
  def live(conn, _params) do
    response = %{
      status: "alive",
      timestamp: DateTime.utc_now() |> DateTime.to_iso8601()
    }

    conn
    |> put_status(:ok)
    |> json(response)
  end

  operation(:ready,
    summary: "Readiness probe",
    description: "Checks if all dependencies are available. Returns 503 if any check fails.",
    responses: [
      ok: {"Service is ready", "application/json", Health.ReadyResponse},
      service_unavailable: {"Service not ready", "application/json", Health.ReadyResponse}
    ]
  )

  @doc """
  Readiness probe - checks if the service is ready to accept traffic.

  Verifies:
  - Database connectivity (via Ecto)
  - Backend Core service availability

  Returns HTTP 503 if any dependency is unavailable.
  """
  def ready(conn, _params) do
    checks = perform_readiness_checks()
    all_healthy = Enum.all?(checks, fn {_name, status} -> status == :healthy end)

    response = %{
      status: if(all_healthy, do: "ready", else: "not_ready"),
      checks: format_checks(checks),
      timestamp: DateTime.utc_now() |> DateTime.to_iso8601()
    }

    status_code = if all_healthy, do: :ok, else: :service_unavailable

    conn
    |> put_status(status_code)
    |> json(response)
  end

  operation(:show,
    summary: "Combined health check",
    description:
      "Comprehensive health check including all dependencies. Returns 503 when unhealthy or degraded.",
    responses: [
      ok: {"Service is healthy", "application/json", Health.HealthResponse},
      service_unavailable:
        {"Service unhealthy or degraded", "application/json", Health.HealthResponse}
    ]
  )

  @doc """
  Combined health check endpoint.

  Returns comprehensive health status including all dependencies.
  HTTP 200 when healthy, HTTP 503 when degraded or unhealthy.
  """
  def show(conn, _params) do
    checks = perform_readiness_checks()
    all_healthy = Enum.all?(checks, fn {_name, status} -> status == :healthy end)

    overall_status =
      cond do
        all_healthy -> "healthy"
        Enum.any?(checks, fn {_name, status} -> status == :unhealthy end) -> "unhealthy"
        true -> "degraded"
      end

    response = %{
      service: "query_service_ex",
      status: overall_status,
      checks: format_checks(checks),
      core_nodes: CoreHealthChecker.node_health_states(),
      timestamp: DateTime.utc_now() |> DateTime.to_iso8601(),
      version: Application.spec(:query_service_ex, :vsn) |> to_string()
    }

    # Return 503 if any critical check fails - this tells Fly.io not to route traffic here
    status_code = if overall_status == "healthy", do: :ok, else: :service_unavailable

    conn
    |> put_status(status_code)
    |> json(response)
  end

  # Private functions

  defp perform_readiness_checks do
    # In standalone mode (no PostgreSQL), skip the database check entirely
    db_result =
      if DevMode.repo_available?() do
        db_task = Task.async(fn -> {:database, check_database()} end)

        db_timeout =
          Application.get_env(
            :query_service_ex,
            :health_check_db_timeout_ms,
            @default_db_timeout_ms
          )

        await_check(db_task, db_timeout, :database)
      else
        {:database, :healthy}
      end

    # Run remaining checks concurrently with timeouts
    backend_task = Task.async(fn -> {:backend, check_backend()} end)
    ws_task = Task.async(fn -> {:websocket, check_websocket()} end)

    backend_timeout =
      Application.get_env(
        :query_service_ex,
        :health_check_backend_timeout_ms,
        @default_backend_timeout_ms
      )

    backend_result = await_check(backend_task, backend_timeout, :backend)
    ws_result = await_check(ws_task, @default_db_timeout_ms, :websocket)

    [db_result, backend_result, ws_result]
  end

  defp await_check(task, timeout, check_name) do
    case Task.yield(task, timeout) || Task.shutdown(task, :brutal_kill) do
      {:ok, result} ->
        result

      nil ->
        Logger.warning(
          "[HealthController] #{check_name} health check timed out after #{timeout}ms"
        )

        :telemetry.execute(
          [:query_service_ex, :health_check, :timeout],
          %{timeout_ms: timeout},
          %{check: check_name}
        )

        {check_name, :timeout}
    end
  end

  defp check_database do
    start_time = System.monotonic_time()

    result =
      case QueryServiceEx.Repo.query("SELECT 1") do
        {:ok, _result} ->
          :healthy

        {:error, reason} ->
          Logger.warning("[HealthController] Database check failed: #{inspect(reason)}")
          :unhealthy
      end

    duration_ms =
      System.convert_time_unit(System.monotonic_time() - start_time, :native, :millisecond)

    :telemetry.execute(
      [:query_service_ex, :health_check, :completed],
      %{duration_ms: duration_ms},
      %{check: :database, status: result}
    )

    result
  rescue
    error ->
      Logger.warning("[HealthController] Database check exception: #{inspect(error)}")
      :unhealthy
  end

  defp check_backend do
    start_time = System.monotonic_time()

    result =
      case RustCoreClient.health_check() do
        {:ok, _health} ->
          :healthy

        {:error, reason} ->
          Logger.warning("[HealthController] Backend check failed: #{inspect(reason)}")
          :degraded
      end

    duration_ms =
      System.convert_time_unit(System.monotonic_time() - start_time, :native, :millisecond)

    :telemetry.execute(
      [:query_service_ex, :health_check, :completed],
      %{duration_ms: duration_ms},
      %{check: :backend, status: result}
    )

    result
  end

  defp check_websocket do
    # Check if WebSocket client is running and connected
    case Process.whereis(CoreWebSocketClient) do
      nil ->
        # WebSocket client might be disabled in test/dev
        if Application.get_env(:query_service_ex, :core_ws_enabled, true) do
          Logger.warning("[HealthController] WebSocket client not running")
          :degraded
        else
          :healthy
        end

      pid when is_pid(pid) ->
        case CoreWebSocketClient.status(pid) do
          :connected -> :healthy
          :disconnected -> :degraded
          {:error, :timeout} -> :degraded
        end
    end
  rescue
    _ -> :degraded
  catch
    :exit, _ -> :degraded
  end

  defp format_checks(checks) do
    checks
    |> Enum.map(fn {name, status} ->
      status_str =
        case status do
          :healthy -> "healthy"
          :unhealthy -> "unhealthy"
          :degraded -> "degraded"
          :timeout -> "timeout"
        end

      {name, status_str}
    end)
    |> Enum.into(%{})
  end
end
