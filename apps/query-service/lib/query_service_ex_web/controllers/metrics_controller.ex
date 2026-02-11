defmodule QueryServiceExWeb.MetricsController do
  @moduledoc """
  Controller for system metrics and monitoring.

  Supports two output formats:
  - JSON (GET /api/metrics): Rich metrics including backend and Elixir runtime info
  - Prometheus text (GET /api/metrics/prometheus): For Prometheus/Grafana/DataDog/NewRelic

  ## APM Integration

  The Prometheus endpoint is compatible with:
  - DataDog Agent (via Prometheus scraping)
  - NewRelic Infrastructure (via Prometheus integration)
  - Grafana Agent
  - Any OpenMetrics-compatible system
  """

  use Phoenix.Controller, formats: [:json]
  use OpenApiSpex.ControllerSpecs

  alias QueryServiceEx.Infrastructure.Adapters.RustCoreClient
  alias QueryServiceEx.PrometheusMetrics
  alias QueryServiceExWeb.Schemas.Metrics

  @prometheus_content_type "text/plain; version=0.0.4; charset=utf-8"

  tags(["Metrics"])

  operation(:show,
    summary: "Get metrics in JSON format",
    description: "Returns Elixir runtime and backend metrics in JSON format.",
    responses: [
      ok: {"Metrics response", "application/json", Metrics.MetricsResponse}
    ]
  )

  @doc """
  Get system metrics in JSON format.

  Returns metrics from the backend and Elixir runtime.
  For Prometheus text format, use GET /api/metrics/prometheus instead.
  """
  def show(conn, _params) do
    json_metrics(conn)
  end

  operation(:prometheus,
    summary: "Get metrics in Prometheus format",
    description: """
    Returns metrics in Prometheus text format for scraping by monitoring systems.

    Compatible with Prometheus, DataDog Agent, NewRelic Infrastructure, and Grafana Agent.
    """,
    responses: [
      ok: {"Prometheus metrics", "text/plain", %OpenApiSpex.Schema{type: :string}}
    ]
  )

  @doc """
  Returns metrics in Prometheus text format.

  This endpoint can be scraped by Prometheus, DataDog, NewRelic, or any
  OpenMetrics-compatible monitoring system.

  Configure your scraper to use this endpoint:

  ```yaml
  # prometheus.yml
  scrape_configs:
    - job_name: 'query_service'
      static_configs:
        - targets: ['localhost:3902']
      metrics_path: '/api/metrics/prometheus'
  ```
  """
  def prometheus(conn, _params) do
    metrics = PrometheusMetrics.scrape()

    conn
    |> put_resp_content_type(@prometheus_content_type)
    |> send_resp(200, metrics)
  end

  # Returns JSON-formatted metrics
  defp json_metrics(conn) do
    backend_metrics =
      case RustCoreClient.get_metrics() do
        {:ok, metrics} -> metrics
        {:error, _} -> %{error: "Backend unavailable"}
      end

    elixir_metrics = %{
      processes: Process.list() |> length(),
      memory: :erlang.memory() |> format_memory(),
      uptime_seconds: :erlang.statistics(:wall_clock) |> elem(0) |> div(1000),
      schedulers: :erlang.system_info(:schedulers_online)
    }

    response = %{
      service: "query_service_ex",
      timestamp: DateTime.utc_now() |> DateTime.to_iso8601(),
      elixir: elixir_metrics,
      backend: backend_metrics
    }

    json(conn, response)
  end

  defp format_memory(memory_list) do
    %{
      total_mb: memory_list[:total] |> bytes_to_mb(),
      processes_mb: memory_list[:processes] |> bytes_to_mb(),
      atom_mb: memory_list[:atom] |> bytes_to_mb(),
      binary_mb: memory_list[:binary] |> bytes_to_mb(),
      ets_mb: memory_list[:ets] |> bytes_to_mb()
    }
  end

  defp bytes_to_mb(bytes), do: Float.round(bytes / 1024 / 1024, 2)
end
