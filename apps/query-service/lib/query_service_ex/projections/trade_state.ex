defmodule QueryServiceEx.Projections.TradeState do
  @moduledoc """
  Projection module for trade entities.

  Folds trade.created, trade.updated, trade.executed, and trade.cancelled
  events into a map representing the current state of each trade.
  """

  @behaviour QueryServiceEx.Projections.Behaviour

  @impl true
  def entity_type, do: "trade"

  @impl true
  def initial_state, do: %{}

  @impl true
  def apply_event(state, %{"event_type" => "trade.created"} = event) do
    Map.merge(state, %{
      "id" => event["entity_id"],
      "symbol" => get_in(event, ["data", "symbol"]),
      "status" => "pending",
      "user_id" => get_in(event, ["data", "user_id"]),
      "created_at" => event["timestamp"],
      "updated_at" => event["timestamp"]
    })
  end

  def apply_event(state, %{"event_type" => "trade.updated"} = event) do
    state
    |> Map.merge(event["data"] || %{})
    |> Map.put("updated_at", event["timestamp"])
  end

  def apply_event(state, %{"event_type" => "trade.executed"} = event) do
    state
    |> Map.merge(event["data"] || %{})
    |> Map.merge(%{"status" => "executed", "updated_at" => event["timestamp"]})
  end

  def apply_event(state, %{"event_type" => "trade.cancelled"} = event) do
    state
    |> Map.merge(event["data"] || %{})
    |> Map.merge(%{"status" => "cancelled", "updated_at" => event["timestamp"]})
  end

  def apply_event(state, _event), do: state

  @impl true
  def filterable_fields, do: ["status", "user_id", "symbol"]
end
