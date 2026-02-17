defmodule QueryServiceEx.Projections.FoldPipelineTest do
  @moduledoc """
  Unit tests for the snapshot-aware FoldPipeline.

  Uses inline mock modules for Core client to test snapshot lookup,
  delta event fetching, and fold logic without hitting Core.
  """
  use ExUnit.Case, async: true

  alias QueryServiceEx.Projections.FoldPipeline

  # Test projection module
  defmodule TestProjection do
    @behaviour QueryServiceEx.Projections.Behaviour

    @impl true
    def entity_type, do: "widget"

    @impl true
    def initial_state, do: %{}

    @impl true
    def apply_event(state, %{"event_type" => "widget.created"} = event) do
      Map.merge(state, %{
        "id" => event["entity_id"],
        "name" => get_in(event, ["data", "name"]),
        "is_deleted" => false,
        "created_at" => event["timestamp"],
        "updated_at" => event["timestamp"]
      })
    end

    def apply_event(state, %{"event_type" => "widget.updated"} = event) do
      state
      |> Map.merge(event["data"] || %{})
      |> Map.put("updated_at", event["timestamp"])
    end

    def apply_event(state, %{"event_type" => "widget.deleted"} = event) do
      Map.merge(state, %{"is_deleted" => true, "updated_at" => event["timestamp"]})
    end

    def apply_event(state, _event), do: state

    @impl true
    def filterable_fields, do: ["is_deleted", "name"]
  end

  # Mock Core client: no snapshots, returns events
  defmodule NoSnapshotClient do
    def list_snapshots(_entity_id), do: {:ok, []}

    def query_events(_tenant_id, _params, _opts) do
      {:ok,
       [
         %{
           "entity_id" => "w1",
           "event_type" => "widget.created",
           "timestamp" => "2024-01-01T00:00:00Z",
           "data" => %{"name" => "Alpha"}
         },
         %{
           "entity_id" => "w2",
           "event_type" => "widget.created",
           "timestamp" => "2024-01-02T00:00:00Z",
           "data" => %{"name" => "Beta"}
         },
         %{
           "entity_id" => "w1",
           "event_type" => "widget.updated",
           "timestamp" => "2024-01-03T00:00:00Z",
           "data" => %{"name" => "Alpha Updated"}
         }
       ]}
    end
  end

  # Mock Core client: has snapshot for w1, returns only delta events
  defmodule WithSnapshotClient do
    def list_snapshots(_entity_id) do
      {:ok,
       [
         %{
           "entity_id" => "w1",
           "as_of" => "2024-01-02T00:00:00Z",
           "state" => %{
             "id" => "w1",
             "name" => "Alpha",
             "is_deleted" => false,
             "created_at" => "2024-01-01T00:00:00Z",
             "updated_at" => "2024-01-01T00:00:00Z"
           }
         }
       ]}
    end

    def query_events(_tenant_id, params, _opts) do
      # Verify the since param is passed through
      if params[:since] == "2024-01-02T00:00:00Z" do
        {:ok,
         [
           %{
             "entity_id" => "w1",
             "event_type" => "widget.updated",
             "timestamp" => "2024-01-03T00:00:00Z",
             "data" => %{"name" => "Alpha Updated"}
           }
         ]}
      else
        {:ok, []}
      end
    end
  end

  # Mock Core client: snapshot exists but no new events
  defmodule SnapshotOnlyClient do
    def list_snapshots(_entity_id) do
      {:ok,
       [
         %{
           "entity_id" => "w1",
           "as_of" => "2024-01-05T00:00:00Z",
           "state" => %{
             "id" => "w1",
             "name" => "Fully Snapped",
             "is_deleted" => false,
             "created_at" => "2024-01-01T00:00:00Z",
             "updated_at" => "2024-01-05T00:00:00Z"
           }
         }
       ]}
    end

    def query_events(_tenant_id, _params, _opts), do: {:ok, []}
  end

  # Mock Core client: snapshot response wrapped in "snapshots" key
  defmodule WrappedSnapshotClient do
    def list_snapshots(_entity_id) do
      {:ok,
       %{
         "snapshots" => [
           %{
             "entity_id" => "w1",
             "as_of" => "2024-01-02T00:00:00Z",
             "state" => %{"id" => "w1", "name" => "Wrapped", "is_deleted" => false}
           }
         ]
       }}
    end

    def query_events(_tenant_id, _params, _opts), do: {:ok, []}
  end

  # Mock Core client: query_events returns error
  defmodule ErrorClient do
    def list_snapshots(_entity_id), do: {:ok, []}
    def query_events(_tenant_id, _params, _opts), do: {:error, "HTTP 500: Internal Server Error"}
  end

  # Mock Core client: no events at all
  defmodule EmptyClient do
    def list_snapshots(_entity_id), do: {:ok, []}
    def query_events(_tenant_id, _params, _opts), do: {:ok, []}
  end

  # Mock Core client: returns many events (exceeds snapshot threshold)
  # Tracks snapshot creation calls via Agent
  defmodule ManyEventsClient do
    def list_snapshots(_entity_id), do: {:ok, []}

    def query_events(_tenant_id, _params, _opts) do
      # Generate 150 events for 3 entities (exceeds default threshold of 100)
      events =
        for i <- 1..150 do
          eid = "w#{rem(i, 3) + 1}"

          %{
            "entity_id" => eid,
            "event_type" => "widget.created",
            "timestamp" => "2024-01-01T00:#{String.pad_leading("#{rem(i, 60)}", 2, "0")}:00Z",
            "data" => %{"name" => "Widget #{i}"}
          }
        end

      {:ok, events}
    end

    def create_snapshot(entity_id, snapshot_type) do
      Agent.update(:snapshot_tracker, fn calls ->
        [{:create_snapshot, entity_id, snapshot_type} | calls]
      end)

      {:ok, %{}}
    end

    def save_projection_state(projection_name, entity_id, state) do
      Agent.update(:snapshot_tracker, fn calls ->
        [{:save_projection_state, projection_name, entity_id, state} | calls]
      end)

      :ok
    end
  end

  # Mock Core client: returns few events (below threshold)
  defmodule FewEventsClient do
    def list_snapshots(_entity_id), do: {:ok, []}

    def query_events(_tenant_id, _params, _opts) do
      {:ok,
       [
         %{
           "entity_id" => "w1",
           "event_type" => "widget.created",
           "timestamp" => "2024-01-01T00:00:00Z",
           "data" => %{"name" => "Alpha"}
         }
       ]}
    end
  end

  describe "fold/3 without snapshots" do
    test "folds all events from scratch" do
      {:ok, result} =
        FoldPipeline.fold(TestProjection, nil,
          tenant_id: "test-tenant",
          core_client: NoSnapshotClient
        )

      assert result.metadata.snapshot_used == false
      assert result.metadata.events_after_snapshot == 3
      assert length(result.state) == 2

      w1 = Enum.find(result.state, &(&1["id"] == "w1"))
      assert w1["name"] == "Alpha Updated"

      w2 = Enum.find(result.state, &(&1["id"] == "w2"))
      assert w2["name"] == "Beta"
    end

    test "returns empty state for no events" do
      {:ok, result} =
        FoldPipeline.fold(TestProjection, nil,
          tenant_id: "test-tenant",
          core_client: EmptyClient
        )

      assert result.state == []
      assert result.metadata.snapshot_used == false
      assert result.metadata.events_after_snapshot == 0
    end
  end

  describe "fold/3 with snapshots" do
    test "uses snapshot as initial state and fetches only delta events" do
      {:ok, result} =
        FoldPipeline.fold(TestProjection, nil,
          tenant_id: "test-tenant",
          core_client: WithSnapshotClient
        )

      assert result.metadata.snapshot_used == true
      assert result.metadata.events_after_snapshot == 1

      [w1] = result.state
      assert w1["name"] == "Alpha Updated"
      assert w1["id"] == "w1"
    end

    test "returns snapshot state when no new events exist" do
      {:ok, result} =
        FoldPipeline.fold(TestProjection, nil,
          tenant_id: "test-tenant",
          core_client: SnapshotOnlyClient
        )

      assert result.metadata.snapshot_used == true
      assert result.metadata.events_after_snapshot == 0

      [w1] = result.state
      assert w1["name"] == "Fully Snapped"
    end

    test "handles wrapped snapshot response format" do
      {:ok, result} =
        FoldPipeline.fold(TestProjection, nil,
          tenant_id: "test-tenant",
          core_client: WrappedSnapshotClient
        )

      assert result.metadata.snapshot_used == true
      [w1] = result.state
      assert w1["name"] == "Wrapped"
    end
  end

  describe "fold/3 error handling" do
    test "propagates Core client errors" do
      assert {:error, _reason} =
               FoldPipeline.fold(TestProjection, nil,
                 tenant_id: "test-tenant",
                 core_client: ErrorClient
               )
    end
  end

  describe "fold/3 metadata" do
    test "includes fold_duration_ms" do
      {:ok, result} =
        FoldPipeline.fold(TestProjection, nil,
          tenant_id: "test-tenant",
          core_client: NoSnapshotClient
        )

      assert is_float(result.metadata.fold_duration_ms)
      assert result.metadata.fold_duration_ms >= 0
    end

    test "event_count includes snapshot contribution" do
      {:ok, result} =
        FoldPipeline.fold(TestProjection, nil,
          tenant_id: "test-tenant",
          core_client: WithSnapshotClient
        )

      # 1 delta event + 1 snapshot entity
      assert result.metadata.event_count == 2
    end

    test "snapshot_created is false when below threshold" do
      {:ok, result} =
        FoldPipeline.fold(TestProjection, nil,
          tenant_id: "test-tenant",
          core_client: FewEventsClient
        )

      assert result.metadata.snapshot_created == false
    end

    test "snapshot_created is false when no events" do
      {:ok, result} =
        FoldPipeline.fold(TestProjection, nil,
          tenant_id: "test-tenant",
          core_client: EmptyClient
        )

      assert result.metadata.snapshot_created == false
    end
  end

  describe "fold/3 lazy snapshot creation" do
    setup do
      {:ok, _pid} = Agent.start_link(fn -> [] end, name: :snapshot_tracker)
      :ok
    end

    test "triggers snapshot creation when events exceed threshold" do
      # Set threshold low for testing
      Application.put_env(:query_service_ex, :snapshot_threshold, 5)

      on_exit(fn ->
        Application.delete_env(:query_service_ex, :snapshot_threshold)
      end)

      {:ok, result} =
        FoldPipeline.fold(TestProjection, nil,
          tenant_id: "test-tenant",
          core_client: ManyEventsClient
        )

      assert result.metadata.snapshot_created == true
      assert result.metadata.events_after_snapshot == 150

      # Give the async Task time to complete
      Process.sleep(50)

      calls = Agent.get(:snapshot_tracker, & &1)

      # Should have create_snapshot calls for each entity
      create_calls =
        Enum.filter(calls, fn
          {:create_snapshot, _, _} -> true
          _ -> false
        end)
      assert length(create_calls) == 3

      # All should be "automatic" type
      assert Enum.all?(create_calls, fn {:create_snapshot, _eid, type} -> type == "automatic" end)

      # Should have save_projection_state calls
      save_calls =
        Enum.filter(calls, fn
          {:save_projection_state, _, _, _} -> true
          _ -> false
        end)

      assert length(save_calls) == 3

      # Projection name should match entity_type
      assert Enum.all?(save_calls, fn {:save_projection_state, name, _eid, _state} ->
               name == "widget"
             end)
    end

    test "does not trigger snapshot when below threshold" do
      Application.put_env(:query_service_ex, :snapshot_threshold, 200)

      on_exit(fn ->
        Application.delete_env(:query_service_ex, :snapshot_threshold)
      end)

      {:ok, result} =
        FoldPipeline.fold(TestProjection, nil,
          tenant_id: "test-tenant",
          core_client: ManyEventsClient
        )

      assert result.metadata.snapshot_created == false

      Process.sleep(50)

      calls = Agent.get(:snapshot_tracker, & &1)
      assert calls == []
    end

    test "uses default threshold of 100" do
      # Don't set any custom threshold — default is 100
      # ManyEventsClient returns 150 events, which exceeds 100
      {:ok, result} =
        FoldPipeline.fold(TestProjection, nil,
          tenant_id: "test-tenant",
          core_client: ManyEventsClient
        )

      assert result.metadata.snapshot_created == true
    end
  end
end
