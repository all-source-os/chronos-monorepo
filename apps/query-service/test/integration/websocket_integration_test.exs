defmodule QueryServiceEx.Integration.WebSocketIntegrationTest do
  @moduledoc """
  Integration tests for WebSocket connectivity between Query Service and Core.

  These tests require Core to be running and are tagged with :integration.
  Run with: mix test --only integration

  Test scenarios:
  1. WebSocket connection establishment
  2. Health check reflects WebSocket status
  3. Real-time event propagation (Core → WebSocket → PubSub)
  4. Reconnection after Core restart
  5. Graceful degradation when Core is unavailable
  """

  use ExUnit.Case, async: false

  alias QueryServiceEx.Infrastructure.Adapters.CoreWebSocketClient
  alias QueryServiceEx.Infrastructure.Adapters.RustCoreClient

  @moduletag :integration
  @moduletag :websocket

  # Core service configuration
  # Tenant the (unauthenticated) test POST is stamped with by Core. Broadcasts
  # are now tenant-scoped (events:<tenant>:...), so subscriptions must match it.
  @tenant System.get_env("TEST_TENANT_ID", "default")
  @core_http_url System.get_env("CORE_HTTP_URL", "http://localhost:3900")
  @core_ws_url System.get_env("CORE_WS_URL", "ws://localhost:3900")

  setup do
    # Skip if Core is not available
    case RustCoreClient.health_check() do
      {:ok, _} ->
        :ok

      {:error, reason} ->
        {:skip, "Core service not available: #{inspect(reason)}"}
    end
  end

  describe "WebSocket connection establishment" do
    test "connects to Core WebSocket endpoint successfully" do
      # Start a fresh WebSocket client with unique name
      name = :"test_ws_client_#{System.unique_integer([:positive])}"

      assert {:ok, pid} =
               CoreWebSocketClient.start_link(
                 enabled: true,
                 url: @core_ws_url <> "/api/v1/events/stream",
                 name: name
               )

      # Wait for connection to establish
      Process.sleep(500)

      # Verify connected status
      assert CoreWebSocketClient.status(pid) == :connected

      # Get stats and verify
      stats = CoreWebSocketClient.stats(pid)
      assert stats.connected == true
      assert stats.reconnect_attempts == 0

      # Cleanup
      GenServer.stop(pid)
    end

    test "reports disconnected status when Core is unreachable" do
      # Start client with invalid URL
      name = :"test_ws_invalid_#{System.unique_integer([:positive])}"

      # This should return :ignore after failed connection attempts
      result =
        CoreWebSocketClient.start_link(
          enabled: true,
          url: "ws://localhost:59999/api/v1/events/stream",
          name: name,
          max_reconnect_attempts: 1,
          initial_backoff_ms: 100
        )

      # Should enter retry loop and return :ignore
      assert result == :ignore
    end
  end

  describe "health check WebSocket status" do
    setup do
      # Start a test WebSocket client
      name = :"health_test_ws_#{System.unique_integer([:positive])}"

      {:ok, pid} =
        CoreWebSocketClient.start_link(
          enabled: true,
          url: @core_ws_url <> "/api/v1/events/stream",
          name: name
        )

      Process.sleep(500)

      on_exit(fn ->
        if Process.alive?(pid), do: GenServer.stop(pid)
      end)

      {:ok, ws_pid: pid, ws_name: name}
    end

    test "health endpoint includes websocket check", %{ws_pid: pid} do
      # Verify WebSocket is connected
      assert CoreWebSocketClient.status(pid) == :connected

      # Call health endpoint
      conn =
        :get
        |> Plug.Test.conn("/api/health")
        |> QueryServiceExWeb.Router.call(QueryServiceExWeb.Router.init([]))

      response = Jason.decode!(conn.resp_body)

      # Verify response structure includes websocket check
      assert Map.has_key?(response, "checks")
      assert Map.has_key?(response["checks"], "websocket")

      # WebSocket should be healthy when connected
      # Note: This depends on the registered process name matching
    end

    test "health endpoint shows degraded when websocket disabled" do
      # Get health with default (disabled in test) websocket
      conn =
        :get
        |> Plug.Test.conn("/api/health")
        |> QueryServiceExWeb.Router.call(QueryServiceExWeb.Router.init([]))

      response = Jason.decode!(conn.resp_body)

      assert Map.has_key?(response["checks"], "websocket")
      # In test env with ws disabled, should still be healthy (not required)
      assert response["checks"]["websocket"] in ["healthy", "degraded"]
    end
  end

  describe "real-time event propagation" do
    setup do
      # Start WebSocket client
      name = :"event_test_ws_#{System.unique_integer([:positive])}"

      {:ok, pid} =
        CoreWebSocketClient.start_link(
          enabled: true,
          url: @core_ws_url <> "/api/v1/events/stream",
          name: name
        )

      Process.sleep(500)

      on_exit(fn ->
        if Process.alive?(pid), do: GenServer.stop(pid)
      end)

      {:ok, ws_pid: pid}
    end

    test "receives event via WebSocket after publishing to Core", %{ws_pid: _pid} do
      # Subscribe to all events topic
      Phoenix.PubSub.subscribe(QueryServiceEx.PubSub, "events:#{@tenant}:all")

      # Generate unique identifiers for this test
      entity_id = "test-entity-#{System.unique_integer([:positive])}"
      event_type = "integration.test.event"

      # Publish event to Core via HTTP
      event_payload = %{
        "event_type" => event_type,
        "entity_id" => entity_id,
        "stream_id" => "test-stream",
        "data" => %{"test" => true, "timestamp" => DateTime.utc_now() |> DateTime.to_iso8601()}
      }

      {:ok, response} = post_event_to_core(event_payload)
      assert Map.has_key?(response, "event_id")

      # Wait for WebSocket to receive and broadcast the event
      assert_receive {:new_event, received_event}, 5_000

      # Verify the event matches what we sent
      assert received_event["entity_id"] == entity_id
      assert received_event["event_type"] == event_type
    end

    test "receives event on entity-specific topic", %{ws_pid: _pid} do
      entity_id = "specific-entity-#{System.unique_integer([:positive])}"

      # Subscribe to entity-specific topic
      Phoenix.PubSub.subscribe(QueryServiceEx.PubSub, "events:#{@tenant}:#{entity_id}")

      # Publish event
      event_payload = %{
        "event_type" => "entity.specific.test",
        "entity_id" => entity_id,
        "stream_id" => "test-stream",
        "data" => %{"specific" => true}
      }

      {:ok, _response} = post_event_to_core(event_payload)

      # Should receive on entity-specific topic
      assert_receive {:new_event, received_event}, 5_000
      assert received_event["entity_id"] == entity_id
    end

    test "receives event on event-type topic", %{ws_pid: _pid} do
      event_type = "type.specific.test.#{System.unique_integer([:positive])}"

      # Subscribe to event-type topic
      Phoenix.PubSub.subscribe(QueryServiceEx.PubSub, "events:#{@tenant}:type:#{event_type}")

      # Publish event
      event_payload = %{
        "event_type" => event_type,
        "entity_id" => "any-entity",
        "stream_id" => "test-stream",
        "data" => %{"type_specific" => true}
      }

      {:ok, _response} = post_event_to_core(event_payload)

      # Should receive on event-type topic
      assert_receive {:new_event, received_event}, 5_000
      assert received_event["event_type"] == event_type
    end
  end

  describe "WebSocket statistics" do
    test "tracks events received count" do
      name = :"stats_test_ws_#{System.unique_integer([:positive])}"

      {:ok, pid} =
        CoreWebSocketClient.start_link(
          enabled: true,
          url: @core_ws_url <> "/api/v1/events/stream",
          name: name
        )

      Process.sleep(500)

      # Get initial stats
      initial_stats = CoreWebSocketClient.stats(pid)
      initial_count = initial_stats.events_received

      # Publish an event
      event_payload = %{
        "event_type" => "stats.test",
        "entity_id" => "stats-entity-#{System.unique_integer([:positive])}",
        "stream_id" => "test-stream",
        "data" => %{}
      }

      {:ok, _} = post_event_to_core(event_payload)

      # Wait for event to be processed
      Process.sleep(1_000)

      # Get updated stats
      updated_stats = CoreWebSocketClient.stats(pid)

      # Event count should have increased
      assert updated_stats.events_received > initial_count

      GenServer.stop(pid)
    end
  end

  describe "graceful degradation" do
    test "service continues when WebSocket is disabled" do
      # Verify service health when WebSocket is explicitly disabled
      # In test env, core_ws_enabled is false by default

      conn =
        :get
        |> Plug.Test.conn("/api/health/live")
        |> QueryServiceExWeb.Router.call(QueryServiceExWeb.Router.init([]))

      # Liveness should always pass
      assert conn.status == 200

      response = Jason.decode!(conn.resp_body)
      assert response["status"] == "alive"
    end

    test "health shows degraded not unhealthy when WebSocket unavailable" do
      # When WebSocket can't connect, it should be "degraded" not "unhealthy"
      # This ensures the service doesn't fail health checks completely

      conn =
        :get
        |> Plug.Test.conn("/api/health")
        |> QueryServiceExWeb.Router.call(QueryServiceExWeb.Router.init([]))

      response = Jason.decode!(conn.resp_body)

      # WebSocket issues should cause degraded, not unhealthy
      if response["checks"]["websocket"] != "healthy" do
        assert response["checks"]["websocket"] == "degraded"
      end
    end
  end

  # Helper functions

  defp post_event_to_core(event_payload) do
    url = @core_http_url <> "/api/v1/events"

    headers = [
      {"Content-Type", "application/json"}
    ]

    body = Jason.encode!(event_payload)

    case :httpc.request(
           :post,
           {String.to_charlist(url), headers, ~c"application/json", body},
           [timeout: 5_000],
           []
         ) do
      {:ok, {{_, 200, _}, _, response_body}} ->
        {:ok, Jason.decode!(to_string(response_body))}

      {:ok, {{_, 201, _}, _, response_body}} ->
        {:ok, Jason.decode!(to_string(response_body))}

      {:ok, {{_, status_code, _}, _, response_body}} ->
        {:error, {status_code, to_string(response_body)}}

      {:error, reason} ->
        {:error, reason}
    end
  end
end
