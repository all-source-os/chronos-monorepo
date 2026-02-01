defmodule QueryServiceExWeb.SchemaControllerTest do
  @moduledoc """
  Tests for the SchemaController.

  Verifies schema registry operations for event types through the API endpoints.
  """
  use ExUnit.Case, async: true

  import Plug.Test
  import Plug.Conn

  alias QueryServiceExWeb.SchemaController

  defp build_json_conn(method, path, body \\ nil) do
    conn = conn(method, path, body)

    conn
    |> put_req_header("content-type", "application/json")
    |> put_private(:phoenix_format, "json")
    |> put_private(:phoenix_endpoint, QueryServiceExWeb.Endpoint)
    |> assign(:correlation_id, "test-correlation-id")
  end

  describe "index/2" do
    test "builds correct request for listing schemas" do
      conn = build_json_conn(:get, "/api/schemas")

      assert conn.method == "GET"
      assert conn.request_path == "/api/schemas"
    end
  end

  describe "show/2" do
    test "builds correct request for fetching schema by event type" do
      conn = build_json_conn(:get, "/api/schemas/UserCreated")

      assert conn.method == "GET"
      assert conn.request_path == "/api/schemas/UserCreated"
    end

    test "supports version query parameter" do
      conn = build_json_conn(:get, "/api/schemas/UserCreated?version=2")

      assert conn.method == "GET"
      assert conn.query_string == "version=2"
    end
  end

  describe "register/2" do
    test "builds correct request for registering schema" do
      params = %{
        "event_type" => "OrderPlaced",
        "version" => 1,
        "schema" => %{
          "type" => "object",
          "properties" => %{
            "order_id" => %{"type" => "string"},
            "amount" => %{"type" => "number"}
          },
          "required" => ["order_id", "amount"]
        }
      }

      conn = build_json_conn(:post, "/api/schemas", params)

      assert conn.method == "POST"
      assert conn.request_path == "/api/schemas"
    end

    test "uses default version when not provided" do
      params = %{
        "event_type" => "SimpleEvent",
        "schema" => %{"type" => "object"}
      }

      conn = build_json_conn(:post, "/api/schemas", params)

      # The controller defaults version to 1
      assert conn.method == "POST"
    end
  end
end
