defmodule QueryServiceExWeb.BillingControllerCoreQueryTest do
  @moduledoc """
  Guards the Core query `GET /api/billing/status` issues (issue #252).

  `query_billing_state/1` used to ask Core for `event_type: "billing.*"` with
  `sort: "desc"`. Core's `event_type` filter is an exact match and Core has no
  `sort` param, so the glob matched nothing and every tenant read as free tier.

  The fake Core here reimplements the three filter rules Core actually applies
  to `GET /api/v1/events/query` — exact `event_type`, prefix `event_type_prefix`,
  and `order=asc|desc` before `limit` — verified against
  `apps/core/src/store.rs` (`query/1`, `apply_filters/2`) and
  `apps/core/src/infrastructure/web/api.rs` (`query_events/4`). It deliberately
  ignores any param Core ignores, so a request Core would answer with nothing
  gets nothing here too.
  """
  use ExUnit.Case, async: false

  import Plug.Test, only: [conn: 3]

  alias QueryServiceExWeb.BillingController

  @events [
    %{
      "id" => "evt-1",
      "entity_id" => "tenant-a",
      "event_type" => "billing.subscription_created",
      "payload" => %{"tier" => "pro", "status" => "active"},
      "timestamp" => "2026-01-01T00:00:00Z"
    },
    %{
      "id" => "evt-2",
      "entity_id" => "tenant-a",
      "event_type" => "billing.subscription_updated",
      "payload" => %{"tier" => "growth", "status" => "active"},
      "timestamp" => "2026-02-01T00:00:00Z"
    }
  ]

  defmodule FakeCore do
    @moduledoc false
    import Plug.Conn

    def init(opts), do: opts

    def call(conn, _opts) do
      conn = fetch_query_params(conn)
      params = conn.query_params
      events = Agent.get(FakeCoreState, & &1.events)
      :ok = Agent.update(FakeCoreState, &%{&1 | query: params})

      matched =
        events
        |> Enum.filter(&matches?(&1, params))
        |> Enum.sort_by(& &1["timestamp"])
        |> order(params["order"])
        |> limit(params["limit"])

      body = %{
        "events" => matched,
        "count" => length(matched),
        "total_count" => length(matched),
        "has_more" => false
      }

      conn
      |> put_resp_content_type("application/json")
      |> send_resp(200, Jason.encode!(body))
    end

    # Core matches `event_type` EXACTLY (type index lookup) and
    # `event_type_prefix` by `String.starts_with?`. No globbing anywhere.
    defp matches?(event, params) do
      type = event["event_type"]

      exact_ok = is_nil(params["event_type"]) or type == params["event_type"]

      prefix_ok =
        is_nil(params["event_type_prefix"]) or
          String.starts_with?(type, params["event_type_prefix"])

      exact_ok and prefix_ok
    end

    # Core orders ascending by (timestamp, version) and reverses for
    # `order=desc`. It has no `sort` param — an unknown param is simply dropped.
    defp order(events, "desc"), do: Enum.reverse(events)
    defp order(events, _), do: events

    defp limit(events, nil), do: events
    defp limit(events, limit), do: Enum.take(events, String.to_integer(limit))
  end

  setup do
    start_supervised!(%{
      id: FakeCoreState,
      start:
        {Agent, :start_link, [fn -> %{query: %{}, events: @events} end, [name: FakeCoreState]]}
    })

    port = free_port()
    start_supervised!({Bandit, plug: FakeCore, scheme: :http, port: port, ip: {127, 0, 0, 1}})

    url = "http://127.0.0.1:#{port}"
    prev_core_url = Application.get_env(:query_service_ex, :core_url)
    prev_read_urls = Application.get_env(:query_service_ex, :core_read_urls)
    prev_write_url = Application.get_env(:query_service_ex, :core_write_url)

    Application.put_env(:query_service_ex, :core_url, url)
    Application.put_env(:query_service_ex, :core_read_urls, [url])
    Application.put_env(:query_service_ex, :core_write_url, url)

    on_exit(fn ->
      restore_env(:core_url, prev_core_url)
      restore_env(:core_read_urls, prev_read_urls)
      restore_env(:core_write_url, prev_write_url)
    end)

    :ok
  end

  describe "GET /api/billing/status" do
    test "reports the tenant's real tier instead of falling back to free" do
      body = call_status()

      # With `event_type: "billing.*"` Core matched nothing and every paying
      # tenant was served `default_billing_state/1` — tier "free".
      assert body["tier"] == "growth"
      assert body["events_quota"] == 10_000_000
      assert body["last_updated"] == "2026-02-01T00:00:00Z"
    end

    test "asks Core for the NEWEST billing event, not the oldest" do
      body = call_status()

      # `sort: "desc"` is not a Core param, so it was dropped and `limit: 1`
      # returned the FIRST billing event — the tier the tenant signed up on,
      # never any later upgrade or downgrade.
      refute body["tier"] == "pro"
      assert body["tier"] == "growth"
    end

    test "sends filters Core understands and no phantom params" do
      call_status()
      forwarded = forwarded_query()

      assert forwarded["event_type_prefix"] == "billing."
      assert forwarded["order"] == "desc"
      assert forwarded["limit"] == "1"
      assert forwarded["tenant_id"] == "tenant-a"

      # Core has no `sort`; a glob in `event_type` matches nothing.
      refute Map.has_key?(forwarded, "sort")
      refute forwarded["event_type"] == "billing.*"
    end

    test "still reports free tier when the tenant has no billing events" do
      Agent.update(FakeCoreState, &%{&1 | events: []})

      body = call_status()

      assert body["tier"] == "free"
      assert body["last_updated"] == nil
    end
  end

  defp call_status do
    conn =
      :get
      |> conn("/api/billing/status", nil)
      |> Plug.Conn.fetch_query_params()
      |> Plug.Conn.assign(:tenant_id, "tenant-a")
      |> then(&BillingController.status(&1, &1.params))

    assert conn.status == 200
    Jason.decode!(conn.resp_body)
  end

  defp forwarded_query, do: Agent.get(FakeCoreState, & &1.query)

  defp restore_env(key, nil), do: Application.delete_env(:query_service_ex, key)
  defp restore_env(key, value), do: Application.put_env(:query_service_ex, key, value)

  defp free_port do
    {:ok, socket} = :gen_tcp.listen(0, [:binary, active: false, ip: {127, 0, 0, 1}])
    {:ok, port} = :inet.port(socket)
    :ok = :gen_tcp.close(socket)
    port
  end
end
