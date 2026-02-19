defmodule QueryServiceEx.Projections.PortfolioStateTest do
  use ExUnit.Case, async: true

  alias QueryServiceEx.Projections.PortfolioState

  describe "entity_type/0" do
    test "returns \"portfolio\"" do
      assert PortfolioState.entity_type() == "portfolio"
    end
  end

  describe "initial_state/0" do
    test "returns empty map" do
      assert PortfolioState.initial_state() == %{}
    end
  end

  describe "filterable_fields/0" do
    test "returns expected fields" do
      assert PortfolioState.filterable_fields() == ["is_deleted", "user_id", "name"]
    end
  end

  describe "apply_event/2" do
    test "portfolio.created sets id, name, user_id, is_deleted, timestamps" do
      event = %{
        "event_type" => "portfolio.created",
        "entity_id" => "port-1",
        "timestamp" => "2026-01-01T00:00:00Z",
        "data" => %{"name" => "Growth Portfolio", "user_id" => "user-1"}
      }

      state = PortfolioState.apply_event(%{}, event)

      assert state["id"] == "port-1"
      assert state["name"] == "Growth Portfolio"
      assert state["user_id"] == "user-1"
      assert state["is_deleted"] == false
      assert state["created_at"] == "2026-01-01T00:00:00Z"
      assert state["updated_at"] == "2026-01-01T00:00:00Z"
    end

    test "portfolio.updated merges data and updates updated_at" do
      initial = %{
        "id" => "port-1",
        "name" => "Growth Portfolio",
        "is_deleted" => false,
        "created_at" => "2026-01-01T00:00:00Z",
        "updated_at" => "2026-01-01T00:00:00Z"
      }

      event = %{
        "event_type" => "portfolio.updated",
        "timestamp" => "2026-01-02T00:00:00Z",
        "data" => %{"name" => "Aggressive Growth", "target_allocation" => 0.8}
      }

      state = PortfolioState.apply_event(initial, event)

      assert state["name"] == "Aggressive Growth"
      assert state["target_allocation"] == 0.8
      assert state["is_deleted"] == false
      assert state["updated_at"] == "2026-01-02T00:00:00Z"
    end

    test "portfolio.rebalanced merges data and updates updated_at" do
      initial = %{
        "id" => "port-1",
        "name" => "Growth Portfolio",
        "is_deleted" => false,
        "created_at" => "2026-01-01T00:00:00Z",
        "updated_at" => "2026-01-01T00:00:00Z"
      }

      event = %{
        "event_type" => "portfolio.rebalanced",
        "timestamp" => "2026-01-03T00:00:00Z",
        "data" => %{"rebalance_count" => 1}
      }

      state = PortfolioState.apply_event(initial, event)

      assert state["rebalance_count"] == 1
      assert state["updated_at"] == "2026-01-03T00:00:00Z"
      assert state["is_deleted"] == false
    end

    test "portfolio.deleted sets is_deleted to true" do
      initial = %{
        "id" => "port-1",
        "name" => "Growth Portfolio",
        "is_deleted" => false,
        "created_at" => "2026-01-01T00:00:00Z",
        "updated_at" => "2026-01-01T00:00:00Z"
      }

      event = %{
        "event_type" => "portfolio.deleted",
        "timestamp" => "2026-01-04T00:00:00Z"
      }

      state = PortfolioState.apply_event(initial, event)

      assert state["is_deleted"] == true
      assert state["updated_at"] == "2026-01-04T00:00:00Z"
      assert state["name"] == "Growth Portfolio"
    end

    test "unrecognized event type returns state unchanged" do
      initial = %{"id" => "port-1", "name" => "Test"}

      event = %{
        "event_type" => "some.unknown.event",
        "timestamp" => "2026-01-04T00:00:00Z",
        "data" => %{"foo" => "bar"}
      }

      assert PortfolioState.apply_event(initial, event) == initial
    end

    test "portfolio.updated with nil data does not crash" do
      initial = %{"id" => "port-1", "name" => "Test"}

      event = %{
        "event_type" => "portfolio.updated",
        "timestamp" => "2026-01-02T00:00:00Z",
        "data" => nil
      }

      state = PortfolioState.apply_event(initial, event)

      assert state["name"] == "Test"
      assert state["updated_at"] == "2026-01-02T00:00:00Z"
    end

    test "full lifecycle: create → update → rebalance → delete" do
      events = [
        %{
          "event_type" => "portfolio.created",
          "entity_id" => "port-1",
          "timestamp" => "2026-01-01T00:00:00Z",
          "data" => %{"name" => "My Portfolio", "user_id" => "user-1"}
        },
        %{
          "event_type" => "portfolio.updated",
          "timestamp" => "2026-01-02T00:00:00Z",
          "data" => %{"name" => "Renamed Portfolio"}
        },
        %{
          "event_type" => "portfolio.rebalanced",
          "timestamp" => "2026-01-03T00:00:00Z",
          "data" => %{"rebalance_count" => 1}
        },
        %{
          "event_type" => "portfolio.deleted",
          "timestamp" => "2026-01-04T00:00:00Z"
        }
      ]

      state =
        Enum.reduce(events, PortfolioState.initial_state(), &PortfolioState.apply_event(&2, &1))

      assert state["id"] == "port-1"
      assert state["name"] == "Renamed Portfolio"
      assert state["rebalance_count"] == 1
      assert state["is_deleted"] == true
      assert state["created_at"] == "2026-01-01T00:00:00Z"
      assert state["updated_at"] == "2026-01-04T00:00:00Z"
    end
  end

  describe "behaviour compliance" do
    test "implements Projections.Behaviour" do
      behaviours =
        PortfolioState.__info__(:attributes)
        |> Keyword.get_values(:behaviour)
        |> List.flatten()

      assert QueryServiceEx.Projections.Behaviour in behaviours
    end
  end
end
