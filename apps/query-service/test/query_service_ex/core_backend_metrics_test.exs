defmodule QueryServiceEx.CoreBackendMetricsTest do
  use ExUnit.Case, async: true

  alias QueryServiceEx.CoreBackendMetrics

  @moduletag :unit

  # Mirrors Core's real /metrics shape: a query-duration histogram partitioned by
  # query_type, plus the storage gauges. storage_size_bytes reads 0 until the Core
  # gauge-population change ships — the summary surfaces it honestly as 0.
  @core_output """
  # HELP allsource_query_duration_seconds Query duration in seconds
  # TYPE allsource_query_duration_seconds histogram
  allsource_query_duration_seconds_bucket{query_type="entity",le="0.005"} 90
  allsource_query_duration_seconds_bucket{query_type="entity",le="0.01"} 100
  allsource_query_duration_seconds_bucket{query_type="entity",le="0.025"} 100
  allsource_query_duration_seconds_bucket{query_type="entity",le="+Inf"} 100
  allsource_query_duration_seconds_bucket{query_type="full_scan",le="0.005"} 0
  allsource_query_duration_seconds_bucket{query_type="full_scan",le="0.01"} 0
  allsource_query_duration_seconds_bucket{query_type="full_scan",le="0.025"} 100
  allsource_query_duration_seconds_bucket{query_type="full_scan",le="+Inf"} 100
  # HELP allsource_storage_size_bytes Total storage size in bytes
  # TYPE allsource_storage_size_bytes gauge
  allsource_storage_size_bytes 1048576
  # HELP allsource_storage_events_total Total number of events in storage
  # TYPE allsource_storage_events_total gauge
  allsource_storage_events_total 24993
  # HELP allsource_parquet_files_total Number of Parquet files
  # TYPE allsource_parquet_files_total gauge
  allsource_parquet_files_total 7
  # HELP allsource_wal_segments_total Number of WAL segments
  # TYPE allsource_wal_segments_total gauge
  allsource_wal_segments_total 2
  """

  describe "from_prometheus_text/2" do
    test "derives a platform p99 (in microseconds) from the aggregated histogram" do
      summary = CoreBackendMetrics.from_prometheus_text(@core_output)

      # Same aggregation as the parser test: p99 ≈ 0.0247 s = 24_700 µs.
      assert_in_delta summary.p99_latency_us, 24_700.0, 50.0
      assert_in_delta summary.p99_latency_ms, 24.7, 0.05
    end

    test "surfaces storage gauges as integers" do
      summary = CoreBackendMetrics.from_prometheus_text(@core_output)

      assert summary.storage_bytes == 1_048_576
      assert summary.storage_events_total == 24_993
      assert summary.parquet_files_total == 7
      assert summary.wal_segments_total == 2
    end

    test "labels the summary as a platform-scoped figure" do
      summary = CoreBackendMetrics.from_prometheus_text(@core_output)
      assert summary.scope == "platform"
    end

    test "keeps the raw exposition text by default, omittable on request" do
      with_raw = CoreBackendMetrics.from_prometheus_text(@core_output)
      assert with_raw.raw == @core_output

      without_raw = CoreBackendMetrics.from_prometheus_text(@core_output, include_raw?: false)
      refute Map.has_key?(without_raw, :raw)
    end

    test "p99 is nil (honest empty) when no queries have been observed" do
      # Only storage gauges, no query histogram → no fabricated latency.
      summary = CoreBackendMetrics.from_prometheus_text("allsource_storage_size_bytes 0\n")

      assert summary.p99_latency_us == nil
      assert summary.p99_latency_ms == nil
      assert summary.storage_bytes == 0
    end

    test "storage_bytes is 0 (not invented) when Core has not populated the gauge" do
      # The current production reality: the gauge is registered but never set.
      summary = CoreBackendMetrics.from_prometheus_text("allsource_storage_size_bytes 0\n")
      assert summary.storage_bytes == 0
    end
  end
end
