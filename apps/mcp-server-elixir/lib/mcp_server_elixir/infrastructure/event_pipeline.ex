defmodule McpServerElixir.Infrastructure.EventPipeline do
  @moduledoc """
  Broadway pipeline for high-throughput event processing with automatic backpressure.

  This module implements a production-grade Broadway pipeline that:
  - Polls events from Core via CoreProducer
  - Processes events in parallel using configurable concurrency
  - Batches projection updates for efficient bulk saves
  - Handles failures gracefully with retry logic

  ## Architecture

  ```
  CoreProducer (GenStage)
       │
       ▼
  Processors (concurrency: schedulers * 2)
       │
       ▼
  Batchers (projection_updates)
       │
       ▼
  ProjectionSync (ETS + Core API)
  ```

  ## Performance Target
  - 10K events/second throughput
  - Sub-millisecond local reads via ETS
  - Efficient batch writes to Core

  ## Configuration

      config :mcp_server_elixir,
        event_pipeline_enabled: true,
        event_pipeline_processor_concurrency: :auto,  # schedulers * 2
        event_pipeline_batch_size: 100,
        event_pipeline_batch_timeout_ms: 50

  ## Example

      # Start pipeline (usually via supervision tree)
      {:ok, _pid} = EventPipeline.start_link()

      # Get pipeline statistics
      EventPipeline.stats()

      # Pause/resume for maintenance
      EventPipeline.pause()
      EventPipeline.resume()
  """

  use Broadway
  require Logger

  alias McpServerElixir.Infrastructure.ProjectionSync
  alias McpServerElixir.Infrastructure.CoreClient

  @default_batch_size 100
  @default_batch_timeout_ms 50

  # Client API

  @doc """
  Start the EventPipeline.

  ## Options
    * `:name` - Broadway name (default: __MODULE__)
    * `:enabled` - Whether to start the pipeline (default: true in prod)
    * `:processor_concurrency` - Number of concurrent processors (default: schedulers * 2)
    * `:batch_size` - Maximum events per batch (default: 100)
    * `:batch_timeout_ms` - Batch timeout in milliseconds (default: 50)
  """
  def start_link(opts \\ []) do
    enabled =
      Keyword.get(
        opts,
        :enabled,
        Application.get_env(:mcp_server_elixir, :event_pipeline_enabled, true)
      )

    if enabled do
      do_start_link(opts)
    else
      Logger.info("[EventPipeline] Disabled, not starting")
      :ignore
    end
  end

  defp do_start_link(opts) do
    name = Keyword.get(opts, :name, __MODULE__)

    processor_concurrency =
      Keyword.get(
        opts,
        :processor_concurrency,
        get_processor_concurrency()
      )

    batch_size =
      Keyword.get(
        opts,
        :batch_size,
        Application.get_env(:mcp_server_elixir, :event_pipeline_batch_size, @default_batch_size)
      )

    batch_timeout_ms =
      Keyword.get(
        opts,
        :batch_timeout_ms,
        Application.get_env(
          :mcp_server_elixir,
          :event_pipeline_batch_timeout_ms,
          @default_batch_timeout_ms
        )
      )

    producer_opts = Keyword.get(opts, :producer_opts, [])

    Broadway.start_link(__MODULE__,
      name: name,
      producer: [
        module: {McpServerElixir.Infrastructure.CoreProducer, producer_opts},
        concurrency: 1
      ],
      processors: [
        default: [
          concurrency: processor_concurrency,
          min_demand: 5,
          max_demand: 10
        ]
      ],
      batchers: [
        projection_updates: [
          concurrency: max(1, div(processor_concurrency, 2)),
          batch_size: batch_size,
          batch_timeout: batch_timeout_ms
        ]
      ],
      context: %{
        core_client: CoreClient.new()
      }
    )
  end

  defp get_processor_concurrency do
    case Application.get_env(:mcp_server_elixir, :event_pipeline_processor_concurrency) do
      :auto -> System.schedulers_online() * 2
      nil -> System.schedulers_online() * 2
      n when is_integer(n) and n > 0 -> n
    end
  end

  @doc """
  Get pipeline statistics.
  """
  def stats(pipeline \\ __MODULE__) do
    try do
      info = Broadway.producer_names(pipeline)

      %{
        producer_names: info,
        processor_concurrency: get_processor_concurrency(),
        batch_size:
          Application.get_env(:mcp_server_elixir, :event_pipeline_batch_size, @default_batch_size),
        started: true
      }
    catch
      :exit, _ ->
        %{started: false, error: "Pipeline not running"}
    end
  end

  # Broadway Callbacks

  @impl Broadway
  def handle_message(_processor, message, _context) do
    event = message.data
    start_time = System.monotonic_time(:microsecond)

    case process_event(event) do
      {:ok, projection_updates} ->
        duration_us = System.monotonic_time(:microsecond) - start_time

        :telemetry.execute(
          [:mcp_server_elixir, :event_pipeline, :message_processed],
          %{duration_us: duration_us, updates_count: length(projection_updates)},
          %{event_type: event["event_type"]}
        )

        # Route to projection updates batcher
        message
        |> Broadway.Message.put_batcher(:projection_updates)
        |> Broadway.Message.put_data(%{event: event, updates: projection_updates})

      {:error, reason} ->
        Logger.warning("[EventPipeline] Failed to process event #{event["id"]}: #{inspect(reason)}")

        :telemetry.execute(
          [:mcp_server_elixir, :event_pipeline, :message_failed],
          %{},
          %{event_type: event["event_type"], error: reason}
        )

        Broadway.Message.failed(message, reason)
    end
  end

  @impl Broadway
  def handle_batch(:projection_updates, messages, _batch_info, context) do
    start_time = System.monotonic_time(:microsecond)

    # Collect all projection updates from messages
    updates =
      messages
      |> Enum.flat_map(fn msg -> msg.data[:updates] || [] end)
      |> Enum.group_by(fn {projection_name, _entity_id, _state} -> projection_name end)

    # Save updates to local ETS via ProjectionSync
    ets_results = save_to_local_ets(updates)

    # Bulk save to Core (fire-and-forget for performance, ProjectionSync handles sync)
    Task.start(fn ->
      bulk_save_to_core(updates, context.core_client)
    end)

    duration_us = System.monotonic_time(:microsecond) - start_time
    total_updates = Enum.reduce(updates, 0, fn {_, list}, acc -> acc + length(list) end)

    :telemetry.execute(
      [:mcp_server_elixir, :event_pipeline, :batch_processed],
      %{
        duration_us: duration_us,
        messages_count: length(messages),
        updates_count: total_updates
      },
      %{}
    )

    Logger.debug(
      "[EventPipeline] Batch processed: #{length(messages)} messages, #{total_updates} updates, #{ets_results.saved} saved to ETS"
    )

    messages
  end

  @impl Broadway
  def handle_failed(messages, _context) do
    # Log at debug level since failures are tracked via telemetry
    # In production, monitoring should alert on telemetry metrics, not log spam
    Logger.debug("[EventPipeline] #{length(messages)} messages failed")

    :telemetry.execute(
      [:mcp_server_elixir, :event_pipeline, :messages_failed],
      %{count: length(messages)},
      %{}
    )

    messages
  end

  # Event Processing Logic

  defp process_event(event) do
    entity_id = event["entity_id"]
    event_type = event["event_type"]

    updates = []

    # Update entity_snapshots projection
    updates =
      if entity_id do
        snapshot_state = build_entity_snapshot(event)
        [{"entity_snapshots", entity_id, snapshot_state} | updates]
      else
        updates
      end

    # Update event_counters projection
    updates =
      if event_type do
        # Get current count and increment
        current_count = get_event_count(event_type)
        counter_state = %{"count" => current_count + 1, "last_event_at" => event["timestamp"]}
        [{"event_counters", event_type, counter_state} | updates]
      else
        updates
      end

    {:ok, updates}
  rescue
    e ->
      {:error, Exception.message(e)}
  end

  defp build_entity_snapshot(event) do
    # Get existing state and merge with new payload
    existing = ProjectionSync.get_state("entity_snapshots", event["entity_id"]) || %{}
    payload = event["payload"] || %{}

    Map.merge(existing, payload)
    |> Map.put("_last_event_id", event["id"])
    |> Map.put("_last_event_type", event["event_type"])
    |> Map.put("_last_updated_at", event["timestamp"])
  end

  defp get_event_count(event_type) do
    case ProjectionSync.get_state("event_counters", event_type) do
      nil -> 0
      %{"count" => count} -> count
      _ -> 0
    end
  rescue
    _ -> 0
  end

  # Projection Storage

  defp save_to_local_ets(updates_by_projection) do
    saved =
      Enum.reduce(updates_by_projection, 0, fn {projection_name, updates}, acc ->
        Enum.each(updates, fn {_proj, entity_id, state} ->
          ProjectionSync.put_state(projection_name, entity_id, state)
        end)

        acc + length(updates)
      end)

    %{saved: saved}
  rescue
    e ->
      Logger.warning("[EventPipeline] Failed to save to ETS: #{Exception.message(e)}")
      %{saved: 0, error: e}
  end

  defp bulk_save_to_core(updates_by_projection, _core_client) do
    base_url = Application.get_env(:mcp_server_elixir, :core_url, "http://localhost:3900")

    Enum.each(updates_by_projection, fn {projection_name, updates} ->
      states =
        Enum.map(updates, fn {_proj, entity_id, state} ->
          %{entity_id: entity_id, state: state}
        end)

      url = "#{base_url}/api/v1/projections/#{projection_name}/bulk/save"
      body = Jason.encode!(%{states: states})

      case Tesla.post(url, body, headers: [{"content-type", "application/json"}]) do
        {:ok, %Tesla.Env{status: status}} when status in [200, 201] ->
          :ok

        {:ok, %Tesla.Env{status: status, body: response_body}} ->
          Logger.warning(
            "[EventPipeline] Bulk save failed for #{projection_name}: HTTP #{status} - #{inspect(response_body)}"
          )

        {:error, reason} ->
          Logger.warning(
            "[EventPipeline] Bulk save failed for #{projection_name}: #{inspect(reason)}"
          )
      end
    end)
  rescue
    e ->
      Logger.warning("[EventPipeline] Bulk save to Core failed: #{Exception.message(e)}")
  end
end
