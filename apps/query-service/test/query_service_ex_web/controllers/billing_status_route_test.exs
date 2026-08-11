defmodule QueryServiceExWeb.BillingStatusRouteTest do
  @moduledoc """
  `GET /api/billing/status` driven through the REAL router, pipelines included.

  The #252 sweep fixed the Core query this endpoint issues but tested the
  controller action directly with a hand-assigned `:tenant_id`. In the router the
  route sat on the `:api` pipeline, which never assigns one — so in production the
  fixed query was never reached: `conn.assigns[:tenant_id]` was nil, and
  `RustCoreClient.query_events/3` (guarded on `is_binary(tenant_id)`) raised a
  FunctionClauseError. Every request 500'd. That is the "HTTP 500 (Core proxy
  error) | broken endpoint" line in docs/checklists/dashboard-data-audit.md.

  These tests hold the pipeline, not just the action: no credentials must be 401
  (never 500), and a credentialed request must reach Core with the tenant from the
  CREDENTIAL — never the `?tenant_id=` the web client puts on the query string.
  """
  use ExUnit.Case, async: false

  import Plug.Test, only: [conn: 3]

  alias QueryServiceExWeb.Router

  @opts Router.init([])

  @billing_event %{
    "id" => "evt-1",
    "entity_id" => "tenant-from-jwt",
    "event_type" => "billing.subscription_updated",
    "payload" => %{"tier" => "growth", "status" => "active"},
    "timestamp" => "2026-02-01T00:00:00Z"
  }

  defmodule FakeCore do
    @moduledoc false
    import Plug.Conn

    def init(opts), do: opts

    def call(conn, _opts) do
      conn = fetch_query_params(conn)
      events = Agent.get(FakeCoreState, & &1.events)
      :ok = Agent.update(FakeCoreState, &%{&1 | query: conn.query_params})

      body = %{"events" => events, "count" => length(events)}

      conn
      |> put_resp_content_type("application/json")
      |> send_resp(200, Jason.encode!(body))
    end
  end

  setup do
    start_supervised!(%{
      id: FakeCoreState,
      start:
        {Agent, :start_link,
         [fn -> %{query: %{}, events: [@billing_event]} end, [name: FakeCoreState]]}
    })

    port = free_port()
    start_supervised!({Bandit, plug: FakeCore, scheme: :http, port: port, ip: {127, 0, 0, 1}})

    url = "http://127.0.0.1:#{port}"
    prev = Enum.map([:core_url, :core_read_urls, :core_write_url], &{&1, get_env(&1)})
    prev_secret = System.get_env("JWT_SECRET")
    prev_auth_disabled = System.get_env("AUTH_DISABLED")

    Application.put_env(:query_service_ex, :core_url, url)
    Application.put_env(:query_service_ex, :core_read_urls, [url])
    Application.put_env(:query_service_ex, :core_write_url, url)

    # The auth pipeline is the thing under test — a dev-mode bypass would assign a
    # tenant for us and hide exactly the defect this file guards.
    System.delete_env("AUTH_DISABLED")
    System.put_env("JWT_SECRET", jwt_secret())

    on_exit(fn ->
      Enum.each(prev, fn {key, value} -> restore_env(key, value) end)
      restore_system_env("JWT_SECRET", prev_secret)
      restore_system_env("AUTH_DISABLED", prev_auth_disabled)
    end)

    :ok
  end

  describe "GET /api/billing/status" do
    test "answers 401 without credentials instead of crashing with a 500" do
      conn = request(nil)

      assert conn.status == 401,
             "expected a fail-closed 401, got #{conn.status} — a 500 here means the " <>
               "route has no tenant context and the Core query is never reached"

      assert %{"error" => %{"code" => "unauthorized"}} = Jason.decode!(conn.resp_body)
    end

    test "serves the tenant's real tier to an authenticated caller" do
      conn = request(token(tenant_id: "tenant-from-jwt"))

      assert conn.status == 200
      body = Jason.decode!(conn.resp_body)

      assert body["tenant_id"] == "tenant-from-jwt"
      assert body["tier"] == "growth"
      assert body["events_quota"] == 10_000_000
    end

    test "scopes the Core read to the credential's tenant, not the query string" do
      # The web client calls `/api/billing/status?tenant_id=<x>`
      # (apps/web/src/lib/api/client.ts getBillingStatus). That param must never
      # reach Core — it would be a cross-tenant billing read.
      conn = request(token(tenant_id: "tenant-from-jwt"), "?tenant_id=someone-elses-tenant")

      assert conn.status == 200

      forwarded = Agent.get(FakeCoreState, & &1.query)
      assert forwarded["tenant_id"] == "tenant-from-jwt"
      refute forwarded["tenant_id"] == "someone-elses-tenant"
    end

    test "rejects an expired token rather than serving billing state" do
      conn = request(token(tenant_id: "tenant-from-jwt", exp: -60))

      assert conn.status == 401
    end
  end

  describe "BillingController.status/2 fail-closed guard" do
    test "returns 401 when the connection carries no tenant assign" do
      # Defense in depth for the router: if a pipeline change ever drops the auth
      # plug again, the action must answer 401 rather than raise a 500 out of the
      # Core client's is_binary/1 guard.
      conn =
        :get
        |> conn("/api/billing/status", nil)
        |> Plug.Conn.fetch_query_params()
        |> QueryServiceExWeb.BillingController.status(%{})

      assert conn.status == 401
    end

    test "returns 401 for a blank tenant assign" do
      conn =
        :get
        |> conn("/api/billing/status", nil)
        |> Plug.Conn.fetch_query_params()
        |> Plug.Conn.assign(:tenant_id, "")
        |> QueryServiceExWeb.BillingController.status(%{})

      assert conn.status == 401
    end
  end

  defp request(token, query_string \\ "") do
    :get
    |> conn("/api/billing/status" <> query_string, nil)
    |> Plug.Conn.put_req_header("content-type", "application/json")
    # The Endpoint fetches query params before the router in production. Without
    # this the controller never even sees `?tenant_id=`, and the cross-tenant
    # assertion below would pass for the wrong reason.
    |> Plug.Conn.fetch_query_params()
    |> maybe_authorize(token)
    |> Router.call(@opts)
  end

  defp maybe_authorize(conn, nil), do: conn

  defp maybe_authorize(conn, token),
    do: Plug.Conn.put_req_header(conn, "authorization", "Bearer " <> token)

  defp token(opts) do
    exp = System.system_time(:second) + Keyword.get(opts, :exp, 3600)

    claims = %{
      "sub" => "user-1",
      "tenant_id" => Keyword.fetch!(opts, :tenant_id),
      "exp" => exp
    }

    jwt_secret()
    |> JOSE.JWK.from_oct()
    |> JOSE.JWT.sign(%{"alg" => "HS256"}, claims)
    |> JOSE.JWS.compact()
    |> elem(1)
  end

  defp jwt_secret, do: "billing-status-route-test-secret-key-0123456789"

  defp get_env(key), do: Application.get_env(:query_service_ex, key)

  defp restore_env(key, nil), do: Application.delete_env(:query_service_ex, key)
  defp restore_env(key, value), do: Application.put_env(:query_service_ex, key, value)

  defp restore_system_env(key, nil), do: System.delete_env(key)
  defp restore_system_env(key, value), do: System.put_env(key, value)

  defp free_port do
    {:ok, socket} = :gen_tcp.listen(0, [:binary, active: false, ip: {127, 0, 0, 1}])
    {:ok, port} = :inet.port(socket)
    :ok = :gen_tcp.close(socket)
    port
  end
end
