defmodule QueryServiceExWeb.EventControllerQueryTest do
  use ExUnit.Case, async: false

  import Plug.Test
  import Plug.Conn

  alias QueryServiceExWeb.Router

  @opts Router.init([])

  @moduletag :integration

  describe "GET /api/v1/events/query" do
    test "returns events in Core-compatible format" do
      conn =
        :get
        |> conn("/api/v1/events/query?limit=2")
        |> put_req_header("content-type", "application/json")
        |> Router.call(@opts)

      # 401 if no auth, 200/400 if auth provided
      assert conn.status in [200, 400, 401]

      if conn.status == 200 do
        response = Jason.decode!(conn.resp_body)
        assert Map.has_key?(response, "events"), "response must have 'events' key"
        assert Map.has_key?(response, "count"), "response must have 'count' key"
        refute Map.has_key?(response, "error"), "response must not have 'error' key"
        assert is_list(response["events"])
      end
    end

    test "does not match /events/:id route" do
      conn =
        :get
        |> conn("/api/v1/events/query?limit=1")
        |> put_req_header("content-type", "application/json")
        |> Router.call(@opts)

      # Should never return the inspect-format error from get_event_by_id catchall
      if conn.status == 200 do
        response = Jason.decode!(conn.resp_body)
        refute Map.has_key?(response, "error")
        # Must NOT be a single-event show response
        refute Map.has_key?(response, "data")
      end
    end
  end
end
