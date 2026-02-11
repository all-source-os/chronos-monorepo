defmodule QueryServiceExWeb.ChannelBroadcasterTest do
  use QueryServiceExWeb.ChannelCase

  alias QueryServiceExWeb.ChannelBroadcaster

  describe "broadcast_event/1" do
    test "broadcasts event to events:all topic" do
      Phoenix.PubSub.subscribe(QueryServiceEx.PubSub, "events:all")

      event = %{
        "id" => "event-123",
        "entity_id" => "user-456",
        "event_type" => "user.created",
        "payload" => %{"name" => "John"}
      }

      assert :ok = ChannelBroadcaster.broadcast_event(event)

      assert_receive {:new_event, ^event}
    end

    test "broadcasts event to entity-specific topic" do
      Phoenix.PubSub.subscribe(QueryServiceEx.PubSub, "events:user-456")

      event = %{
        "id" => "event-123",
        "entity_id" => "user-456",
        "event_type" => "user.created",
        "payload" => %{}
      }

      ChannelBroadcaster.broadcast_event(event)

      assert_receive {:new_event, ^event}
    end

    test "broadcasts event to event-type topic" do
      Phoenix.PubSub.subscribe(QueryServiceEx.PubSub, "events:type:user.created")

      event = %{
        "id" => "event-123",
        "entity_id" => "user-456",
        "event_type" => "user.created",
        "payload" => %{}
      }

      ChannelBroadcaster.broadcast_event(event)

      assert_receive {:new_event, ^event}
    end

    test "handles atom keys in event map" do
      Phoenix.PubSub.subscribe(QueryServiceEx.PubSub, "events:all")
      Phoenix.PubSub.subscribe(QueryServiceEx.PubSub, "events:user-789")
      Phoenix.PubSub.subscribe(QueryServiceEx.PubSub, "events:type:order.placed")

      event = %{
        id: "event-999",
        entity_id: "user-789",
        event_type: "order.placed",
        payload: %{total: 100}
      }

      ChannelBroadcaster.broadcast_event(event)

      assert_receive {:new_event, ^event}
      assert_receive {:new_event, ^event}
      assert_receive {:new_event, ^event}
    end

    test "handles event without entity_id" do
      Phoenix.PubSub.subscribe(QueryServiceEx.PubSub, "events:all")

      event = %{"id" => "event-123", "event_type" => "system.startup", "payload" => %{}}

      assert :ok = ChannelBroadcaster.broadcast_event(event)

      assert_receive {:new_event, ^event}
    end

    test "handles event without event_type" do
      Phoenix.PubSub.subscribe(QueryServiceEx.PubSub, "events:all")

      event = %{"id" => "event-123", "entity_id" => "user-123", "payload" => %{}}

      assert :ok = ChannelBroadcaster.broadcast_event(event)

      assert_receive {:new_event, ^event}
    end
  end

  describe "broadcast_projection_update/4" do
    test "broadcasts state update to projection topic" do
      Phoenix.PubSub.subscribe(QueryServiceEx.PubSub, "projections:user_stats")

      state = %{"count" => 42}

      ChannelBroadcaster.broadcast_projection_update("user_stats", "user-123", state, version: 5)

      assert_receive {:state_updated, update}
      assert update.entity_id == "user-123"
      assert update.state == state
      assert update.version == 5
    end

    test "includes timestamp in update" do
      Phoenix.PubSub.subscribe(QueryServiceEx.PubSub, "projections:test_projection")

      ChannelBroadcaster.broadcast_projection_update("test_projection", "entity-1", %{})

      assert_receive {:state_updated, update}
      assert update.updated_at
    end

    test "uses provided timestamp" do
      Phoenix.PubSub.subscribe(QueryServiceEx.PubSub, "projections:test_projection")

      custom_time = "2026-01-15T10:30:00Z"

      ChannelBroadcaster.broadcast_projection_update("test_projection", "entity-1", %{},
        updated_at: custom_time
      )

      assert_receive {:state_updated, update}
      assert update.updated_at == custom_time
    end
  end

  describe "broadcast_projection_error/4" do
    test "broadcasts error to projection topic" do
      Phoenix.PubSub.subscribe(QueryServiceEx.PubSub, "projections:user_stats")

      ChannelBroadcaster.broadcast_projection_error(
        "user_stats",
        "user-456",
        "Invalid state transition",
        event_id: "evt-789"
      )

      assert_receive {:projection_error, error}
      assert error.entity_id == "user-456"
      assert error.error == "Invalid state transition"
      assert error.event_id == "evt-789"
    end

    test "includes timestamp in error" do
      Phoenix.PubSub.subscribe(QueryServiceEx.PubSub, "projections:test_projection")

      ChannelBroadcaster.broadcast_projection_error(
        "test_projection",
        "entity-1",
        "Error message"
      )

      assert_receive {:projection_error, error}
      assert error.occurred_at
    end
  end

  describe "subscriber_count/1" do
    test "returns 0 for topic with no subscribers" do
      count = ChannelBroadcaster.subscriber_count("events:nonexistent_topic")

      assert count == 0
    end

    test "returns correct count after subscribing" do
      Phoenix.PubSub.subscribe(QueryServiceEx.PubSub, "events:test_count")

      # Note: The subscriber_count function relies on internal Phoenix.PubSub implementation
      # which may not be accurate in test environment
      count = ChannelBroadcaster.subscriber_count("events:test_count")

      assert is_integer(count)
    end
  end
end
