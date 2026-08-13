defmodule QueryServiceExWeb.ReplayControllerTest do
  use ExUnit.Case, async: false

  import Plug.Conn
  import Plug.Test

  alias QueryServiceEx.Projections.Enablement
  alias QueryServiceEx.Projections.TenantProjections
  alias QueryServiceExWeb.ReplayController

  defmodule StubCore do
    def start, do: Agent.start_link(fn -> %{} end, name: __MODULE__)
    def reset, do: Agent.update(__MODULE__, fn _ -> %{} end)

    def get_tenant(tenant_id) do
      metadata = Agent.get(__MODULE__, &Map.get(&1, tenant_id, %{}))
      {:ok, %{"id" => tenant_id, "metadata" => metadata}}
    end

    def merge_tenant_metadata(tenant_id, partial) do
      Agent.update(__MODULE__, fn state -> Map.put(state, tenant_id, partial) end)
      {:ok, %{}}
    end
  end

  setup do
    TenantProjections.init_tables()

    if Process.whereis(QueryServiceEx.Projections.BackfillSupervisor) == nil do
      {:ok, _} = Task.Supervisor.start_link(name: QueryServiceEx.Projections.BackfillSupervisor)
    end

    if Process.whereis(TenantProjections) == nil do
      {:ok, _} = TenantProjections.start_link([])
    end

    case StubCore.start() do
      {:ok, _} -> :ok
      {:error, {:already_started, _}} -> StubCore.reset()
    end

    Application.put_env(:query_service_ex, :projections_core_client, StubCore)

    Application.put_env(:query_service_ex, :tenant_projection_query_fun, fn tenant, _params ->
      {:ok, [%{"tenant_id" => tenant, "event_type" => "order.created", "entity_id" => "1"}]}
    end)

    on_exit(fn ->
      Application.delete_env(:query_service_ex, :projections_core_client)
      Application.delete_env(:query_service_ex, :tenant_projection_query_fun)
      :ets.delete_all_objects(:tenant_projection_state)
      :ets.delete_all_objects(:tenant_projection_status)
      :ets.delete_all_objects(:tenant_projection_generation)
    end)

    :ok
  end

  test "starts and lists a tenant-scoped projection rebuild" do
    tenant = "tenant-replay-controller"
    assert {:ok, _} = Enablement.enable(tenant, "event-count")
    wait_until(fn -> TenantProjections.status(tenant, "event-count") == :ready end)

    conn =
      ReplayController.create(tenant_conn(:post, "/api/replay", tenant), %{
        "projection_name" => "event-count"
      })

    assert conn.status == 202
    replay_id = body(conn)["data"]["replay_id"]

    wait_until(fn ->
      match?({:ok, %{status: "completed"}}, TenantProjections.get_replay(tenant, replay_id))
    end)

    list_conn = ReplayController.index(tenant_conn(:get, "/api/replay", tenant), %{})
    assert list_conn.status == 200

    assert [%{"projection_name" => "event-count", "status" => "completed"}] =
             body(list_conn)["data"]
  end

  test "does not reveal another tenant's replay" do
    owner = "tenant-owner"
    assert {:ok, _} = Enablement.enable(owner, "event-count")
    wait_until(fn -> TenantProjections.status(owner, "event-count") == :ready end)
    assert {:ok, replay} = TenantProjections.rebuild(owner, "event-count")

    conn =
      ReplayController.show(
        tenant_conn(:get, "/api/replay/#{replay.replay_id}", "tenant-other"),
        %{
          "id" => replay.replay_id
        }
      )

    assert conn.status == 404
    assert body(conn)["error"] == "Replay not found"
  end

  test "rejects missing or disabled target" do
    conn = ReplayController.create(tenant_conn(:post, "/api/replay", "tenant-empty"), %{})
    assert conn.status == 422
    assert body(conn)["error"] == "Choose an enabled projection to rebuild"
  end

  defp tenant_conn(method, path, tenant_id) do
    conn(method, path)
    |> put_req_header("content-type", "application/json")
    |> put_private(:phoenix_endpoint, QueryServiceExWeb.Endpoint)
    |> assign(:tenant_id, tenant_id)
  end

  defp body(conn), do: Jason.decode!(conn.resp_body)

  defp wait_until(fun, retries \\ 100)
  defp wait_until(_fun, 0), do: flunk("condition not met in time")

  defp wait_until(fun, retries) do
    if fun.() do
      :ok
    else
      Process.sleep(20)
      wait_until(fun, retries - 1)
    end
  end
end
