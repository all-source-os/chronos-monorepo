defmodule QueryServiceEx.Telemetry do
  @moduledoc """
  Telemetry handlers for structured logging and observability.

  Attaches handlers to telemetry events emitted by:
  - Phoenix endpoint (HTTP requests)
  - Ecto repository (database queries)
  - Application services (event pipeline, projections, etc.)

  All telemetry events are logged with structured metadata for
  production debugging and monitoring.
  """

  require Logger

  @doc """
  Attaches all telemetry handlers. Should be called during application startup.
  """
  def attach_handlers do
    handlers = [
      # Phoenix HTTP request events
      {[:phoenix, :endpoint, :stop], &__MODULE__.handle_phoenix_stop/4},

      # Ecto database query events
      {[:query_service_ex, :repo, :query], &__MODULE__.handle_ecto_query/4},

      # Event pipeline events
      {[:query_service_ex, :event_pipeline, :event_processed],
       &__MODULE__.handle_event_processed/4},
      {[:query_service_ex, :event_pipeline, :projections_synced],
       &__MODULE__.handle_projections_synced/4},
      {[:query_service_ex, :event_pipeline, :events_failed], &__MODULE__.handle_events_failed/4},

      # Projection events
      {[:query_service_ex, :projection, :sync_started],
       &__MODULE__.handle_projection_sync_started/4},
      {[:query_service_ex, :projection, :sync_completed],
       &__MODULE__.handle_projection_sync_completed/4},
      {[:query_service_ex, :projection, :sync_failed],
       &__MODULE__.handle_projection_sync_failed/4},

      # WebSocket client events (connection to Core)
      {[:query_service_ex, :websocket, :connected], &__MODULE__.handle_websocket_connected/4},
      {[:query_service_ex, :websocket, :disconnected],
       &__MODULE__.handle_websocket_disconnected/4},
      {[:query_service_ex, :websocket, :message_received],
       &__MODULE__.handle_websocket_message/4},
      {[:query_service_ex, :websocket, :reconnect_attempt],
       &__MODULE__.handle_websocket_reconnect_attempt/4},
      {[:query_service_ex, :websocket, :reconnect_exhausted],
       &__MODULE__.handle_websocket_reconnect_exhausted/4},

      # Phoenix Channel events (external client connections)
      {[:query_service_ex, :channel, :joined], &__MODULE__.handle_channel_joined/4},
      {[:query_service_ex, :channel, :left], &__MODULE__.handle_channel_left/4},

      # Circuit breaker events
      {[:query_service_ex, :circuit_breaker, :opened], &__MODULE__.handle_circuit_opened/4},
      {[:query_service_ex, :circuit_breaker, :closed], &__MODULE__.handle_circuit_closed/4},
      {[:query_service_ex, :circuit_breaker, :half_open], &__MODULE__.handle_circuit_half_open/4},
      {[:query_service_ex, :circuit_breaker, :rejected], &__MODULE__.handle_circuit_rejected/4},

      # Health check events
      {[:query_service_ex, :health_check, :timeout], &__MODULE__.handle_health_check_timeout/4},
      {[:query_service_ex, :health_check, :completed],
       &__MODULE__.handle_health_check_completed/4},

      # HTTP error events
      {[:query_service_ex, :http, :error], &__MODULE__.handle_http_error/4},

      # APM custom events
      {[:query_service_ex, :apm, :event], &__MODULE__.handle_apm_event/4},
      {[:query_service_ex, :apm, :span_start], &__MODULE__.handle_apm_span_start/4},
      {[:query_service_ex, :apm, :span_end], &__MODULE__.handle_apm_span_end/4}
    ]

    for {event, handler} <- handlers do
      :telemetry.attach(
        "query-service-#{Enum.join(event, "-")}",
        event,
        handler,
        %{}
      )
    end

    :ok
  end

  @doc """
  Detaches all telemetry handlers. Useful for testing.
  """
  def detach_handlers do
    handlers = [
      "query-service-phoenix-endpoint-stop",
      "query-service-query_service_ex-repo-query",
      "query-service-query_service_ex-event_pipeline-event_processed",
      "query-service-query_service_ex-event_pipeline-projections_synced",
      "query-service-query_service_ex-event_pipeline-events_failed",
      "query-service-query_service_ex-projection-sync_started",
      "query-service-query_service_ex-projection-sync_completed",
      "query-service-query_service_ex-projection-sync_failed",
      "query-service-query_service_ex-websocket-connected",
      "query-service-query_service_ex-websocket-disconnected",
      "query-service-query_service_ex-websocket-message_received",
      "query-service-query_service_ex-websocket-reconnect_attempt",
      "query-service-query_service_ex-websocket-reconnect_exhausted",
      "query-service-query_service_ex-channel-joined",
      "query-service-query_service_ex-channel-left",
      "query-service-query_service_ex-circuit_breaker-opened",
      "query-service-query_service_ex-circuit_breaker-closed",
      "query-service-query_service_ex-circuit_breaker-half_open",
      "query-service-query_service_ex-circuit_breaker-rejected",
      "query-service-query_service_ex-health_check-timeout",
      "query-service-query_service_ex-health_check-completed",
      "query-service-query_service_ex-http-error",
      "query-service-query_service_ex-apm-event",
      "query-service-query_service_ex-apm-span_start",
      "query-service-query_service_ex-apm-span_end"
    ]

    for handler_id <- handlers do
      :telemetry.detach(handler_id)
    end

    :ok
  end

  # Phoenix HTTP request handler
  def handle_phoenix_stop(_event, measurements, metadata, _config) do
    duration_ms = System.convert_time_unit(measurements.duration, :native, :millisecond)

    level =
      if health_check_path?(metadata.conn.request_path), do: :debug, else: :info

    Logger.log(
      level,
      "HTTP request completed",
      method: metadata.conn.method,
      path: metadata.conn.request_path,
      status: metadata.conn.status,
      duration_ms: duration_ms,
      telemetry_event: "phoenix.endpoint.stop"
    )
  end

  defp health_check_path?("/api/health" <> _), do: true
  defp health_check_path?("/health" <> _), do: true
  defp health_check_path?(_), do: false

  # Ecto query handler
  def handle_ecto_query(_event, measurements, metadata, _config) do
    duration_ms = System.convert_time_unit(measurements.total_time || 0, :native, :millisecond)

    # Only log slow queries (> 100ms) at warning level, others at debug
    level = if duration_ms > 100, do: :warning, else: :debug

    Logger.log(
      level,
      "Database query executed",
      source: metadata[:source],
      duration_ms: duration_ms,
      queue_time_ms:
        measurements[:queue_time] &&
          System.convert_time_unit(measurements[:queue_time], :native, :millisecond),
      decode_time_ms:
        measurements[:decode_time] &&
          System.convert_time_unit(measurements[:decode_time], :native, :millisecond),
      telemetry_event: "ecto.query"
    )
  end

  # Event pipeline handlers
  def handle_event_processed(_event, measurements, metadata, _config) do
    Logger.info(
      "Event processed",
      event_id: metadata[:event_id],
      event_type: metadata[:event_type],
      entity_id: metadata[:entity_id],
      processing_time_ms: measurements[:duration_ms],
      telemetry_event: "event_pipeline.event_processed"
    )
  end

  def handle_projections_synced(_event, measurements, metadata, _config) do
    Logger.info(
      "Projections synced",
      projection_count: metadata[:projection_count],
      event_id: metadata[:event_id],
      sync_time_ms: measurements[:duration_ms],
      telemetry_event: "event_pipeline.projections_synced"
    )
  end

  def handle_events_failed(_event, _measurements, metadata, _config) do
    Logger.error(
      "Events processing failed",
      event_ids: metadata[:event_ids],
      error: inspect(metadata[:error]),
      telemetry_event: "event_pipeline.events_failed"
    )
  end

  # Projection sync handlers
  def handle_projection_sync_started(_event, _measurements, metadata, _config) do
    Logger.debug(
      "Projection sync started",
      projection_name: metadata[:projection_name],
      entity_id: metadata[:entity_id],
      telemetry_event: "projection.sync_started"
    )
  end

  def handle_projection_sync_completed(_event, measurements, metadata, _config) do
    Logger.info(
      "Projection sync completed",
      projection_name: metadata[:projection_name],
      entity_id: metadata[:entity_id],
      duration_ms: measurements[:duration_ms],
      telemetry_event: "projection.sync_completed"
    )
  end

  def handle_projection_sync_failed(_event, _measurements, metadata, _config) do
    Logger.error(
      "Projection sync failed",
      projection_name: metadata[:projection_name],
      entity_id: metadata[:entity_id],
      error: inspect(metadata[:error]),
      telemetry_event: "projection.sync_failed"
    )
  end

  # WebSocket client handlers
  def handle_websocket_connected(_event, _measurements, metadata, _config) do
    Logger.info(
      "WebSocket connected to Core",
      url: metadata[:url],
      telemetry_event: "websocket.connected"
    )
  end

  def handle_websocket_disconnected(_event, _measurements, metadata, _config) do
    Logger.warning(
      "WebSocket disconnected from Core",
      url: metadata[:url],
      reason: inspect(metadata[:reason]),
      telemetry_event: "websocket.disconnected"
    )
  end

  def handle_websocket_message(_event, measurements, metadata, _config) do
    Logger.debug(
      "WebSocket message received",
      message_type: metadata[:message_type],
      message_size_bytes: measurements[:size_bytes],
      telemetry_event: "websocket.message_received"
    )
  end

  def handle_websocket_reconnect_attempt(_event, measurements, metadata, _config) do
    Logger.info(
      "WebSocket reconnection attempt",
      url: metadata[:url],
      attempt: measurements[:attempt],
      backoff_ms: measurements[:backoff_ms],
      telemetry_event: "websocket.reconnect_attempt"
    )
  end

  def handle_websocket_reconnect_exhausted(_event, measurements, metadata, _config) do
    Logger.error(
      "WebSocket reconnection attempts exhausted",
      url: metadata[:url],
      attempts: measurements[:attempts],
      last_error: inspect(metadata[:last_error]),
      telemetry_event: "websocket.reconnect_exhausted"
    )
  end

  # Circuit breaker handlers
  def handle_circuit_opened(_event, measurements, metadata, _config) do
    Logger.warning(
      "Circuit breaker opened",
      from_state: metadata[:from_state],
      failure_count: measurements[:failure_count],
      telemetry_event: "circuit_breaker.opened"
    )
  end

  def handle_circuit_closed(_event, _measurements, _metadata, _config) do
    Logger.info(
      "Circuit breaker closed (recovered)",
      telemetry_event: "circuit_breaker.closed"
    )
  end

  def handle_circuit_half_open(_event, _measurements, metadata, _config) do
    Logger.info(
      "Circuit breaker half-open (testing recovery)",
      circuit: metadata[:circuit],
      telemetry_event: "circuit_breaker.half_open"
    )
  end

  def handle_circuit_rejected(_event, _measurements, metadata, _config) do
    Logger.warning(
      "Request rejected by circuit breaker",
      circuit: metadata[:circuit],
      telemetry_event: "circuit_breaker.rejected"
    )
  end

  # Health check handlers
  def handle_health_check_timeout(_event, measurements, metadata, _config) do
    Logger.warning(
      "Health check timed out",
      check: metadata[:check],
      timeout_ms: measurements[:timeout_ms],
      telemetry_event: "health_check.timeout"
    )
  end

  def handle_health_check_completed(_event, measurements, metadata, _config) do
    Logger.debug(
      "Health check completed",
      check: metadata[:check],
      status: metadata[:status],
      duration_ms: measurements[:duration_ms],
      telemetry_event: "health_check.completed"
    )
  end

  # HTTP error handler
  def handle_http_error(_event, _measurements, metadata, _config) do
    Logger.error(
      "HTTP error occurred",
      correlation_id: metadata[:correlation_id],
      error_type: metadata[:error_type],
      status: metadata[:status],
      telemetry_event: "http.error"
    )
  end

  # Phoenix Channel handlers (external client connections)
  def handle_channel_joined(_event, _measurements, metadata, _config) do
    Logger.info(
      "Client joined channel",
      channel: metadata[:channel],
      telemetry_event: "channel.joined"
    )
  end

  def handle_channel_left(_event, _measurements, metadata, _config) do
    Logger.info(
      "Client left channel",
      channel: metadata[:channel],
      telemetry_event: "channel.left"
    )
  end

  # APM event handlers
  def handle_apm_event(_event, _measurements, metadata, _config) do
    Logger.debug(
      "APM event tracked",
      event_name: metadata[:event_name],
      attributes: inspect(metadata[:attributes]),
      telemetry_event: "apm.event"
    )
  end

  def handle_apm_span_start(_event, _measurements, metadata, _config) do
    Logger.debug(
      "APM span started",
      span_name: metadata[:span_name],
      trace_id: metadata[:trace_id],
      span_id: metadata[:span_id],
      telemetry_event: "apm.span_start"
    )
  end

  def handle_apm_span_end(_event, measurements, metadata, _config) do
    duration_ms = System.convert_time_unit(measurements[:duration], :native, :millisecond)

    Logger.debug(
      "APM span ended",
      span_name: metadata[:span_name],
      trace_id: metadata[:trace_id],
      span_id: metadata[:span_id],
      status: metadata[:status],
      duration_ms: duration_ms,
      telemetry_event: "apm.span_end"
    )
  end
end
