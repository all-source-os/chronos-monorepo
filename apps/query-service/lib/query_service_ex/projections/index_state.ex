defmodule QueryServiceEx.Projections.IndexState do
  @moduledoc """
  Projection module for index entities.

  Folds index.created, index.updated, and index.deleted events into
  a map representing the current state of each index.
  """

  @behaviour QueryServiceEx.Projections.Behaviour

  @impl true
  def entity_type, do: "index"

  @impl true
  def initial_state, do: %{}

  @impl true
  def apply_event(state, %{"event_type" => "index.created"} = event) do
    Map.merge(state, %{
      "id" => event["entity_id"],
      "name" => get_in(event, ["data", "name"]),
      "is_deleted" => false,
      "created_at" => event["timestamp"],
      "updated_at" => event["timestamp"]
    })
  end

  def apply_event(state, %{"event_type" => "index.updated"} = event) do
    state
    |> Map.merge(event["data"] || %{})
    |> Map.put("updated_at", event["timestamp"])
  end

  def apply_event(state, %{"event_type" => "index.deleted"} = event) do
    Map.merge(state, %{"is_deleted" => true, "updated_at" => event["timestamp"]})
  end

  def apply_event(state, _event), do: state

  @impl true
  def filterable_fields, do: ["is_deleted", "user_id", "name"]
end
