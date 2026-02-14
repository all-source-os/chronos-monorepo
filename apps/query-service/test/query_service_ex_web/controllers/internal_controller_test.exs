defmodule QueryServiceExWeb.InternalControllerTest do
  use ExUnit.Case, async: false

  import Plug.Conn
  import Phoenix.ConnTest

  alias QueryServiceEx.TenantCache

  @endpoint QueryServiceExWeb.Endpoint

  setup do
    TenantCache.clear()
    :ok
  end

  describe "POST /internal/tenant-updated" do
    test "returns 401 without API key" do
      System.put_env("INTERNAL_API_KEY", "test-secret-key")

      conn =
        build_conn()
        |> put_req_header("content-type", "application/json")
        |> post("/internal/tenant-updated", %{tenant_id: "t1", action: "updated"})

      assert json_response(conn, 401)["error"] == "unauthorized"

      System.delete_env("INTERNAL_API_KEY")
    end

    test "returns 401 with wrong API key" do
      System.put_env("INTERNAL_API_KEY", "test-secret-key")

      conn =
        build_conn()
        |> put_req_header("content-type", "application/json")
        |> put_req_header("x-internal-api-key", "wrong-key")
        |> post("/internal/tenant-updated", %{tenant_id: "t1", action: "updated"})

      assert json_response(conn, 401)["error"] == "unauthorized"

      System.delete_env("INTERNAL_API_KEY")
    end

    test "returns 401 when INTERNAL_API_KEY not configured" do
      System.delete_env("INTERNAL_API_KEY")

      conn =
        build_conn()
        |> put_req_header("content-type", "application/json")
        |> put_req_header("x-internal-api-key", "some-key")
        |> post("/internal/tenant-updated", %{tenant_id: "t1", action: "updated"})

      assert json_response(conn, 401)["error"] == "unauthorized"
    end

    test "invalidates cache on valid request" do
      System.put_env("INTERNAL_API_KEY", "test-secret-key")

      # Pre-populate cache
      TenantCache.put("tenant-123", %{id: "tenant-123", name: "Test Tenant"})
      assert TenantCache.cached?("tenant-123")

      conn =
        build_conn()
        |> put_req_header("content-type", "application/json")
        |> put_req_header("x-internal-api-key", "test-secret-key")
        |> post("/internal/tenant-updated", %{tenant_id: "tenant-123", action: "suspended"})

      body = json_response(conn, 200)
      assert body["status"] == "ok"
      assert body["tenant_id"] == "tenant-123"
      assert body["action"] == "suspended"
      assert body["was_cached"] == true

      # Cache should be invalidated
      refute TenantCache.cached?("tenant-123")

      System.delete_env("INTERNAL_API_KEY")
    end

    test "returns 200 even if tenant was not cached (idempotent)" do
      System.put_env("INTERNAL_API_KEY", "test-secret-key")

      conn =
        build_conn()
        |> put_req_header("content-type", "application/json")
        |> put_req_header("x-internal-api-key", "test-secret-key")
        |> post("/internal/tenant-updated", %{tenant_id: "nonexistent", action: "deleted"})

      body = json_response(conn, 200)
      assert body["status"] == "ok"
      assert body["was_cached"] == false

      System.delete_env("INTERNAL_API_KEY")
    end

    test "accepts all valid actions" do
      System.put_env("INTERNAL_API_KEY", "test-secret-key")

      for action <- ~w(suspended activated updated deleted) do
        conn =
          build_conn()
          |> put_req_header("content-type", "application/json")
          |> put_req_header("x-internal-api-key", "test-secret-key")
          |> post("/internal/tenant-updated", %{tenant_id: "t1", action: action})

        assert json_response(conn, 200)["action"] == action
      end

      System.delete_env("INTERNAL_API_KEY")
    end

    test "returns 400 for invalid action" do
      System.put_env("INTERNAL_API_KEY", "test-secret-key")

      conn =
        build_conn()
        |> put_req_header("content-type", "application/json")
        |> put_req_header("x-internal-api-key", "test-secret-key")
        |> post("/internal/tenant-updated", %{tenant_id: "t1", action: "invalid"})

      assert json_response(conn, 400)["error"] =~ "missing or invalid"

      System.delete_env("INTERNAL_API_KEY")
    end

    test "returns 400 for missing fields" do
      System.put_env("INTERNAL_API_KEY", "test-secret-key")

      conn =
        build_conn()
        |> put_req_header("content-type", "application/json")
        |> put_req_header("x-internal-api-key", "test-secret-key")
        |> post("/internal/tenant-updated", %{})

      assert json_response(conn, 400)["error"] =~ "missing or invalid"

      System.delete_env("INTERNAL_API_KEY")
    end
  end
end
