defmodule QueryServiceEx.Projections.TenantProjectionsTest do
  @moduledoc """
  Tests for the QS-owned per-tenant projection state store + folding.

  Verifies tenant-keyed state, background backfill folding a seeded event
  history, live folding off the tenant-scoped PubSub topic, and cross-tenant
  isolation. The backfill source is injected via app env (no HTTP/Core needed).
  """
  use ExUnit.Case, async: false

  alias QueryServiceEx.Projections.TenantProjections

  @state_table :tenant_projection_state
  @status_table :tenant_projection_status

  setup do
    TenantProjections.init_tables()

    # Ensure the GenServer + backfill task supervisor are running. The app may
    # already have started them (singletons); only start what is missing.
    if Process.whereis(QueryServiceEx.Projections.BackfillSupervisor) == nil do
      start_supervised!({Task.Supervisor, name: QueryServiceEx.Projections.BackfillSupervisor})
    end

    if Process.whereis(TenantProjections) == nil do
      start_supervised!(TenantProjections)
    end

    on_exit(fn ->
      :ets.delete_all_objects(@state_table)
      :ets.delete_all_objects(@status_table)
      Application.delete_env(:query_service_ex, :tenant_projection_query_fun)
    end)

    :ok
  end

  # Inject a deterministic backfill source keyed by tenant. The backfill runs in
  # a Task, so we override the module's query function via app env rather than a
  # process-local HTTP mock.
  defp seed_core(events_by_tenant) do
    Application.put_env(:query_service_ex, :tenant_projection_query_fun, fn tenant, params ->
      offset = params[:offset] || 0
      limit = params[:limit] || 1000
      all = Map.get(events_by_tenant, tenant, [])
      {:ok, all |> Enum.drop(offset) |> Enum.take(limit)}
    end)
  end

  describe "enable + backfill" do
    test "folds a seeded event history into expected state" do
      tenant = "tenant-aaa"

      events = [
        %{"event_type" => "user.created", "entity_id" => "u1", "tenant_id" => tenant},
        %{"event_type" => "user.created", "entity_id" => "u2", "tenant_id" => tenant},
        %{"event_type" => "order.placed", "entity_id" => "o1", "tenant_id" => tenant}
      ]

      seed_core(%{tenant => events})

      assert :ok = TenantProjections.enable(tenant, "event-count")

      # status begins :building
      assert TenantProjections.status(tenant, "event-count") in [:building, :ready]

      # wait for backfill to complete
      wait_until(fn -> TenantProjections.status(tenant, "event-count") == :ready end)

      assert {:ok, state} =
               TenantProjections.get_state(
                 tenant,
                 "event-count",
                 QueryServiceEx.Projections.Catalog.tenant_key()
               )

      assert state["total"] == 3
      assert state["by_event_type"] == %{"user.created" => 2, "order.placed" => 1}
    end

    test "entity-activity backfill folds per entity" do
      tenant = "tenant-ent"

      events = [
        %{"event_type" => "a", "entity_id" => "x", "tenant_id" => tenant, "timestamp" => "t1"},
        %{"event_type" => "b", "entity_id" => "x", "tenant_id" => tenant, "timestamp" => "t2"},
        %{"event_type" => "c", "entity_id" => "y", "tenant_id" => tenant, "timestamp" => "t3"}
      ]

      seed_core(%{tenant => events})

      assert :ok = TenantProjections.enable(tenant, "entity-activity")
      wait_until(fn -> TenantProjections.status(tenant, "entity-activity") == :ready end)

      assert {:ok, sx} = TenantProjections.get_state(tenant, "entity-activity", "x")
      assert sx["event_count"] == 2
      assert sx["last_event_type"] == "b"

      assert {:ok, sy} = TenantProjections.get_state(tenant, "entity-activity", "y")
      assert sy["event_count"] == 1
    end

    test "unknown template is rejected" do
      assert {:error, :unknown_template} = TenantProjections.enable("t", "nope")
    end
  end

  describe "live folding" do
    test "a new event off the tenant topic folds incrementally" do
      tenant = "tenant-live"
      seed_core(%{tenant => []})

      assert :ok = TenantProjections.enable(tenant, "event-count")
      wait_until(fn -> TenantProjections.status(tenant, "event-count") == :ready end)

      # broadcast a live event on the tenant-scoped topic
      Phoenix.PubSub.broadcast(
        QueryServiceEx.PubSub,
        "events:#{tenant}:all",
        {:new_event, %{"event_type" => "x.y", "entity_id" => "e1", "tenant_id" => tenant}}
      )

      wait_until(fn ->
        match?(
          {:ok, %{"total" => 1}},
          TenantProjections.get_state(
            tenant,
            "event-count",
            QueryServiceEx.Projections.Catalog.tenant_key()
          )
        )
      end)
    end
  end

  describe "cross-tenant isolation" do
    test "tenant A's state is never visible to tenant B" do
      a = "tenant-A"
      b = "tenant-B"

      seed_core(%{
        a => [%{"event_type" => "e", "entity_id" => "1", "tenant_id" => a}],
        b => []
      })

      assert :ok = TenantProjections.enable(a, "event-count")
      assert :ok = TenantProjections.enable(b, "event-count")

      wait_until(fn ->
        TenantProjections.status(a, "event-count") == :ready and
          TenantProjections.status(b, "event-count") == :ready
      end)

      key = QueryServiceEx.Projections.Catalog.tenant_key()
      assert {:ok, %{"total" => 1}} = TenantProjections.get_state(a, "event-count", key)
      # B has no events — its bucket either absent or zero, never A's data
      assert TenantProjections.get_state(b, "event-count", key) == {:error, :not_found}

      # list is tenant-scoped
      assert [%{name: "event-count"}] = TenantProjections.list(a)
      assert [%{name: "event-count"}] = TenantProjections.list(b)
    end
  end

  describe "disable" do
    test "drops the tenant's state and status" do
      tenant = "tenant-del"
      seed_core(%{tenant => [%{"event_type" => "e", "entity_id" => "1", "tenant_id" => tenant}]})

      assert :ok = TenantProjections.enable(tenant, "event-count")
      wait_until(fn -> TenantProjections.status(tenant, "event-count") == :ready end)

      assert :ok = TenantProjections.disable(tenant, "event-count")
      assert TenantProjections.status(tenant, "event-count") == nil

      key = QueryServiceEx.Projections.Catalog.tenant_key()
      assert TenantProjections.get_state(tenant, "event-count", key) == {:error, :not_found}
    end
  end

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
