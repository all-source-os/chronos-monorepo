defmodule QueryServiceEx.LoggingTest do
  @moduledoc """
  Tests for the Logging utility module.

  Verifies telemetry emission, measurement functions, and logger context management.
  """
  use ExUnit.Case, async: true

  alias QueryServiceEx.Logging

  describe "emit_telemetry/3" do
    setup do
      ref = make_ref()
      test_pid = self()

      :telemetry.attach(
        "test-logging-#{inspect(ref)}",
        [:test, :event],
        fn event, measurements, metadata, _config ->
          send(test_pid, {:telemetry, event, measurements, metadata})
        end,
        nil
      )

      on_exit(fn ->
        :telemetry.detach("test-logging-#{inspect(ref)}")
      end)

      :ok
    end

    test "emits telemetry event with measurements and metadata" do
      Logging.emit_telemetry([:test, :event], %{count: 1}, %{source: "test"})

      assert_receive {:telemetry, [:test, :event], %{count: 1}, %{source: "test"}}
    end

    test "emits telemetry with default empty measurements" do
      Logging.emit_telemetry([:test, :event])

      assert_receive {:telemetry, [:test, :event], %{}, %{}}
    end

    test "emits telemetry with default empty metadata" do
      Logging.emit_telemetry([:test, :event], %{value: 42})

      assert_receive {:telemetry, [:test, :event], %{value: 42}, %{}}
    end
  end

  describe "measure/3" do
    setup do
      ref = make_ref()
      test_pid = self()

      :telemetry.attach(
        "test-measure-#{inspect(ref)}",
        [:test, :measured, :operation],
        fn event, measurements, metadata, _config ->
          send(test_pid, {:telemetry, event, measurements, metadata})
        end,
        nil
      )

      on_exit(fn ->
        :telemetry.detach("test-measure-#{inspect(ref)}")
      end)

      :ok
    end

    test "executes function and returns result" do
      result =
        Logging.measure([:test, :measured, :operation], %{}, fn ->
          {:ok, "success"}
        end)

      assert result == {:ok, "success"}
    end

    test "emits telemetry with duration_ms" do
      Logging.measure([:test, :measured, :operation], %{}, fn ->
        # Use 50ms sleep for more reliable timing on slow CI systems
        Process.sleep(50)
        :ok
      end)

      assert_receive {:telemetry, [:test, :measured, :operation], measurements, _metadata}
      assert Map.has_key?(measurements, :duration_ms)
      # Use a lower bound that accounts for timing variability
      assert measurements.duration_ms >= 40
    end

    test "includes metadata in telemetry event" do
      Logging.measure([:test, :measured, :operation], %{query_type: "dsl"}, fn ->
        :ok
      end)

      assert_receive {:telemetry, [:test, :measured, :operation], _measurements,
                      %{query_type: "dsl"}}
    end

    test "uses default empty metadata" do
      Logging.measure([:test, :measured, :operation], fn -> :ok end)

      assert_receive {:telemetry, [:test, :measured, :operation], _measurements, %{}}
    end

    test "propagates raised exceptions" do
      assert_raise RuntimeError, "boom", fn ->
        Logging.measure([:test, :measured, :operation], fn ->
          raise "boom"
        end)
      end
    end
  end

  describe "set_context/1" do
    setup do
      # Clear Logger metadata before each test
      Logger.metadata([])
      on_exit(fn -> Logger.metadata([]) end)
      :ok
    end

    test "sets Logger metadata" do
      Logging.set_context(request_id: "req-123", user_id: "user-456")

      metadata = Logger.metadata()
      assert metadata[:request_id] == "req-123"
      assert metadata[:user_id] == "user-456"
    end

    test "merges with existing metadata" do
      Logger.metadata(existing: "value")
      Logging.set_context(new: "value")

      metadata = Logger.metadata()
      assert metadata[:existing] == "value"
      assert metadata[:new] == "value"
    end
  end

  describe "clear_context/0" do
    setup do
      Logger.metadata(some: "value", other: "data")
      :ok
    end

    test "clears all Logger metadata" do
      Logging.clear_context()

      metadata = Logger.metadata()
      assert metadata == []
    end
  end

  describe "with_context/2" do
    setup do
      Logger.metadata([])
      on_exit(fn -> Logger.metadata([]) end)
      :ok
    end

    test "sets context for the duration of the function" do
      result =
        Logging.with_context([request_id: "temp-123"], fn ->
          metadata = Logger.metadata()
          assert metadata[:request_id] == "temp-123"
          :function_result
        end)

      assert result == :function_result
    end

    test "restores original context after function completes" do
      Logger.metadata(original: "value")

      Logging.with_context([temporary: "context"], fn ->
        :ok
      end)

      metadata = Logger.metadata()
      assert metadata[:original] == "value"
      refute Keyword.has_key?(metadata, :temporary)
    end

    test "restores original context after function raises" do
      Logger.metadata(original: "value")

      assert_raise RuntimeError, fn ->
        Logging.with_context([temporary: "context"], fn ->
          raise "error"
        end)
      end

      metadata = Logger.metadata()
      assert metadata[:original] == "value"
      refute Keyword.has_key?(metadata, :temporary)
    end

    test "allows nested contexts" do
      Logging.with_context([level1: "a"], fn ->
        assert Logger.metadata()[:level1] == "a"

        Logging.with_context([level2: "b"], fn ->
          # Both should be present
          metadata = Logger.metadata()
          assert metadata[:level1] == "a"
          assert metadata[:level2] == "b"
        end)

        # After inner context, level2 should be gone
        metadata = Logger.metadata()
        assert metadata[:level1] == "a"
        refute Keyword.has_key?(metadata, :level2)
      end)

      # After outer context, level1 should be gone too
      metadata = Logger.metadata()
      refute Keyword.has_key?(metadata, :level1)
    end
  end
end
