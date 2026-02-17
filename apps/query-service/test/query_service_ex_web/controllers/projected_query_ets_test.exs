defmodule QueryServiceExWeb.ProjectedQueryEtsTest do
  @moduledoc """
  Tests for the dual read path in ProjectedQueryController:
  ETS read when ProjectionServer is running, fold-on-read fallback otherwise.
  """
  use ExUnit.Case, async: false

  alias QueryServiceEx.Projections.ProjectionServer
  alias QueryServiceEx.Projections.ProjectionSupervisor

  # Stop the app-level projection supervisor's children to avoid conflicts
  setup do
    # The app-level ProjectionSupervisor may have started "indices" already.
    # Stop it to get a clean slate for testing.
    ProjectionSupervisor.stop_projection("indices")
    Process.sleep(20)

    on_exit(fn ->
      # Restart "indices" for other tests
      ProjectionSupervisor.start_projection("indices")
    end)

    :ok
  end

  defp via_name(projection_name) do
    {:via, Elixir.Registry, {QueryServiceEx.ProjectionRegistry, {:projection_server, projection_name}}}
  end

  # Minimal Core client that returns empty state (for ProjectionServer hydration)
  defmodule EmptyCoreClient do
    def get_projection_state_summary(_name), do: {:ok, []}
    def save_projection_state(_name, _eid, _state), do: :ok
  end

  # Core client for fold-on-read fallback — returns canned events
  defmodule FoldCoreClient do
    def list_snapshots(_entity_id), do: {:ok, []}

    def query_events(_tenant_id, _params, _opts) do
      {:ok, [
        %{
          "event_type" => "index.created",
          "entity_id" => "fold-1",
          "data" => %{"name" => "Folded Index"},
          "timestamp" => "2026-01-01T00:00:00Z"
        }
      ]}
    end
  end

  describe "ETS read path" do
    test "reads from ETS when ProjectionServer is running" do
      # Start a ProjectionServer for "indices" with via-name registration
      {:ok, pid} =
        ProjectionServer.start_link(
          projection: QueryServiceEx.Projections.IndexState,
          name: via_name("indices"),
          core_client: EmptyCoreClient,
          sync_interval: 100_000,
          pubsub: QueryServiceEx.PubSub
        )

      Process.sleep(30)

      # Inject some state via PubSub event
      send(pid, {:new_event, %{
        "event_type" => "index.created",
        "entity_id" => "ets-1",
        "data" => %{"name" => "ETS Index"},
        "timestamp" => "2026-01-01T00:00:00Z"
      }})

      Process.sleep(10)

      # Build a conn that simulates an authenticated request
      conn =
        Plug.Test.conn(:post, "/api/query/projected", "")
        |> Plug.Conn.assign(:tenant_id, "test-tenant")
        |> Plug.Conn.assign(:consistency, :eventual)

      params = %{
        "projection" => "indices",
        "filters" => %{},
        "page" => 1,
        "page_size" => 50
      }

      # Call execute directly (bypass router/auth)
      conn = QueryServiceExWeb.ProjectedQueryController.execute(conn, params)

      response = Jason.decode!(conn.resp_body)

      assert response["projection"] == "indices"
      assert is_list(response["data"])
      assert length(response["data"]) == 1
      assert hd(response["data"])["name"] == "ETS Index"
      assert response["folded_from"]["source"] == "ets"
      assert response["count"] == 1
      assert response["total"] == 1

      GenServer.stop(pid)
    end

    test "returns source: ets in metadata when reading from ETS" do
      {:ok, pid} =
        ProjectionServer.start_link(
          projection: QueryServiceEx.Projections.IndexState,
          name: via_name("indices"),
          core_client: EmptyCoreClient,
          sync_interval: 100_000,
          pubsub: QueryServiceEx.PubSub
        )

      Process.sleep(30)

      conn =
        Plug.Test.conn(:post, "/api/query/projected", "")
        |> Plug.Conn.assign(:tenant_id, "test-tenant")
        |> Plug.Conn.assign(:consistency, :eventual)

      params = %{"projection" => "indices", "filters" => %{}}

      conn = QueryServiceExWeb.ProjectedQueryController.execute(conn, params)
      response = Jason.decode!(conn.resp_body)

      assert response["folded_from"]["source"] == "ets"
      assert is_number(response["folded_from"]["fold_duration_ms"])

      GenServer.stop(pid)
    end

    test "applies filters on ETS results" do
      {:ok, pid} =
        ProjectionServer.start_link(
          projection: QueryServiceEx.Projections.IndexState,
          name: via_name("indices"),
          core_client: EmptyCoreClient,
          sync_interval: 100_000,
          pubsub: QueryServiceEx.PubSub
        )

      Process.sleep(30)

      # Insert two entities: one deleted, one not
      send(pid, {:new_event, %{
        "event_type" => "index.created",
        "entity_id" => "ets-alive",
        "data" => %{"name" => "Alive"},
        "timestamp" => "2026-01-01T00:00:00Z"
      }})

      send(pid, {:new_event, %{
        "event_type" => "index.created",
        "entity_id" => "ets-deleted",
        "data" => %{"name" => "Deleted"},
        "timestamp" => "2026-01-01T00:00:01Z"
      }})

      send(pid, {:new_event, %{
        "event_type" => "index.deleted",
        "entity_id" => "ets-deleted",
        "data" => %{},
        "timestamp" => "2026-01-01T00:00:02Z"
      }})

      Process.sleep(10)

      conn =
        Plug.Test.conn(:post, "/api/query/projected", "")
        |> Plug.Conn.assign(:tenant_id, "test-tenant")
        |> Plug.Conn.assign(:consistency, :eventual)

      params = %{
        "projection" => "indices",
        "filters" => %{"is_deleted" => false}
      }

      conn = QueryServiceExWeb.ProjectedQueryController.execute(conn, params)
      response = Jason.decode!(conn.resp_body)

      assert response["total"] == 1
      assert hd(response["data"])["name"] == "Alive"

      GenServer.stop(pid)
    end

    test "paginates ETS results" do
      {:ok, pid} =
        ProjectionServer.start_link(
          projection: QueryServiceEx.Projections.IndexState,
          name: via_name("indices"),
          core_client: EmptyCoreClient,
          sync_interval: 100_000,
          pubsub: QueryServiceEx.PubSub
        )

      Process.sleep(30)

      for i <- 1..5 do
        send(pid, {:new_event, %{
          "event_type" => "index.created",
          "entity_id" => "ets-#{i}",
          "data" => %{"name" => "Index #{i}"},
          "timestamp" => "2026-01-01T00:00:0#{i}Z"
        }})
      end

      Process.sleep(10)

      conn =
        Plug.Test.conn(:post, "/api/query/projected", "")
        |> Plug.Conn.assign(:tenant_id, "test-tenant")
        |> Plug.Conn.assign(:consistency, :eventual)

      params = %{"projection" => "indices", "filters" => %{}, "page" => 1, "page_size" => 2}
      conn = QueryServiceExWeb.ProjectedQueryController.execute(conn, params)
      response = Jason.decode!(conn.resp_body)

      assert response["count"] == 2
      assert response["total"] == 5
      assert response["page"] == 1
      assert response["page_size"] == 2

      GenServer.stop(pid)
    end
  end

  describe "consistency=strong forces fold-on-read" do
    test "uses fold-on-read when consistency is :strong even if ProjectionServer is running" do
      {:ok, pid} =
        ProjectionServer.start_link(
          projection: QueryServiceEx.Projections.IndexState,
          name: via_name("indices"),
          core_client: EmptyCoreClient,
          sync_interval: 100_000,
          pubsub: QueryServiceEx.PubSub
        )

      Process.sleep(30)

      # Insert state into ETS
      send(pid, {:new_event, %{
        "event_type" => "index.created",
        "entity_id" => "ets-strong",
        "data" => %{"name" => "ETS Only"},
        "timestamp" => "2026-01-01T00:00:00Z"
      }})

      Process.sleep(10)

      # With consistency=strong, should NOT read from ETS — falls back to fold-on-read.
      # When calling controller directly, action_fallback doesn't fire,
      # so {:error, reason} is returned as-is if Core is unreachable.
      conn =
        Plug.Test.conn(:post, "/api/query/projected", "")
        |> Plug.Conn.assign(:tenant_id, "test-tenant")
        |> Plug.Conn.assign(:consistency, :strong)

      params = %{"projection" => "indices", "filters" => %{}}
      result = QueryServiceExWeb.ProjectedQueryController.execute(conn, params)

      case result do
        {:error, _reason} ->
          # Expected: bypassed ETS, tried fold-on-read, Core unreachable
          :ok

        %Plug.Conn{} = returned_conn ->
          response = Jason.decode!(returned_conn.resp_body)
          assert response["folded_from"]["source"] == "fold_on_read"
      end

      GenServer.stop(pid)
    end
  end

  describe "fold-on-read fallback" do
    test "uses fold-on-read when no ProjectionServer is running" do
      # Ensure no ProjectionServer is registered for "indices"
      assert GenServer.whereis(via_name("indices")) == nil

      conn =
        Plug.Test.conn(:post, "/api/query/projected", "")
        |> Plug.Conn.assign(:tenant_id, "test-tenant")
        |> Plug.Conn.assign(:consistency, :eventual)

      params = %{"projection" => "indices", "filters" => %{}}
      result = QueryServiceExWeb.ProjectedQueryController.execute(conn, params)

      case result do
        {:error, _reason} ->
          # Expected: no ProjectionServer, fold-on-read tried, Core unreachable
          :ok

        %Plug.Conn{} = returned_conn ->
          response = Jason.decode!(returned_conn.resp_body)
          assert response["folded_from"]["source"] == "fold_on_read"
      end
    end
  end
end
