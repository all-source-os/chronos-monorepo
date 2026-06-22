defmodule QueryServiceEx.UsageReporterTest do
  use ExUnit.Case, async: false

  alias QueryServiceEx.UsageReporter

  @moduletag :capture_log

  # Tests use a separate named instance (:test_usage_reporter) to avoid
  # interfering with the app-supervised UsageReporter.

  @test_name :test_usage_reporter

  defp success_sender(_tenant_id, _count, _meter), do: :ok
  defp failure_sender(_tenant_id, _count, _meter), do: {:error, :test_failure}

  defp tracking_sender(test_pid) do
    fn tenant_id, count, meter ->
      send(test_pid, {:sent, tenant_id, count, meter})
      :ok
    end
  end

  defp flaky_sender(counter_agent) do
    fn tenant_id, count, meter ->
      current = Agent.get_and_update(counter_agent, fn n -> {n, n + 1} end)

      if current < 2 do
        {:error, :transient_failure}
      else
        send(self(), {:sent, tenant_id, count, meter})
        :ok
      end
    end
  end

  defp start_reporter(opts \\ []) do
    defaults = [
      name: @test_name,
      flush_interval: 60_000,
      flush_threshold: 100,
      sender_fn: &success_sender/3,
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
  defp record(tenant_id, count \\ 1, meter \\ :events) do
    GenServer.cast(@test_name, {:record, tenant_id, count, meter})
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
    assert result[{"tenant-1", :events}] == 8
    assert result[{"tenant-2", :events}] == 10
  end

  test "record defaults to count of 1 and meter of events" do
    start_reporter()

    record("tenant-1")

    result = pending()
    assert result[{"tenant-1", :events}] == 1
  end

  test "events and queries buffer into separate keys" do
    start_reporter()

    record("tenant-1", 5, :events)
    record("tenant-1", 2, :queries)
    record("tenant-1", 1, :events)

    result = pending()
    assert result[{"tenant-1", :events}] == 6
    assert result[{"tenant-1", :queries}] == 2
  end

  test "flush clears the buffer on success" do
    start_reporter()

    record("tenant-1", 5)
    assert map_size(pending()) == 1

    flush()

    assert pending() == %{}
  end

  test "flush clears the buffer even on failure" do
    start_reporter(sender_fn: &failure_sender/3)

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
    assert result[{"tenant-1", :events}] == 50
  end

  test "threshold triggers immediate flush per meter" do
    tracking_fn = tracking_sender(self())
    start_reporter(flush_threshold: 5, sender_fn: tracking_fn)

    record("tenant-1", 5, :events)

    assert_receive {:sent, "tenant-1", 5, :events}, 1_000

    assert pending() == %{}
  end

  test "flush sends batched count and meter to sender" do
    tracking_fn = tracking_sender(self())
    start_reporter(sender_fn: tracking_fn)

    record("tenant-1", 3, :events)
    record("tenant-1", 7, :events)
    record("tenant-1", 4, :queries)
    record("tenant-2", 2, :events)

    flush()

    assert_receive {:sent, "tenant-1", 10, :events}
    assert_receive {:sent, "tenant-1", 4, :queries}
    assert_receive {:sent, "tenant-2", 2, :events}
  end

  test "emits flushed telemetry on success with meter metadata" do
    ref =
      :telemetry_test.attach_event_handlers(self(), [
        [:query_service_ex, :usage_reporter, :flushed]
      ])

    start_reporter()

    record("tenant-1", 3, :queries)
    flush()

    assert_receive {[:query_service_ex, :usage_reporter, :flushed], ^ref, %{count: 3},
                    %{tenant_id: "tenant-1", meter: :queries}}
  end

  test "emits dropped telemetry on persistent failure with meter metadata" do
    ref =
      :telemetry_test.attach_event_handlers(self(), [
        [:query_service_ex, :usage_reporter, :dropped]
      ])

    start_reporter(sender_fn: &failure_sender/3)

    record("tenant-1", 3, :events)
    flush()

    assert_receive {[:query_service_ex, :usage_reporter, :dropped], ^ref, %{count: 3},
                    %{tenant_id: "tenant-1", meter: :events}}
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
                    %{tenant_id: "tenant-1", meter: :events}}

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
                    %{tenant_id: "tenant-1", meter: :events}}
  end

  test "periodic flush via timer" do
    tracking_fn = tracking_sender(self())
    start_reporter(flush_interval: 100, sender_fn: tracking_fn)

    record("tenant-1", 3)

    assert_receive {:sent, "tenant-1", 3, :events}, 1_000
  end

  test "public record/3 rejects an unknown meter" do
    start_reporter()

    assert_raise FunctionClauseError, fn ->
      UsageReporter.record("tenant-1", 1, :bogus)
    end
  end
end
