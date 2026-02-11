defmodule QueryServiceEx.Infrastructure.Adapters.AnalyticsCacheTest do
  use ExUnit.Case, async: false

  alias QueryServiceEx.Infrastructure.Adapters.AnalyticsCache

  @moduletag :unit

  setup do
    # Ensure cache is started for tests
    case GenServer.whereis(AnalyticsCache) do
      nil ->
        {:ok, _pid} = AnalyticsCache.start_link([])

      _pid ->
        AnalyticsCache.clear()
    end

    on_exit(fn ->
      AnalyticsCache.clear()
    end)

    :ok
  end

  describe "put/3 and get/1" do
    test "stores and retrieves values" do
      AnalyticsCache.put(:test_key, %{value: 42}, 300)

      assert {:ok, %{value: 42}} = AnalyticsCache.get(:test_key)
    end

    test "returns :miss for non-existent keys" do
      assert :miss = AnalyticsCache.get(:non_existent)
    end

    test "returns :miss for expired entries" do
      # Use a 0-second TTL so it expires immediately
      AnalyticsCache.put(:expired_key, "value", 0)

      # Sleep briefly to ensure expiration
      Process.sleep(10)

      assert :miss = AnalyticsCache.get(:expired_key)
    end

    test "accepts various key types" do
      AnalyticsCache.put({:tuple, "key"}, "tuple_value", 300)
      AnalyticsCache.put("string_key", "string_value", 300)
      AnalyticsCache.put(12_345, "integer_value", 300)

      assert {:ok, "tuple_value"} = AnalyticsCache.get({:tuple, "key"})
      assert {:ok, "string_value"} = AnalyticsCache.get("string_key")
      assert {:ok, "integer_value"} = AnalyticsCache.get(12_345)
    end
  end

  describe "fetch/3" do
    test "returns cached value without calling function" do
      AnalyticsCache.put(:cached_key, "cached_value", 300)

      result =
        AnalyticsCache.fetch(:cached_key, 300, fn ->
          raise "Should not be called"
        end)

      # fetch/3 returns the raw value, not wrapped in {:ok, ...}
      assert result == "cached_value"
    end

    test "calls function and caches result on miss" do
      call_count = :counters.new(1, [:atomics])

      result =
        AnalyticsCache.fetch(:new_key, 300, fn ->
          :counters.add(call_count, 1, 1)
          {:ok, "computed_value"}
        end)

      assert result == {:ok, "computed_value"}
      assert :counters.get(call_count, 1) == 1

      # Second call should use cache
      result2 =
        AnalyticsCache.fetch(:new_key, 300, fn ->
          :counters.add(call_count, 1, 1)
          {:ok, "new_value"}
        end)

      assert result2 == {:ok, "computed_value"}
      assert :counters.get(call_count, 1) == 1
    end

    test "respects TTL for cached results" do
      # First call - cache miss
      result1 =
        AnalyticsCache.fetch(:ttl_key, 0, fn ->
          {:ok, "first_value"}
        end)

      assert result1 == {:ok, "first_value"}

      # Sleep briefly to ensure expiration
      Process.sleep(10)

      # Second call - should recompute due to expiration
      result2 =
        AnalyticsCache.fetch(:ttl_key, 300, fn ->
          {:ok, "second_value"}
        end)

      assert result2 == {:ok, "second_value"}
    end
  end

  describe "delete/1" do
    test "removes entry from cache" do
      AnalyticsCache.put(:delete_key, "value", 300)
      assert {:ok, "value"} = AnalyticsCache.get(:delete_key)

      AnalyticsCache.delete(:delete_key)
      assert :miss = AnalyticsCache.get(:delete_key)
    end

    test "is idempotent for non-existent keys" do
      assert :ok = AnalyticsCache.delete(:non_existent)
    end
  end

  describe "clear/0" do
    test "removes all entries" do
      AnalyticsCache.put(:key1, "value1", 300)
      AnalyticsCache.put(:key2, "value2", 300)
      AnalyticsCache.put(:key3, "value3", 300)

      AnalyticsCache.clear()

      assert :miss = AnalyticsCache.get(:key1)
      assert :miss = AnalyticsCache.get(:key2)
      assert :miss = AnalyticsCache.get(:key3)
    end
  end

  describe "stats/0" do
    test "returns cache statistics" do
      AnalyticsCache.clear()
      AnalyticsCache.put(:stats_key1, "value1", 300)
      AnalyticsCache.put(:stats_key2, "value2", 300)

      stats = AnalyticsCache.stats()

      assert is_map(stats)
      assert Map.has_key?(stats, :size)
      assert Map.has_key?(stats, :memory)
      assert stats.size == 2
      assert is_integer(stats.memory)
      assert stats.memory > 0
    end
  end

  describe "invalidate_tenant/1" do
    test "removes entries for specific tenant" do
      # Store entries with tenant-prefixed keys (matching AnalyticsClient pattern)
      AnalyticsCache.put({:frequency, "tenant-1", %{window: "day"}}, "data1", 300)
      AnalyticsCache.put({:summary, "tenant-1", %{}}, "data2", 300)
      AnalyticsCache.put({:frequency, "tenant-2", %{window: "day"}}, "data3", 300)

      AnalyticsCache.invalidate_tenant("tenant-1")

      # Tenant-1 entries should be gone
      assert :miss = AnalyticsCache.get({:frequency, "tenant-1", %{window: "day"}})
      assert :miss = AnalyticsCache.get({:summary, "tenant-1", %{}})

      # Tenant-2 entries should remain
      assert {:ok, "data3"} = AnalyticsCache.get({:frequency, "tenant-2", %{window: "day"}})
    end
  end

  describe "concurrent access" do
    test "handles concurrent reads and writes" do
      tasks =
        for i <- 1..100 do
          Task.async(fn ->
            key = :"concurrent_key_#{rem(i, 10)}"
            value = "value_#{i}"

            # Random mix of operations
            case rem(i, 3) do
              0 -> AnalyticsCache.put(key, value, 300)
              1 -> AnalyticsCache.get(key)
              2 -> AnalyticsCache.fetch(key, 300, fn -> {:ok, value} end)
            end
          end)
        end

      # All tasks should complete without errors
      results = Task.await_many(tasks, 5000)
      assert length(results) == 100
    end
  end
end
