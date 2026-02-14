defmodule QueryServiceEx.UsageReporterTest do
  use ExUnit.Case, async: false

  alias QueryServiceEx.UsageReporter

  @moduletag :capture_log

  # Tests use a separate named instance (:test_usage_reporter) to avoid
  # interfering with the app-supervised UsageReporter.

  @test_name :test_usage_reporter

  defp success_sender(_tenant_id, _count), do: :ok
  defp failure_sender(_tenant_id, _count), do: {:error, :test_failure}

  defp tracking_sender(test_pid) do
    fn tenant_id, count ->
      send(test_pid, {:sent, tenant_id, count})
      :ok
    end
  end

  defp flaky_sender(counter_agent) do
    fn tenant_id, count ->
      current = Agent.get_and_update(counter_agent, fn n -> {n, n + 1} end)

      if current < 2 do
        {:error, :transient_failure}
      else
        send(self(), {:sent, tenant_id, count})
        :ok
      end
    end
  end

  defp start_reporter(opts \\ []) do
    defaults = [
      name: @test_name,
      flush_interval: 60_000,
      flush_threshold: 100,
      sender_fn: &success_sender/2,
      base_delay: 0
    ]

    {:ok, pid} = UsageReporter.start_link(Keyword.merge(defaults, opts))

    on_exit(fn ->
      if Process.alive?(pid) do
        ref = Process.monitor(pid)
        Process.exit(pid, :kill)

        receive do
          {:DOWN, ^ref, :process, ^pid, _} -> :ok
        after
          1_000 -> :ok
        end
      end
    end)

    pid
  end

  # Client helpers that target our test instance
  defp record(tenant_id, count \\ 1) do
    GenServer.cast(@test_name, {:record, tenant_id, count})
  end

  defp flush do
    GenServer.call(@test_name, :flush, 5_000)
  end

  defp pending do
    GenServer.call(@test_name, :pending)
  end

  test "record buffers increments without immediate flush" do
    start_reporter()

    record("tenant-1", 5)
    record("tenant-1", 3)
    record("tenant-2", 10)

    result = pending()
    assert result["tenant-1"] == 8
    assert result["tenant-2"] == 10
  end

  test "record defaults to count of 1" do
    start_reporter()

    record("tenant-1")

    result = pending()
    assert result["tenant-1"] == 1
  end

  test "flush clears the buffer on success" do
    start_reporter()

    record("tenant-1", 5)
    assert map_size(pending()) == 1

    flush()

    assert pending() == %{}
  end

  test "flush clears the buffer even on failure" do
    start_reporter(sender_fn: &failure_sender/2)

    record("tenant-1", 5)
    flush()

    assert pending() == %{}
  end

  test "pending returns empty map initially" do
    start_reporter()
    assert pending() == %{}
  end

  test "multiple records for same tenant accumulate" do
    start_reporter()

    for _ <- 1..50 do
      record("tenant-1", 1)
    end

    result = pending()
    assert result["tenant-1"] == 50
  end

  test "threshold triggers immediate flush" do
    tracking_fn = tracking_sender(self())
    start_reporter(flush_threshold: 5, sender_fn: tracking_fn)

    record("tenant-1", 5)

    assert_receive {:sent, "tenant-1", 5}, 1_000

    assert pending() == %{}
  end

  test "flush sends batched count to sender" do
    tracking_fn = tracking_sender(self())
    start_reporter(sender_fn: tracking_fn)

    record("tenant-1", 3)
    record("tenant-1", 7)
    record("tenant-2", 2)

    flush()

    assert_receive {:sent, "tenant-1", 10}
    assert_receive {:sent, "tenant-2", 2}
  end

  test "emits flushed telemetry on success" do
    ref =
      :telemetry_test.attach_event_handlers(self(), [
        [:query_service_ex, :usage_reporter, :flushed]
      ])

    start_reporter()

    record("tenant-1", 3)
    flush()

    assert_receive {[:query_service_ex, :usage_reporter, :flushed], ^ref, %{count: 3},
                    %{tenant_id: "tenant-1"}}
  end

  test "emits dropped telemetry on persistent failure" do
    ref =
      :telemetry_test.attach_event_handlers(self(), [
        [:query_service_ex, :usage_reporter, :dropped]
      ])

    start_reporter(sender_fn: &failure_sender/2)

    record("tenant-1", 3)
    flush()

    assert_receive {[:query_service_ex, :usage_reporter, :dropped], ^ref, %{count: 3},
                    %{tenant_id: "tenant-1"}}
  end

  test "retries on transient failure then succeeds" do
    {:ok, counter} = Agent.start_link(fn -> 0 end)
    flaky_fn = flaky_sender(counter)

    start_reporter(sender_fn: flaky_fn)

    record("tenant-1", 5)

    ref =
      :telemetry_test.attach_event_handlers(self(), [
        [:query_service_ex, :usage_reporter, :flushed]
      ])

    flush()

    assert_receive {[:query_service_ex, :usage_reporter, :flushed], ^ref, %{count: 5},
                    %{tenant_id: "tenant-1"}}

    Agent.stop(counter)
  end

  test "graceful shutdown flushes pending buffer" do
    ref =
      :telemetry_test.attach_event_handlers(self(), [
        [:query_service_ex, :usage_reporter, :flushed]
      ])

    pid = start_reporter()

    record("tenant-1", 7)

    GenServer.stop(pid, :normal, 5_000)

    assert_receive {[:query_service_ex, :usage_reporter, :flushed], ^ref, %{count: 7},
                    %{tenant_id: "tenant-1"}}
  end

  test "periodic flush via timer" do
    tracking_fn = tracking_sender(self())
    start_reporter(flush_interval: 100, sender_fn: tracking_fn)

    record("tenant-1", 3)

    assert_receive {:sent, "tenant-1", 3}, 1_000
  end
end
