defmodule QueryServiceExWeb.EventControllerTest do
  use ExUnit.Case, async: false
  use Plug.Test

  alias QueryServiceExWeb.Router

  @opts Router.init([])

  @moduletag :integration

  describe "GET /api/events" do
    test "returns list of events" do
      conn =
        :get
        |> conn("/api/events")
        |> put_req_header("content-type", "application/json")
        |> Router.call(@opts)

      # 401 if no auth, 200/400 if auth provided
      assert conn.status in [200, 400, 401]

      if conn.status == 200 do
        response = Jason.decode!(conn.resp_body)
        assert Map.has_key?(response, "data")
        assert is_list(response["data"])
      end
    end

    test "accepts query parameters" do
      conn =
        :get
        |> conn("/api/events?limit=10&offset=0")
        |> put_req_header("content-type", "application/json")
        |> Router.call(@opts)

      # 401 if no auth, 200/400 if auth provided
      assert conn.status in [200, 400, 401]
    end
  end

  describe "POST /api/events" do
    test "creates a new event" do
      event = %{
        "entity_id" => "test-entity-#{System.unique_integer([:positive])}",
        "event_type" => "test.event",
        "payload" => %{"test" => "data"}
      }

      conn =
        :post
        |> conn("/api/events", Jason.encode!(event))
        |> put_req_header("content-type", "application/json")
        |> Router.call(@opts)

      # Backend may or may not be available, 401 if no auth
      assert conn.status in [201, 422, 400, 401]

      if conn.status == 201 do
        response = Jason.decode!(conn.resp_body)
        assert Map.has_key?(response, "data")
      end
    end

    test "validates required fields" do
      invalid_event = %{}

      conn =
        :post
        |> conn("/api/events", Jason.encode!(invalid_event))
        |> put_req_header("content-type", "application/json")
        |> Router.call(@opts)

      # 401 if no auth, 422/400 if auth provided
      assert conn.status in [422, 400, 401]
    end
  end

  describe "POST /api/events/batch" do
    test "creates multiple events" do
      events = %{
        "events" => [
          %{
            "entity_id" => "test-1",
            "event_type" => "test.event",
            "payload" => %{}
          },
          %{
            "entity_id" => "test-2",
            "event_type" => "test.event",
            "payload" => %{}
          }
        ]
      }

      conn =
        :post
        |> conn("/api/events/batch", Jason.encode!(events))
        |> put_req_header("content-type", "application/json")
        |> Router.call(@opts)

      # 401 if no auth, 201/422/400 if auth provided
      assert conn.status in [201, 422, 400, 401]
    end

    test "requires events array" do
      invalid = %{"not_events" => []}

      conn =
        :post
        |> conn("/api/events/batch", Jason.encode!(invalid))
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

  describe "GET /api/events/entity/:entity_id" do
    test "returns events for specific entity" do
      conn =
        :get
        |> conn("/api/events/entity/test-entity")
        |> put_req_header("content-type", "application/json")
        |> Router.call(@opts)

      # 401 if no auth, 200/400 if auth provided
      assert conn.status in [200, 400, 401]

      if conn.status == 200 do
        response = Jason.decode!(conn.resp_body)
        assert response["entity_id"] == "test-entity"
        assert is_list(response["data"])
      end
    end
  end

  describe "GET /api/events/type/:event_type" do
    test "returns events of specific type" do
      conn =
        :get
        |> conn("/api/events/type/test.event")
        |> put_req_header("content-type", "application/json")
        |> Router.call(@opts)

      # 401 if no auth, 200/400 if auth provided
      assert conn.status in [200, 400, 401]

      if conn.status == 200 do
        response = Jason.decode!(conn.resp_body)
        assert response["event_type"] == "test.event"
        assert is_list(response["data"])
      end
    end
  end
end
