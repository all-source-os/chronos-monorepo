defmodule QueryServiceEx.Logging do
  @moduledoc """
  Structured logging utilities for the Query Service.

  Provides consistent logging patterns with structured metadata for
  production observability and debugging.

  ## Usage

      require QueryServiceEx.Logging
      alias QueryServiceEx.Logging

      # Log with structured metadata
      Logging.info("Event processed",
        event_id: event.id,
        event_type: event.type,
        entity_id: event.entity_id
      )

      # Log with automatic module/function context
      Logging.debug("Starting operation", context: :startup)

  ## Metadata Keys

  Standard metadata keys used across the service:
  - `:request_id` - HTTP request identifier
  - `:correlation_id` - Distributed tracing ID
  - `:entity_id` - Entity being operated on
  - `:event_type` - Type of event being processed
  - `:event_id` - Unique event identifier
  - `:projection_name` - Name of projection
  - `:duration_ms` - Operation duration in milliseconds
  """

  require Logger

  @doc """
  Emits a telemetry event with the given name and metadata.
  Use this for metrics and observability.
  """
  def emit_telemetry(event_name, measurements \\ %{}, metadata \\ %{}) when is_list(event_name) do
    :telemetry.execute(event_name, measurements, metadata)
  end

  @doc """
  Wraps a function and measures its execution time, emitting a telemetry event.
  Returns the result of the function.

  ## Example

      Logging.measure([:query_service_ex, :query, :execute], %{query_type: "dsl"}, fn ->
        execute_query(query)
      end)
  """
  def measure(event_name, metadata \\ %{}, fun) when is_function(fun, 0) do
    start_time = System.monotonic_time()
    result = fun.()
    duration = System.monotonic_time() - start_time
    duration_ms = System.convert_time_unit(duration, :native, :millisecond)

    emit_telemetry(event_name, %{duration_ms: duration_ms}, metadata)

    result
  end

  @doc """
  Sets logger metadata for the current process.
  Useful for setting context at the start of an operation.
  """
  def set_context(metadata) when is_list(metadata) do
    Logger.metadata(metadata)
  end

  @doc """
  Clears logger metadata for the current process.
  """
  def clear_context do
    Logger.metadata([])
  end

  @doc """
  Creates a child logger context with additional metadata.
  Returns a function that restores the original context.
  """
  def with_context(metadata, fun) when is_list(metadata) and is_function(fun, 0) do
    original = Logger.metadata()
    Logger.metadata(metadata)

    try do
      fun.()
    after
      Logger.metadata(original)
    end
  end
end
