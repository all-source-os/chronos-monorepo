defmodule QueryServiceEx.Domain.Entities.ProjectionTest do
  use ExUnit.Case, async: true

  alias QueryServiceEx.Domain.Entities.Projection
  alias QueryServiceEx.Domain.Entities.Projection.{Definition, Snapshot}

  doctest Projection

  describe "Definition.new/1" do
    test "creates a projection definition with all required fields" do
      project_fn = fn state, _event -> state + 1 end

      definition =
        Definition.new(
          name: :counter,
          version: 1,
          initial_state: 0,
          project_fn: project_fn
        )

      assert definition.name == :counter
      assert definition.version == 1
      assert definition.initial_state == 0
      assert is_function(definition.project_fn, 2)
      assert definition.metadata == %{}
    end

    test "creates a projection with custom metadata" do
      project_fn = fn state, _event -> state end
      metadata = %{author: "team", description: "Test projection"}

      definition =
        Definition.new(
          name: :test,
          version: 1,
          initial_state: %{},
          project_fn: project_fn,
          metadata: metadata
        )

      assert definition.metadata == metadata
    end

    test "creates a projection with complex initial state" do
      initial_state = %{
        count: 0,
        total_amount: 0.0,
        items: []
      }

      definition =
        Definition.new(
          name: :order_stats,
          version: 1,
          initial_state: initial_state,
          project_fn: fn state, _event -> state end
        )

      assert definition.initial_state == initial_state
    end
  end

  describe "Snapshot.new/1" do
    test "creates a snapshot with all required fields" do
      timestamp = DateTime.utc_now()

      snapshot =
        Snapshot.new(
          projection_name: :user_stats,
          entity_id: "user-123",
          state: %{count: 42},
          version: 1,
          timestamp: timestamp
        )

      assert snapshot.projection_name == :user_stats
      assert snapshot.entity_id == "user-123"
      assert snapshot.state == %{count: 42}
      assert snapshot.version == 1
      assert snapshot.timestamp == timestamp
    end
  end

  describe "valid_version?/1" do
    test "validates positive integers as valid versions" do
      assert Projection.valid_version?(1)
      assert Projection.valid_version?(2)
      assert Projection.valid_version?(100)
    end

    test "rejects zero and negative versions" do
      refute Projection.valid_version?(0)
      refute Projection.valid_version?(-1)
    end

    test "rejects non-integers" do
      refute Projection.valid_version?(1.5)
      refute Projection.valid_version?("1")
      refute Projection.valid_version?(nil)
    end
  end

  describe "valid_name?/1" do
    test "validates atoms as valid names" do
      assert Projection.valid_name?(:user_stats)
      assert Projection.valid_name?(:order_totals)
      assert Projection.valid_name?(:test)
    end

    test "rejects non-atoms" do
      refute Projection.valid_name?("user_stats")
      refute Projection.valid_name?(123)
      refute Projection.valid_name?(%{})
    end
  end

  describe "valid_projection?/1" do
    test "validates a complete projection definition" do
      definition =
        Definition.new(
          name: :counter,
          version: 1,
          initial_state: 0,
          project_fn: fn state, _event -> state + 1 end
        )

      assert Projection.valid_projection?(definition)
    end

    test "validates projection with complex state" do
      definition =
        Definition.new(
          name: :order_stats,
          version: 1,
          initial_state: %{count: 0, total: 0.0},
          project_fn: fn state, event ->
            Map.update!(state, :count, &(&1 + 1))
          end
        )

      assert Projection.valid_projection?(definition)
    end

    test "rejects projection with invalid name" do
      definition = %Definition{
        name: "invalid",
        version: 1,
        initial_state: 0,
        project_fn: fn state, _event -> state end
      }

      refute Projection.valid_projection?(definition)
    end

    test "rejects projection with invalid version" do
      definition = %Definition{
        name: :test,
        version: 0,
        initial_state: 0,
        project_fn: fn state, _event -> state end
      }

      refute Projection.valid_projection?(definition)
    end

    test "rejects projection with nil initial state" do
      definition = %Definition{
        name: :test,
        version: 1,
        initial_state: nil,
        project_fn: fn state, _event -> state end
      }

      refute Projection.valid_projection?(definition)
    end

    test "rejects projection with non-function project_fn" do
      definition = %Definition{
        name: :test,
        version: 1,
        initial_state: 0,
        project_fn: :not_a_function
      }

      refute Projection.valid_projection?(definition)
    end

    test "rejects projection with wrong arity project_fn" do
      definition = %Definition{
        name: :test,
        version: 1,
        initial_state: 0,
        # arity 1 instead of 2
        project_fn: fn state -> state end
      }

      refute Projection.valid_projection?(definition)
    end

    test "rejects non-definition structs" do
      refute Projection.valid_projection?(%{})
      refute Projection.valid_projection?(nil)
    end
  end

  describe "apply_event/3" do
    test "applies a single event to state using project_fn" do
      definition =
        Definition.new(
          name: :counter,
          version: 1,
          initial_state: 0,
          project_fn: fn state, _event -> state + 1 end
        )

      event = %{event_type: "test.event"}
      new_state = Projection.apply_event(definition, 0, event)

      assert new_state == 1
    end

    test "applies event with state transformation" do
      definition =
        Definition.new(
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

      event = %{event_type: "order.placed", amount: 100.50}
      new_state = Projection.apply_event(definition, %{count: 0, total_amount: 0.0}, event)

      assert new_state.count == 1
      assert new_state.total_amount == 100.50
    end

    test "returns unchanged state for unmatched events" do
      definition =
        Definition.new(
          name: :selective,
          version: 1,
          initial_state: 0,
          project_fn: fn state, event ->
            if event.event_type == "increment", do: state + 1, else: state
          end
        )

      event = %{event_type: "other"}
      new_state = Projection.apply_event(definition, 5, event)

      assert new_state == 5
    end
  end

  describe "apply_events/3" do
    test "applies multiple events in sequence" do
      definition =
        Definition.new(
          name: :counter,
          version: 1,
          initial_state: 0,
          project_fn: fn state, _event -> state + 1 end
        )

      events = [
        %{event_type: "event1"},
        %{event_type: "event2"},
        %{event_type: "event3"}
      ]

      final_state = Projection.apply_events(definition, 0, events)

      assert final_state == 3
    end

    test "applies events with complex transformations" do
      definition =
        Definition.new(
          name: :order_processor,
          version: 1,
          initial_state: %{orders: 0, cancellations: 0, revenue: 0.0},
          project_fn: fn state, event ->
            case event.event_type do
              "order.placed" ->
                state
                |> Map.update!(:orders, &(&1 + 1))
                |> Map.update!(:revenue, &(&1 + event.amount))

              "order.cancelled" ->
                Map.update!(state, :cancellations, &(&1 + 1))

              _ ->
                state
            end
          end
        )

      events = [
        %{event_type: "order.placed", amount: 100.0},
        %{event_type: "order.placed", amount: 200.0},
        %{event_type: "order.cancelled"},
        %{event_type: "order.placed", amount: 50.0}
      ]

      final_state = Projection.apply_events(definition, definition.initial_state, events)

      assert final_state.orders == 3
      assert final_state.cancellations == 1
      assert final_state.revenue == 350.0
    end

    test "handles empty event list" do
      definition =
        Definition.new(
          name: :counter,
          version: 1,
          initial_state: 0,
          project_fn: fn state, _event -> state + 1 end
        )

      final_state = Projection.apply_events(definition, 5, [])

      assert final_state == 5
    end
  end

  describe "initialize_state/1" do
    test "returns the initial state from definition" do
      definition =
        Definition.new(
          name: :test,
          version: 1,
          initial_state: %{count: 0},
          project_fn: fn state, _event -> state end
        )

      state = Projection.initialize_state(definition)

      assert state == %{count: 0}
    end

    test "returns complex initial state" do
      initial = %{
        users: %{},
        orders: [],
        stats: %{total: 0, avg: 0.0}
      }

      definition =
        Definition.new(
          name: :complex,
          version: 1,
          initial_state: initial,
          project_fn: fn state, _event -> state end
        )

      state = Projection.initialize_state(definition)

      assert state == initial
    end
  end

  describe "migrate_state/2" do
    test "migrates state using migration function" do
      old_state = %{count: 42}
      migration_fn = fn old -> Map.put(old, :total, old.count * 2) end

      new_state = Projection.migrate_state(old_state, migration_fn)

      assert new_state == %{count: 42, total: 84}
    end

    test "migrates with structure change" do
      old_state = %{value: 100}
      migration_fn = fn old -> %{data: %{amount: old.value}, version: 2} end

      new_state = Projection.migrate_state(old_state, migration_fn)

      assert new_state == %{data: %{amount: 100}, version: 2}
    end
  end

  describe "accessor functions" do
    setup do
      definition =
        Definition.new(
          name: :test_projection,
          version: 3,
          initial_state: %{value: 0},
          project_fn: fn state, _event -> state end,
          metadata: %{author: "test"}
        )

      %{definition: definition}
    end

    test "get_name/1 returns projection name", %{definition: definition} do
      assert Projection.get_name(definition) == :test_projection
    end

    test "get_version/1 returns projection version", %{definition: definition} do
      assert Projection.get_version(definition) == 3
    end

    test "get_initial_state/1 returns initial state", %{definition: definition} do
      assert Projection.get_initial_state(definition) == %{value: 0}
    end

    test "get_metadata/1 returns metadata", %{definition: definition} do
      assert Projection.get_metadata(definition) == %{author: "test"}
    end
  end

  describe "snapshot_state/4" do
    test "creates a snapshot with current timestamp" do
      before = DateTime.utc_now()

      snapshot =
        Projection.snapshot_state(
          :user_stats,
          "user-123",
          %{count: 42},
          1
        )

      after_time = DateTime.utc_now()

      assert snapshot.projection_name == :user_stats
      assert snapshot.entity_id == "user-123"
      assert snapshot.state == %{count: 42}
      assert snapshot.version == 1
      assert DateTime.compare(snapshot.timestamp, before) in [:gt, :eq]
      assert DateTime.compare(snapshot.timestamp, after_time) in [:lt, :eq]
    end

    test "creates snapshot with complex state" do
      complex_state = %{
        orders: [1, 2, 3],
        total_revenue: 1234.56,
        metadata: %{last_updated: "2024-01-01"}
      }

      snapshot =
        Projection.snapshot_state(
          :order_stats,
          "tenant-456",
          complex_state,
          2
        )

      assert snapshot.state == complex_state
      assert snapshot.version == 2
    end
  end

  describe "restore_from_snapshot/1" do
    test "restores state from snapshot" do
      snapshot =
        Snapshot.new(
          projection_name: :user_stats,
          entity_id: "user-123",
          state: %{count: 42, total: 1000.0},
          version: 1,
          timestamp: DateTime.utc_now()
        )

      restored_state = Projection.restore_from_snapshot(snapshot)

      assert restored_state == %{count: 42, total: 1000.0}
    end

    test "restores complex state from snapshot" do
      complex_state = %{
        data: %{nested: %{value: 123}},
        list: [1, 2, 3]
      }

      snapshot =
        Snapshot.new(
          projection_name: :complex,
          entity_id: "entity-789",
          state: complex_state,
          version: 3,
          timestamp: DateTime.utc_now()
        )

      restored = Projection.restore_from_snapshot(snapshot)

      assert restored == complex_state
    end
  end

  describe "integration: complete projection lifecycle" do
    test "creates, applies events, snapshots, and restores" do
      # 1. Create projection definition
      definition =
        Definition.new(
          name: :user_activity,
          version: 1,
          initial_state: %{login_count: 0, last_login: nil},
          project_fn: fn state, event ->
            case event.event_type do
              "user.login" ->
                %{state | login_count: state.login_count + 1, last_login: event.timestamp}

              _ ->
                state
            end
          end
        )

      # 2. Initialize state
      state = Projection.initialize_state(definition)
      assert state == %{login_count: 0, last_login: nil}

      # 3. Apply events
      events = [
        %{event_type: "user.login", timestamp: "2024-01-01T10:00:00Z"},
        %{event_type: "user.login", timestamp: "2024-01-01T14:00:00Z"},
        %{event_type: "user.login", timestamp: "2024-01-01T18:00:00Z"}
      ]

      final_state = Projection.apply_events(definition, state, events)
      assert final_state.login_count == 3
      assert final_state.last_login == "2024-01-01T18:00:00Z"

      # 4. Create snapshot
      snapshot =
        Projection.snapshot_state(
          definition.name,
          "user-123",
          final_state,
          definition.version
        )

      assert snapshot.state.login_count == 3

      # 5. Restore from snapshot
      restored_state = Projection.restore_from_snapshot(snapshot)
      assert restored_state == final_state
    end

    test "migration scenario: upgrade from v1 to v2" do
      # V1 projection
      v1_definition =
        Definition.new(
          name: :order_stats,
          version: 1,
          initial_state: %{count: 0},
          project_fn: fn state, event ->
            if event.event_type == "order.placed" do
              Map.update!(state, :count, &(&1 + 1))
            else
              state
            end
          end
        )

      # Apply some events to v1
      v1_events = [
        %{event_type: "order.placed"},
        %{event_type: "order.placed"}
      ]

      v1_state = Projection.apply_events(v1_definition, v1_definition.initial_state, v1_events)
      assert v1_state == %{count: 2}

      # Migrate to v2 (add total_amount field)
      migration_fn = fn old_state ->
        Map.put(old_state, :total_amount, 0.0)
      end

      v2_state = Projection.migrate_state(v1_state, migration_fn)
      assert v2_state == %{count: 2, total_amount: 0.0}

      # V2 projection with new field
      v2_definition =
        Definition.new(
          name: :order_stats,
          version: 2,
          initial_state: %{count: 0, total_amount: 0.0},
          project_fn: fn state, event ->
            if event.event_type == "order.placed" do
              state
              |> Map.update!(:count, &(&1 + 1))
              |> Map.update!(:total_amount, &(&1 + event.amount))
            else
              state
            end
          end
        )

      # Apply new events to v2
      v2_events = [
        %{event_type: "order.placed", amount: 100.0},
        %{event_type: "order.placed", amount: 200.0}
      ]

      final_state = Projection.apply_events(v2_definition, v2_state, v2_events)
      assert final_state == %{count: 4, total_amount: 300.0}
    end
  end
end
