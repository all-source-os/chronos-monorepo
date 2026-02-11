defmodule QueryServiceExWeb.MetricsControllerTest do
  use ExUnit.Case, async: true

  import Plug.Test
  import Plug.Conn

  alias QueryServiceExWeb.Router

  @opts Router.init([])

  describe "GET /api/metrics (JSON format)" do
    test "returns system metrics in JSON by default" do
      conn =
        :get
        |> conn("/api/metrics")
        |> put_req_header("content-type", "application/json")
        |> Router.call(@opts)

      assert conn.status == 200
      response = Jason.decode!(conn.resp_body)

      assert response["service"] == "query_service_ex"
      assert Map.has_key?(response, "timestamp")
      assert Map.has_key?(response, "elixir")
      assert Map.has_key?(response, "backend")
    end

    test "includes Elixir runtime metrics" do
      conn =
        :get
        |> conn("/api/metrics")
        |> Router.call(@opts)

      response = Jason.decode!(conn.resp_body)
      elixir_metrics = response["elixir"]

      assert Map.has_key?(elixir_metrics, "processes")
      assert Map.has_key?(elixir_metrics, "memory")
      assert Map.has_key?(elixir_metrics, "uptime_seconds")
      assert Map.has_key?(elixir_metrics, "schedulers")

      # Memory should have breakdown
      memory = elixir_metrics["memory"]
      assert Map.has_key?(memory, "total_mb")
      assert Map.has_key?(memory, "processes_mb")
    end

    test "handles backend unavailability gracefully" do
      conn =
        :get
        |> conn("/api/metrics")
        |> Router.call(@opts)

      assert conn.status == 200
      response = Jason.decode!(conn.resp_body)

      # Should still return metrics even if backend is down
      assert Map.has_key?(response, "backend")
    end
  end

  describe "GET /api/metrics/prometheus" do
    test "returns Prometheus format directly" do
      conn =
        :get
        |> conn("/api/metrics/prometheus")
        |> Router.call(@opts)

      assert conn.status == 200
      assert get_resp_header(conn, "content-type") |> List.first() =~ "text/plain"
      assert is_binary(conn.resp_body)
    end

    test "always returns Prometheus format regardless of Accept header" do
      conn =
        :get
        |> conn("/api/metrics/prometheus")
        |> put_req_header("accept", "application/json")
        |> Router.call(@opts)

      assert conn.status == 200
      assert get_resp_header(conn, "content-type") |> List.first() =~ "text/plain"
    end
  end
end
