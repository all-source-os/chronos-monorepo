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
