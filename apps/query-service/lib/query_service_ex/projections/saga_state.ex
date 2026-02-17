defmodule QueryServiceEx.Projections.SagaState do
  @moduledoc """
  Projection module for saga entities.

  Folds saga.created, saga.step_completed, saga.failed, and saga.completed
  events into a map representing the current state of each saga.
  """

  @behaviour QueryServiceEx.Projections.Behaviour

  @impl true
  def entity_type, do: "saga"

  @impl true
  def initial_state, do: %{}

  @impl true
  def apply_event(state, %{"event_type" => "saga.created"} = event) do
    Map.merge(state, %{
      "id" => event["entity_id"],
      "type" => get_in(event, ["data", "type"]),
      "status" => "running",
      "user_id" => get_in(event, ["data", "user_id"]),
      "steps_completed" => 0,
      "created_at" => event["timestamp"],
      "updated_at" => event["timestamp"]
    })
  end

  def apply_event(state, %{"event_type" => "saga.step_completed"} = event) do
    state
    |> Map.merge(event["data"] || %{})
    |> Map.update("steps_completed", 1, &(&1 + 1))
    |> Map.put("updated_at", event["timestamp"])
  end

  def apply_event(state, %{"event_type" => "saga.failed"} = event) do
    state
    |> Map.merge(event["data"] || %{})
    |> Map.merge(%{"status" => "failed", "updated_at" => event["timestamp"]})
  end

  def apply_event(state, %{"event_type" => "saga.completed"} = event) do
    state
    |> Map.merge(event["data"] || %{})
    |> Map.merge(%{"status" => "completed", "updated_at" => event["timestamp"]})
  end

  def apply_event(state, _event), do: state

  @impl true
  def filterable_fields, do: ["status", "type", "user_id"]
end
