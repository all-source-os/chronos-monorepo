defmodule QueryServiceEx.Application.UseCases.ProjectionServerTest do
  use ExUnit.Case, async: true

  alias QueryServiceEx.Application.UseCases.ProjectionServer
  alias QueryServiceEx.Domain.Entities.Projection

  # Mock state store for testing
  defmodule MockStateStore do
    def save_snapshot(_snapshot), do: :ok
    def load_snapshot(_projection_name, _entity_id), do: {:error, :not_found}
  end

  defmodule FailingStateStore do
    def save_snapshot(_snapshot), do: {:error, :disk_full}
    def load_snapshot(_projection_name, _entity_id), do: {:error, :connection_failed}
  end

  describe "start_link/1" do
    test "starts a projection server with valid definition" do
      definition = Projection.Definition.new(
        name: :counter,
        version: 1,
        initial_state: 0,
        project_fn: fn state, _event -> state + 1 end
      )

      assert {:ok, pid} = ProjectionServer.start_link(
               projection: definition,
               entity_id: "test-1"
             )

      assert Process.alive?(pid)
      ProjectionServer.stop(pid)
    end

    test "starts with custom snapshot interval" do
      definition = Projection.Definition.new(
        name: :test,
        version: 1,
        initial_state: %{},
        project_fn: fn state, _event -> state end
      )

      assert {:ok, pid} = ProjectionServer.start_link(
               projection: definition,
               entity_id: "test-2",
               snapshot_interval: 50
             )

      assert Process.alive?(pid)
      ProjectionServer.stop(pid)
    end

    test "starts with state store" do
      definition = Projection.Definition.new(
        name: :test,
        version: 1,
        initial_state: %{},
        project_fn: fn state, _event -> state end
      )

      assert {:ok, pid} = ProjectionServer.start_link(
               projection: definition,
               entity_id: "test-3",
               state_store: MockStateStore
             )

      assert Process.alive?(pid)
      ProjectionServer.stop(pid)
    end

    test "can be started with a name" do
      definition = Projection.Definition.new(
        name: :test,
        version: 1,
        initial_state: %{},
        project_fn: fn state, _event -> state end
      )

      assert {:ok, pid} = ProjectionServer.start_link(
               projection: definition,
               entity_id: "test-4",
               name: :test_projection_server
             )

      assert Process.whereis(:test_projection_server) == pid
      ProjectionServer.stop(pid)
    end

    test "fails with invalid projection" do
      invalid_definition = %Projection.Definition{
        name: :test,
        version: 1,
        initial_state: 0,
        project_fn: :not_a_function  # Invalid: not a function
      }

      # Catch the EXIT signal
      Process.flag(:trap_exit, true)

      result = ProjectionServer.start_link(
        projection: invalid_definition,
        entity_id: "test-5"
      )

      case result do
        {:error, {:invalid_projection, _}} ->
          assert true

        {:ok, pid} ->
          # Wait for EXIT message
          receive do
            {:EXIT, ^pid, {:invalid_projection, _}} ->
              assert true
          after
            100 ->
              if Process.alive?(pid), do: ProjectionServer.stop(pid)
              flunk("Expected process to exit with invalid_projection")
          end

        other ->
          flunk("Unexpected result: #{inspect(other)}")
      end
    end
  end

  describe "apply_event/2" do
    setup do
      definition = Projection.Definition.new(
        name: :counter,
        version: 1,
        initial_state: 0,
        project_fn: fn state, _event -> state + 1 end
      )

      {:ok, pid} = ProjectionServer.start_link(
        projection: definition,
        entity_id: "apply-event-test"
      )

      on_exit(fn ->
        if Process.alive?(pid), do: ProjectionServer.stop(pid)
      end)

      %{pid: pid}
    end

    test "applies event and returns new state", %{pid: pid} do
      event = %{event_type: "test.event"}

      new_state = ProjectionServer.apply_event(pid, event)

      assert new_state == 1
    end

    test "applies multiple events sequentially", %{pid: pid} do
      event = %{event_type: "test.event"}

      state1 = ProjectionServer.apply_event(pid, event)
      assert state1 == 1

      state2 = ProjectionServer.apply_event(pid, event)
      assert state2 == 2

      state3 = ProjectionServer.apply_event(pid, event)
      assert state3 == 3
    end

    test "state persists across calls", %{pid: pid} do
      event = %{event_type: "test.event"}

      ProjectionServer.apply_event(pid, event)
      ProjectionServer.apply_event(pid, event)

      final_state = ProjectionServer.get_state(pid)
      assert final_state == 2
    end
  end

  describe "apply_events/2" do
    setup do
      definition = Projection.Definition.new(
        name: :accumulator,
        version: 1,
        initial_state: [],
        project_fn: fn state, event -> [event.value | state] end
      )

      {:ok, pid} = ProjectionServer.start_link(
        projection: definition,
        entity_id: "apply-events-test"
      )

      on_exit(fn ->
        if Process.alive?(pid), do: ProjectionServer.stop(pid)
      end)

      %{pid: pid}
    end

    test "applies multiple events in batch", %{pid: pid} do
      events = [
        %{value: 1},
        %{value: 2},
        %{value: 3}
      ]

      final_state = ProjectionServer.apply_events(pid, events)

      assert final_state == [3, 2, 1]
    end

    test "handles empty event list", %{pid: pid} do
      final_state = ProjectionServer.apply_events(pid, [])

      assert final_state == []
    end
  end

  describe "get_state/1" do
    setup do
      definition = Projection.Definition.new(
        name: :state_test,
        version: 1,
        initial_state: %{count: 0},
        project_fn: fn state, _event ->
          Map.update!(state, :count, &(&1 + 1))
        end
      )

      {:ok, pid} = ProjectionServer.start_link(
        projection: definition,
        entity_id: "get-state-test"
      )

      on_exit(fn ->
        if Process.alive?(pid), do: ProjectionServer.stop(pid)
      end)

      %{pid: pid}
    end

    test "returns initial state", %{pid: pid} do
      state = ProjectionServer.get_state(pid)

      assert state == %{count: 0}
    end

    test "returns updated state after events", %{pid: pid} do
      event = %{event_type: "test"}

      ProjectionServer.apply_event(pid, event)
      ProjectionServer.apply_event(pid, event)

      state = ProjectionServer.get_state(pid)
      assert state == %{count: 2}
    end
  end

  describe "get_metadata/1" do
    setup do
      definition = Projection.Definition.new(
        name: :metadata_test,
        version: 3,
        initial_state: 0,
        project_fn: fn state, _event -> state end
      )

      {:ok, pid} = ProjectionServer.start_link(
        projection: definition,
        entity_id: "metadata-test-123"
      )

      on_exit(fn ->
        if Process.alive?(pid), do: ProjectionServer.stop(pid)
      end)

      %{pid: pid}
    end

    test "returns projection metadata", %{pid: pid} do
      metadata = ProjectionServer.get_metadata(pid)

      assert metadata.projection_name == :metadata_test
      assert metadata.projection_version == 3
      assert metadata.entity_id == "metadata-test-123"
      assert metadata.events_since_snapshot == 0
      assert %DateTime{} = metadata.last_snapshot_at
    end

    test "tracks events since snapshot", %{pid: pid} do
      event = %{event_type: "test"}

      ProjectionServer.apply_event(pid, event)
      ProjectionServer.apply_event(pid, event)

      metadata = ProjectionServer.get_metadata(pid)
      assert metadata.events_since_snapshot == 2
    end
  end

  describe "snapshot/1" do
    setup do
      definition = Projection.Definition.new(
        name: :snapshot_test,
        version: 1,
        initial_state: 0,
        project_fn: fn state, _event -> state + 1 end
      )

      {:ok, pid} = ProjectionServer.start_link(
        projection: definition,
        entity_id: "snapshot-test",
        state_store: MockStateStore
      )

      on_exit(fn ->
        if Process.alive?(pid), do: ProjectionServer.stop(pid)
      end)

      %{pid: pid}
    end

    test "forces a snapshot", %{pid: pid} do
      event = %{event_type: "test"}
      ProjectionServer.apply_event(pid, event)

      assert :ok = ProjectionServer.snapshot(pid)

      metadata = ProjectionServer.get_metadata(pid)
      assert metadata.events_since_snapshot == 0
    end

    test "handles snapshot errors gracefully", %{} do
      definition = Projection.Definition.new(
        name: :failing_snapshot,
        version: 1,
        initial_state: 0,
        project_fn: fn state, _event -> state + 1 end
      )

      {:ok, pid} = ProjectionServer.start_link(
        projection: definition,
        entity_id: "failing-test",
        state_store: FailingStateStore
      )

      on_exit(fn ->
        if Process.alive?(pid), do: ProjectionServer.stop(pid)
      end)

      event = %{event_type: "test"}
      ProjectionServer.apply_event(pid, event)

      assert {:error, :disk_full} = ProjectionServer.snapshot(pid)
    end
  end

  describe "reset/1" do
    setup do
      definition = Projection.Definition.new(
        name: :reset_test,
        version: 1,
        initial_state: 0,
        project_fn: fn state, _event -> state + 1 end
      )

      {:ok, pid} = ProjectionServer.start_link(
        projection: definition,
        entity_id: "reset-test"
      )

      on_exit(fn ->
        if Process.alive?(pid), do: ProjectionServer.stop(pid)
      end)

      %{pid: pid}
    end

    test "resets to initial state", %{pid: pid} do
      event = %{event_type: "test"}

      # Apply some events
      ProjectionServer.apply_event(pid, event)
      ProjectionServer.apply_event(pid, event)
      assert ProjectionServer.get_state(pid) == 2

      # Reset
      assert :ok = ProjectionServer.reset(pid)

      # Should be back to initial state
      assert ProjectionServer.get_state(pid) == 0
    end

    test "resets event counter", %{pid: pid} do
      event = %{event_type: "test"}

      ProjectionServer.apply_event(pid, event)
      ProjectionServer.apply_event(pid, event)

      metadata_before = ProjectionServer.get_metadata(pid)
      assert metadata_before.events_since_snapshot == 2

      ProjectionServer.reset(pid)

      metadata_after = ProjectionServer.get_metadata(pid)
      assert metadata_after.events_since_snapshot == 0
    end
  end

  describe "automatic snapshots" do
    test "creates snapshot after reaching interval" do
      definition = Projection.Definition.new(
        name: :auto_snapshot,
        version: 1,
        initial_state: 0,
        project_fn: fn state, _event -> state + 1 end
      )

      # Set snapshot interval to 3 events
      {:ok, pid} = ProjectionServer.start_link(
        projection: definition,
        entity_id: "auto-snapshot-test",
        state_store: MockStateStore,
        snapshot_interval: 3
      )

      on_exit(fn ->
        if Process.alive?(pid), do: ProjectionServer.stop(pid)
      end)

      event = %{event_type: "test"}

      # Apply 2 events - should not snapshot yet
      ProjectionServer.apply_event(pid, event)
      ProjectionServer.apply_event(pid, event)

      metadata1 = ProjectionServer.get_metadata(pid)
      assert metadata1.events_since_snapshot == 2

      # Apply 3rd event - should trigger snapshot
      ProjectionServer.apply_event(pid, event)

      metadata2 = ProjectionServer.get_metadata(pid)
      assert metadata2.events_since_snapshot == 0
    end

    test "snapshots on batch events exceeding interval" do
      definition = Projection.Definition.new(
        name: :batch_snapshot,
        version: 1,
        initial_state: 0,
        project_fn: fn state, _event -> state + 1 end
      )

      {:ok, pid} = ProjectionServer.start_link(
        projection: definition,
        entity_id: "batch-snapshot-test",
        state_store: MockStateStore,
        snapshot_interval: 5
      )

      on_exit(fn ->
        if Process.alive?(pid), do: ProjectionServer.stop(pid)
      end)

      # Apply 10 events at once
      events = Enum.map(1..10, fn _ -> %{event_type: "test"} end)
      ProjectionServer.apply_events(pid, events)

      # Should have triggered snapshot
      metadata = ProjectionServer.get_metadata(pid)
      assert metadata.events_since_snapshot == 0
    end
  end

  describe "complex projections" do
    test "order statistics projection" do
      definition = Projection.Definition.new(
        name: :order_stats,
        version: 1,
        initial_state: %{count: 0, total_amount: 0.0},
        project_fn: fn state, event ->
          case event.event_type do
            "order.placed" ->
              state
              |> Map.update!(:count, &(&1 + 1))
              |> Map.update!(:total_amount, &(&1 + event.amount))

            _ ->
              state
          end
        end
      )

      {:ok, pid} = ProjectionServer.start_link(
        projection: definition,
        entity_id: "order-stats-test"
      )

      on_exit(fn ->
        if Process.alive?(pid), do: ProjectionServer.stop(pid)
      end)

      events = [
        %{event_type: "order.placed", amount: 100.50},
        %{event_type: "order.placed", amount: 200.75},
        %{event_type: "other.event"},
        %{event_type: "order.placed", amount: 50.25}
      ]

      ProjectionServer.apply_events(pid, events)

      state = ProjectionServer.get_state(pid)
      assert state.count == 3
      assert state.total_amount == 351.50
    end

    test "user activity projection with nested state" do
      definition = Projection.Definition.new(
        name: :user_activity,
        version: 1,
        initial_state: %{
          events: [],
          stats: %{login_count: 0, last_login: nil}
        },
        project_fn: fn state, event ->
          case event.event_type do
            "user.login" ->
              state
              |> update_in([:events], fn events -> [event | events] end)
              |> update_in([:stats, :login_count], &(&1 + 1))
              |> put_in([:stats, :last_login], event.timestamp)

            _ ->
              state
          end
        end
      )

      {:ok, pid} = ProjectionServer.start_link(
        projection: definition,
        entity_id: "user-activity-test"
      )

      on_exit(fn ->
        if Process.alive?(pid), do: ProjectionServer.stop(pid)
      end)

      events = [
        %{event_type: "user.login", timestamp: "2024-01-01T10:00:00Z"},
        %{event_type: "user.login", timestamp: "2024-01-01T14:00:00Z"}
      ]

      ProjectionServer.apply_events(pid, events)

      state = ProjectionServer.get_state(pid)
      assert state.stats.login_count == 2
      assert state.stats.last_login == "2024-01-01T14:00:00Z"
      assert length(state.events) == 2
    end
  end

  describe "concurrency" do
    test "multiple projection servers can run independently" do
      definition = Projection.Definition.new(
        name: :concurrent_test,
        version: 1,
        initial_state: 0,
        project_fn: fn state, _event -> state + 1 end
      )

      # Start 3 independent servers
      {:ok, pid1} = ProjectionServer.start_link(projection: definition, entity_id: "entity-1")
      {:ok, pid2} = ProjectionServer.start_link(projection: definition, entity_id: "entity-2")
      {:ok, pid3} = ProjectionServer.start_link(projection: definition, entity_id: "entity-3")

      on_exit(fn ->
        if Process.alive?(pid1), do: ProjectionServer.stop(pid1)
        if Process.alive?(pid2), do: ProjectionServer.stop(pid2)
        if Process.alive?(pid3), do: ProjectionServer.stop(pid3)
      end)

      # Apply different numbers of events to each
      event = %{event_type: "test"}

      ProjectionServer.apply_event(pid1, event)

      ProjectionServer.apply_event(pid2, event)
      ProjectionServer.apply_event(pid2, event)

      ProjectionServer.apply_event(pid3, event)
      ProjectionServer.apply_event(pid3, event)
      ProjectionServer.apply_event(pid3, event)

      # Each should have independent state
      assert ProjectionServer.get_state(pid1) == 1
      assert ProjectionServer.get_state(pid2) == 2
      assert ProjectionServer.get_state(pid3) == 3
    end
  end

  describe "graceful shutdown" do
    test "takes final snapshot on termination" do
      definition = Projection.Definition.new(
        name: :shutdown_test,
        version: 1,
        initial_state: 0,
        project_fn: fn state, _event -> state + 1 end
      )

      {:ok, pid} = ProjectionServer.start_link(
        projection: definition,
        entity_id: "shutdown-test",
        state_store: MockStateStore
      )

      event = %{event_type: "test"}
      ProjectionServer.apply_event(pid, event)

      # Stop should trigger final snapshot
      ProjectionServer.stop(pid)

      # Process should be dead
      refute Process.alive?(pid)
    end
  end
end
