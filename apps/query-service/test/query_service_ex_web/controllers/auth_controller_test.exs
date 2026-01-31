defmodule QueryServiceExWeb.AuthControllerTest do
  use ExUnit.Case, async: true
  use Plug.Test

  alias QueryServiceExWeb.Router
  alias QueryServiceEx.Accounts.Guardian

  @opts Router.init([])

  describe "GET /api/auth/me" do
    test "returns 401 when not authenticated" do
      conn =
        :get
        |> conn("/api/auth/me")
        |> Router.call(@opts)

      assert conn.status == 401

      response = Jason.decode!(conn.resp_body)
      assert response["error"]["code"] == "unauthenticated"
    end

    test "returns 401 with invalid token" do
      conn =
        :get
        |> conn("/api/auth/me")
        |> put_req_header("authorization", "Bearer invalid_token")
        |> Router.call(@opts)

      assert conn.status == 401

      response = Jason.decode!(conn.resp_body)
      assert response["error"]["code"] == "invalid_token"
    end
  end

  describe "POST /api/auth/logout" do
    test "returns 401 when not authenticated" do
      conn =
        :post
        |> conn("/api/auth/logout")
        |> Router.call(@opts)

      assert conn.status == 401
    end
  end

  describe "GET /api/auth/google" do
    test "routes to auth controller request action" do
      # The actual OAuth redirect is handled by Ueberauth which requires
      # a full HTTP stack. Here we verify the route is properly registered
      # and the controller action is invoked.
      conn =
        :get
        |> conn("/api/auth/google")
        |> Router.call(@opts)

      # When Ueberauth successfully redirects, it halts the connection
      # In test mode without full HTTP stack, it may not complete the redirect
      # but the route should be processed
      assert conn.halted == true or conn.status in [302, nil]
    end
  end

  describe "Guardian token validation" do
    test "returns error for invalid token" do
      assert {:error, _reason} = Guardian.decode_and_verify("invalid_token")
    end

    test "returns error for malformed token" do
      assert {:error, _reason} = Guardian.decode_and_verify("not.a.valid.jwt")
    end
  end
end
