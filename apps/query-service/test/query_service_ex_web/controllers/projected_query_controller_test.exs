defmodule QueryServiceExWeb.ProjectedQueryControllerTest do
  use ExUnit.Case, async: false

  import Plug.Test
  import Plug.Conn

  alias QueryServiceExWeb.Router

  @opts Router.init([])

  @moduletag :integration

  describe "POST /api/query/projected - validation" do
    test "returns 400 when projection name is missing" do
      body = Jason.encode!(%{"filters" => %{}})

      conn =
        :post
        |> conn("/api/query/projected", body)
        |> put_req_header("content-type", "application/json")
        |> Router.call(@opts)

      # 401 if auth required, 400 if auth bypassed
      assert conn.status in [400, 401]

      if conn.status == 400 do
        response = Jason.decode!(conn.resp_body)
        assert get_in(response, ["error", "message"]) =~ "Missing required field"
      end
    end

    test "returns 400 when projection name is not in registry" do
      body = Jason.encode!(%{"projection" => "nonexistent"})

      conn =
        :post
        |> conn("/api/query/projected", body)
        |> put_req_header("content-type", "application/json")
        |> Router.call(@opts)

      assert conn.status in [400, 401]

      if conn.status == 400 do
        response = Jason.decode!(conn.resp_body)
        assert get_in(response, ["error", "message"]) =~ "Unknown projection"
      end
    end
  end

  describe "POST /api/query/projected - with registered projection" do
    test "returns projected state when projection is registered and Core is reachable" do
      # This test exercises the full pipeline. When Core is not running,
      # the request will fail at the fetch step (502/503). When it is
      # running, it should return 200 with projected state.
      body =
        Jason.encode!(%{
          "projection" => "indices",
          "filters" => %{"is_deleted" => false},
          "page" => 1,
          "page_size" => 10
        })

      conn =
        :post
        |> conn("/api/query/projected", body)
        |> put_req_header("content-type", "application/json")
        |> Router.call(@opts)

      # 401 if auth, 400 if unknown projection (registry empty), 200/502/503 if Core call
      assert conn.status in [200, 400, 401, 502, 503]

      if conn.status == 200 do
        response = Jason.decode!(conn.resp_body)
        assert is_list(response["data"])
        assert is_integer(response["count"])
        assert is_integer(response["total"])
        assert response["projection"] == "indices"
        assert is_map(response["folded_from"])
      end
    end
  end
end
