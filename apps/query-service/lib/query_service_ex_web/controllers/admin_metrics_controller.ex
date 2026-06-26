defmodule QueryServiceExWeb.AdminMetricsController do
  @moduledoc """
  Admin metrics aggregation endpoints.

  Fetches Core's Prometheus `/metrics` endpoint, parses the text format, and
  returns aggregated JSON summaries and time-series data for the admin dashboard.

  Metrics are cached for 15 seconds to avoid excessive load on Core.
  """

  use Phoenix.Controller, formats: [:json]

  alias QueryServiceEx.Infrastructure.Adapters.RustCoreClient
  alias QueryServiceEx.MetricsCache
  alias QueryServiceEx.PrometheusParser

  @doc """
  GET /api/admin/metrics/summary

  Returns an aggregated JSON summary of Core's current metrics:

  ```json
  {
    "uptime_seconds": 86400,
    "events_total": 1500000,
    "events_per_second": 469.0,
    "query_latency_p99_ms": 11.9,
    "error_rate_percent": 0.02,
    "active_tenants": 12
  }
  ```
  """
  def summary(conn, _params) do
    case MetricsCache.get_or_fetch(&fetch_core_metrics/0) do
      {:ok, parsed} when is_map(parsed) ->
        summary = build_summary(parsed)
        json(conn, summary)

      {:error, reason} ->
        conn
        |> put_status(502)
        |> json(%{error: "Failed to fetch metrics from Core", detail: inspect(reason)})
    end
  end

  @doc """
  GET /api/admin/metrics/timeseries?metric=events_per_second&range=1h

  Returns time-bucketed metric data for charting.

  ## Query Parameters

  - `metric` (required) - One of: `events_total`, `events_per_second`,
    `query_latency_p99_ms`, `error_rate_percent`, `active_tenants`, `uptime_seconds`
  - `range` (optional, default `"1h"`) - One of: `"1h"`, `"24h"`, `"7d"`

  ## Response

  ```json
  {
    "metric": "events_per_second",
    "range": "1h",
    "points": [
      {"timestamp": "2026-03-08T12:00:00Z", "value": 469.0},
      ...
    ]
  }
  ```
  """
  def timeseries(conn, params) do
    metric = Map.get(params, "metric", "events_per_second")
    range = Map.get(params, "range", "1h")

    valid_metrics = [
      "events_total",
      "events_per_second",
      "query_latency_p99_ms",
      "error_rate_percent",
      "active_tenants",
      "uptime_seconds"
    ]

    valid_ranges = ["1h", "24h", "7d"]

    cond do
      metric not in valid_metrics ->
        conn
        |> put_status(400)
        |> json(%{
          error: "Invalid metric",
          valid_metrics: valid_metrics
        })

      range not in valid_ranges ->
        conn
        |> put_status(400)
        |> json(%{
          error: "Invalid range",
          valid_ranges: valid_ranges
        })

      true ->
        # Ensure we have a fresh snapshot in the cache before querying timeseries
        _ = MetricsCache.get_or_fetch(&fetch_core_metrics/0)

        case MetricsCache.get_timeseries(metric, range) do
          {:ok, points} ->
            json(conn, %{
              metric: metric,
              range: range,
              points: points
            })

          {:error, reason} ->
            conn
            |> put_status(500)
            |> json(%{error: "Failed to retrieve timeseries", detail: inspect(reason)})
        end
    end
  end

  # -------------------------------------------------------------------
  # Private helpers
  # -------------------------------------------------------------------

  defp fetch_core_metrics do
    RustCoreClient.get_metrics_raw()
  end

  @doc false
  # Public for testability (regression guard on the metric-name mapping). Maps a
  # parsed Prometheus metric map into the summary the admin /monitoring page reads.
  def build_summary(parsed) when is_map(parsed) do
    # If parsed is already a structured map (not from Prometheus parser),
    # return it directly — happens when Core returns JSON instead of Prometheus text
    if Map.has_key?(parsed, "uptime_seconds") or Map.has_key?(parsed, :uptime_seconds) do
      parsed
    else
      # The metric NAMES below are the ones Core's exporter actually emits. The
      # original draft read placeholder names (`allsource_events_total`,
      # `allsource_active_tenants`, `allsource_uptime_seconds`,
      # `allsource_http_*`) that Core never exports, so every field parsed to 0 and
      # /monitoring showed a dead all-zeros summary even with real traffic. We map
      # to the real series (verified against the live /metrics):
      #   - events lifetime total -> `allsource_storage_events_total` (durable count),
      #     falling back to `allsource_events_ingested_total` (this-process count).
      #   - p99 query latency -> the `allsource_query_duration_seconds` HISTOGRAM via
      #     histogram_quantile (Core emits `_bucket{le=...}`, not summary quantiles,
      #     so get_quantile always returned 0). Buckets are per query_type; the
      #     parser aggregates across them. Seconds -> ms.
      #   - error rate -> ingestion errors / ingested (the real counters).
      # active_tenants / uptime_seconds / events_per_second have no Core series yet;
      # they stay 0 until Core's exporter adds them (documented gap, not a crash).
      events_total =
        first_positive([
          PrometheusParser.get_metric(parsed, "allsource_storage_events_total"),
          PrometheusParser.get_metric(parsed, "allsource_events_ingested_total")
        ])

      %{
        uptime_seconds: PrometheusParser.get_metric(parsed, "allsource_uptime_seconds"),
        events_total: events_total,
        events_per_second: PrometheusParser.get_metric(parsed, "allsource_events_per_second"),
        query_latency_p99_ms:
          PrometheusParser.histogram_quantile(
            parsed,
            "allsource_query_duration_seconds",
            0.99
          ) * 1000,
        error_rate_percent: compute_error_rate(parsed),
        active_tenants: PrometheusParser.get_metric(parsed, "allsource_active_tenants")
      }
    end
  end

  # Returns the first strictly-positive value in the list, or 0.0 if none — used to
  # prefer the durable lifetime counter but fall back to the session counter.
  defp first_positive(values) do
    Enum.find(values, 0.0, fn v -> is_number(v) and v > 0 end)
  end

  defp compute_error_rate(parsed) do
    # Core emits ingestion counters (not http_* request/error counters), so derive
    # the error rate from those: errors / (successful ingests + errors).
    errors = PrometheusParser.get_metric(parsed, "allsource_ingestion_errors_total")
    ingested = PrometheusParser.get_metric(parsed, "allsource_events_ingested_total")
    denom = ingested + errors

    cond do
      denom > 0 -> Float.round(errors / denom * 100, 2)
      true -> 0.0
    end
  end
end
