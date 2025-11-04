defmodule QueryServiceExWeb.HealthControllerTest do
  use ExUnit.Case, async: true
  use Plug.Test

  alias QueryServiceExWeb.Router

  @opts Router.init([])

  describe "GET /api/health" do
    test "returns health status" do
      conn =
        :get
        |> conn("/api/health")
        |> Router.call(@opts)

      assert conn.status == 200
      assert conn.resp_body != ""

      response = Jason.decode!(conn.resp_body)
      assert response["service"] == "query_service_ex"
      assert response["status"] == "healthy"
      assert Map.has_key?(response, "backend")
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
end
