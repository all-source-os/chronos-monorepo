defmodule QueryServiceExWeb.TenantControllerTest do
  @moduledoc """
  Tests for TenantController.

  These tests require a running PostgreSQL database.
  Run with: mix test --include database
  """
  use QueryServiceExWeb.ConnCase

  import Plug.Conn
  import Plug.Test

  alias QueryServiceEx.Accounts
  alias QueryServiceEx.Accounts.Guardian
  alias QueryServiceEx.Tenants

  @moduletag :database

  setup %{conn: conn} do
    # Create a tenant and user for testing
    {:ok, tenant} = Tenants.create_tenant(%{name: "Test Workspace", slug: "test-workspace"})

    {:ok, user} =
      Accounts.create_user(%{
        email: "test@example.com",
        name: "Test User",
        provider: "google",
        google_id: "google_123",
        tenant_id: tenant.id
      })

    user = QueryServiceEx.Repo.preload(user, :tenant)

    # Create a valid JWT token
    {:ok, token, _claims} = Guardian.encode_and_sign(user)

    conn =
      conn
      |> put_req_header("authorization", "Bearer #{token}")
      |> put_req_header("accept", "application/json")

    {:ok, conn: conn, tenant: tenant, user: user}
  end

  describe "GET /api/tenant" do
    test "returns current tenant info", %{conn: conn, tenant: tenant} do
      conn = get(conn, "/api/tenant")

      assert json_response(conn, 200)["data"]["id"] == tenant.id
      assert json_response(conn, 200)["data"]["name"] == tenant.name
      assert json_response(conn, 200)["data"]["slug"] == tenant.slug
      assert json_response(conn, 200)["data"]["subscription_status"] == "trialing"
      assert json_response(conn, 200)["data"]["subscription_tier"] == "free"
    end

    test "returns 401 without auth token", %{conn: conn} do
      conn =
        conn
        |> delete_req_header("authorization")
        |> get("/api/tenant")

      assert json_response(conn, 401)
    end
  end

  describe "PUT /api/tenant" do
    test "updates tenant name", %{conn: conn} do
      conn = put(conn, "/api/tenant", %{"name" => "Updated Workspace"})

      assert json_response(conn, 200)["data"]["name"] == "Updated Workspace"
    end

    test "updates tenant settings", %{conn: conn} do
      settings = %{"theme" => "dark", "notifications" => true}
      conn = put(conn, "/api/tenant", %{"settings" => settings})

      assert json_response(conn, 200)["data"]["settings"] == settings
    end
  end

  describe "GET /api/tenant/usage" do
    test "returns usage statistics", %{conn: conn, tenant: tenant} do
      # Add some usage
      {:ok, _} = Tenants.increment_events_usage(tenant.id, 500)
      {:ok, _} = Tenants.increment_queries_usage(tenant.id, 50)

      conn = get(conn, "/api/tenant/usage")
      response = json_response(conn, 200)["data"]

      assert response["events"]["used"] == 500
      assert response["events"]["quota"] == 10_000
      assert response["queries"]["used"] == 50
      assert response["queries"]["quota"] == 1_000
    end
  end
end
