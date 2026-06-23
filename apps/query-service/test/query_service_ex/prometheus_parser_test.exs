defmodule QueryServiceEx.PrometheusParserTest do
  use ExUnit.Case, async: true

  alias QueryServiceEx.PrometheusParser

  @moduletag :unit

  @sample_prometheus_output """
  # HELP allsource_uptime_seconds Total uptime in seconds
  # TYPE allsource_uptime_seconds gauge
  allsource_uptime_seconds 86400
  # HELP allsource_events_total Total number of events ingested
  # TYPE allsource_events_total counter
  allsource_events_total 1500000
  # HELP allsource_events_per_second Current event ingestion rate
  # TYPE allsource_events_per_second gauge
  allsource_events_per_second 469.2
  # HELP allsource_query_duration_seconds Query latency histogram
  # TYPE allsource_query_duration_seconds summary
  allsource_query_duration_seconds{quantile="0.5"} 0.0021
  allsource_query_duration_seconds{quantile="0.9"} 0.0085
  allsource_query_duration_seconds{quantile="0.99"} 0.0119
  allsource_query_duration_seconds_sum 452.8
  allsource_query_duration_seconds_count 42000
  # HELP allsource_http_requests_total Total HTTP requests
  # TYPE allsource_http_requests_total counter
  allsource_http_requests_total 100000
  # HELP allsource_http_errors_total Total HTTP errors (4xx and 5xx)
  # TYPE allsource_http_errors_total counter
  allsource_http_errors_total 20
  # HELP allsource_active_tenants Number of active tenants
  # TYPE allsource_active_tenants gauge
  allsource_active_tenants 12
  # HELP allsource_wal_bytes_written Total WAL bytes written
  # TYPE allsource_wal_bytes_written counter
  allsource_wal_bytes_written 5.2428e+09
  """

  describe "parse/1" do
    test "parses simple counter metrics" do
      parsed = PrometheusParser.parse(@sample_prometheus_output)

      assert [%{labels: %{}, value: 1_500_000.0}] = parsed["allsource_events_total"]
    end

    test "parses simple gauge metrics" do
      parsed = PrometheusParser.parse(@sample_prometheus_output)

      assert [%{labels: %{}, value: 86_400.0}] = parsed["allsource_uptime_seconds"]
      assert [%{labels: %{}, value: 469.2}] = parsed["allsource_events_per_second"]
    end

    test "parses metrics with labels" do
      parsed = PrometheusParser.parse(@sample_prometheus_output)

      entries = parsed["allsource_query_duration_seconds"]
      assert length(entries) == 3

      quantiles = Enum.map(entries, fn %{labels: %{"quantile" => q}, value: v} -> {q, v} end)
      assert {"0.5", 0.0021} in quantiles
      assert {"0.9", 0.0085} in quantiles
      assert {"0.99", 0.0119} in quantiles
    end

    test "parses histogram _sum and _count metrics" do
      parsed = PrometheusParser.parse(@sample_prometheus_output)

      assert [%{value: 452.8}] = parsed["allsource_query_duration_seconds_sum"]
      assert [%{value: 42_000.0}] = parsed["allsource_query_duration_seconds_count"]
    end

    test "parses scientific notation values" do
      parsed = PrometheusParser.parse(@sample_prometheus_output)

      assert [%{value: value}] = parsed["allsource_wal_bytes_written"]
      assert_in_delta value, 5.2_428e9, 1.0
    end

    test "ignores comment and blank lines" do
      parsed = PrometheusParser.parse(@sample_prometheus_output)

      # No keys that start with #
      refute Enum.any?(Map.keys(parsed), &String.starts_with?(&1, "#"))
    end

    test "returns empty map for empty input" do
      assert %{} == PrometheusParser.parse("")
    end

    test "returns empty map for only comments" do
      input = """
      # HELP some_metric A metric
      # TYPE some_metric counter
      """

      assert %{} == PrometheusParser.parse(input)
    end

    test "parses metrics with multiple labels" do
      input = ~s(http_requests_total{method="GET",code="200",handler="/api"} 1027\n)

      parsed = PrometheusParser.parse(input)

      assert [%{labels: labels, value: 1027.0}] = parsed["http_requests_total"]
      assert labels["method"] == "GET"
      assert labels["code"] == "200"
      assert labels["handler"] == "/api"
    end

    test "parses metrics with timestamp (ignores timestamp)" do
      input = "events_total 42 1625000000\n"

      parsed = PrometheusParser.parse(input)

      assert [%{labels: %{}, value: 42.0}] = parsed["events_total"]
    end
  end

  describe "get_metric/3" do
    test "extracts a scalar metric value" do
      parsed = PrometheusParser.parse(@sample_prometheus_output)

      assert PrometheusParser.get_metric(parsed, "allsource_events_total") == 1_500_000.0
      assert PrometheusParser.get_metric(parsed, "allsource_uptime_seconds") == 86_400.0
      assert PrometheusParser.get_metric(parsed, "allsource_active_tenants") == 12.0
    end

    test "returns default for missing metrics" do
      parsed = PrometheusParser.parse(@sample_prometheus_output)

      assert PrometheusParser.get_metric(parsed, "nonexistent_metric") == 0.0
      assert PrometheusParser.get_metric(parsed, "nonexistent_metric", -1.0) == -1.0
    end
  end

  describe "get_quantile/4" do
    test "extracts quantile values from summary metrics" do
      parsed = PrometheusParser.parse(@sample_prometheus_output)

      p50 =
        PrometheusParser.get_quantile(parsed, "allsource_query_duration_seconds", "0.5")

      p90 =
        PrometheusParser.get_quantile(parsed, "allsource_query_duration_seconds", "0.9")

      p99 =
        PrometheusParser.get_quantile(parsed, "allsource_query_duration_seconds", "0.99")

      assert p50 == 0.0021
      assert p90 == 0.0085
      assert p99 == 0.0119
    end

    test "returns default for missing quantile" do
      parsed = PrometheusParser.parse(@sample_prometheus_output)

      assert PrometheusParser.get_quantile(
               parsed,
               "allsource_query_duration_seconds",
               "0.999"
             ) == 0.0
    end

    test "returns default for missing metric name" do
      parsed = PrometheusParser.parse(@sample_prometheus_output)

      assert PrometheusParser.get_quantile(parsed, "nonexistent_metric", "0.99") == 0.0
    end
  end

  # A real Core query-duration histogram, partitioned by query_type (this is how
  # Core actually exposes it). The overall p99 must AGGREGATE buckets across every
  # query_type — sum cumulative counts per `le` — then interpolate.
  @histogram_output """
  # HELP allsource_query_duration_seconds Query duration in seconds
  # TYPE allsource_query_duration_seconds histogram
  allsource_query_duration_seconds_bucket{query_type="entity",le="0.005"} 90
  allsource_query_duration_seconds_bucket{query_type="entity",le="0.01"} 100
  allsource_query_duration_seconds_bucket{query_type="entity",le="0.025"} 100
  allsource_query_duration_seconds_bucket{query_type="entity",le="0.05"} 100
  allsource_query_duration_seconds_bucket{query_type="entity",le="+Inf"} 100
  allsource_query_duration_seconds_bucket{query_type="full_scan",le="0.005"} 0
  allsource_query_duration_seconds_bucket{query_type="full_scan",le="0.01"} 0
  allsource_query_duration_seconds_bucket{query_type="full_scan",le="0.025"} 100
  allsource_query_duration_seconds_bucket{query_type="full_scan",le="0.05"} 100
  allsource_query_duration_seconds_bucket{query_type="full_scan",le="+Inf"} 100
  """

  describe "histogram_quantile/4" do
    test "aggregates buckets across labels and interpolates the quantile" do
      parsed = PrometheusParser.parse(@histogram_output)

      # Aggregated cumulative buckets (entity + full_scan):
      #   le=0.005 -> 90,  le=0.01 -> 100,  le=0.025 -> 200,
      #   le=0.05 -> 200,  +Inf -> 200.  total = 200.
      # p99 target rank = 0.99 * 200 = 198, which falls in the (0.01, 0.025] band
      # over counts (100, 200]: 0.01 + (0.025-0.01)*((198-100)/(200-100))
      #   = 0.01 + 0.015 * 0.98 = 0.0247 s.
      p99 = PrometheusParser.histogram_quantile(parsed, "allsource_query_duration_seconds", 0.99)
      assert_in_delta p99, 0.0247, 0.0001

      # p50 target rank = 100, satisfied exactly at le=0.01 (count first reaches
      # 100 there); interpolation within (0.005, 0.01] over (90, 100].
      p50 = PrometheusParser.histogram_quantile(parsed, "allsource_query_duration_seconds", 0.5)
      assert_in_delta p50, 0.01, 0.0006
    end

    test "lowest bucket interpolates from zero" do
      input = """
      m_bucket{le="0.01"} 100
      m_bucket{le="+Inf"} 100
      """

      parsed = PrometheusParser.parse(input)
      # All mass in the first finite bucket; p50 target = 50 over (0, 0.01] / (0,100].
      assert_in_delta PrometheusParser.histogram_quantile(parsed, "m", 0.5), 0.005, 0.0001
    end

    test "returns largest finite bound when the quantile lands in the +Inf bucket" do
      input = """
      m_bucket{le="0.01"} 50
      m_bucket{le="+Inf"} 100
      """

      parsed = PrometheusParser.parse(input)
      # p99 target = 99 > 50, only covered by +Inf → fall back to 0.01 (not infinity).
      assert PrometheusParser.histogram_quantile(parsed, "m", 0.99) == 0.01
    end

    test "returns default for an absent or empty histogram" do
      assert PrometheusParser.histogram_quantile(%{}, "missing", 0.99) == 0.0
      assert PrometheusParser.histogram_quantile(%{}, "missing", 0.99, nil) == nil

      zero = PrometheusParser.parse("m_bucket{le=\"+Inf\"} 0\n")
      assert PrometheusParser.histogram_quantile(zero, "m", 0.99, nil) == nil
    end
  end

  describe "full pipeline: parse -> summary" do
    test "parses sample output into a complete admin summary" do
      parsed = PrometheusParser.parse(@sample_prometheus_output)

      summary = %{
        uptime_seconds: PrometheusParser.get_metric(parsed, "allsource_uptime_seconds"),
        events_total: PrometheusParser.get_metric(parsed, "allsource_events_total"),
        events_per_second: PrometheusParser.get_metric(parsed, "allsource_events_per_second"),
        query_latency_p99_ms:
          PrometheusParser.get_quantile(
            parsed,
            "allsource_query_duration_seconds",
            "0.99"
          ) * 1000,
        error_rate_percent:
          Float.round(
            PrometheusParser.get_metric(parsed, "allsource_http_errors_total") /
              PrometheusParser.get_metric(parsed, "allsource_http_requests_total") * 100,
            2
          ),
        active_tenants: PrometheusParser.get_metric(parsed, "allsource_active_tenants")
      }

      assert summary.uptime_seconds == 86_400.0
      assert summary.events_total == 1_500_000.0
      assert summary.events_per_second == 469.2
      assert_in_delta summary.query_latency_p99_ms, 11.9, 0.01
      assert summary.error_rate_percent == 0.02
      assert summary.active_tenants == 12.0
    end
  end
end
