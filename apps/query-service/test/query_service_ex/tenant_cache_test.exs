defmodule QueryServiceEx.TenantCacheTest do
  use ExUnit.Case, async: false

  alias QueryServiceEx.TenantCache

  setup do
    # Clear cache between tests
    TenantCache.clear()
    :ok
  end

  describe "fetch/2" do
    test "caches the result of the function" do
      call_count = :counters.new(1, [])

      result =
        TenantCache.fetch("tenant-1", fn ->
          :counters.add(call_count, 1, 1)
          %{id: "tenant-1", name: "Test"}
        end)

      assert result == %{id: "tenant-1", name: "Test"}
      assert :counters.get(call_count, 1) == 1

      # Second call should use cache
      result2 =
        TenantCache.fetch("tenant-1", fn ->
          :counters.add(call_count, 1, 1)
          %{id: "tenant-1", name: "Updated"}
        end)

      assert result2 == %{id: "tenant-1", name: "Test"}
      assert :counters.get(call_count, 1) == 1
    end

    test "does not cache nil results" do
      call_count = :counters.new(1, [])

      result =
        TenantCache.fetch("missing", fn ->
          :counters.add(call_count, 1, 1)
          nil
        end)

      assert result == nil
      assert :counters.get(call_count, 1) == 1

      # Should call again since nil wasn't cached
      TenantCache.fetch("missing", fn ->
        :counters.add(call_count, 1, 1)
        nil
      end)

      assert :counters.get(call_count, 1) == 2
    end
  end

  describe "invalidate/1" do
    test "removes a cached entry" do
      TenantCache.put("tenant-1", %{id: "tenant-1"})
      assert TenantCache.cached?("tenant-1")

      TenantCache.invalidate("tenant-1")
      refute TenantCache.cached?("tenant-1")
    end

    test "returns :ok even if entry doesn't exist" do
      assert TenantCache.invalidate("nonexistent") == :ok
    end
  end

  describe "cached?/1" do
    test "returns true for cached entries" do
      TenantCache.put("tenant-1", %{id: "tenant-1"})
      assert TenantCache.cached?("tenant-1")
    end

    test "returns false for missing entries" do
      refute TenantCache.cached?("missing")
    end

    test "returns false for expired entries" do
      TenantCache.put("tenant-1", %{id: "tenant-1"}, 0)
      Process.sleep(10)
      refute TenantCache.cached?("tenant-1")
    end
  end

  describe "clear/0" do
    test "removes all entries" do
      TenantCache.put("t1", %{id: "t1"})
      TenantCache.put("t2", %{id: "t2"})
      TenantCache.clear()
      refute TenantCache.cached?("t1")
      refute TenantCache.cached?("t2")
    end
  end
end
