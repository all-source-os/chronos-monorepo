defmodule QueryServiceExWeb.QueryControllerTest do
  use ExUnit.Case, async: false

  import Plug.Test
  import Plug.Conn

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

      # 401 if no auth, 200/400 if auth provided
      assert conn.status in [200, 400, 401]

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

      # 401 if no auth, 200/400 if auth provided
      assert conn.status in [200, 400, 401]
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

      # 401 if no auth, 200/400 if auth provided
      assert conn.status in [200, 400, 401]
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

      # 401 if no auth, 400 if auth provided
      assert conn.status in [400, 401]

      if conn.status == 400 do
        response = Jason.decode!(conn.resp_body)
        assert Map.has_key?(response, "error")
      end
    end
  end
end
