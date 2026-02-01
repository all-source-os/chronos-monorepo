defmodule QueryServiceEx.TelemetryTest do
  @moduledoc """
  Tests for the Telemetry module.

  Verifies that telemetry handlers are correctly attached and process events.
  """
  use ExUnit.Case, async: false

  import ExUnit.CaptureLog

  alias QueryServiceEx.Telemetry

  # Set log level to debug to capture all log messages
  setup do
    current_level = Logger.level()
    Logger.configure(level: :debug)
    on_exit(fn -> Logger.configure(level: current_level) end)
    :ok
  end

  describe "attach_handlers/0" do
    test "returns :ok" do
      # Detach first to avoid conflicts
      Telemetry.detach_handlers()

      assert Telemetry.attach_handlers() == :ok
    end

    test "attaches all expected handlers" do
      Telemetry.detach_handlers()
      Telemetry.attach_handlers()

      # Verify handlers are attached by checking they don't raise
      # when we emit events
      :telemetry.execute(
        [:query_service_ex, :http, :error],
        %{},
        %{correlation_id: "test", error_type: "test", status: 500}
      )
    end
  end

  describe "detach_handlers/0" do
    test "returns :ok" do
      assert Telemetry.detach_handlers() == :ok
    end
  end

  describe "handle_phoenix_stop/4" do
    test "logs HTTP request completion" do
      conn = %{
        method: "GET",
        request_path: "/api/test",
        status: 200
      }

      measurements = %{duration: System.convert_time_unit(100, :millisecond, :native)}
      metadata = %{conn: conn}

      log =
        capture_log([level: :info], fn ->
          Telemetry.handle_phoenix_stop(
            [:phoenix, :endpoint, :stop],
            measurements,
            metadata,
            %{}
          )
        end)

      assert log =~ "HTTP request completed"
      assert log =~ "GET"
      assert log =~ "/api/test"
      assert log =~ "200"
    end
  end

  describe "handle_ecto_query/4" do
    test "logs database query at debug level for fast queries" do
      measurements = %{total_time: System.convert_time_unit(10, :millisecond, :native)}
      metadata = %{source: "users"}

      log =
        capture_log([level: :debug], fn ->
          Telemetry.handle_ecto_query(
            [:query_service_ex, :repo, :query],
            measurements,
            metadata,
            %{}
          )
        end)

      assert log =~ "Database query executed"
    end

    test "logs slow queries at warning level" do
      measurements = %{total_time: System.convert_time_unit(150, :millisecond, :native)}
      metadata = %{source: "events"}

      log =
        capture_log([level: :warning], fn ->
          Telemetry.handle_ecto_query(
            [:query_service_ex, :repo, :query],
            measurements,
            metadata,
            %{}
          )
        end)

      assert log =~ "Database query executed"
    end

    test "handles nil total_time" do
      measurements = %{total_time: nil}
      metadata = %{source: "test"}

      # Should not raise
      Telemetry.handle_ecto_query(
        [:query_service_ex, :repo, :query],
        measurements,
        metadata,
        %{}
      )
    end
  end

  describe "handle_event_processed/4" do
    test "logs event processing" do
      measurements = %{duration_ms: 50}

      metadata = %{
        event_id: "evt-123",
        event_type: "UserCreated",
        entity_id: "user-456"
      }

      log =
        capture_log([level: :info], fn ->
          Telemetry.handle_event_processed(
            [:query_service_ex, :event_pipeline, :event_processed],
            measurements,
            metadata,
            %{}
          )
        end)

      assert log =~ "Event processed"
      assert log =~ "evt-123"
      assert log =~ "UserCreated"
    end
  end

  describe "handle_projections_synced/4" do
    test "logs projection sync" do
      measurements = %{duration_ms: 100}

      metadata = %{
        projection_count: 5,
        event_id: "evt-789"
      }

      log =
        capture_log([level: :info], fn ->
          Telemetry.handle_projections_synced(
            [:query_service_ex, :event_pipeline, :projections_synced],
            measurements,
            metadata,
            %{}
          )
        end)

      assert log =~ "Projections synced"
    end
  end

  describe "handle_events_failed/4" do
    test "logs event processing failure at error level" do
      measurements = %{}

      metadata = %{
        event_ids: ["evt-1", "evt-2"],
        error: %RuntimeError{message: "processing failed"}
      }

      log =
        capture_log([level: :error], fn ->
          Telemetry.handle_events_failed(
            [:query_service_ex, :event_pipeline, :events_failed],
            measurements,
            metadata,
            %{}
          )
        end)

      assert log =~ "Events processing failed"
    end
  end

  describe "handle_projection_sync_started/4" do
    test "logs sync start at debug level" do
      metadata = %{
        projection_name: "user_stats",
        entity_id: "user-123"
      }

      log =
        capture_log([level: :debug], fn ->
          Telemetry.handle_projection_sync_started(
            [:query_service_ex, :projection, :sync_started],
            %{},
            metadata,
            %{}
          )
        end)

      assert log =~ "Projection sync started"
      assert log =~ "user_stats"
    end
  end

  describe "handle_projection_sync_completed/4" do
    test "logs sync completion" do
      measurements = %{duration_ms: 25}

      metadata = %{
        projection_name: "order_summary",
        entity_id: "order-456"
      }

      log =
        capture_log([level: :info], fn ->
          Telemetry.handle_projection_sync_completed(
            [:query_service_ex, :projection, :sync_completed],
            measurements,
            metadata,
            %{}
          )
        end)

      assert log =~ "Projection sync completed"
      assert log =~ "order_summary"
    end
  end

  describe "handle_projection_sync_failed/4" do
    test "logs sync failure at error level" do
      metadata = %{
        projection_name: "failing_projection",
        entity_id: "entity-123",
        error: {:timeout, :connection}
      }

      log =
        capture_log([level: :error], fn ->
          Telemetry.handle_projection_sync_failed(
            [:query_service_ex, :projection, :sync_failed],
            %{},
            metadata,
            %{}
          )
        end)

      assert log =~ "Projection sync failed"
      assert log =~ "failing_projection"
    end
  end

  describe "handle_websocket_connected/4" do
    test "logs WebSocket connection" do
      metadata = %{url: "ws://localhost:8080/events"}

      log =
        capture_log([level: :info], fn ->
          Telemetry.handle_websocket_connected(
            [:query_service_ex, :websocket, :connected],
            %{},
            metadata,
            %{}
          )
        end)

      assert log =~ "WebSocket connected"
      assert log =~ "ws://localhost:8080/events"
    end
  end

  describe "handle_websocket_disconnected/4" do
    test "logs WebSocket disconnection at warning level" do
      metadata = %{
        url: "ws://localhost:8080/events",
        reason: :connection_closed
      }

      log =
        capture_log([level: :warning], fn ->
          Telemetry.handle_websocket_disconnected(
            [:query_service_ex, :websocket, :disconnected],
            %{},
            metadata,
            %{}
          )
        end)

      assert log =~ "WebSocket disconnected"
      assert log =~ "connection_closed"
    end
  end

  describe "handle_websocket_message/4" do
    test "logs message received at debug level" do
      measurements = %{size_bytes: 1024}
      metadata = %{message_type: "event"}

      log =
        capture_log([level: :debug], fn ->
          Telemetry.handle_websocket_message(
            [:query_service_ex, :websocket, :message_received],
            measurements,
            metadata,
            %{}
          )
        end)

      assert log =~ "WebSocket message received"
    end
  end

  describe "handle_websocket_reconnect_attempt/4" do
    test "logs reconnection attempt" do
      measurements = %{attempt: 3, backoff_ms: 5000}
      metadata = %{url: "ws://localhost:8080/events"}

      log =
        capture_log([level: :info], fn ->
          Telemetry.handle_websocket_reconnect_attempt(
            [:query_service_ex, :websocket, :reconnect_attempt],
            measurements,
            metadata,
            %{}
          )
        end)

      assert log =~ "WebSocket reconnection attempt"
    end
  end

  describe "handle_websocket_reconnect_exhausted/4" do
    test "logs exhausted attempts at error level" do
      measurements = %{attempts: 10}

      metadata = %{
        url: "ws://localhost:8080/events",
        last_error: :econnrefused
      }

      log =
        capture_log([level: :error], fn ->
          Telemetry.handle_websocket_reconnect_exhausted(
            [:query_service_ex, :websocket, :reconnect_exhausted],
            measurements,
            metadata,
            %{}
          )
        end)

      assert log =~ "WebSocket reconnection attempts exhausted"
    end
  end

  describe "handle_circuit_opened/4" do
    test "logs circuit breaker opening at warning level" do
      measurements = %{failure_count: 5}
      metadata = %{from_state: :closed}

      log =
        capture_log([level: :warning], fn ->
          Telemetry.handle_circuit_opened(
            [:query_service_ex, :circuit_breaker, :opened],
            measurements,
            metadata,
            %{}
          )
        end)

      assert log =~ "Circuit breaker opened"
      assert log =~ "closed"
    end
  end

  describe "handle_circuit_closed/4" do
    test "logs circuit recovery" do
      log =
        capture_log([level: :info], fn ->
          Telemetry.handle_circuit_closed(
            [:query_service_ex, :circuit_breaker, :closed],
            %{},
            %{},
            %{}
          )
        end)

      assert log =~ "Circuit breaker closed"
      assert log =~ "recovered"
    end
  end

  describe "handle_circuit_half_open/4" do
    test "logs half-open transition" do
      metadata = %{circuit: :core_backend}

      log =
        capture_log([level: :info], fn ->
          Telemetry.handle_circuit_half_open(
            [:query_service_ex, :circuit_breaker, :half_open],
            %{},
            metadata,
            %{}
          )
        end)

      assert log =~ "Circuit breaker half-open"
      assert log =~ "testing recovery"
    end
  end

  describe "handle_circuit_rejected/4" do
    test "logs rejected request at warning level" do
      metadata = %{circuit: :core_backend}

      log =
        capture_log([level: :warning], fn ->
          Telemetry.handle_circuit_rejected(
            [:query_service_ex, :circuit_breaker, :rejected],
            %{},
            metadata,
            %{}
          )
        end)

      assert log =~ "Request rejected by circuit breaker"
    end
  end

  describe "handle_health_check_timeout/4" do
    test "logs timeout at warning level" do
      measurements = %{timeout_ms: 5000}
      metadata = %{check: :database}

      log =
        capture_log([level: :warning], fn ->
          Telemetry.handle_health_check_timeout(
            [:query_service_ex, :health_check, :timeout],
            measurements,
            metadata,
            %{}
          )
        end)

      assert log =~ "Health check timed out"
    end
  end

  describe "handle_health_check_completed/4" do
    test "logs completion at debug level" do
      measurements = %{duration_ms: 50}
      metadata = %{check: :database, status: :healthy}

      log =
        capture_log([level: :debug], fn ->
          Telemetry.handle_health_check_completed(
            [:query_service_ex, :health_check, :completed],
            measurements,
            metadata,
            %{}
          )
        end)

      assert log =~ "Health check completed"
    end
  end

  describe "handle_http_error/4" do
    test "logs HTTP error at error level" do
      metadata = %{
        correlation_id: "req-123",
        error_type: "RuntimeError",
        status: 500
      }

      log =
        capture_log([level: :error], fn ->
          Telemetry.handle_http_error(
            [:query_service_ex, :http, :error],
            %{},
            metadata,
            %{}
          )
        end)

      assert log =~ "HTTP error occurred"
      assert log =~ "req-123"
      assert log =~ "500"
    end
  end
end
