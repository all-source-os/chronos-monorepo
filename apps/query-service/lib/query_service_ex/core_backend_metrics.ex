defmodule QueryServiceEx.CoreBackendMetrics do
  @moduledoc """
  Turns Core's raw Prometheus `/metrics` text into a small, typed summary the
  dashboard can read directly.

  ## Why this exists

  Core exposes `/metrics` as Prometheus text exposition. The Query Service's
  `/api/metrics` historically forwarded that text verbatim under the `backend`
  key, so every consumer (the web dashboard especially) had to re-parse a blob to
  read one number — and the dashboard simply *couldn't*, so cards rendered `—`.
  Parsing once, here, lets `MetricsController` return
  `backend: %{p99_latency_us, storage_bytes, ...}` that the
  `use-dashboard-stats` hook already reads.

  ## Honesty / scope of these numbers (platform vs tenant)

  Every value here is derived from Core's **process-global** metrics, which are
  reset on restart and aggregate **all tenants**. They are therefore
  **platform/system** figures, NOT per-tenant:

    * `p99_latency_us` — the 0.99 quantile of `allsource_query_duration_seconds`
      across *all* query types and *all* tenants. A platform latency property.
      Honest to show as a system metric; must never be labelled as one tenant's.
    * `storage_bytes` — `allsource_storage_size_bytes`, the on-disk size of the
      whole Core data directory (every tenant's Parquet + WAL). Platform-wide.
      (Populated by Core; reads 0 until the Core change that sets the gauge ships.)
    * `storage_events_total` — `allsource_storage_events_total`, the resident
      event count for the whole process. Exposed for completeness/observability;
      the dashboard deliberately does NOT use it as a tenant "total events" number
      (that comes from tenant-scoped event-store queries elsewhere).

  Per-tenant magnitudes (a tenant's events/queries/storage) must come from
  tenant-scoped endpoints, never from this summary.

  The raw text is preserved under `:raw` so existing/raw consumers keep working.
  """

  alias QueryServiceEx.PrometheusParser

  @query_duration_histogram "allsource_query_duration_seconds"
  @storage_size_gauge "allsource_storage_size_bytes"
  @storage_events_gauge "allsource_storage_events_total"
  @parquet_files_gauge "allsource_parquet_files_total"
  @wal_segments_gauge "allsource_wal_segments_total"

  @typedoc """
  Structured backend metrics returned under the `/api/metrics` `backend` key.

  All figures are platform-wide (see module doc). `p99_latency_us` is `nil` when
  the histogram is empty (no queries observed yet) so the dashboard can render an
  honest empty state instead of a fabricated `0`.
  """
  @type t :: %{
          optional(:raw) => String.t(),
          p99_latency_us: number() | nil,
          p99_latency_ms: number() | nil,
          storage_bytes: number(),
          storage_events_total: number(),
          parquet_files_total: number(),
          wal_segments_total: number(),
          scope: String.t()
        }

  @doc """
  Builds the typed summary from Core's Prometheus exposition `text`.

  Pass `include_raw?: false` to omit the original text from the result (defaults
  to keeping it under `:raw`).
  """
  @spec from_prometheus_text(String.t(), keyword()) :: t()
  def from_prometheus_text(text, opts \\ []) when is_binary(text) do
    parsed = PrometheusParser.parse(text)
    summary = from_parsed(parsed)

    if Keyword.get(opts, :include_raw?, true) do
      Map.put(summary, :raw, text)
    else
      summary
    end
  end

  @doc """
  Builds the typed summary from an already-parsed metrics map (the shape returned
  by `PrometheusParser.parse/1`). Does not attach `:raw`.
  """
  @spec from_parsed(map()) :: t()
  def from_parsed(parsed) when is_map(parsed) do
    p99_seconds =
      PrometheusParser.histogram_quantile(parsed, @query_duration_histogram, 0.99, nil)

    %{
      # Platform p99 across all query types (see module doc). nil → honest empty.
      p99_latency_us: seconds_to_us(p99_seconds),
      p99_latency_ms: seconds_to_ms(p99_seconds),
      storage_bytes: round_metric(PrometheusParser.get_metric(parsed, @storage_size_gauge)),
      storage_events_total:
        round_metric(PrometheusParser.get_metric(parsed, @storage_events_gauge)),
      parquet_files_total:
        round_metric(PrometheusParser.get_metric(parsed, @parquet_files_gauge)),
      wal_segments_total: round_metric(PrometheusParser.get_metric(parsed, @wal_segments_gauge)),
      scope: "platform"
    }
  end

  defp seconds_to_us(nil), do: nil
  defp seconds_to_us(seconds) when is_number(seconds), do: Float.round(seconds * 1_000_000, 1)

  defp seconds_to_ms(nil), do: nil
  defp seconds_to_ms(seconds) when is_number(seconds), do: Float.round(seconds * 1_000, 3)

  # Gauges come back as floats from the parser; integral byte/count gauges read
  # cleaner as integers. Non-numeric sentinels collapse to 0.
  defp round_metric(v) when is_number(v), do: round(v)
end
