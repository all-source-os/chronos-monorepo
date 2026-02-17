defmodule QueryServiceEx.Projections.IndexStateTest do
  use ExUnit.Case, async: true

  alias QueryServiceEx.Projections.IndexState

  describe "entity_type/0" do
    test "returns \"index\"" do
      assert IndexState.entity_type() == "index"
    end
  end

  describe "initial_state/0" do
    test "returns empty map" do
      assert IndexState.initial_state() == %{}
    end
  end

  describe "filterable_fields/0" do
    test "returns expected fields" do
      assert IndexState.filterable_fields() == ["is_deleted", "user_id", "name"]
    end
  end

  describe "apply_event/2" do
    test "index.created sets id, name, is_deleted, timestamps" do
      event = %{
        "event_type" => "index.created",
        "entity_id" => "idx-1",
        "timestamp" => "2026-01-01T00:00:00Z",
        "data" => %{"name" => "S&P 500"}
      }

      state = IndexState.apply_event(%{}, event)

      assert state["id"] == "idx-1"
      assert state["name"] == "S&P 500"
      assert state["is_deleted"] == false
      assert state["created_at"] == "2026-01-01T00:00:00Z"
      assert state["updated_at"] == "2026-01-01T00:00:00Z"
    end

    test "index.updated merges data and updates updated_at" do
      initial = %{
        "id" => "idx-1",
        "name" => "S&P 500",
        "is_deleted" => false,
        "created_at" => "2026-01-01T00:00:00Z",
        "updated_at" => "2026-01-01T00:00:00Z"
      }

      event = %{
        "event_type" => "index.updated",
        "timestamp" => "2026-01-02T00:00:00Z",
        "data" => %{"name" => "S&P 500 v2", "description" => "Updated index"}
      }

      state = IndexState.apply_event(initial, event)

      assert state["name"] == "S&P 500 v2"
      assert state["description"] == "Updated index"
      assert state["updated_at"] == "2026-01-02T00:00:00Z"
      assert state["created_at"] == "2026-01-01T00:00:00Z"
      assert state["is_deleted"] == false
    end

    test "index.deleted sets is_deleted to true" do
      initial = %{
        "id" => "idx-1",
        "name" => "S&P 500",
        "is_deleted" => false,
        "created_at" => "2026-01-01T00:00:00Z",
        "updated_at" => "2026-01-01T00:00:00Z"
      }

      event = %{
        "event_type" => "index.deleted",
        "timestamp" => "2026-01-03T00:00:00Z"
      }

      state = IndexState.apply_event(initial, event)

      assert state["is_deleted"] == true
      assert state["updated_at"] == "2026-01-03T00:00:00Z"
      assert state["name"] == "S&P 500"
    end

    test "unrecognized event type returns state unchanged" do
      initial = %{"id" => "idx-1", "name" => "Test"}

      event = %{
        "event_type" => "some.unknown.event",
        "timestamp" => "2026-01-04T00:00:00Z",
        "data" => %{"foo" => "bar"}
      }

      assert IndexState.apply_event(initial, event) == initial
    end

    test "full lifecycle: create → update → delete" do
      events = [
        %{
          "event_type" => "index.created",
          "entity_id" => "idx-1",
          "timestamp" => "2026-01-01T00:00:00Z",
          "data" => %{"name" => "My Index"}
        },
        %{
          "event_type" => "index.updated",
          "timestamp" => "2026-01-02T00:00:00Z",
          "data" => %{"name" => "Renamed Index", "weight" => 0.5}
        },
        %{
          "event_type" => "index.updated",
          "timestamp" => "2026-01-03T00:00:00Z",
          "data" => %{"weight" => 0.75}
        },
        %{
          "event_type" => "index.deleted",
          "timestamp" => "2026-01-04T00:00:00Z"
        }
      ]

      state = Enum.reduce(events, IndexState.initial_state(), &IndexState.apply_event(&2, &1))

      assert state["id"] == "idx-1"
      assert state["name"] == "Renamed Index"
      assert state["weight"] == 0.75
      assert state["is_deleted"] == true
      assert state["created_at"] == "2026-01-01T00:00:00Z"
      assert state["updated_at"] == "2026-01-04T00:00:00Z"
    end

    test "index.updated with nil data does not crash" do
      initial = %{"id" => "idx-1", "name" => "Test"}

      event = %{
        "event_type" => "index.updated",
        "timestamp" => "2026-01-02T00:00:00Z",
        "data" => nil
      }

      state = IndexState.apply_event(initial, event)

      assert state["name"] == "Test"
      assert state["updated_at"] == "2026-01-02T00:00:00Z"
    end
  end

  describe "behaviour compliance" do
    test "implements Projections.Behaviour" do
      behaviours =
        IndexState.__info__(:attributes)
        |> Keyword.get_values(:behaviour)
        |> List.flatten()

      assert QueryServiceEx.Projections.Behaviour in behaviours
    end
  end
end
