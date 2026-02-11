defmodule QueryServiceExWeb.EventChannelTest do
  use QueryServiceExWeb.ChannelCase

  alias QueryServiceExWeb.EventChannel

  setup do
    {:ok, socket, user, _token} = create_authenticated_socket()
    {:ok, socket: socket, user: user}
  end

  describe "join events:all" do
    test "successfully joins events:all channel", %{socket: socket} do
      {:ok, reply, _socket} = subscribe_and_join(socket, EventChannel, "events:all")

      assert reply.status == "subscribed"
      assert reply.topic == "events:all"
    end

    test "receives events after joining events:all", %{socket: socket} do
      {:ok, _reply, _socket} = subscribe_and_join(socket, EventChannel, "events:all")

      event = %{
        "id" => "event-#{:rand.uniform(100_000)}",
        "entity_id" => "user-123",
        "event_type" => "user.created",
        "payload" => %{"name" => "John"},
        "timestamp" => DateTime.utc_now() |> DateTime.to_iso8601()
      }

      broadcast_event(event)

      assert_push("new_event", pushed_event)
      assert pushed_event["id"] == event["id"]
      assert pushed_event["entity_id"] == "user-123"
    end

    test "receives presence_state after joining", %{socket: socket} do
      {:ok, _reply, _socket} = subscribe_and_join(socket, EventChannel, "events:all")

      assert_push("presence_state", _state)
    end
  end

  describe "join events:{entity_id}" do
    test "successfully joins entity-specific channel", %{socket: socket} do
      {:ok, reply, _socket} = subscribe_and_join(socket, EventChannel, "events:user-456")

      assert reply.status == "subscribed"
      assert reply.topic == "events:user-456"
      assert reply.entity_id == "user-456"
    end

    test "receives only events for subscribed entity", %{socket: socket} do
      {:ok, _reply, _socket} = subscribe_and_join(socket, EventChannel, "events:user-456")

      # First broadcast event for different entity (should not be received)
      non_matching_event = %{
        "id" => "event-2",
        "entity_id" => "user-789",
        "event_type" => "user.updated",
        "payload" => %{}
      }

      broadcast_event(non_matching_event)

      # Should not receive event for different entity
      refute_push("new_event", _, 50)

      # Now broadcast event for subscribed entity
      matching_event = %{
        "id" => "event-1",
        "entity_id" => "user-456",
        "event_type" => "user.updated",
        "payload" => %{}
      }

      broadcast_event(matching_event)

      assert_push("new_event", pushed_event)
      assert pushed_event["entity_id"] == "user-456"
    end
  end

  describe "join events:type:{event_type}" do
    test "successfully joins event type channel", %{socket: socket} do
      {:ok, reply, _socket} = subscribe_and_join(socket, EventChannel, "events:type:order.placed")

      assert reply.status == "subscribed"
      assert reply.topic == "events:type:order.placed"
      assert reply.event_type == "order.placed"
    end

    test "receives only events of subscribed type", %{socket: socket} do
      {:ok, _reply, _socket} =
        subscribe_and_join(socket, EventChannel, "events:type:order.placed")

      # First broadcast non-matching event (should not be received)
      non_matching_event = %{
        "id" => "event-2",
        "entity_id" => "order-456",
        "event_type" => "order.shipped",
        "payload" => %{}
      }

      broadcast_event(non_matching_event)

      # Should not receive event with different type
      refute_push("new_event", _, 50)

      # Now broadcast matching event
      matching_event = %{
        "id" => "event-1",
        "entity_id" => "order-123",
        "event_type" => "order.placed",
        "payload" => %{"total" => 100}
      }

      broadcast_event(matching_event)

      assert_push("new_event", pushed_event)
      assert pushed_event["event_type"] == "order.placed"
    end
  end

  describe "multiple subscriptions" do
    test "can join multiple event channels", %{socket: socket} do
      {:ok, _, socket} = subscribe_and_join(socket, EventChannel, "events:all")
      {:ok, _, _socket} = subscribe_and_join(socket, EventChannel, "events:user-123")

      # Both channels should work
      event = %{
        "id" => "event-1",
        "entity_id" => "user-123",
        "event_type" => "user.created",
        "payload" => %{}
      }

      broadcast_event(event)

      # Should receive from events:all
      assert_push("new_event", _)

      # The entity-specific subscription happens on a different process,
      # so we test that the join was successful
    end
  end

  describe "presence tracking" do
    test "tracks user presence on join", %{socket: socket, user: user} do
      {:ok, _reply, _socket} = subscribe_and_join(socket, EventChannel, "events:all")

      # Wait for presence to be tracked
      assert_push("presence_state", state)

      assert Map.has_key?(state, to_string(user.id))
      user_presence = state[to_string(user.id)]
      assert length(user_presence.metas) > 0
    end
  end
end
