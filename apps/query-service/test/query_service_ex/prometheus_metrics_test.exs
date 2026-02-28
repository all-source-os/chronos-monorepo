defmodule QueryServiceEx.PrometheusMetricsTest do
  @moduledoc """
  Tests for the Prometheus metrics exporter.

  Verifies that metrics are correctly defined and exposed in Prometheus format.
  """
  use ExUnit.Case, async: false

  alias QueryServiceEx.PrometheusMetrics

  # Helper to convert metric name to a string for comparison
  # Metric names can be atoms, lists, or strings
  defp metric_name_to_string(name) when is_list(name) do
    Enum.map_join(name, ".", &to_string/1)
  end

  defp metric_name_to_string(name) when is_atom(name), do: Atom.to_string(name)
  defp metric_name_to_string(name) when is_binary(name), do: name

  describe "metrics/0" do
    test "returns a list of metric definitions" do
      metrics = PrometheusMetrics.metrics()
      assert is_list(metrics)
      assert metrics != []
    end

    test "includes HTTP metrics" do
      metrics = PrometheusMetrics.metrics()
      metric_names = Enum.map(metrics, &metric_name_to_string(&1.name))

      assert Enum.any?(metric_names, &String.contains?(&1, "phoenix.endpoint"))
    end

    test "includes event processing metrics" do
      metrics = PrometheusMetrics.metrics()
      metric_names = Enum.map(metrics, &metric_name_to_string(&1.name))

      assert Enum.any?(metric_names, &String.contains?(&1, "event_pipeline.event_processed"))
    end

    test "includes projection metrics" do
      metrics = PrometheusMetrics.metrics()
      metric_names = Enum.map(metrics, &metric_name_to_string(&1.name))

      assert Enum.any?(metric_names, &String.contains?(&1, "projection.sync"))
    end

    test "includes WebSocket metrics" do
      metrics = PrometheusMetrics.metrics()
      metric_names = Enum.map(metrics, &metric_name_to_string(&1.name))

      assert Enum.any?(metric_names, &String.contains?(&1, "websocket"))
    end

    test "includes circuit breaker metrics" do
      metrics = PrometheusMetrics.metrics()
      metric_names = Enum.map(metrics, &metric_name_to_string(&1.name))

      assert Enum.any?(metric_names, &String.contains?(&1, "circuit_breaker"))
    end

    test "includes VM metrics" do
      metrics = PrometheusMetrics.metrics()
      metric_names = Enum.map(metrics, &metric_name_to_string(&1.name))

      assert Enum.any?(metric_names, &String.contains?(&1, "vm.memory"))
    end

    test "includes channel metrics" do
      metrics = PrometheusMetrics.metrics()
      metric_names = Enum.map(metrics, &metric_name_to_string(&1.name))

      assert Enum.any?(metric_names, &String.contains?(&1, "channel.joined"))
      assert Enum.any?(metric_names, &String.contains?(&1, "channel.left"))
    end
  end

  describe "metric types" do
    test "includes counter metrics" do
      metrics = PrometheusMetrics.metrics()
      counters = Enum.filter(metrics, &(&1.__struct__ == Telemetry.Metrics.Counter))

      assert counters != []
    end

    test "includes distribution metrics" do
      metrics = PrometheusMetrics.metrics()
      distributions = Enum.filter(metrics, &(&1.__struct__ == Telemetry.Metrics.Distribution))

      assert distributions != []
    end

    test "includes last_value metrics" do
      metrics = PrometheusMetrics.metrics()
      last_values = Enum.filter(metrics, &(&1.__struct__ == Telemetry.Metrics.LastValue))

      assert last_values != []
    end
  end

  describe "metric tags" do
    test "HTTP metrics have appropriate tags" do
      metrics = PrometheusMetrics.metrics()

      http_metric =
        Enum.find(metrics, fn m ->
          metric_name_to_string(m.name) |> String.contains?("phoenix.endpoint.stop.count")
        end)

      assert http_metric != nil
      assert :method in http_metric.tags
      assert :status in http_metric.tags
    end

    test "event processing metrics have event_type tag" do
      metrics = PrometheusMetrics.metrics()

      event_metric =
        Enum.find(metrics, fn m ->
          metric_name_to_string(m.name)
          |> String.contains?("event_pipeline.event_processed.count")
        end)

      assert event_metric != nil
      assert :event_type in event_metric.tags
    end

    test "projection metrics have projection_name tag" do
      metrics = PrometheusMetrics.metrics()

      projection_metric =
        Enum.find(metrics, fn m ->
          metric_name_to_string(m.name) |> String.contains?("projection.sync_completed.count")
        end)

      assert projection_metric != nil
      assert :projection_name in projection_metric.tags
    end
  end

  describe "metric descriptions" do
    test "all metrics have descriptions" do
      metrics = PrometheusMetrics.metrics()

      for metric <- metrics do
        name_str = metric_name_to_string(metric.name)
        assert metric.description != nil, "Metric #{name_str} should have a description"
        assert metric.description != "", "Metric #{name_str} should have a non-empty description"
      end
    end
  end
end
