defmodule QueryServiceExWeb.EventControllerCoreCompatTest do
  @moduledoc """
  Fidelity tests for the Core-compatible query endpoint (issue #252).

  These drive the real controller action against a fake Core so both hops are
  exercised: the query string the gateway sends upstream, and the envelope it
  returns to SDK clients.
  """
  use ExUnit.Case, async: false

  import Plug.Test, only: [conn: 3]

  alias QueryServiceExWeb.EventController

  defmodule FakeCore do
    @moduledoc false
    import Plug.Conn

    def init(opts), do: opts

    def call(conn, _opts) do
      conn = fetch_query_params(conn)
      body = Agent.get_and_update(FakeCoreState, &{&1.body, %{&1 | query: conn.query_params}})

      conn
      |> put_resp_content_type("application/json")
      |> send_resp(200, Jason.encode!(body))
    end
  end

  @event %{
    "id" => "evt-1",
    "entity_id" => "order-1",
    "event_type" => "order.placed",
    "payload" => %{},
    "timestamp" => "2026-01-01T00:00:00Z"
  }

  @core_body %{
    "events" => [@event],
    "count" => 1,
    "total_count" => 7,
    "has_more" => true,
    "entity_version" => 3
  }

  setup do
    start_supervised!(%{
      id: FakeCoreState,
      start:
        {Agent, :start_link, [fn -> %{query: %{}, body: @core_body} end, [name: FakeCoreState]]}
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

  describe "GET /api/v1/events/query (Core-compatible)" do
    test "forwards every filter Core accepts" do
      query =
        URI.encode_query(%{
          "entity_id" => "order-1",
          "event_type" => "order.placed",
          "event_type_prefix" => "order.",
          "exclude_event_type_prefix" => "audit.,_system.",
          "payload_filter" => ~s({"user_id":"abc-123"}),
          "since" => "2026-01-01T00:00:00Z",
          "until" => "2026-02-01T00:00:00Z",
          "as_of" => "2026-01-15T00:00:00Z",
          "order" => "desc",
          "limit" => "25",
          "offset" => "50"
        })

      call_query(query)

      forwarded = forwarded_query()

      assert forwarded["entity_id"] == "order-1"
      assert forwarded["event_type"] == "order.placed"
      assert forwarded["event_type_prefix"] == "order."
      assert forwarded["exclude_event_type_prefix"] == "audit.,_system."
      assert forwarded["payload_filter"] == ~s({"user_id":"abc-123"})
      assert forwarded["since"] == "2026-01-01T00:00:00Z"
      assert forwarded["until"] == "2026-02-01T00:00:00Z"
      assert forwarded["as_of"] == "2026-01-15T00:00:00Z"
      assert forwarded["order"] == "desc"
      assert forwarded["limit"] == "25"
      assert forwarded["offset"] == "50"
      assert forwarded["tenant_id"] == "tenant-a"
    end

    test "passes Core's pagination metadata through to the client" do
      conn = call_query("limit=1&offset=0")

      assert conn.status == 200
      body = Jason.decode!(conn.resp_body)

      assert body["count"] == 1
      assert body["total_count"] == 7
      assert body["has_more"] == true
      assert body["entity_version"] == 3
      assert length(body["events"]) == 1
    end

    test "omits metadata Core did not send instead of synthesizing it" do
      # An older Core omits `has_more`; emitting a made-up `false` would stop a
      # paginator early, so the key must simply be absent.
      set_core_body(%{"events" => [@event], "count" => 1})

      conn = call_query("limit=1")
      body = Jason.decode!(conn.resp_body)

      assert body["count"] == 1
      refute Map.has_key?(body, "has_more")
      refute Map.has_key?(body, "total_count")
    end
  end

  describe "GET /api/v1/events/recent" do
    test "asks Core for newest-first ordering" do
      :get
      |> conn("/api/v1/events/recent?limit=5", nil)
      |> Plug.Conn.fetch_query_params()
      |> Plug.Conn.assign(:tenant_id, "tenant-a")
      |> then(&EventController.recent(&1, &1.params))

      forwarded = forwarded_query()

      # Core orders by `order=asc|desc`; it has no `sort` param, so a `sort`
      # value is silently discarded and "recent" returns the OLDEST events.
      assert forwarded["order"] == "desc"
      assert forwarded["limit"] == "5"
    end
  end

  defp call_query(query_string) do
    :get
    |> conn("/api/v1/events/query?" <> query_string, nil)
    |> Plug.Conn.fetch_query_params()
    |> Plug.Conn.assign(:tenant_id, "tenant-a")
    |> then(&EventController.query_core_compat(&1, &1.params))
  end

  defp forwarded_query, do: Agent.get(FakeCoreState, & &1.query)

  defp set_core_body(body), do: Agent.update(FakeCoreState, &%{&1 | body: body})

  defp restore_env(key, nil), do: Application.delete_env(:query_service_ex, key)
  defp restore_env(key, value), do: Application.put_env(:query_service_ex, key, value)

  defp free_port do
    {:ok, socket} = :gen_tcp.listen(0, [:binary, active: false, ip: {127, 0, 0, 1}])
    {:ok, port} = :inet.port(socket)
    :ok = :gen_tcp.close(socket)
    port
  end
end
