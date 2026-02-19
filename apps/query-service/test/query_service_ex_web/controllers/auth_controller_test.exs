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
end
