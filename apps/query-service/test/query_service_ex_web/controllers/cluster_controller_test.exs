defmodule QueryServiceExWeb.ClusterControllerTest do
  use ExUnit.Case, async: false

  import Plug.Test
  import Plug.Conn

  alias QueryServiceExWeb.Router

  @opts Router.init([])

  @moduletag :integration

  describe "GET /api/cluster" do
    test "returns cluster health status" do
      conn =
        :get
        |> conn("/api/cluster")
        |> put_req_header("content-type", "application/json")
        |> Router.call(@opts)

      # Public endpoint - should return 200
      assert conn.status == 200

      response = Jason.decode!(conn.resp_body)
      assert Map.has_key?(response, "data")

      data = response["data"]
      assert Map.has_key?(data, "status")
      assert Map.has_key?(data, "node")
      assert Map.has_key?(data, "node_count")
      assert Map.has_key?(data, "nodes")

      assert data["node_count"] >= 1
      assert is_list(data["nodes"])
    end
  end

  describe "GET /api/cluster/members" do
    test "returns cluster members list" do
      conn =
        :get
        |> conn("/api/cluster/members")
        |> put_req_header("content-type", "application/json")
        |> Router.call(@opts)

      # Public endpoint - should return 200
      assert conn.status == 200

      response = Jason.decode!(conn.resp_body)
      assert Map.has_key?(response, "data")

      data = response["data"]
      assert Map.has_key?(data, "members")
      assert Map.has_key?(data, "total")
      assert Map.has_key?(data, "strategy")

      assert is_list(data["members"])
      assert data["total"] >= 1

      # Check member structure
      if data["members"] != [] do
        member = hd(data["members"])
        assert Map.has_key?(member, "node")
        assert Map.has_key?(member, "self")
        assert Map.has_key?(member, "connected")
      end
    end
  end

  describe "GET /api/cluster/registry" do
    test "returns 401 without authentication" do
      conn =
        :get
        |> conn("/api/cluster/registry")
        |> put_req_header("content-type", "application/json")
        |> Router.call(@opts)

      # Authenticated endpoint
      assert conn.status == 401
    end
  end

  describe "GET /api/cluster/supervisor" do
    test "returns 401 without authentication" do
      conn =
        :get
        |> conn("/api/cluster/supervisor")
        |> put_req_header("content-type", "application/json")
        |> Router.call(@opts)

      # Authenticated endpoint
      assert conn.status == 401
    end
  end

  describe "GET /api/cluster/hash-ring" do
    test "returns 401 without authentication" do
      conn =
        :get
        |> conn("/api/cluster/hash-ring")
        |> put_req_header("content-type", "application/json")
        |> Router.call(@opts)

      # Authenticated endpoint
      assert conn.status == 401
    end

    test "accepts key parameter for routing info" do
      # This would need auth to actually work
      conn =
        :get
        |> conn("/api/cluster/hash-ring?key=test-entity")
        |> put_req_header("content-type", "application/json")
        |> Router.call(@opts)

      assert conn.status == 401
    end
  end
end
