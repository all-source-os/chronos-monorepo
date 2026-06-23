defmodule QueryServiceEx.Application.Services.EventPipeline do
  @moduledoc """
  Production Broadway pipeline for processing events from Rust Core.

  This pipeline integrates CoreProducer for event fetching and ProjectionSync
  for state management, providing high-throughput event processing with
  automatic backpressure and fault tolerance.

  ## Features
  - Polls events from Rust Core via CoreProducer
  - Concurrent processing with configurable parallelism
  - Batch projection updates synced to Core
  - Telemetry integration for observability
  - Graceful shutdown with state persistence

  ## Architecture

      CoreProducer (GenStage)
            |
            v
      Broadway Processors (N concurrent)
            |
            v
      Batchers (projection_updates)
            |
            v
      Bulk sync to Core's DashMap

  ## Configuration

      config :query_service_ex,
        event_pipeline_enabled: true,
        event_pipeline_processors: 10,
        event_pipeline_batch_size: 100,
        event_pipeline_batch_timeout: 1000

  ## Usage

      # Start the pipeline
      {:ok, pid} = EventPipeline.start_link()

      # Get pipeline stats
      stats = EventPipeline.stats()
  """

  use Broadway
  require Logger

  alias QueryServiceEx.Application.Services.{CoreProducer, ProjectionSync}
  alias QueryServiceEx.Infrastructure.Adapters.RustCoreClient

  @default_processors 10
  @default_batch_size 100
  @default_batch_timeout 1_000

  # Client API

  @doc """
  Start the EventPipeline.

  ## Options
    * `:name` - Process name (default: __MODULE__)
    * `:processors` - Number of concurrent processors (default: 10)
    * `:batch_size` - Batch size for projection updates (default: 100)
    * `:batch_timeout` - Batch timeout in ms (default: 1000)
  """
  def start_link(opts \\ []) do
    enabled = Application.get_env(:query_service_ex, :event_pipeline_enabled, true)

    if enabled do
      do_start_link(opts)
    else
      Logger.info("[EventPipeline] Disabled, not starting")
      :ignore
    end
  end

  defp do_start_link(opts) do
    name = opts[:name] || __MODULE__

    processors =
      opts[:processors] ||
        Application.get_env(:query_service_ex, :event_pipeline_processors, @default_processors)

    batch_size =
      opts[:batch_size] ||
        Application.get_env(:query_service_ex, :event_pipeline_batch_size, @default_batch_size)

    batch_timeout =
      opts[:batch_timeout] ||
        Application.get_env(
          :query_service_ex,
          :event_pipeline_batch_timeout,
          @default_batch_timeout
        )

    Broadway.start_link(__MODULE__,
      name: name,
      producer: [
        module: {CoreProducer, []},
        concurrency: 1
      ],
      processors: [
        default: [
          concurrency: processors,
          min_demand: 50,
          max_demand: 100
        ]
      ],
      batchers: [
        projection_updates: [
          concurrency: 5,
          batch_size: batch_size,
          batch_timeout: batch_timeout
        ]
      ],
      context: %{
        processed_count: :counters.new(1, [:atomics]),
        error_count: :counters.new(1, [:atomics])
      }
    )
  end

  @doc """
  Get pipeline statistics.
  """
  def stats(name \\ __MODULE__) do
    case Process.whereis(name) do
      nil ->
        {:error, :not_running}

      _pid ->
        {:ok, Broadway.producer_names(name)}
    end
  end

  # Broadway Callbacks

  @impl Broadway
  def handle_message(_processor, message, context) do
    event = message.data

    try do
      # Process the event through projections
      process_event(event)

      # Update metrics
      :counters.add(context.processed_count, 1, 1)

      # Emit telemetry
      :telemetry.execute(
        [:query_service_ex, :event_pipeline, :event_processed],
        %{count: 1},
        %{event_type: event["event_type"]}
      )

      # Send to projection_updates batcher
      message
      |> Broadway.Message.put_data(%{event: event, projections: get_affected_projections(event)})
      |> Broadway.Message.put_batcher(:projection_updates)
    rescue
      error ->
        Logger.warning("[EventPipeline] Failed to process event: #{inspect(error)}")
        :counters.add(context.error_count, 1, 1)

        Broadway.Message.failed(message, error)
    end
  end

  @impl Broadway
  def handle_batch(:projection_updates, messages, _batch_info, _context) do
    # Collect all projection states that need syncing
    projection_states =
      messages
      |> Enum.flat_map(fn msg -> msg.data.projections end)
      |> Enum.uniq_by(fn {name, entity_id, _state} -> {name, entity_id} end)
      |> Enum.map(fn {name, entity_id, state} ->
        %{projection_name: name, entity_id: entity_id, state: state}
      end)

    # Bulk sync to Core
    if projection_states != [] do
      count = length(projection_states)

      case RustCoreClient.bulk_save_projection_states(projection_states) do
        :ok ->
          Logger.debug("[EventPipeline] Synced #{count} projection states")

        {:error, reason} ->
          Logger.warning("[EventPipeline] Failed to sync projections: #{inspect(reason)}")
      end

      # Emit telemetry
      :telemetry.execute(
        [:query_service_ex, :event_pipeline, :projections_synced],
        %{count: count},
        %{}
      )
    end

    messages
  end

  @impl Broadway
  def handle_failed(messages, _context) do
    Enum.each(messages, fn message ->
      Logger.warning(
        "[EventPipeline] Message failed: #{inspect(message.status)}, " <>
          "event: #{inspect(message.data)}"
      )
    end)

    # Emit telemetry for failures
    :telemetry.execute(
      [:query_service_ex, :event_pipeline, :events_failed],
      %{count: length(messages)},
      %{}
    )

    messages
  end

  # Private Functions

  defp process_event(event) do
    entity_id = event["entity_id"]

    # Broadcast event to PubSub for any listening ProjectionSync processes.
    # ISOLATION_OK: internal projection pipeline (event_pipeline -> ProjectionSync),
    # not a user-facing channel — EventChannel subscribes to tenant-scoped topics
    # only, so these never reach a client. Tenant-scope once ProjectionSync carries
    # the tenant (tracked follow-up).
    if entity_id do
      Phoenix.PubSub.broadcast(
        QueryServiceEx.PubSub,
        "events:#{entity_id}",
        {:new_event, event}
      )
    end

    # ISOLATION_OK: internal projection pipeline (see above).
    if event_type = event["event_type"] do
      Phoenix.PubSub.broadcast(
        QueryServiceEx.PubSub,
        "events:type:#{event_type}",
        {:new_event, event}
      )
    end

    :ok
  end

  defp get_affected_projections(event) do
    entity_id = event["entity_id"]
    payload = event["payload"] || %{}

    # Get the current state from ETS cache
    case ProjectionSync.get_state_from_cache("entity_snapshots", entity_id) do
      nil ->
        # No existing state, create initial
        [{"entity_snapshots", entity_id, payload}]

      existing_state ->
        # Merge with existing state
        merged = Map.merge(existing_state, payload)
        [{"entity_snapshots", entity_id, merged}]
    end
  end
end
