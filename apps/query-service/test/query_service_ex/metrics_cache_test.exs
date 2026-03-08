defmodule QueryServiceEx.MetricsCacheTest do
  use ExUnit.Case, async: false

  alias QueryServiceEx.MetricsCache

  @moduletag :unit

  @sample_prometheus """
  allsource_uptime_seconds 86400
  allsource_events_total 1500000
  allsource_events_per_second 469.2
  allsource_query_duration_seconds{quantile="0.99"} 0.0119
  allsource_http_requests_total 100000
  allsource_http_errors_total 20
  allsource_active_tenants 12
  """

  setup do
    # Start a uniquely-named cache for each test to avoid conflicts
    name = :"metrics_cache_#{:erlang.unique_integer([:positive])}"
    {:ok, pid} = MetricsCache.start_link(name: name, ttl_ms: 100)

    on_exit(fn ->
      if Process.alive?(pid), do: GenServer.stop(pid)
    end)

    %{name: name}
  end

  describe "get_or_fetch/2" do
    test "calls fetch_fn on first request", %{name: name} do
      call_count = :counters.new(1, [:atomics])

      {:ok, parsed} =
        MetricsCache.get_or_fetch(
          fn ->
            :counters.add(call_count, 1, 1)
            {:ok, @sample_prometheus}
          end,
          name
        )

      assert is_map(parsed)
      assert Map.has_key?(parsed, "allsource_uptime_seconds")
      assert :counters.get(call_count, 1) == 1
    end

    test "returns cached result within TTL", %{name: name} do
      call_count = :counters.new(1, [:atomics])

      fetch_fn = fn ->
        :counters.add(call_count, 1, 1)
        {:ok, @sample_prometheus}
      end

      {:ok, _} = MetricsCache.get_or_fetch(fetch_fn, name)
      {:ok, _} = MetricsCache.get_or_fetch(fetch_fn, name)
      {:ok, _} = MetricsCache.get_or_fetch(fetch_fn, name)

      # Should only have been called once
      assert :counters.get(call_count, 1) == 1
    end

    test "refetches after TTL expires", %{name: name} do
      call_count = :counters.new(1, [:atomics])

      fetch_fn = fn ->
        :counters.add(call_count, 1, 1)
        {:ok, @sample_prometheus}
      end

      {:ok, _} = MetricsCache.get_or_fetch(fetch_fn, name)
      assert :counters.get(call_count, 1) == 1

      # Wait for TTL to expire (100ms configured in setup)
      Process.sleep(150)

      {:ok, _} = MetricsCache.get_or_fetch(fetch_fn, name)
      assert :counters.get(call_count, 1) == 2
    end

    test "returns stale cache on fetch error", %{name: name} do
      # First successful fetch
      {:ok, _} = MetricsCache.get_or_fetch(fn -> {:ok, @sample_prometheus} end, name)

      # Wait for TTL to expire
      Process.sleep(150)

      # Second fetch fails — should return stale cache
      {:ok, parsed} =
        MetricsCache.get_or_fetch(fn -> {:error, :connection_refused} end, name)

      assert is_map(parsed)
      assert Map.has_key?(parsed, "allsource_uptime_seconds")
    end

    test "propagates error when no cached data exists", %{name: name} do
      result =
        MetricsCache.get_or_fetch(fn -> {:error, :connection_refused} end, name)

      assert {:error, :connection_refused} = result
    end
  end

  describe "get_timeseries/3" do
    test "returns empty list when no data recorded", %{name: name} do
      {:ok, points} = MetricsCache.get_timeseries("events_total", "1h", name)
      assert points == []
    end

    test "returns data points after fetch populates timeseries", %{name: name} do
      {:ok, _} = MetricsCache.get_or_fetch(fn -> {:ok, @sample_prometheus} end, name)

      {:ok, points} = MetricsCache.get_timeseries("events_total", "1h", name)

      assert length(points) == 1
      assert [%{timestamp: ts, value: value}] = points
      assert is_binary(ts)
      assert value == 1_500_000.0
    end

    test "filters by time range", %{name: name} do
      # Populate with a data point
      {:ok, _} = MetricsCache.get_or_fetch(fn -> {:ok, @sample_prometheus} end, name)

      # All ranges should return the recent data point
      {:ok, points_1h} = MetricsCache.get_timeseries("events_total", "1h", name)
      {:ok, points_24h} = MetricsCache.get_timeseries("events_total", "24h", name)
      {:ok, points_7d} = MetricsCache.get_timeseries("events_total", "7d", name)

      assert length(points_1h) == 1
      assert length(points_24h) == 1
      assert length(points_7d) == 1
    end
  end

  describe "clear/1" do
    test "removes all cached data", %{name: name} do
      {:ok, _} = MetricsCache.get_or_fetch(fn -> {:ok, @sample_prometheus} end, name)

      :ok = MetricsCache.clear(name)

      # After clear, should need to fetch again
      call_count = :counters.new(1, [:atomics])

      {:ok, _} =
        MetricsCache.get_or_fetch(
          fn ->
            :counters.add(call_count, 1, 1)
            {:ok, @sample_prometheus}
          end,
          name
        )

      assert :counters.get(call_count, 1) == 1

      # Timeseries should also be cleared
      {:ok, points} = MetricsCache.get_timeseries("events_total", "1h", name)
      # 1 point from the new fetch above
      assert length(points) == 1
    end
  end
end
