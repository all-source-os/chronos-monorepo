defmodule QueryServiceEx.Projections.ProjectionServerTest do
  use ExUnit.Case, async: false

  alias QueryServiceEx.Projections.ProjectionServer

  # Inline projection module for testing
  defmodule TestProjection do
    @behaviour QueryServiceEx.Projections.Behaviour

    @impl true
    def entity_type, do: "test_ps"

    @impl true
    def initial_state, do: %{}

    @impl true
    def apply_event(state, %{"event_type" => "test_ps.created"} = event) do
      Map.merge(state, %{
        "id" => event["entity_id"],
        "name" => get_in(event, ["data", "name"]),
        "count" => 1
      })
    end

    def apply_event(state, %{"event_type" => "test_ps.updated"} = event) do
      state
      |> Map.merge(event["data"] || %{})
      |> Map.update("count", 1, &(&1 + 1))
    end

    def apply_event(state, _event), do: state

    @impl true
    def filterable_fields, do: ["name"]
  end

  # Mock Core client that tracks calls via Agent
  defmodule MockCoreClient do
    def start_tracker do
      Agent.start_link(fn -> [] end, name: __MODULE__)
    end

    def get_calls do
      Agent.get(__MODULE__, & &1)
    end

    def get_projection_state_summary(_name) do
      {:ok, []}
    end

    def save_projection_state(projection_name, entity_id, state) do
      Agent.update(__MODULE__, fn calls ->
        [{:save, projection_name, entity_id, state} | calls]
      end)

      :ok
    end
  end

  # Mock Core client that returns existing state for hydration
  defmodule HydrationCoreClient do
    def get_projection_state_summary(_name) do
      {:ok, [
        %{"entity_id" => "ent-1", "state" => %{"id" => "ent-1", "name" => "Alpha", "count" => 3}},
        %{"entity_id" => "ent-2", "state" => %{"id" => "ent-2", "name" => "Beta", "count" => 1}}
      ]}
    end

    def save_projection_state(_projection_name, _entity_id, _state), do: :ok
  end

  # Mock Core client that fails on hydration
  defmodule FailingCoreClient do
    def get_projection_state_summary(_name) do
      {:error, :connection_refused}
    end

    def save_projection_state(_projection_name, _entity_id, _state), do: :ok
  end

  # Mock Core client that fails on save
  defmodule FailingSaveCoreClient do
    def start_tracker do
      Agent.start_link(fn -> [] end, name: __MODULE__)
    end

    def get_calls do
      Agent.get(__MODULE__, & &1)
    end

    def get_projection_state_summary(_name), do: {:ok, []}

    def save_projection_state(projection_name, entity_id, _state) do
      Agent.update(__MODULE__, fn calls ->
        [{:save_failed, projection_name, entity_id} | calls]
      end)

      {:error, :connection_refused}
    end
  end

  setup do
    # Ensure ETS table from previous test is cleaned up
    table = :projection_test_ps

    if :ets.whereis(table) != :undefined do
      :ets.delete(table)
    end

    :ok
  end

  defp start_server(opts \\ []) do
    defaults = [
      projection: TestProjection,
      core_client: MockCoreClient,
      sync_interval: 100_000,
      pubsub: QueryServiceEx.PubSub
    ]

    merged = Keyword.merge(defaults, opts)
    {:ok, pid} = ProjectionServer.start_link(merged)
    pid
  end

  describe "init" do
    test "creates ETS table and subscribes to PubSub" do
      pid = start_server()

      # ETS table exists
      assert :ets.whereis(:projection_test_ps) != :undefined

      # Process is alive
      assert Process.alive?(pid)

      GenServer.stop(pid)
    end

    test "hydrates state from Core on startup" do
      pid = start_server(core_client: HydrationCoreClient)
      # Give hydration message time to be processed
      Process.sleep(50)

      assert ProjectionServer.get_entity_state(pid, "ent-1") == %{
               "id" => "ent-1",
               "name" => "Alpha",
               "count" => 3
             }

      assert ProjectionServer.get_entity_state(pid, "ent-2") == %{
               "id" => "ent-2",
               "name" => "Beta",
               "count" => 1
             }

      GenServer.stop(pid)
    end

    test "handles Core unavailability on hydration gracefully" do
      pid = start_server(core_client: FailingCoreClient)
      Process.sleep(50)

      # Server is still alive, just no pre-existing state
      assert Process.alive?(pid)
      assert ProjectionServer.get_entity_state(pid, "anything") == nil

      GenServer.stop(pid)
    end
  end

  describe "PubSub event handling" do
    test "folds events into ETS state" do
      pid = start_server()
      Process.sleep(50)

      event = %{
        "event_type" => "test_ps.created",
        "entity_id" => "idx-1",
        "data" => %{"name" => "My Index"},
        "timestamp" => "2026-01-01T00:00:00Z"
      }

      send(pid, {:new_event, event})
      Process.sleep(10)

      assert ProjectionServer.get_entity_state(pid, "idx-1") == %{
               "id" => "idx-1",
               "name" => "My Index",
               "count" => 1
             }

      GenServer.stop(pid)
    end

    test "folds multiple events for same entity incrementally" do
      pid = start_server()
      Process.sleep(50)

      send(pid, {:new_event, %{
        "event_type" => "test_ps.created",
        "entity_id" => "idx-1",
        "data" => %{"name" => "Original"},
        "timestamp" => "2026-01-01T00:00:00Z"
      }})

      send(pid, {:new_event, %{
        "event_type" => "test_ps.updated",
        "entity_id" => "idx-1",
        "data" => %{"name" => "Updated"},
        "timestamp" => "2026-01-01T00:01:00Z"
      }})

      Process.sleep(10)

      state = ProjectionServer.get_entity_state(pid, "idx-1")
      assert state["name"] == "Updated"
      assert state["count"] == 2

      GenServer.stop(pid)
    end

    test "handles events for different entities independently" do
      pid = start_server()
      Process.sleep(50)

      send(pid, {:new_event, %{
        "event_type" => "test_ps.created",
        "entity_id" => "idx-1",
        "data" => %{"name" => "First"},
        "timestamp" => "2026-01-01T00:00:00Z"
      }})

      send(pid, {:new_event, %{
        "event_type" => "test_ps.created",
        "entity_id" => "idx-2",
        "data" => %{"name" => "Second"},
        "timestamp" => "2026-01-01T00:00:01Z"
      }})

      Process.sleep(10)

      assert ProjectionServer.get_entity_state(pid, "idx-1")["name"] == "First"
      assert ProjectionServer.get_entity_state(pid, "idx-2")["name"] == "Second"

      GenServer.stop(pid)
    end

    test "ignores events without entity_id" do
      pid = start_server()
      Process.sleep(50)

      send(pid, {:new_event, %{"event_type" => "test_ps.created", "data" => %{"name" => "No ID"}}})
      Process.sleep(10)

      assert ProjectionServer.get_all_states(pid) == []

      GenServer.stop(pid)
    end

    test "ignores unrecognized event types" do
      pid = start_server()
      Process.sleep(50)

      send(pid, {:new_event, %{
        "event_type" => "unknown.event",
        "entity_id" => "idx-1",
        "data" => %{}
      }})

      Process.sleep(10)

      # State should be initial_state (empty map), not nil
      assert ProjectionServer.get_entity_state(pid, "idx-1") == %{}

      GenServer.stop(pid)
    end
  end

  describe "dirty entity sync" do
    test "syncs dirty entities to Core on sync tick" do
      MockCoreClient.start_tracker()

      pid = start_server(sync_interval: 50)
      Process.sleep(30)

      send(pid, {:new_event, %{
        "event_type" => "test_ps.created",
        "entity_id" => "idx-1",
        "data" => %{"name" => "Synced"},
        "timestamp" => "2026-01-01T00:00:00Z"
      }})

      # Wait for sync tick
      Process.sleep(100)

      calls = MockCoreClient.get_calls()
      save_calls = Enum.filter(calls, fn {op, _, _, _} -> op == :save end)
      assert length(save_calls) >= 1

      {_, proj_name, eid, state} = hd(save_calls)
      assert eid == "idx-1"
      assert state["name"] == "Synced"
      assert is_binary(proj_name)

      GenServer.stop(pid)
    end

    test "retains failed syncs for next tick" do
      FailingSaveCoreClient.start_tracker()

      pid = start_server(core_client: FailingSaveCoreClient, sync_interval: 50)
      Process.sleep(30)

      send(pid, {:new_event, %{
        "event_type" => "test_ps.created",
        "entity_id" => "idx-1",
        "data" => %{"name" => "WillFail"},
        "timestamp" => "2026-01-01T00:00:00Z"
      }})

      # Wait for two sync ticks
      Process.sleep(150)

      calls = FailingSaveCoreClient.get_calls()
      # Should have retried at least twice (once per sync tick)
      assert length(calls) >= 2

      GenServer.stop(pid)
    end
  end

  describe "get_all_states" do
    test "returns all entity states" do
      pid = start_server()
      Process.sleep(50)

      for i <- 1..3 do
        send(pid, {:new_event, %{
          "event_type" => "test_ps.created",
          "entity_id" => "idx-#{i}",
          "data" => %{"name" => "Entity #{i}"},
          "timestamp" => "2026-01-01T00:00:0#{i}Z"
        }})
      end

      Process.sleep(10)

      all = ProjectionServer.get_all_states(pid)
      assert length(all) == 3
      names = Enum.map(all, & &1["name"]) |> Enum.sort()
      assert names == ["Entity 1", "Entity 2", "Entity 3"]

      GenServer.stop(pid)
    end
  end
end
