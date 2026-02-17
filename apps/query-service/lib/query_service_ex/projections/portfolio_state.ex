defmodule QueryServiceEx.Projections.PortfolioState do
  @moduledoc """
  Projection module for portfolio entities.

  Folds portfolio.created, portfolio.updated, portfolio.rebalanced, and
  portfolio.deleted events into a map representing the current state of each portfolio.
  """

  @behaviour QueryServiceEx.Projections.Behaviour

  @impl true
  def entity_type, do: "portfolio"

  @impl true
  def initial_state, do: %{}

  @impl true
  def apply_event(state, %{"event_type" => "portfolio.created"} = event) do
    Map.merge(state, %{
      "id" => event["entity_id"],
      "name" => get_in(event, ["data", "name"]),
      "user_id" => get_in(event, ["data", "user_id"]),
      "is_deleted" => false,
      "created_at" => event["timestamp"],
      "updated_at" => event["timestamp"]
    })
  end

  def apply_event(state, %{"event_type" => "portfolio.updated"} = event) do
    state
    |> Map.merge(event["data"] || %{})
    |> Map.put("updated_at", event["timestamp"])
  end

  def apply_event(state, %{"event_type" => "portfolio.rebalanced"} = event) do
    state
    |> Map.merge(event["data"] || %{})
    |> Map.put("updated_at", event["timestamp"])
  end

  def apply_event(state, %{"event_type" => "portfolio.deleted"} = event) do
    Map.merge(state, %{"is_deleted" => true, "updated_at" => event["timestamp"]})
  end

  def apply_event(state, _event), do: state

  @impl true
  def filterable_fields, do: ["is_deleted", "user_id", "name"]
end
