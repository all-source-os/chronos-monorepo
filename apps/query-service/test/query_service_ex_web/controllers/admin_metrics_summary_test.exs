defmodule QueryServiceExWeb.AdminMetricsSummaryTest do
  @moduledoc """
  Regression guard for the admin /monitoring summary mapping.

  The original draft read placeholder metric names that Core's exporter never
  emits (`allsource_events_total`, `allsource_http_*`, summary quantiles), so the
  summary was a dead all-zeros payload even under real traffic. These tests feed a
  trimmed slice of the REAL Core /metrics output (verified live) through the
  parser + build_summary and assert the real series are surfaced.
  """
  use ExUnit.Case, async: true

  alias QueryServiceEx.PrometheusParser
  alias QueryServiceExWeb.AdminMetricsController

  # A representative excerpt of Core's actual Prometheus exposition (names + shape
  # taken verbatim from the live allsource-core /metrics).
  @core_metrics """
  # HELP allsource_storage_events_total Total events in storage
  # TYPE allsource_storage_events_total gauge
  allsource_storage_events_total 13354
  # HELP allsource_events_ingested_total Total events ingested
  # TYPE allsource_events_ingested_total counter
  allsource_events_ingested_total 469
  # HELP allsource_ingestion_errors_total Total number of ingestion errors
  # TYPE allsource_ingestion_errors_total counter
  allsource_ingestion_errors_total 0
  # TYPE allsource_query_duration_seconds histogram
  allsource_query_duration_seconds_bucket{query_type="entity",le="0.005"} 793
  allsource_query_duration_seconds_bucket{query_type="entity",le="0.01"} 793
  allsource_query_duration_seconds_bucket{query_type="entity",le="+Inf"} 793
  allsource_query_duration_seconds_bucket{query_type="full_scan",le="0.005"} 0
  allsource_query_duration_seconds_bucket{query_type="full_scan",le="0.01"} 1000
  allsource_query_duration_seconds_bucket{query_type="full_scan",le="+Inf"} 2124
  """

  test "events_total comes from the real storage_events_total series, not a placeholder" do
    summary = @core_metrics |> PrometheusParser.parse() |> AdminMetricsController.build_summary()
    assert summary.events_total == 13354
  end

  test "events_total falls back to the session counter when storage total is absent" do
    metrics = "allsource_events_ingested_total 469\n"
    summary = metrics |> PrometheusParser.parse() |> AdminMetricsController.build_summary()
    assert summary.events_total == 469
  end

  test "query p99 is computed from the duration HISTOGRAM (ms), not a missing summary quantile" do
    summary = @core_metrics |> PrometheusParser.parse() |> AdminMetricsController.build_summary()
    # Buckets exist → a real, positive p99 in milliseconds (the old get_quantile
    # path returned 0 because Core emits no summary quantiles).
    assert summary.query_latency_p99_ms > 0.0
  end

  test "error rate derives from ingestion counters (0 errors over real ingests → 0%)" do
    summary = @core_metrics |> PrometheusParser.parse() |> AdminMetricsController.build_summary()
    assert summary.error_rate_percent == 0.0
  end

  test "error rate is non-zero when ingestion errors are present" do
    metrics = """
    allsource_events_ingested_total 90
    allsource_ingestion_errors_total 10
    """

    summary = metrics |> PrometheusParser.parse() |> AdminMetricsController.build_summary()
    # 10 / (90 + 10) = 10%
    assert summary.error_rate_percent == 10.0
  end

  test "absent Core series (active_tenants/uptime) degrade to 0, never crash" do
    summary = @core_metrics |> PrometheusParser.parse() |> AdminMetricsController.build_summary()
    assert summary.active_tenants == 0.0
    assert summary.uptime_seconds == 0.0
  end
end
