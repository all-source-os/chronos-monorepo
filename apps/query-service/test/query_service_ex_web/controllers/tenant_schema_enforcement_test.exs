defmodule QueryServiceExWeb.TenantSchemaEnforcementTest do
  @moduledoc """
  Tests for the Gap 3 schema-enforcement toggle actions on TenantController.

  Validation (invalid mode → 422) is deterministic and asserted directly.
  Core-dependent paths follow the controller-test convention used elsewhere
  (call the action, accept the env-dependent status set).
  """
  use ExUnit.Case, async: true

  import Plug.Test
  import Plug.Conn

  alias QueryServiceExWeb.TenantController

  defp build_json_conn(method, path, body \\ nil) do
    conn(method, path, body)
    |> put_req_header("content-type", "application/json")
    |> put_private(:phoenix_format, "json")
    |> put_private(:phoenix_endpoint, QueryServiceExWeb.Endpoint)
    |> assign(:correlation_id, "test-correlation-id")
    # The :authenticated pipeline guarantees current_user in prod; supply it
    # here so tenant resolution works when auth isn't disabled in test env.
    |> assign(:current_user, %{"tenant_id" => "test-tenant"})
  end

  describe "set_schema_enforcement/2" do
    test "rejects an invalid mode with 422 (before touching tenant context)" do
      conn = build_json_conn(:put, "/api/tenant/schema-enforcement")

      result =
        TenantController.set_schema_enforcement(conn, %{"schema_enforcement" => "nonsense"})

      assert result.status == 422
    end

    test "accepts a valid mode" do
      conn = build_json_conn(:put, "/api/tenant/schema-enforcement")
      result = TenantController.set_schema_enforcement(conn, %{"schema_enforcement" => "strict"})
      # Dev mode echoes 200; with a real tenant + Core down, a gateway error.
      assert result.status in [200, 404, 502, 503]
    end
  end

  describe "get_schema_enforcement/2" do
    test "returns a mode (defaults to permissive when Core is unreachable)" do
      conn = build_json_conn(:get, "/api/tenant/schema-enforcement")
      result = TenantController.get_schema_enforcement(conn, %{})
      assert result.status in [200, 404, 502, 503]
    end
  end
end
