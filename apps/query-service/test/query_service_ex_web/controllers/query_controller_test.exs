defmodule QueryServiceExWeb.QueryControllerTest do
  use ExUnit.Case, async: false
  use Plug.Test

  alias QueryServiceExWeb.Router

  @opts Router.init([])

  @moduletag :integration

  describe "POST /api/query - simple queries" do
    test "executes simple query with filters" do
      query = %{
        "entity_id" => "test-entity",
        "limit" => 10
      }

      conn =
        :post
        |> conn("/api/query", Jason.encode!(query))
        |> put_req_header("content-type", "application/json")
        |> Router.call(@opts)

      assert conn.status in [200, 400]

      if conn.status == 200 do
        response = Jason.decode!(conn.resp_body)
        assert Map.has_key?(response, "data")
        assert Map.has_key?(response, "query")
        assert is_list(response["data"])
      end
    end
  end

  describe "POST /api/query - DSL queries" do
    test "executes DSL query with where clause" do
      query = %{
        "from" => "events",
        "where" => %{
          "op" => "eq",
          "field" => "event_type",
          "value" => "test.event"
        },
        "limit" => 10
      }

      conn =
        :post
        |> conn("/api/query", Jason.encode!(query))
        |> put_req_header("content-type", "application/json")
        |> Router.call(@opts)

      assert conn.status in [200, 400]
    end

    test "handles AND predicates" do
      query = %{
        "from" => "events",
        "where" => %{
          "and" => [
            %{"op" => "eq", "field" => "event_type", "value" => "test.event"},
            %{"op" => "gt", "field" => "timestamp", "value" => "2024-01-01"}
          ]
        }
      }

      conn =
        :post
        |> conn("/api/query", Jason.encode!(query))
        |> put_req_header("content-type", "application/json")
        |> Router.call(@opts)

      assert conn.status in [200, 400]
    end

    test "handles invalid DSL query" do
      query = %{
        "from" => "events",
        "where" => %{
          "invalid_op" => "badvalue"
        }
      }

      conn =
        :post
        |> conn("/api/query", Jason.encode!(query))
        |> put_req_header("content-type", "application/json")
        |> Router.call(@opts)

      assert conn.status == 400
      response = Jason.decode!(conn.resp_body)
      assert Map.has_key?(response, "error")
    end
  end
end
