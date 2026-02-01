defmodule QueryServiceExWeb.BillingControllerTest do
  @moduledoc """
  Tests for BillingController.

  These tests require a running PostgreSQL database.
  Run with: mix test --include database
  """
  use QueryServiceExWeb.ConnCase

  alias QueryServiceEx.Accounts
  alias QueryServiceEx.Accounts.Guardian
  alias QueryServiceEx.Tenants

  @moduletag :database

  setup %{conn: conn} do
    {:ok, tenant} = Tenants.create_tenant(%{name: "Test Workspace", slug: "billing-test"})

    {:ok, user} =
      Accounts.create_user(%{
        email: "billing@example.com",
        name: "Billing User",
        provider: "google",
        google_id: "google_billing_123",
        tenant_id: tenant.id
      })

    user = QueryServiceEx.Repo.preload(user, :tenant)
    {:ok, token, _claims} = Guardian.encode_and_sign(user)

    conn =
      conn
      |> put_req_header("authorization", "Bearer #{token}")
      |> put_req_header("accept", "application/json")

    {:ok, conn: conn, tenant: tenant, user: user}
  end

  describe "POST /api/billing/checkout" do
    test "returns error when LemonSqueezy not configured", %{conn: conn} do
      conn = post(conn, "/api/billing/checkout", %{"variant_id" => "variant_123"})

      assert json_response(conn, 422)["error"]["code"] == "checkout_failed"
    end

    test "returns bad request without variant_id", %{conn: conn} do
      conn = post(conn, "/api/billing/checkout", %{})

      assert json_response(conn, 400)["error"]["code"] == "missing_variant_id"
    end

    test "returns 401 without auth token", %{conn: conn} do
      conn =
        conn
        |> delete_req_header("authorization")
        |> post("/api/billing/checkout", %{"variant_id" => "variant_123"})

      assert json_response(conn, 401)
    end
  end

  describe "GET /api/billing/portal" do
    test "returns no subscription message for new tenant", %{conn: conn} do
      conn = get(conn, "/api/billing/portal")
      response = json_response(conn, 200)["data"]

      assert response["has_subscription"] == false
      assert response["message"] =~ "No active subscription"
    end

    test "returns error when tenant has customer_id but portal fetch fails", %{
      conn: conn,
      tenant: tenant
    } do
      {:ok, _tenant} =
        Tenants.update_subscription(tenant, %{
          lemon_squeezy_customer_id: "cust_123"
        })

      conn = get(conn, "/api/billing/portal")

      assert json_response(conn, 422)["error"]["code"] == "portal_failed"
    end

    test "returns 401 without auth token", %{conn: conn} do
      conn =
        conn
        |> delete_req_header("authorization")
        |> get("/api/billing/portal")

      assert json_response(conn, 401)
    end
  end
end
