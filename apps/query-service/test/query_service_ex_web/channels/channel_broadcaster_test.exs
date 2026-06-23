defmodule QueryServiceExWeb.ChannelBroadcasterTest do
  use QueryServiceExWeb.ChannelCase

  alias QueryServiceExWeb.ChannelBroadcaster

  @tenant "tenant-1"
  @other "tenant-2"

  describe "broadcast_event/1 (tenant-scoped)" do
    test "broadcasts to the tenant's events:<tenant>:all topic" do
      Phoenix.PubSub.subscribe(QueryServiceEx.PubSub, "events:#{@tenant}:all")

      event = %{
        "id" => "event-123",
        "tenant_id" => @tenant,
        "entity_id" => "user-456",
        "event_type" => "user.created",
        "payload" => %{"name" => "John"}
      }

      assert :ok = ChannelBroadcaster.broadcast_event(event)
      assert_receive {:new_event, ^event}
    end

    test "broadcasts to the tenant's entity topic" do
      Phoenix.PubSub.subscribe(QueryServiceEx.PubSub, "events:#{@tenant}:user-456")

      event = %{
        "id" => "event-123",
        "tenant_id" => @tenant,
        "entity_id" => "user-456",
        "event_type" => "user.created",
        "payload" => %{}
      }

      ChannelBroadcaster.broadcast_event(event)
      assert_receive {:new_event, ^event}
    end

    test "broadcasts to the tenant's event-type topic" do
      Phoenix.PubSub.subscribe(QueryServiceEx.PubSub, "events:#{@tenant}:type:user.created")

      event = %{
        "id" => "event-123",
        "tenant_id" => @tenant,
        "entity_id" => "user-456",
        "event_type" => "user.created",
        "payload" => %{}
      }

      ChannelBroadcaster.broadcast_event(event)
      assert_receive {:new_event, ^event}
    end

    test "handles atom keys in event map" do
      Phoenix.PubSub.subscribe(QueryServiceEx.PubSub, "events:#{@tenant}:all")
      Phoenix.PubSub.subscribe(QueryServiceEx.PubSub, "events:#{@tenant}:user-789")
      Phoenix.PubSub.subscribe(QueryServiceEx.PubSub, "events:#{@tenant}:type:order.placed")

      event = %{
        id: "event-999",
        tenant_id: @tenant,
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
      Phoenix.PubSub.subscribe(QueryServiceEx.PubSub, "events:#{@tenant}:all")

      event = %{
        "id" => "event-123",
        "tenant_id" => @tenant,
        "event_type" => "system.startup",
        "payload" => %{}
      }

      assert :ok = ChannelBroadcaster.broadcast_event(event)
      assert_receive {:new_event, ^event}
    end

    test "handles event without event_type" do
      Phoenix.PubSub.subscribe(QueryServiceEx.PubSub, "events:#{@tenant}:all")

      event = %{
        "id" => "event-123",
        "tenant_id" => @tenant,
        "entity_id" => "user-123",
        "payload" => %{}
      }

      assert :ok = ChannelBroadcaster.broadcast_event(event)
      assert_receive {:new_event, ^event}
    end

    # --- tenant-isolation gate ---

    test "fails closed: an event without a tenant_id is NOT broadcast" do
      Phoenix.PubSub.subscribe(QueryServiceEx.PubSub, "events:#{@tenant}:all")

      event = %{"id" => "event-x", "entity_id" => "user-1", "event_type" => "user.created"}

      assert :ok = ChannelBroadcaster.broadcast_event(event)
      refute_receive {:new_event, _}, 100
    end

    test "isolation: tenant-2's event never reaches a tenant-1 subscriber" do
      # Subscribe as tenant-1 across all topic shapes.
      Phoenix.PubSub.subscribe(QueryServiceEx.PubSub, "events:#{@tenant}:all")
      Phoenix.PubSub.subscribe(QueryServiceEx.PubSub, "events:#{@tenant}:user-456")
      Phoenix.PubSub.subscribe(QueryServiceEx.PubSub, "events:#{@tenant}:type:user.created")

      # Broadcast an event owned by tenant-2 with the SAME entity_id + type.
      event = %{
        "id" => "event-leak",
        "tenant_id" => @other,
        "entity_id" => "user-456",
        "event_type" => "user.created",
        "payload" => %{}
      }

      ChannelBroadcaster.broadcast_event(event)
      refute_receive {:new_event, _}, 100
    end
  end

  describe "broadcast_projection_update/5 (tenant-scoped)" do
    test "broadcasts state update to the tenant's projection topic" do
      Phoenix.PubSub.subscribe(QueryServiceEx.PubSub, "projections:#{@tenant}:user_stats")

      state = %{"count" => 42}

      ChannelBroadcaster.broadcast_projection_update(@tenant, "user_stats", "user-123", state,
        version: 5
      )

      assert_receive {:state_updated, update}
      assert update.entity_id == "user-123"
      assert update.state == state
      assert update.version == 5
    end

    test "isolation: tenant-2's projection update never reaches tenant-1" do
      Phoenix.PubSub.subscribe(QueryServiceEx.PubSub, "projections:#{@tenant}:user_stats")

      ChannelBroadcaster.broadcast_projection_update(@other, "user_stats", "user-1", %{"c" => 1})

      refute_receive {:state_updated, _}, 100
    end
  end

  describe "broadcast_projection_error/5 (tenant-scoped)" do
    test "broadcasts error to the tenant's projection topic" do
      Phoenix.PubSub.subscribe(QueryServiceEx.PubSub, "projections:#{@tenant}:user_stats")

      ChannelBroadcaster.broadcast_projection_error(
        @tenant,
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
  end

  describe "subscriber_count/1" do
    test "returns 0 for topic with no subscribers" do
      assert ChannelBroadcaster.subscriber_count("events:nonexistent_topic") == 0
    end

    test "returns an integer after subscribing" do
      Phoenix.PubSub.subscribe(QueryServiceEx.PubSub, "events:test_count")
      assert is_integer(ChannelBroadcaster.subscriber_count("events:test_count"))
    end
  end
end
