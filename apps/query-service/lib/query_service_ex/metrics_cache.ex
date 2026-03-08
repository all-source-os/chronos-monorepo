defmodule QueryServiceEx.MetricsCache do
  @moduledoc """
  GenServer-based cache for admin metrics data fetched from Core.

  Caches parsed Prometheus metrics for a configurable TTL (default 15 seconds)
  to avoid hammering Core's /metrics endpoint. Also maintains a time-series
  ring buffer of metric snapshots for historical queries.
  """

  use GenServer

  require Logger

  @default_ttl_ms 15_000
  @max_timeseries_points 10_080

  # -------------------------------------------------------------------
  # Public API
  # -------------------------------------------------------------------

  @doc """
  Starts the MetricsCache GenServer.
  """
  def start_link(opts \\ []) do
    name = Keyword.get(opts, :name, __MODULE__)
    GenServer.start_link(__MODULE__, opts, name: name)
  end

  @doc """
  Gets cached metrics, fetching fresh data if the cache is stale.

  The `fetch_fn` is called only when the cache has expired. It should return
  `{:ok, raw_prometheus_text}` or `{:error, reason}`.

  Returns `{:ok, parsed_metrics}` or `{:error, reason}`.
  """
  def get_or_fetch(fetch_fn, name \\ __MODULE__) do
    GenServer.call(name, {:get_or_fetch, fetch_fn})
  end

  @doc """
  Returns time-series data for a given metric name and time range.

  The `range` parameter accepts: `"1h"`, `"24h"`, `"7d"`.

  Returns a list of `%{timestamp: iso8601, value: float}` maps.
  """
  def get_timeseries(metric_name, range, name \\ __MODULE__) do
    GenServer.call(name, {:get_timeseries, metric_name, range})
  end

  @doc """
  Records a metric snapshot into the time-series buffer.

  Called internally after each successful fetch.
  """
  def record_snapshot(snapshot, name \\ __MODULE__) do
    GenServer.cast(name, {:record_snapshot, snapshot})
  end

  @doc """
  Clears all cached data. Useful for testing.
  """
  def clear(name \\ __MODULE__) do
    GenServer.call(name, :clear)
  end

  # -------------------------------------------------------------------
  # GenServer callbacks
  # -------------------------------------------------------------------

  @impl true
  def init(opts) do
    ttl_ms = Keyword.get(opts, :ttl_ms, @default_ttl_ms)

    state = %{
      cached_metrics: nil,
      cached_at: 0,
      ttl_ms: ttl_ms,
      timeseries: []
    }

    {:ok, state}
  end

  @impl true
  def handle_call({:get_or_fetch, fetch_fn}, _from, state) do
    now = System.monotonic_time(:millisecond)

    if state.cached_metrics != nil and now - state.cached_at < state.ttl_ms do
      {:reply, {:ok, state.cached_metrics}, state}
    else
      case fetch_fn.() do
        {:ok, raw_text} when is_binary(raw_text) ->
          parsed = QueryServiceEx.PrometheusParser.parse(raw_text)
          snapshot = build_snapshot(parsed)

          new_timeseries =
            append_timeseries(state.timeseries, snapshot, @max_timeseries_points)

          new_state = %{
            state
            | cached_metrics: parsed,
              cached_at: now,
              timeseries: new_timeseries
          }

          {:reply, {:ok, parsed}, new_state}

        {:ok, body} when is_map(body) ->
          # Core returned JSON (already decoded by Tesla) -- not Prometheus text.
          # Wrap it so callers can still use it, but it won't have Prometheus metrics.
          {:reply, {:ok, body}, %{state | cached_metrics: body, cached_at: now}}

        {:error, reason} = err ->
          # Return stale cache if available, otherwise propagate error
          if state.cached_metrics != nil do
            {:reply, {:ok, state.cached_metrics}, state}
          else
            {:reply, err, state}
          end
      end
    end
  end

  @impl true
  def handle_call({:get_timeseries, metric_name, range}, _from, state) do
    cutoff = timeseries_cutoff(range)

    points =
      state.timeseries
      |> Enum.filter(fn %{timestamp: ts} -> DateTime.compare(ts, cutoff) == :gt end)
      |> Enum.map(fn snapshot ->
        %{
          timestamp: DateTime.to_iso8601(snapshot.timestamp),
          value: Map.get(snapshot.metrics, metric_name, 0.0)
        }
      end)

    {:reply, {:ok, points}, state}
  end

  @impl true
  def handle_call(:clear, _from, _state) do
    {:reply, :ok,
     %{
       cached_metrics: nil,
       cached_at: 0,
       ttl_ms: @default_ttl_ms,
       timeseries: []
     }}
  end

  @impl true
  def handle_cast({:record_snapshot, snapshot}, state) do
    new_timeseries =
      append_timeseries(state.timeseries, snapshot, @max_timeseries_points)

    {:noreply, %{state | timeseries: new_timeseries}}
  end

  # -------------------------------------------------------------------
  # Private helpers
  # -------------------------------------------------------------------

  defp build_snapshot(parsed) when is_map(parsed) do
    alias QueryServiceEx.PrometheusParser

    %{
      timestamp: DateTime.utc_now(),
      metrics: %{
        "events_total" => PrometheusParser.get_metric(parsed, "allsource_events_total"),
        "events_per_second" => PrometheusParser.get_metric(parsed, "allsource_events_per_second"),
        "query_latency_p99_ms" =>
          PrometheusParser.get_quantile(parsed, "allsource_query_duration_seconds", "0.99") *
            1000,
        "error_rate_percent" => compute_error_rate(parsed),
        "active_tenants" => PrometheusParser.get_metric(parsed, "allsource_active_tenants"),
        "uptime_seconds" => PrometheusParser.get_metric(parsed, "allsource_uptime_seconds")
      }
    }
  end

  defp compute_error_rate(parsed) do
    alias QueryServiceEx.PrometheusParser

    total = PrometheusParser.get_metric(parsed, "allsource_http_requests_total")
    errors = PrometheusParser.get_metric(parsed, "allsource_http_errors_total")

    if total > 0 do
      Float.round(errors / total * 100, 2)
    else
      0.0
    end
  end

  defp append_timeseries(timeseries, snapshot, max_points) do
    new_series = timeseries ++ [snapshot]

    if length(new_series) > max_points do
      Enum.drop(new_series, length(new_series) - max_points)
    else
      new_series
    end
  end

  defp timeseries_cutoff("1h") do
    DateTime.add(DateTime.utc_now(), -3600, :second)
  end

  defp timeseries_cutoff("24h") do
    DateTime.add(DateTime.utc_now(), -86400, :second)
  end

  defp timeseries_cutoff("7d") do
    DateTime.add(DateTime.utc_now(), -604_800, :second)
  end

  defp timeseries_cutoff(_) do
    # Default to 1 hour
    DateTime.add(DateTime.utc_now(), -3600, :second)
  end
end
