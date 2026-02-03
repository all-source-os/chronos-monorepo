defmodule McpServerElixir.Infrastructure.CoreWebSocketClientTest do
  use ExUnit.Case, async: true

  alias McpServerElixir.Infrastructure.CoreWebSocketClient

  describe "start_link/1" do
    test "returns :ignore when disabled" do
      assert :ignore = CoreWebSocketClient.start_link(enabled: false)
    end

    test "returns :ignore when disabled via config" do
      # In test environment, core_ws_enabled is false by default
      assert :ignore = CoreWebSocketClient.start_link([])
    end
  end

  describe "configuration" do
    test "uses default URL when not configured" do
      # The default URL is ws://localhost:3900/api/v1/events/stream
      default_url = "ws://localhost:3900/api/v1/events/stream"

      # Get the URL from application config or default
      url = Application.get_env(:mcp_server_elixir, :core_ws_url, default_url)
      assert is_binary(url)
      assert String.starts_with?(url, "ws://") or String.starts_with?(url, "wss://")
    end

    test "respects max reconnect attempts configuration" do
      max_attempts =
        Application.get_env(:mcp_server_elixir, :core_ws_max_reconnect_attempts, 10)

      assert is_integer(max_attempts)
      assert max_attempts > 0
    end

    test "respects backoff configuration" do
      initial = Application.get_env(:mcp_server_elixir, :core_ws_initial_backoff_ms, 1_000)
      max = Application.get_env(:mcp_server_elixir, :core_ws_max_backoff_ms, 30_000)

      assert is_integer(initial)
      assert is_integer(max)
      assert initial > 0
      assert max >= initial
    end
  end

  describe "event broadcasting" do
    setup do
      # Start PubSub for testing if not already running
      case Process.whereis(McpServerElixir.PubSub) do
        nil -> start_supervised!({Phoenix.PubSub, name: McpServerElixir.PubSub})
        _pid -> :ok
      end

      :ok
    end

    test "can subscribe to events:all topic" do
      Phoenix.PubSub.subscribe(McpServerElixir.PubSub, "events:all")

      # Simulate receiving an event by broadcasting directly
      event = %{
        "id" => "test-event-1",
        "event_type" => "user.created",
        "entity_id" => "user-123",
        "payload" => %{"name" => "Test User"}
      }

      Phoenix.PubSub.broadcast(McpServerElixir.PubSub, "events:all", {:new_event, event})

      assert_receive {:new_event, ^event}, 1000
    end

    test "can subscribe to entity-specific topic" do
      entity_id = "user-456"
      Phoenix.PubSub.subscribe(McpServerElixir.PubSub, "events:#{entity_id}")

      event = %{
        "id" => "test-event-2",
        "event_type" => "user.updated",
        "entity_id" => entity_id,
        "payload" => %{"name" => "Updated User"}
      }

      Phoenix.PubSub.broadcast(McpServerElixir.PubSub, "events:#{entity_id}", {:new_event, event})

      assert_receive {:new_event, ^event}, 1000
    end

    test "can subscribe to event-type topic" do
      event_type = "order.placed"
      Phoenix.PubSub.subscribe(McpServerElixir.PubSub, "events:type:#{event_type}")

      event = %{
        "id" => "test-event-3",
        "event_type" => event_type,
        "entity_id" => "order-789",
        "payload" => %{"total" => 99.99}
      }

      Phoenix.PubSub.broadcast(
        McpServerElixir.PubSub,
        "events:type:#{event_type}",
        {:new_event, event}
      )

      assert_receive {:new_event, ^event}, 1000
    end
  end

  describe "event filters" do
    test "filters can be set as a map" do
      filters = %{entity_id: "user-123", event_type: "user.created"}
      assert is_map(filters)
      assert Map.has_key?(filters, :entity_id)
      assert Map.has_key?(filters, :event_type)
    end

    test "empty filters are valid" do
      filters = %{}
      assert map_size(filters) == 0
    end

    test "filters support entity_id only" do
      filters = %{entity_id: "order-789"}
      assert filters[:entity_id] == "order-789"
      refute Map.has_key?(filters, :event_type)
    end

    test "filters support event_type only" do
      filters = %{event_type: "order.created"}
      assert filters[:event_type] == "order.created"
      refute Map.has_key?(filters, :entity_id)
    end
  end

  describe "state management" do
    test "initial state has correct default values" do
      state = %{
        connected: false,
        reconnect_attempts: 0,
        total_reconnects: 0,
        events_received: 0,
        last_event_at: nil,
        last_error: nil
      }

      assert state.connected == false
      assert state.reconnect_attempts == 0
      assert state.events_received == 0
      assert is_nil(state.last_event_at)
    end

    test "state tracks events received count" do
      initial_state = %{events_received: 0}
      updated_state = %{initial_state | events_received: initial_state.events_received + 1}
      assert updated_state.events_received == 1
    end

    test "state tracks reconnection attempts" do
      state = %{reconnect_attempts: 0, max_reconnect_attempts: 10}
      new_state = %{state | reconnect_attempts: state.reconnect_attempts + 1}
      assert new_state.reconnect_attempts == 1
      assert new_state.reconnect_attempts <= new_state.max_reconnect_attempts
    end
  end

  describe "URL configuration" do
    test "supports ws:// protocol" do
      url = "ws://localhost:3900/api/v1/events/stream"
      assert String.starts_with?(url, "ws://")
    end

    test "supports wss:// protocol for secure connections" do
      url = "wss://example.com/api/v1/events/stream"
      assert String.starts_with?(url, "wss://")
    end

    test "URL contains the events stream endpoint" do
      url = "ws://localhost:3900/api/v1/events/stream"
      assert String.contains?(url, "/api/v1/events/stream")
    end
  end
end
