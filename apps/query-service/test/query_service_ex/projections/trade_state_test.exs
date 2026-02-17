defmodule QueryServiceEx.Projections.TradeStateTest do
  use ExUnit.Case, async: true

  alias QueryServiceEx.Projections.TradeState

  describe "entity_type/0" do
    test "returns \"trade\"" do
      assert TradeState.entity_type() == "trade"
    end
  end

  describe "initial_state/0" do
    test "returns empty map" do
      assert TradeState.initial_state() == %{}
    end
  end

  describe "filterable_fields/0" do
    test "returns expected fields" do
      assert TradeState.filterable_fields() == ["status", "user_id", "symbol"]
    end
  end

  describe "apply_event/2" do
    test "trade.created sets id, symbol, status, user_id, timestamps" do
      event = %{
        "event_type" => "trade.created",
        "entity_id" => "trade-1",
        "timestamp" => "2026-01-01T00:00:00Z",
        "data" => %{"symbol" => "AAPL", "user_id" => "user-1"}
      }

      state = TradeState.apply_event(%{}, event)

      assert state["id"] == "trade-1"
      assert state["symbol"] == "AAPL"
      assert state["status"] == "pending"
      assert state["user_id"] == "user-1"
      assert state["created_at"] == "2026-01-01T00:00:00Z"
      assert state["updated_at"] == "2026-01-01T00:00:00Z"
    end

    test "trade.updated merges data and updates updated_at" do
      initial = %{
        "id" => "trade-1",
        "symbol" => "AAPL",
        "status" => "pending",
        "created_at" => "2026-01-01T00:00:00Z",
        "updated_at" => "2026-01-01T00:00:00Z"
      }

      event = %{
        "event_type" => "trade.updated",
        "timestamp" => "2026-01-02T00:00:00Z",
        "data" => %{"quantity" => 100, "price" => 150.50}
      }

      state = TradeState.apply_event(initial, event)

      assert state["quantity"] == 100
      assert state["price"] == 150.50
      assert state["status"] == "pending"
      assert state["updated_at"] == "2026-01-02T00:00:00Z"
    end

    test "trade.executed sets status to executed" do
      initial = %{
        "id" => "trade-1",
        "symbol" => "AAPL",
        "status" => "pending",
        "created_at" => "2026-01-01T00:00:00Z",
        "updated_at" => "2026-01-01T00:00:00Z"
      }

      event = %{
        "event_type" => "trade.executed",
        "timestamp" => "2026-01-03T00:00:00Z",
        "data" => %{"executed_price" => 151.00}
      }

      state = TradeState.apply_event(initial, event)

      assert state["status"] == "executed"
      assert state["executed_price"] == 151.00
      assert state["updated_at"] == "2026-01-03T00:00:00Z"
    end

    test "trade.cancelled sets status to cancelled" do
      initial = %{
        "id" => "trade-1",
        "symbol" => "AAPL",
        "status" => "pending",
        "created_at" => "2026-01-01T00:00:00Z",
        "updated_at" => "2026-01-01T00:00:00Z"
      }

      event = %{
        "event_type" => "trade.cancelled",
        "timestamp" => "2026-01-03T00:00:00Z",
        "data" => %{"reason" => "user_requested"}
      }

      state = TradeState.apply_event(initial, event)

      assert state["status"] == "cancelled"
      assert state["reason"] == "user_requested"
      assert state["updated_at"] == "2026-01-03T00:00:00Z"
    end

    test "unrecognized event type returns state unchanged" do
      initial = %{"id" => "trade-1", "symbol" => "AAPL"}

      event = %{
        "event_type" => "some.unknown.event",
        "timestamp" => "2026-01-04T00:00:00Z",
        "data" => %{"foo" => "bar"}
      }

      assert TradeState.apply_event(initial, event) == initial
    end

    test "trade.updated with nil data does not crash" do
      initial = %{"id" => "trade-1", "symbol" => "AAPL"}

      event = %{
        "event_type" => "trade.updated",
        "timestamp" => "2026-01-02T00:00:00Z",
        "data" => nil
      }

      state = TradeState.apply_event(initial, event)

      assert state["symbol"] == "AAPL"
      assert state["updated_at"] == "2026-01-02T00:00:00Z"
    end

    test "full lifecycle: create → update → execute" do
      events = [
        %{
          "event_type" => "trade.created",
          "entity_id" => "trade-1",
          "timestamp" => "2026-01-01T00:00:00Z",
          "data" => %{"symbol" => "AAPL", "user_id" => "user-1"}
        },
        %{
          "event_type" => "trade.updated",
          "timestamp" => "2026-01-02T00:00:00Z",
          "data" => %{"quantity" => 50}
        },
        %{
          "event_type" => "trade.executed",
          "timestamp" => "2026-01-03T00:00:00Z",
          "data" => %{"executed_price" => 155.00}
        }
      ]

      state = Enum.reduce(events, TradeState.initial_state(), &TradeState.apply_event(&2, &1))

      assert state["id"] == "trade-1"
      assert state["symbol"] == "AAPL"
      assert state["status"] == "executed"
      assert state["quantity"] == 50
      assert state["executed_price"] == 155.00
      assert state["created_at"] == "2026-01-01T00:00:00Z"
      assert state["updated_at"] == "2026-01-03T00:00:00Z"
    end
  end

  describe "behaviour compliance" do
    test "implements Projections.Behaviour" do
      behaviours =
        TradeState.__info__(:attributes)
        |> Keyword.get_values(:behaviour)
        |> List.flatten()

      assert QueryServiceEx.Projections.Behaviour in behaviours
    end
  end
end
