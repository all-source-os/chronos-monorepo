defmodule QueryServiceExWeb.PrimeControllerTest do
  @moduledoc """
  Tests for the PrimeController gateway routes that proxy Core's
  /api/v1/prime/* projection + provenance endpoints.

  Follows the ProjectionController test convention: verify conn/routing
  setup for the happy path, and call actions with Core unavailable to
  confirm error statuses (Core is not running in the test env).
  """
  use ExUnit.Case, async: true

  import Plug.Test
  import Plug.Conn

  alias QueryServiceExWeb.PrimeController

  defp build_json_conn(method, path, body \\ nil) do
    conn(method, path, body)
    |> put_req_header("content-type", "application/json")
    |> put_private(:phoenix_format, "json")
    |> put_private(:phoenix_endpoint, QueryServiceExWeb.Endpoint)
    |> assign(:correlation_id, "test-correlation-id")
  end

  describe "index/2" do
    test "returns error status when Core is unavailable" do
      conn = build_json_conn(:get, "/api/v1/prime/projections")
      result = PrimeController.index(conn, %{})
      assert result.status in [400, 502, 503]
    end
  end

  describe "create/2" do
    test "returns error status when Core is unavailable" do
      params = %{
        "entity_type" => "contact",
        "field_policies" => %{"status" => "last_write"}
      }

      conn = build_json_conn(:post, "/api/v1/prime/projections", params)
      result = PrimeController.create(conn, params)
      assert result.status in [400, 422, 502, 503]
    end
  end

  describe "project/2" do
    test "returns error status when Core is unavailable" do
      conn = build_json_conn(:post, "/api/v1/prime/nodes/node:contact:1/project")
      result = PrimeController.project(conn, %{"id" => "node:contact:1"})
      assert result.status in [400, 502, 503]
    end
  end

  describe "provenance/2" do
    test "returns error status when Core is unavailable" do
      conn =
        build_json_conn(
          :get,
          "/api/v1/prime/nodes/node:contact:1/fields/status/provenance"
        )

      result =
        PrimeController.provenance(conn, %{"id" => "node:contact:1", "field" => "status"})

      # :not_found (404) only when Core answers; with Core down expect a
      # gateway/transport error.
      assert result.status in [400, 404, 502, 503]
    end
  end
end
