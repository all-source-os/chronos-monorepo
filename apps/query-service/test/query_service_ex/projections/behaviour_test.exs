defmodule QueryServiceEx.Projections.BehaviourTest do
  use ExUnit.Case, async: true

  defmodule TestProjection do
    @behaviour QueryServiceEx.Projections.Behaviour

    @impl true
    def entity_type, do: "test_entity"

    @impl true
    def initial_state, do: %{}

    @impl true
    def apply_event(state, %{"event_type" => "test.created"} = event) do
      Map.put(state, "name", event["data"]["name"])
    end

    def apply_event(state, _event), do: state

    @impl true
    def filterable_fields, do: ["name"]
  end

  describe "behaviour contract" do
    test "entity_type returns a string" do
      assert TestProjection.entity_type() == "test_entity"
    end

    test "initial_state returns a map" do
      assert TestProjection.initial_state() == %{}
    end

    test "apply_event folds matching events into state" do
      event = %{"event_type" => "test.created", "data" => %{"name" => "foo"}}
      assert TestProjection.apply_event(%{}, event) == %{"name" => "foo"}
    end

    test "apply_event ignores unmatched events" do
      event = %{"event_type" => "unknown.event"}
      assert TestProjection.apply_event(%{"existing" => true}, event) == %{"existing" => true}
    end

    test "filterable_fields returns a list of strings" do
      assert TestProjection.filterable_fields() == ["name"]
    end
  end
end
