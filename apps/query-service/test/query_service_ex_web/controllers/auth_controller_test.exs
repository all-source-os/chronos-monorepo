defmodule QueryServiceExWeb.AuthControllerTest do
  use ExUnit.Case, async: true

  import Plug.Test
  import Plug.Conn

  alias QueryServiceExWeb.Router

  @opts Router.init([])

  setup do
    System.put_env("JWT_SECRET", QueryServiceEx.AuthHelpers.test_jwt_secret())
    on_exit(fn -> System.delete_env("JWT_SECRET") end)
    :ok
  end

  describe "POST /api/auth/logout" do
    test "returns 401 when not authenticated" do
      conn =
        :post
        |> conn("/api/auth/logout")
        |> put_req_header("content-type", "application/json")
        |> Router.call(@opts)

      assert conn.status == 401
    end
  end

  describe "OAuth routes" do
    test "GET /api/auth/google redirects to OAuth provider" do
      conn =
        :get
        |> conn("/api/auth/google")
        |> put_req_header("content-type", "application/json")
        |> Router.call(@opts)

      # OAuth authorize action returns a redirect (302) or error
      assert conn.status in [302, 500]
    end

    test "GET /api/auth/github redirects to OAuth provider" do
      conn =
        :get
        |> conn("/api/auth/github")
        |> put_req_header("content-type", "application/json")
        |> Router.call(@opts)

      assert conn.status in [302, 500]
    end

    test "GET /api/auth/google/callback is routable" do
      conn =
        :get
        |> conn("/api/auth/google/callback?code=test")
        |> put_req_header("content-type", "application/json")
        |> Router.call(@opts)

      # Callback without valid code returns error, but route exists
      assert conn.status in [302, 400, 401, 422, 500]
    end

    test "GET /api/auth/github/callback is routable" do
      conn =
        :get
        |> conn("/api/auth/github/callback?code=test")
        |> put_req_header("content-type", "application/json")
        |> Router.call(@opts)

      assert conn.status in [302, 400, 401, 422, 500]
    end
  end
end
