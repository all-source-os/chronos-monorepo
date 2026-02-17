defmodule QueryServiceEx.Projections.SagaStateTest do
  use ExUnit.Case, async: true

  alias QueryServiceEx.Projections.SagaState

  describe "entity_type/0" do
    test "returns \"saga\"" do
      assert SagaState.entity_type() == "saga"
    end
  end

  describe "initial_state/0" do
    test "returns empty map" do
      assert SagaState.initial_state() == %{}
    end
  end

  describe "filterable_fields/0" do
    test "returns expected fields" do
      assert SagaState.filterable_fields() == ["status", "type", "user_id"]
    end
  end

  describe "apply_event/2" do
    test "saga.created sets id, type, status, user_id, steps_completed, timestamps" do
      event = %{
        "event_type" => "saga.created",
        "entity_id" => "saga-1",
        "timestamp" => "2026-01-01T00:00:00Z",
        "data" => %{"type" => "rebalance", "user_id" => "user-1"}
      }

      state = SagaState.apply_event(%{}, event)

      assert state["id"] == "saga-1"
      assert state["type"] == "rebalance"
      assert state["status"] == "running"
      assert state["user_id"] == "user-1"
      assert state["steps_completed"] == 0
      assert state["created_at"] == "2026-01-01T00:00:00Z"
      assert state["updated_at"] == "2026-01-01T00:00:00Z"
    end

    test "saga.step_completed increments steps_completed and merges data" do
      initial = %{
        "id" => "saga-1",
        "status" => "running",
        "steps_completed" => 0,
        "created_at" => "2026-01-01T00:00:00Z",
        "updated_at" => "2026-01-01T00:00:00Z"
      }

      event = %{
        "event_type" => "saga.step_completed",
        "timestamp" => "2026-01-02T00:00:00Z",
        "data" => %{"step_name" => "validate_holdings"}
      }

      state = SagaState.apply_event(initial, event)

      assert state["steps_completed"] == 1
      assert state["step_name"] == "validate_holdings"
      assert state["updated_at"] == "2026-01-02T00:00:00Z"
    end

    test "saga.step_completed increments multiple times" do
      events = [
        %{
          "event_type" => "saga.created",
          "entity_id" => "saga-1",
          "timestamp" => "2026-01-01T00:00:00Z",
          "data" => %{"type" => "rebalance", "user_id" => "user-1"}
        },
        %{
          "event_type" => "saga.step_completed",
          "timestamp" => "2026-01-01T01:00:00Z",
          "data" => %{}
        },
        %{
          "event_type" => "saga.step_completed",
          "timestamp" => "2026-01-01T02:00:00Z",
          "data" => %{}
        },
        %{
          "event_type" => "saga.step_completed",
          "timestamp" => "2026-01-01T03:00:00Z",
          "data" => %{}
        }
      ]

      state = Enum.reduce(events, SagaState.initial_state(), &SagaState.apply_event(&2, &1))

      assert state["steps_completed"] == 3
    end

    test "saga.failed sets status to failed" do
      initial = %{
        "id" => "saga-1",
        "status" => "running",
        "steps_completed" => 2,
        "created_at" => "2026-01-01T00:00:00Z",
        "updated_at" => "2026-01-01T00:00:00Z"
      }

      event = %{
        "event_type" => "saga.failed",
        "timestamp" => "2026-01-03T00:00:00Z",
        "data" => %{"error" => "insufficient_funds"}
      }

      state = SagaState.apply_event(initial, event)

      assert state["status"] == "failed"
      assert state["error"] == "insufficient_funds"
      assert state["steps_completed"] == 2
      assert state["updated_at"] == "2026-01-03T00:00:00Z"
    end

    test "saga.completed sets status to completed" do
      initial = %{
        "id" => "saga-1",
        "status" => "running",
        "steps_completed" => 3,
        "created_at" => "2026-01-01T00:00:00Z",
        "updated_at" => "2026-01-01T00:00:00Z"
      }

      event = %{
        "event_type" => "saga.completed",
        "timestamp" => "2026-01-04T00:00:00Z",
        "data" => %{"result" => "success"}
      }

      state = SagaState.apply_event(initial, event)

      assert state["status"] == "completed"
      assert state["result"] == "success"
      assert state["steps_completed"] == 3
      assert state["updated_at"] == "2026-01-04T00:00:00Z"
    end

    test "unrecognized event type returns state unchanged" do
      initial = %{"id" => "saga-1", "status" => "running"}

      event = %{
        "event_type" => "some.unknown.event",
        "timestamp" => "2026-01-04T00:00:00Z",
        "data" => %{"foo" => "bar"}
      }

      assert SagaState.apply_event(initial, event) == initial
    end

    test "saga.step_completed with nil data does not crash" do
      initial = %{"id" => "saga-1", "steps_completed" => 0}

      event = %{
        "event_type" => "saga.step_completed",
        "timestamp" => "2026-01-02T00:00:00Z",
        "data" => nil
      }

      state = SagaState.apply_event(initial, event)

      assert state["steps_completed"] == 1
      assert state["updated_at"] == "2026-01-02T00:00:00Z"
    end

    test "full lifecycle: create → steps → complete" do
      events = [
        %{
          "event_type" => "saga.created",
          "entity_id" => "saga-1",
          "timestamp" => "2026-01-01T00:00:00Z",
          "data" => %{"type" => "rebalance", "user_id" => "user-1"}
        },
        %{
          "event_type" => "saga.step_completed",
          "timestamp" => "2026-01-01T01:00:00Z",
          "data" => %{"step_name" => "validate"}
        },
        %{
          "event_type" => "saga.step_completed",
          "timestamp" => "2026-01-01T02:00:00Z",
          "data" => %{"step_name" => "execute"}
        },
        %{
          "event_type" => "saga.completed",
          "timestamp" => "2026-01-01T03:00:00Z",
          "data" => %{"result" => "all_trades_placed"}
        }
      ]

      state = Enum.reduce(events, SagaState.initial_state(), &SagaState.apply_event(&2, &1))

      assert state["id"] == "saga-1"
      assert state["type"] == "rebalance"
      assert state["status"] == "completed"
      assert state["steps_completed"] == 2
      assert state["result"] == "all_trades_placed"
      assert state["created_at"] == "2026-01-01T00:00:00Z"
      assert state["updated_at"] == "2026-01-01T03:00:00Z"
    end
  end

  describe "behaviour compliance" do
    test "implements Projections.Behaviour" do
      behaviours =
        SagaState.__info__(:attributes)
        |> Keyword.get_values(:behaviour)
        |> List.flatten()

      assert QueryServiceEx.Projections.Behaviour in behaviours
    end
  end
end
