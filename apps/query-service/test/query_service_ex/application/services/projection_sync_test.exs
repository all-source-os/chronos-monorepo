defmodule QueryServiceEx.Application.Services.ProjectionSyncTest do
  use ExUnit.Case, async: false

  alias QueryServiceEx.Application.Services.ProjectionSync

  @moduletag :projection_sync

  # Helper to wait for async GenServer operations with exponential backoff
  # This handles CI timing variability better than fixed sleeps
  defp wait_for_genserver(max_attempts \\ 10, initial_delay \\ 20) do
    Enum.each(1..max_attempts, fn attempt ->
      delay = min(initial_delay * attempt, 200)
      Process.sleep(delay)
    end)
  end

  setup do
    # Initialize ETS cache
    ProjectionSync.init_cache()

    # Clean up any existing entries
    :ets.delete_all_objects(:projection_state_cache)

    :ok
  end

  describe "init_cache/0" do
    test "creates ETS table" do
      assert :ok = ProjectionSync.init_cache()
      assert :ets.whereis(:projection_state_cache) != :undefined
    end

    test "is idempotent" do
      assert :ok = ProjectionSync.init_cache()
      assert :ok = ProjectionSync.init_cache()
    end
  end

  describe "get_state_from_cache/2" do
    test "returns nil when not found" do
      assert nil == ProjectionSync.get_state_from_cache("test", "unknown")
    end

    test "returns cached state" do
      :ets.insert(:projection_state_cache, {{"test", "entity-1"}, %{name: "Test"}})

      state = ProjectionSync.get_state_from_cache("test", "entity-1")
      assert state == %{name: "Test"}
    end
  end

  describe "start_link/1" do
    test "starts a process with required options" do
      opts = [
        projection_name: "test_projection",
        entity_id: "entity-123",
        name: :test_sync_1
      ]

      assert {:ok, pid} = ProjectionSync.start_link(opts)
      assert Process.alive?(pid)

      # Clean up
      GenServer.stop(pid)
    end

    test "loads initial state from opts when Core unavailable" do
      initial = %{balance: 100}

      opts = [
        projection_name: "test_projection",
        entity_id: "entity-456",
        initial_state: initial,
        name: :test_sync_2
      ]

      {:ok, pid} = ProjectionSync.start_link(opts)

      # State should be the initial state (Core not running)
      state = ProjectionSync.get_state(pid)
      assert state == initial

      GenServer.stop(pid)
    end
  end

  describe "apply_event/2" do
    test "updates state with event payload" do
      opts = [
        projection_name: "user_state",
        entity_id: "user-789",
        initial_state: %{},
        name: :test_sync_3
      ]

      {:ok, pid} = ProjectionSync.start_link(opts)

      # Apply an event
      event = %{"payload" => %{"name" => "John", "age" => 30}}
      ProjectionSync.apply_event(pid, event)

      # Give it a moment to process
      wait_for_genserver()

      state = ProjectionSync.get_state(pid)
      assert state["name"] == "John"
      assert state["age"] == 30

      GenServer.stop(pid)
    end

    test "marks state as dirty" do
      opts = [
        projection_name: "dirty_test",
        entity_id: "entity-dirty",
        initial_state: %{},
        name: :test_sync_4
      ]

      {:ok, pid} = ProjectionSync.start_link(opts)

      # Initially not dirty
      refute ProjectionSync.dirty?(pid)

      # Apply event
      ProjectionSync.apply_event(pid, %{"payload" => %{"value" => 1}})
      wait_for_genserver()

      # Now dirty
      assert ProjectionSync.dirty?(pid)

      GenServer.stop(pid)
    end
  end

  describe "ETS cache integration" do
    test "updates ETS cache when event applied" do
      opts = [
        projection_name: "cache_test",
        entity_id: "entity-cache",
        initial_state: %{},
        name: :test_sync_5
      ]

      {:ok, pid} = ProjectionSync.start_link(opts)

      # Apply event
      ProjectionSync.apply_event(pid, %{"payload" => %{"cached" => true}})
      wait_for_genserver()

      # Check ETS cache directly
      cached = ProjectionSync.get_state_from_cache("cache_test", "entity-cache")
      assert cached["cached"] == true

      GenServer.stop(pid)
    end
  end

  describe "custom project function" do
    test "uses custom function to apply events" do
      custom_fn = fn state, _event ->
        count = Map.get(state, :count, 0)
        Map.put(state, :count, count + 1)
      end

      opts = [
        projection_name: "custom_fn",
        entity_id: "entity-custom",
        initial_state: %{count: 0},
        project_fn: custom_fn,
        name: :test_sync_6
      ]

      {:ok, pid} = ProjectionSync.start_link(opts)

      # Apply multiple events
      ProjectionSync.apply_event(pid, %{})
      ProjectionSync.apply_event(pid, %{})
      ProjectionSync.apply_event(pid, %{})
      wait_for_genserver()

      state = ProjectionSync.get_state(pid)
      assert state.count == 3

      GenServer.stop(pid)
    end
  end

  describe "force_sync/1" do
    test "syncs immediately" do
      opts = [
        projection_name: "force_sync",
        entity_id: "entity-force",
        initial_state: %{value: 42},
        name: :test_sync_7
      ]

      {:ok, pid} = ProjectionSync.start_link(opts)

      # Force sync (will fail since Core not running, but should not crash)
      assert :ok = ProjectionSync.force_sync(pid)

      GenServer.stop(pid)
    end
  end
end
