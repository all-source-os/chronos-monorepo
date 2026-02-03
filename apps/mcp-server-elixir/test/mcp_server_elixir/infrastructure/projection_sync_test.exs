defmodule McpServerElixir.Infrastructure.ProjectionSyncTest do
  use ExUnit.Case, async: false

  alias McpServerElixir.Infrastructure.ProjectionSync

  @ets_state_table :projection_state
  @ets_dirty_table :projection_dirty

  setup do
    # Clean up ETS tables before each test
    try do
      :ets.delete_all_objects(@ets_state_table)
    rescue
      ArgumentError -> :ok
    end

    try do
      :ets.delete_all_objects(@ets_dirty_table)
    rescue
      ArgumentError -> :ok
    end

    :ok
  end

  defp stop_server(pid) when is_pid(pid) do
    if Process.alive?(pid) do
      GenServer.stop(pid, :normal, 5000)
    end
  rescue
    _ -> :ok
  end

  defp stop_server(_), do: :ok

  describe "start_link/1" do
    test "returns :ignore when disabled" do
      assert :ignore = ProjectionSync.start_link(enabled: false)
    end

    test "starts successfully when enabled" do
      name = :"test_projection_sync_#{System.unique_integer([:positive])}"
      assert {:ok, pid} = ProjectionSync.start_link(enabled: true, name: name)
      assert Process.alive?(pid)
      stop_server(pid)
    end

    test "uses custom sync interval" do
      name = :"test_projection_sync_#{System.unique_integer([:positive])}"

      assert {:ok, pid} =
               ProjectionSync.start_link(enabled: true, name: name, sync_interval_ms: 500)

      stats = ProjectionSync.stats(name)
      assert stats.sync_interval_ms == 500

      stop_server(pid)
    end

    test "uses custom projections list" do
      name = :"test_projection_sync_#{System.unique_integer([:positive])}"

      assert {:ok, pid} =
               ProjectionSync.start_link(
                 enabled: true,
                 name: name,
                 projections: ["custom_projection"]
               )

      assert Process.alive?(pid)
      stop_server(pid)
    end
  end

  describe "get_state/3" do
    setup do
      name = :"test_projection_sync_#{System.unique_integer([:positive])}"
      {:ok, pid} = ProjectionSync.start_link(enabled: true, name: name, sync_interval_ms: 10_000)
      on_exit(fn -> stop_server(pid) end)
      %{server: name, pid: pid}
    end

    test "returns nil for non-existent state", %{server: server} do
      assert nil == ProjectionSync.get_state("entity_snapshots", "nonexistent", server)
    end

    test "returns state after put_state", %{server: server} do
      state = %{"name" => "Test User", "age" => 30}
      :ok = ProjectionSync.put_state("entity_snapshots", "user-123", state, server)

      assert ^state = ProjectionSync.get_state("entity_snapshots", "user-123", server)
    end

    test "returns correct state for different entities", %{server: server} do
      state1 = %{"name" => "User 1"}
      state2 = %{"name" => "User 2"}

      :ok = ProjectionSync.put_state("entity_snapshots", "user-1", state1, server)
      :ok = ProjectionSync.put_state("entity_snapshots", "user-2", state2, server)

      assert ^state1 = ProjectionSync.get_state("entity_snapshots", "user-1", server)
      assert ^state2 = ProjectionSync.get_state("entity_snapshots", "user-2", server)
    end

    test "returns correct state for different projections", %{server: server} do
      snapshot_state = %{"data" => "snapshot"}
      counter_state = %{"count" => 5}

      :ok = ProjectionSync.put_state("entity_snapshots", "entity-1", snapshot_state, server)
      :ok = ProjectionSync.put_state("event_counters", "entity-1", counter_state, server)

      assert ^snapshot_state = ProjectionSync.get_state("entity_snapshots", "entity-1", server)
      assert ^counter_state = ProjectionSync.get_state("event_counters", "entity-1", server)
    end
  end

  describe "put_state/4" do
    setup do
      name = :"test_projection_sync_#{System.unique_integer([:positive])}"
      {:ok, pid} = ProjectionSync.start_link(enabled: true, name: name, sync_interval_ms: 10_000)
      on_exit(fn -> stop_server(pid) end)
      %{server: name, pid: pid}
    end

    test "stores state successfully", %{server: server} do
      state = %{"key" => "value"}
      assert :ok = ProjectionSync.put_state("test_projection", "entity-1", state, server)
    end

    test "updates existing state", %{server: server} do
      initial_state = %{"version" => 1}
      updated_state = %{"version" => 2}

      :ok = ProjectionSync.put_state("test_projection", "entity-1", initial_state, server)
      :ok = ProjectionSync.put_state("test_projection", "entity-1", updated_state, server)

      assert ^updated_state = ProjectionSync.get_state("test_projection", "entity-1", server)
    end

    test "marks state as dirty", %{server: server} do
      :ok = ProjectionSync.put_state("test_projection", "entity-1", %{"data" => "test"}, server)

      stats = ProjectionSync.stats(server)
      assert stats.dirty_count >= 1
    end

    test "handles complex nested state", %{server: server} do
      complex_state = %{
        "user" => %{
          "id" => "user-123",
          "profile" => %{
            "name" => "John",
            "settings" => %{"theme" => "dark"}
          },
          "roles" => ["admin", "user"]
        }
      }

      :ok = ProjectionSync.put_state("test_projection", "entity-1", complex_state, server)

      result = ProjectionSync.get_state("test_projection", "entity-1", server)
      assert result["user"]["profile"]["name"] == "John"
      assert result["user"]["roles"] == ["admin", "user"]
    end
  end

  describe "delete_state/3" do
    setup do
      name = :"test_projection_sync_#{System.unique_integer([:positive])}"
      {:ok, pid} = ProjectionSync.start_link(enabled: true, name: name, sync_interval_ms: 10_000)
      on_exit(fn -> stop_server(pid) end)
      %{server: name, pid: pid}
    end

    test "deletes existing state", %{server: server} do
      :ok = ProjectionSync.put_state("test_projection", "entity-1", %{"data" => "test"}, server)
      assert %{"data" => "test"} = ProjectionSync.get_state("test_projection", "entity-1", server)

      :ok = ProjectionSync.delete_state("test_projection", "entity-1", server)
      assert nil == ProjectionSync.get_state("test_projection", "entity-1", server)
    end

    test "handles deleting non-existent state", %{server: server} do
      assert :ok = ProjectionSync.delete_state("test_projection", "nonexistent", server)
    end

    test "removes from dirty tracking", %{server: server} do
      :ok = ProjectionSync.put_state("test_projection", "entity-1", %{"data" => "test"}, server)

      initial_stats = ProjectionSync.stats(server)
      initial_dirty = initial_stats.dirty_count

      :ok = ProjectionSync.delete_state("test_projection", "entity-1", server)

      final_stats = ProjectionSync.stats(server)
      assert final_stats.dirty_count < initial_dirty
    end
  end

  describe "sync_now/1" do
    setup do
      name = :"test_projection_sync_#{System.unique_integer([:positive])}"
      {:ok, pid} = ProjectionSync.start_link(enabled: true, name: name, sync_interval_ms: 10_000)
      on_exit(fn -> stop_server(pid) end)
      %{server: name, pid: pid}
    end

    test "returns ok with 0 when nothing dirty", %{server: server} do
      assert {:ok, 0} = ProjectionSync.sync_now(server)
    end

    test "attempts to sync dirty state", %{server: server} do
      :ok = ProjectionSync.put_state("test_projection", "entity-1", %{"data" => "test"}, server)

      # This will fail since Core isn't running, but should handle gracefully
      result = ProjectionSync.sync_now(server)
      # Either succeeds or fails gracefully
      assert match?({:ok, _}, result) or match?({:error, _}, result)
    end
  end

  describe "stats/1" do
    setup do
      name = :"test_projection_sync_#{System.unique_integer([:positive])}"
      {:ok, pid} = ProjectionSync.start_link(enabled: true, name: name, sync_interval_ms: 10_000)
      on_exit(fn -> stop_server(pid) end)
      %{server: name, pid: pid}
    end

    test "returns correct statistics structure", %{server: server} do
      stats = ProjectionSync.stats(server)

      assert is_integer(stats.synced_count)
      assert is_integer(stats.sync_errors)
      assert is_integer(stats.sync_interval_ms)
      assert is_integer(stats.dirty_count)
      assert is_integer(stats.state_count)
      assert is_boolean(stats.core_available)
      assert %DateTime{} = stats.started_at
    end

    test "tracks state count correctly", %{server: server} do
      initial_stats = ProjectionSync.stats(server)
      initial_count = initial_stats.state_count

      :ok = ProjectionSync.put_state("test", "entity-1", %{"a" => 1}, server)
      :ok = ProjectionSync.put_state("test", "entity-2", %{"b" => 2}, server)

      stats = ProjectionSync.stats(server)
      assert stats.state_count == initial_count + 2
    end

    test "tracks dirty count correctly", %{server: server} do
      initial_stats = ProjectionSync.stats(server)
      initial_dirty = initial_stats.dirty_count

      :ok = ProjectionSync.put_state("test", "entity-1", %{"a" => 1}, server)
      stats_after = ProjectionSync.stats(server)
      assert stats_after.dirty_count == initial_dirty + 1
    end
  end

  describe "healthy?/1" do
    setup do
      name = :"test_projection_sync_#{System.unique_integer([:positive])}"
      {:ok, pid} = ProjectionSync.start_link(enabled: true, name: name, sync_interval_ms: 10_000)
      on_exit(fn -> stop_server(pid) end)
      %{server: name, pid: pid}
    end

    test "returns true when healthy", %{server: server} do
      assert true == ProjectionSync.healthy?(server)
    end
  end

  describe "ETS performance" do
    setup do
      name = :"test_projection_sync_#{System.unique_integer([:positive])}"
      {:ok, pid} = ProjectionSync.start_link(enabled: true, name: name, sync_interval_ms: 10_000)
      on_exit(fn -> stop_server(pid) end)
      %{server: name, pid: pid}
    end

    test "read operations are fast (sub-ms)", %{server: server} do
      # Populate with some data
      Enum.each(1..100, fn i ->
        ProjectionSync.put_state("test", "entity-#{i}", %{"index" => i}, server)
      end)

      # Measure read time
      {time_us, _result} =
        :timer.tc(fn ->
          Enum.each(1..1000, fn i ->
            ProjectionSync.get_state("test", "entity-#{rem(i, 100) + 1}", server)
          end)
        end)

      # 1000 reads should complete in well under 1 second (sub-ms per read)
      avg_time_us = time_us / 1000
      assert avg_time_us < 1000, "Average read time #{avg_time_us}us should be under 1ms"
    end

    test "handles many entities efficiently", %{server: server} do
      # Insert 1000 entities
      Enum.each(1..1000, fn i ->
        ProjectionSync.put_state(
          "test",
          "entity-#{i}",
          %{"index" => i, "data" => "test data"},
          server
        )
      end)

      stats = ProjectionSync.stats(server)
      assert stats.state_count >= 1000
    end
  end

  describe "dirty flag tracking" do
    setup do
      name = :"test_projection_sync_#{System.unique_integer([:positive])}"
      {:ok, pid} = ProjectionSync.start_link(enabled: true, name: name, sync_interval_ms: 10_000)
      on_exit(fn -> stop_server(pid) end)
      %{server: name, pid: pid}
    end

    test "multiple puts to same key only count once as dirty", %{server: server} do
      :ok = ProjectionSync.put_state("test", "entity-1", %{"v" => 1}, server)
      :ok = ProjectionSync.put_state("test", "entity-1", %{"v" => 2}, server)
      :ok = ProjectionSync.put_state("test", "entity-1", %{"v" => 3}, server)

      stats = ProjectionSync.stats(server)
      # Should only be 1 dirty entry, not 3
      assert stats.dirty_count == 1
    end

    test "different keys are tracked separately", %{server: server} do
      :ok = ProjectionSync.put_state("test", "entity-1", %{"a" => 1}, server)
      :ok = ProjectionSync.put_state("test", "entity-2", %{"b" => 2}, server)
      :ok = ProjectionSync.put_state("test", "entity-3", %{"c" => 3}, server)

      stats = ProjectionSync.stats(server)
      assert stats.dirty_count == 3
    end
  end

  describe "configuration" do
    test "uses default sync interval when not configured" do
      name = :"test_projection_sync_#{System.unique_integer([:positive])}"
      {:ok, pid} = ProjectionSync.start_link(enabled: true, name: name)

      stats = ProjectionSync.stats(name)
      assert stats.sync_interval_ms == 100

      stop_server(pid)
    end

    test "respects custom sync interval" do
      name = :"test_projection_sync_#{System.unique_integer([:positive])}"
      {:ok, pid} = ProjectionSync.start_link(enabled: true, name: name, sync_interval_ms: 250)

      stats = ProjectionSync.stats(name)
      assert stats.sync_interval_ms == 250

      stop_server(pid)
    end
  end

  describe "graceful degradation" do
    setup do
      name = :"test_projection_sync_#{System.unique_integer([:positive])}"
      {:ok, pid} = ProjectionSync.start_link(enabled: true, name: name, sync_interval_ms: 10_000)
      on_exit(fn -> stop_server(pid) end)
      %{server: name, pid: pid}
    end

    test "continues operating when Core unavailable", %{server: server} do
      # Add state locally - should work even without Core
      :ok = ProjectionSync.put_state("test", "entity-1", %{"data" => "test"}, server)
      assert %{"data" => "test"} = ProjectionSync.get_state("test", "entity-1", server)

      # Sync will fail but shouldn't crash
      _result = ProjectionSync.sync_now(server)

      # Still operational
      assert ProjectionSync.healthy?(server)
      :ok = ProjectionSync.put_state("test", "entity-2", %{"data" => "test2"}, server)
      assert %{"data" => "test2"} = ProjectionSync.get_state("test", "entity-2", server)
    end
  end

  describe "load_from_core/3" do
    setup do
      name = :"test_projection_sync_#{System.unique_integer([:positive])}"
      {:ok, pid} = ProjectionSync.start_link(enabled: true, name: name, sync_interval_ms: 10_000)
      on_exit(fn -> stop_server(pid) end)
      %{server: name, pid: pid}
    end

    test "handles empty entity list", %{server: server} do
      result = ProjectionSync.load_from_core("test_projection", [], server)
      # Will fail since Core isn't running, but handles gracefully
      assert match?({:ok, _}, result) or match?({:error, _}, result)
    end

    test "handles Core unavailability gracefully", %{server: server, pid: pid} do
      result = ProjectionSync.load_from_core("test_projection", ["entity-1"], server)
      # Should return error but not crash
      assert match?({:ok, _}, result) or match?({:error, _}, result)
      assert Process.alive?(pid)
    end
  end

  describe "concurrent access" do
    setup do
      name = :"test_projection_sync_#{System.unique_integer([:positive])}"
      {:ok, pid} = ProjectionSync.start_link(enabled: true, name: name, sync_interval_ms: 10_000)
      on_exit(fn -> stop_server(pid) end)
      %{server: name, pid: pid}
    end

    test "handles concurrent reads and writes", %{server: server} do
      # Start multiple concurrent tasks
      tasks =
        Enum.map(1..20, fn i ->
          Task.async(fn ->
            # Each task does multiple operations
            Enum.each(1..10, fn j ->
              entity_id = "entity-#{i}-#{j}"
              ProjectionSync.put_state("test", entity_id, %{"i" => i, "j" => j}, server)
              ProjectionSync.get_state("test", entity_id, server)
            end)
          end)
        end)

      # Wait for all tasks to complete
      Enum.each(tasks, &Task.await(&1, 5000))

      stats = ProjectionSync.stats(server)
      assert stats.state_count == 200
    end
  end

  describe "state types" do
    setup do
      name = :"test_projection_sync_#{System.unique_integer([:positive])}"
      {:ok, pid} = ProjectionSync.start_link(enabled: true, name: name, sync_interval_ms: 10_000)
      on_exit(fn -> stop_server(pid) end)
      %{server: name, pid: pid}
    end

    test "handles map with string keys", %{server: server} do
      state = %{"key1" => "value1", "key2" => "value2"}
      :ok = ProjectionSync.put_state("test", "entity", state, server)
      assert ^state = ProjectionSync.get_state("test", "entity", server)
    end

    test "handles map with atom keys", %{server: server} do
      state = %{key1: "value1", key2: "value2"}
      :ok = ProjectionSync.put_state("test", "entity", state, server)
      assert ^state = ProjectionSync.get_state("test", "entity", server)
    end

    test "handles nested maps", %{server: server} do
      state = %{
        "level1" => %{
          "level2" => %{
            "level3" => "deep value"
          }
        }
      }

      :ok = ProjectionSync.put_state("test", "entity", state, server)
      result = ProjectionSync.get_state("test", "entity", server)
      assert result["level1"]["level2"]["level3"] == "deep value"
    end

    test "handles lists in state", %{server: server} do
      state = %{"items" => [1, 2, 3, 4, 5]}
      :ok = ProjectionSync.put_state("test", "entity", state, server)
      result = ProjectionSync.get_state("test", "entity", server)
      assert result["items"] == [1, 2, 3, 4, 5]
    end

    test "handles mixed types", %{server: server} do
      state = %{
        "string" => "value",
        "number" => 42,
        "float" => 3.14,
        "boolean" => true,
        "null" => nil,
        "list" => [1, "two", 3.0],
        "nested" => %{"a" => 1}
      }

      :ok = ProjectionSync.put_state("test", "entity", state, server)
      result = ProjectionSync.get_state("test", "entity", server)
      assert result["string"] == "value"
      assert result["number"] == 42
      assert result["boolean"] == true
      assert result["null"] == nil
    end
  end
end
