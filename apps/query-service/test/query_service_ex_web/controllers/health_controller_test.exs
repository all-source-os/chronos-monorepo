defmodule QueryServiceExWeb.HealthControllerTest do
  use ExUnit.Case, async: true

  import Plug.Test
  import Plug.Conn

  alias QueryServiceExWeb.Router

  @opts Router.init([])

  describe "GET /api/health" do
    test "returns health status with proper structure" do
      conn =
        :get
        |> conn("/api/health")
        |> Router.call(@opts)

      # Status depends on dependencies being available
      # In test environment without DB/backend, expect 503
      assert conn.status in [200, 503]
      assert conn.resp_body != ""

      response = Jason.decode!(conn.resp_body)
      assert response["service"] == "query_service_ex"
      assert response["status"] in ["healthy", "degraded", "unhealthy"]
      assert Map.has_key?(response, "checks")
      assert Map.has_key?(response, "timestamp")
      assert Map.has_key?(response, "version")
    end

    test "includes timestamp in ISO8601 format" do
      conn =
        :get
        |> conn("/api/health")
        |> Router.call(@opts)

      response = Jason.decode!(conn.resp_body)
      {:ok, _datetime, _offset} = DateTime.from_iso8601(response["timestamp"])
    end
  end

  describe "GET /api/health/live" do
    test "returns alive status" do
      conn =
        :get
        |> conn("/api/health/live")
        |> Router.call(@opts)

      assert conn.status == 200

      response = Jason.decode!(conn.resp_body)
      assert response["status"] == "alive"
      assert Map.has_key?(response, "timestamp")
    end
  end

  describe "GET /api/health/ready" do
    test "returns readiness status with checks" do
      conn =
        :get
        |> conn("/api/health/ready")
        |> Router.call(@opts)

      # Status depends on dependencies being available
      assert conn.status in [200, 503]

      response = Jason.decode!(conn.resp_body)
      assert response["status"] in ["ready", "not_ready"]
      assert Map.has_key?(response, "checks")
      assert Map.has_key?(response, "timestamp")
    end
  end
end
