defmodule QueryServiceEx.Application.Services.CoreProducerTest do
  use ExUnit.Case, async: false

  alias QueryServiceEx.Application.Services.CoreProducer

  @moduletag :broadway

  describe "start_link/1" do
    test "starts with default options" do
      {:ok, pid} = CoreProducer.start_link(name: :test_producer_1)

      assert Process.alive?(pid)

      GenStage.stop(pid)
    end

    test "accepts custom options" do
      opts = [
        poll_interval_ms: 500,
        batch_size: 100,
        name: :test_producer_2
      ]

      {:ok, pid} = CoreProducer.start_link(opts)

      assert Process.alive?(pid)

      GenStage.stop(pid)
    end
  end

  describe "get_cursor/1" do
    test "returns current cursor" do
      {:ok, pid} = CoreProducer.start_link(name: :test_producer_3)

      cursor = CoreProducer.get_cursor(pid)
      assert is_binary(cursor)

      GenStage.stop(pid)
    end
  end

  describe "reset_cursor/2" do
    test "updates cursor" do
      {:ok, pid} = CoreProducer.start_link(name: :test_producer_4)

      new_cursor = "2025-01-01T00:00:00Z"
      assert :ok = CoreProducer.reset_cursor(pid, new_cursor)
      assert CoreProducer.get_cursor(pid) == new_cursor

      GenStage.stop(pid)
    end
  end

  describe "stats/1" do
    test "returns statistics" do
      {:ok, pid} = CoreProducer.start_link(name: :test_producer_5)

      stats = CoreProducer.stats(pid)

      assert is_map(stats)
      assert Map.has_key?(stats, :cursor)
      assert Map.has_key?(stats, :demand)
      assert Map.has_key?(stats, :events_fetched)
      assert Map.has_key?(stats, :polls_count)

      GenStage.stop(pid)
    end
  end

  describe "cursor persistence" do
    test "persists cursor to ETS" do
      {:ok, pid} = CoreProducer.start_link(name: :test_producer_6)

      new_cursor = "2025-06-15T12:00:00Z"
      CoreProducer.reset_cursor(pid, new_cursor)

      # Verify ETS has the cursor
      [{:event_cursor, stored}] = :ets.lookup(:core_producer_cursors, :event_cursor)
      assert stored == new_cursor

      GenStage.stop(pid)
    end
  end
end
