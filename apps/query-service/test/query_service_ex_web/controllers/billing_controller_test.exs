defmodule QueryServiceExWeb.BillingControllerTest do
  @moduledoc """
  Tests for BillingController 301 redirects to Control Plane.
  """
  use QueryServiceExWeb.ConnCase

  @mgmt_url "https://cp.all-source.xyz"

  setup %{conn: conn} do
    System.put_env("MGMT_PLANE_URL", @mgmt_url)
    on_exit(fn -> System.delete_env("MGMT_PLANE_URL") end)

    conn = put_req_header(conn, "accept", "application/json")
    {:ok, conn: conn}
  end

  describe "POST /api/billing/checkout" do
    test "returns 301 with Location header to CP", %{conn: conn} do
      conn = post(conn, "/api/billing/checkout", %{})

      assert conn.status == 301
      assert get_resp_header(conn, "location") == ["#{@mgmt_url}/api/v1/billing/checkout"]
      assert json_response(conn, 301)["error"]["code"] == "moved_permanently"
    end
  end

  describe "GET /api/billing/portal" do
    test "returns 301 with Location header to CP", %{conn: conn} do
      conn = get(conn, "/api/billing/portal")

      assert conn.status == 301
      assert get_resp_header(conn, "location") == ["#{@mgmt_url}/api/v1/billing/portal"]
      assert json_response(conn, 301)["error"]["location"] =~ "/api/v1/billing/portal"
    end
  end

  describe "GET /api/billing/overage" do
    test "returns 301 with Location header to CP", %{conn: conn} do
      conn = get(conn, "/api/billing/overage")

      assert conn.status == 301
      assert get_resp_header(conn, "location") == ["#{@mgmt_url}/api/v1/billing/overage"]
    end
  end

  describe "POST /api/billing/overage/enable" do
    test "returns 301 with Location header to CP", %{conn: conn} do
      conn = post(conn, "/api/billing/overage/enable", %{})

      assert conn.status == 301
      assert get_resp_header(conn, "location") == ["#{@mgmt_url}/api/v1/billing/overage/enable"]
    end
  end

  describe "POST /api/billing/overage/disable" do
    test "returns 301 with Location header to CP", %{conn: conn} do
      conn = post(conn, "/api/billing/overage/disable", %{})

      assert conn.status == 301
      assert get_resp_header(conn, "location") == ["#{@mgmt_url}/api/v1/billing/overage/disable"]
    end
  end

  describe "GET /api/billing/projected-charges" do
    test "returns 301 with Location header to CP", %{conn: conn} do
      conn = get(conn, "/api/billing/projected-charges")

      assert conn.status == 301

      assert get_resp_header(conn, "location") == [
               "#{@mgmt_url}/api/v1/billing/projected-charges"
             ]
    end
  end

  describe "redirect without MGMT_PLANE_URL" do
    test "returns 301 with relative path when env var not set", %{conn: conn} do
      System.delete_env("MGMT_PLANE_URL")

      conn = get(conn, "/api/billing/portal")

      assert conn.status == 301
      assert get_resp_header(conn, "location") == ["/api/v1/billing/portal"]
    end
  end
end
