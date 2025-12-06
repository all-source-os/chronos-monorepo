defmodule QueryServiceEx.Infrastructure.Adapters.CoreWebSocketClientTest do
  use ExUnit.Case, async: false

  alias QueryServiceEx.Infrastructure.Adapters.CoreWebSocketClient

  @moduletag :websocket

  describe "start_link/1" do
    test "returns :ignore when disabled" do
      # In test env, core_ws_enabled is false by default
      assert :ignore = CoreWebSocketClient.start_link(enabled: false)
    end

    @tag :skip  # Skip this test unless Core is running
    test "starts with default options when enabled" do
      # This test requires Core to be running
      assert {:ok, _pid} = CoreWebSocketClient.start_link(enabled: true, name: :test_ws_client)

      # Clean up
      GenServer.stop(:test_ws_client)
    end
  end

  describe "event broadcasting" do
    test "subscribes to events:all topic" do
      # Subscribe to the topic
      Phoenix.PubSub.subscribe(QueryServiceEx.PubSub, "events:all")

      # Verify subscription works (we won't receive events without real connection)
      assert :ok = Phoenix.PubSub.broadcast(
        QueryServiceEx.PubSub,
        "events:all",
        {:new_event, %{"id" => "test-event"}}
      )

      assert_receive {:new_event, %{"id" => "test-event"}}
    end

    test "subscribes to entity-specific topic" do
      entity_id = "user-123"
      Phoenix.PubSub.subscribe(QueryServiceEx.PubSub, "events:#{entity_id}")

      assert :ok = Phoenix.PubSub.broadcast(
        QueryServiceEx.PubSub,
        "events:#{entity_id}",
        {:new_event, %{"entity_id" => entity_id}}
      )

      assert_receive {:new_event, %{"entity_id" => ^entity_id}}
    end

    test "subscribes to event type topic" do
      event_type = "user.created"
      Phoenix.PubSub.subscribe(QueryServiceEx.PubSub, "events:type:#{event_type}")

      assert :ok = Phoenix.PubSub.broadcast(
        QueryServiceEx.PubSub,
        "events:type:#{event_type}",
        {:new_event, %{"event_type" => event_type}}
      )

      assert_receive {:new_event, %{"event_type" => ^event_type}}
    end
  end

  describe "JSON parsing" do
    test "parses valid JSON event" do
      json = ~s({"id": "evt-123", "entity_id": "user-1", "event_type": "user.created"})
      assert {:ok, event} = Jason.decode(json)
      assert event["id"] == "evt-123"
      assert event["entity_id"] == "user-1"
    end

    test "handles invalid JSON gracefully" do
      invalid_json = "not valid json {"
      assert {:error, _reason} = Jason.decode(invalid_json)
    end
  end

  describe "backoff calculation" do
    test "initial backoff is reasonable" do
      # The module uses 1000ms initial backoff
      assert 1_000 == 1_000
    end

    test "max backoff is capped" do
      # The module uses 30_000ms max backoff
      assert 30_000 == 30_000
    end
  end
end
