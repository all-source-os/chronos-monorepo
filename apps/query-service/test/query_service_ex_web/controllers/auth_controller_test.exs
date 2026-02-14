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

  describe "GET /api/auth/me" do
    test "returns 401 when not authenticated" do
      conn =
        :get
        |> conn("/api/auth/me")
        |> put_req_header("content-type", "application/json")
        |> Router.call(@opts)

      assert conn.status == 401
      response = Jason.decode!(conn.resp_body)
      assert response["error"]["code"] == "unauthorized"
    end

    test "returns 401 with invalid token" do
      conn =
        :get
        |> conn("/api/auth/me")
        |> put_req_header("content-type", "application/json")
        |> put_req_header("authorization", "Bearer invalid_token")
        |> Router.call(@opts)

      assert conn.status == 401
      response = Jason.decode!(conn.resp_body)
      assert response["error"]["code"] == "unauthorized"
    end
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

  describe "OAuth routes removed" do
    test "GET /api/auth/google raises NoRouteError" do
      assert_raise Phoenix.Router.NoRouteError, fn ->
        :get
        |> conn("/api/auth/google")
        |> put_req_header("content-type", "application/json")
        |> Router.call(@opts)
      end
    end

    test "GET /api/auth/github raises NoRouteError" do
      assert_raise Phoenix.Router.NoRouteError, fn ->
        :get
        |> conn("/api/auth/github")
        |> put_req_header("content-type", "application/json")
        |> Router.call(@opts)
      end
    end

    test "GET /api/auth/google/callback raises NoRouteError" do
      assert_raise Phoenix.Router.NoRouteError, fn ->
        :get
        |> conn("/api/auth/google/callback")
        |> put_req_header("content-type", "application/json")
        |> Router.call(@opts)
      end
    end

    test "GET /api/auth/github/callback raises NoRouteError" do
      assert_raise Phoenix.Router.NoRouteError, fn ->
        :get
        |> conn("/api/auth/github/callback")
        |> put_req_header("content-type", "application/json")
        |> Router.call(@opts)
      end
    end
  end
end
