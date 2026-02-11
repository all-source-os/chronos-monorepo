defmodule QueryServiceEx.APMTest do
  @moduledoc """
  Tests for the APM integration module.

  Verifies that APM events and metrics are correctly emitted via telemetry.
  """
  use ExUnit.Case, async: true

  alias QueryServiceEx.APM

  describe "config/0" do
    test "returns a keyword list" do
      config = APM.config()
      assert is_list(config)
    end
  end

  describe "enabled?/0" do
    test "returns false by default" do
      refute APM.enabled?()
    end
  end

  describe "provider/0" do
    test "returns :prometheus by default" do
      assert APM.provider() == :prometheus
    end
  end

  describe "service_name/0" do
    test "returns default service name" do
      assert APM.service_name() == "query-service"
    end
  end

  describe "default_tags/0" do
    test "includes service tag" do
      tags = APM.default_tags()
      assert Keyword.get(tags, :service) == "query-service"
    end

    test "includes runtime tag" do
      tags = APM.default_tags()
      assert Keyword.get(tags, :runtime) == "beam"
    end

    test "includes language tag" do
      tags = APM.default_tags()
      assert Keyword.get(tags, :language) == "elixir"
    end
  end

  describe "track_event/3" do
    test "emits telemetry event" do
      ref = make_ref()
      self_pid = self()

      :telemetry.attach(
        "test-apm-event-#{inspect(ref)}",
        [:query_service_ex, :apm, :event],
        fn _event, measurements, metadata, _config ->
          send(self_pid, {:telemetry, measurements, metadata})
        end,
        nil
      )

      APM.track_event("test.event", %{key: "value"})

      assert_receive {:telemetry, measurements, metadata}
      assert is_integer(measurements.timestamp)
      assert metadata.event_name == "test.event"
      assert metadata.attributes == %{key: "value"}

      :telemetry.detach("test-apm-event-#{inspect(ref)}")
    end

    test "includes custom tags" do
      ref = make_ref()
      self_pid = self()

      :telemetry.attach(
        "test-apm-event-tags-#{inspect(ref)}",
        [:query_service_ex, :apm, :event],
        fn _event, _measurements, metadata, _config ->
          send(self_pid, {:telemetry, metadata})
        end,
        nil
      )

      APM.track_event("test.event", %{}, tags: [custom: "tag"])

      assert_receive {:telemetry, metadata}
      assert Keyword.get(metadata.tags, :custom) == "tag"

      :telemetry.detach("test-apm-event-tags-#{inspect(ref)}")
    end
  end

  describe "track_metric/3" do
    test "emits telemetry metric" do
      ref = make_ref()
      self_pid = self()

      :telemetry.attach(
        "test-apm-metric-#{inspect(ref)}",
        [:query_service_ex, :apm, :metric],
        fn _event, measurements, metadata, _config ->
          send(self_pid, {:telemetry, measurements, metadata})
        end,
        nil
      )

      APM.track_metric("test.metric", 42.5)

      assert_receive {:telemetry, measurements, metadata}
      assert measurements.value == 42.5
      assert metadata.metric_name == "test.metric"
      assert metadata.metric_type == :gauge

      :telemetry.detach("test-apm-metric-#{inspect(ref)}")
    end

    test "supports custom metric type" do
      ref = make_ref()
      self_pid = self()

      :telemetry.attach(
        "test-apm-metric-type-#{inspect(ref)}",
        [:query_service_ex, :apm, :metric],
        fn _event, _measurements, metadata, _config ->
          send(self_pid, {:telemetry, metadata})
        end,
        nil
      )

      APM.track_metric("test.metric", 10, type: :counter)

      assert_receive {:telemetry, metadata}
      assert metadata.metric_type == :counter

      :telemetry.detach("test-apm-metric-type-#{inspect(ref)}")
    end
  end

  describe "track_histogram/3" do
    test "emits telemetry histogram" do
      ref = make_ref()
      self_pid = self()

      :telemetry.attach(
        "test-apm-histogram-#{inspect(ref)}",
        [:query_service_ex, :apm, :histogram],
        fn _event, measurements, metadata, _config ->
          send(self_pid, {:telemetry, measurements, metadata})
        end,
        nil
      )

      APM.track_histogram("test.histogram", 1024, unit: :bytes)

      assert_receive {:telemetry, measurements, metadata}
      assert measurements.value == 1024
      assert metadata.metric_name == "test.histogram"
      assert metadata.unit == :bytes

      :telemetry.detach("test-apm-histogram-#{inspect(ref)}")
    end
  end

  describe "increment/3" do
    test "emits telemetry counter" do
      ref = make_ref()
      self_pid = self()

      :telemetry.attach(
        "test-apm-counter-#{inspect(ref)}",
        [:query_service_ex, :apm, :counter],
        fn _event, measurements, metadata, _config ->
          send(self_pid, {:telemetry, measurements, metadata})
        end,
        nil
      )

      APM.increment("test.counter")

      assert_receive {:telemetry, measurements, metadata}
      assert measurements.count == 1
      assert metadata.metric_name == "test.counter"

      :telemetry.detach("test-apm-counter-#{inspect(ref)}")
    end

    test "supports custom count" do
      ref = make_ref()
      self_pid = self()

      :telemetry.attach(
        "test-apm-counter-custom-#{inspect(ref)}",
        [:query_service_ex, :apm, :counter],
        fn _event, measurements, _metadata, _config ->
          send(self_pid, {:telemetry, measurements})
        end,
        nil
      )

      APM.increment("test.counter", 10)

      assert_receive {:telemetry, measurements}
      assert measurements.count == 10

      :telemetry.detach("test-apm-counter-custom-#{inspect(ref)}")
    end
  end

  describe "with_span/3" do
    test "executes function and returns result" do
      result = APM.with_span("test.span", fn -> {:ok, 42} end)
      assert result == {:ok, 42}
    end

    test "emits span_start telemetry" do
      ref = make_ref()
      self_pid = self()

      :telemetry.attach(
        "test-apm-span-start-#{inspect(ref)}",
        [:query_service_ex, :apm, :span_start],
        fn _event, _measurements, metadata, _config ->
          send(self_pid, {:span_start, metadata})
        end,
        nil
      )

      APM.with_span("test.span", fn -> :ok end)

      assert_receive {:span_start, metadata}
      assert metadata.span_name == "test.span"
      assert is_binary(metadata.trace_id)
      assert is_binary(metadata.span_id)

      :telemetry.detach("test-apm-span-start-#{inspect(ref)}")
    end

    test "emits span_end telemetry with duration" do
      ref = make_ref()
      self_pid = self()

      :telemetry.attach(
        "test-apm-span-end-#{inspect(ref)}",
        [:query_service_ex, :apm, :span_end],
        fn _event, measurements, metadata, _config ->
          send(self_pid, {:span_end, measurements, metadata})
        end,
        nil
      )

      APM.with_span("test.span", fn ->
        Process.sleep(10)
        :ok
      end)

      assert_receive {:span_end, measurements, metadata}
      assert metadata.span_name == "test.span"
      assert metadata.status == :ok
      assert is_integer(measurements.duration)
      assert measurements.duration > 0

      :telemetry.detach("test-apm-span-end-#{inspect(ref)}")
    end

    test "captures errors and re-raises" do
      ref = make_ref()
      self_pid = self()

      :telemetry.attach(
        "test-apm-span-error-#{inspect(ref)}",
        [:query_service_ex, :apm, :span_end],
        fn _event, _measurements, metadata, _config ->
          send(self_pid, {:span_end, metadata})
        end,
        nil
      )

      assert_raise RuntimeError, "test error", fn ->
        APM.with_span("test.span", fn ->
          raise "test error"
        end)
      end

      assert_receive {:span_end, metadata}
      assert metadata.status == :error
      assert metadata.error == "test error"

      :telemetry.detach("test-apm-span-error-#{inspect(ref)}")
    end
  end

  describe "trace_context/0" do
    test "returns trace context map" do
      context = APM.trace_context()

      assert is_map(context)
      assert is_binary(context.trace_id)
      assert is_binary(context.span_id)
      assert context.sampled == true
      assert context.service == "query-service"
    end

    test "generates unique trace IDs" do
      context1 = APM.trace_context()
      context2 = APM.trace_context()

      assert context1.trace_id != context2.trace_id
    end
  end

  describe "health_metrics/0" do
    test "returns health metrics map" do
      metrics = APM.health_metrics()

      assert is_map(metrics)
      assert metrics.healthy == true
      assert is_integer(metrics.uptime_seconds)
      assert is_integer(metrics.memory_bytes)
      assert is_integer(metrics.process_count)
      assert is_integer(metrics.scheduler_count)
      assert is_binary(metrics.elixir_version)
    end

    test "memory is greater than zero" do
      metrics = APM.health_metrics()
      assert metrics.memory_bytes > 0
    end

    test "process count is greater than zero" do
      metrics = APM.health_metrics()
      assert metrics.process_count > 0
    end
  end
end
