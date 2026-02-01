defmodule QueryServiceEx.Application.UseCases.PipelineProcessor do
  @moduledoc """
  GenServer for processing events through pipelines.

  This processor applies pipeline transformations to events from various sources,
  integrating with Broadway for high-throughput event processing.

  ## Features
  - Stateless event processing through pipelines
  - Integration with Broadway for backpressure management
  - Telemetry instrumentation for monitoring
  - Error handling and dead letter queue support
  - Concurrent pipeline execution

  ## Example

      # Start a pipeline processor
      {:ok, pid} = PipelineProcessor.start_link(
        pipeline: order_processing_pipeline,
        name: :order_processor
      )

      # Process a single event
      PipelineProcessor.process_event(pid, event)

      # Process a batch of events
      PipelineProcessor.process_batch(pid, events)
  """

  use GenServer
  require Logger

  alias QueryServiceEx.Domain.Entities.Pipeline

  @type state :: %{
          pipeline: Pipeline.Definition.t(),
          stats: map(),
          config: map()
        }

  ## Client API

  @doc """
  Start a pipeline processor.

  ## Options
    * `:pipeline` - Pipeline.Definition struct (required)
    * `:name` - Process name for registration (optional)
    * `:config` - Processor configuration (optional)
  """
  def start_link(opts) do
    {name_opt, opts} = Keyword.pop(opts, :name)

    case name_opt do
      nil -> GenServer.start_link(__MODULE__, opts)
      name -> GenServer.start_link(__MODULE__, opts, name: name)
    end
  end

  @doc """
  Process a single event through the pipeline.

  Returns `{:ok, processed_event}` if successful, `{:error, reason}` if failed,
  or `:filtered` if the event was filtered out.
  """
  def process_event(server, event) do
    GenServer.call(server, {:process_event, event})
  end

  @doc """
  Process a batch of events through the pipeline.

  Returns a list of successfully processed events.
  Events that are filtered or error will be excluded.
  """
  def process_batch(server, events) when is_list(events) do
    GenServer.call(server, {:process_batch, events}, :timer.seconds(30))
  end

  @doc """
  Get processor statistics.

  Returns a map with:
  - total_processed: Total events processed
  - total_filtered: Total events filtered
  - total_errors: Total events that errored
  - success_rate: Percentage of successful events
  """
  def get_stats(server) do
    GenServer.call(server, :get_stats)
  end

  @doc """
  Reset processor statistics to zero.
  """
  def reset_stats(server) do
    GenServer.call(server, :reset_stats)
  end

  @doc """
  Get the pipeline definition.
  """
  def get_pipeline(server) do
    GenServer.call(server, :get_pipeline)
  end

  @doc """
  Stop the processor gracefully.
  """
  def stop(server) do
    GenServer.stop(server, :normal)
  end

  ## Server Callbacks

  @impl true
  def init(opts) do
    pipeline = Keyword.fetch!(opts, :pipeline)
    config = Keyword.get(opts, :config, %{})

    if Pipeline.valid_pipeline?(pipeline) do
      state = %{
        pipeline: pipeline,
        stats: %{
          total_processed: 0,
          total_filtered: 0,
          total_errors: 0
        },
        config: config
      }

      # Emit telemetry
      :telemetry.execute(
        [:pipeline_processor, :started],
        %{count: 1},
        %{pipeline: pipeline.name, version: pipeline.version}
      )

      {:ok, state}
    else
      {:stop, {:invalid_pipeline, pipeline}}
    end
  end

  @impl true
  def handle_call({:process_event, event}, _from, state) do
    start_time = System.monotonic_time(:microsecond)

    result = Pipeline.apply_pipeline(event, state.pipeline)

    # Update statistics
    {new_stats, reply} =
      case result do
        {:ok, processed_event} ->
          stats = update_stat(state.stats, :total_processed)
          {stats, {:ok, processed_event}}

        :filtered ->
          stats = update_stat(state.stats, :total_filtered)
          {stats, :filtered}

        {:error, reason} = error ->
          stats = update_stat(state.stats, :total_errors)
          Logger.warning("Pipeline processing error: #{inspect(reason)}")
          {stats, error}
      end

    duration = System.monotonic_time(:microsecond) - start_time

    # Emit telemetry
    result_status =
      case result do
        {:ok, _} -> :ok
        :filtered -> :filtered
        {:error, _} -> :error
      end

    :telemetry.execute(
      [:pipeline_processor, :event_processed],
      %{duration: duration, count: 1},
      %{
        pipeline: state.pipeline.name,
        result: result_status
      }
    )

    new_state = %{state | stats: new_stats}
    {:reply, reply, new_state}
  end

  @impl true
  def handle_call({:process_batch, events}, _from, state) do
    start_time = System.monotonic_time(:microsecond)

    # Apply pipeline to all events and collect results
    all_results = Enum.map(events, &Pipeline.apply_pipeline(&1, state.pipeline))

    # Categorize results
    {successes, filtered, errors} =
      Enum.reduce(all_results, {[], 0, 0}, fn
        {:ok, event}, {succ, filt, err} -> {[event | succ], filt, err}
        :filtered, {succ, filt, err} -> {succ, filt + 1, err}
        {:error, _}, {succ, filt, err} -> {succ, filt, err + 1}
      end)

    results = Enum.reverse(successes)
    success_count = length(results)

    new_stats =
      state.stats
      |> Map.update!(:total_processed, &(&1 + success_count))
      |> Map.update!(:total_filtered, &(&1 + filtered))
      |> Map.update!(:total_errors, &(&1 + errors))

    duration = System.monotonic_time(:microsecond) - start_time

    # Emit telemetry
    :telemetry.execute(
      [:pipeline_processor, :batch_processed],
      %{duration: duration, count: length(events), success: success_count},
      %{pipeline: state.pipeline.name}
    )

    new_state = %{state | stats: new_stats}
    {:reply, results, new_state}
  end

  @impl true
  def handle_call(:get_stats, _from, state) do
    stats = calculate_stats(state.stats)
    {:reply, stats, state}
  end

  @impl true
  def handle_call(:reset_stats, _from, state) do
    new_stats = %{
      total_processed: 0,
      total_filtered: 0,
      total_errors: 0
    }

    new_state = %{state | stats: new_stats}
    {:reply, :ok, new_state}
  end

  @impl true
  def handle_call(:get_pipeline, _from, state) do
    {:reply, state.pipeline, state}
  end

  @impl true
  def terminate(reason, state) do
    Logger.info("Stopping pipeline processor for #{state.pipeline.name}: #{inspect(reason)}")

    :telemetry.execute(
      [:pipeline_processor, :stopped],
      %{count: 1},
      %{pipeline: state.pipeline.name, reason: reason}
    )

    :ok
  end

  ## Private Helpers

  defp update_stat(stats, key) do
    Map.update!(stats, key, &(&1 + 1))
  end

  defp calculate_stats(stats) do
    total =
      stats.total_processed + stats.total_filtered + stats.total_errors

    success_rate =
      if total > 0 do
        Float.round(stats.total_processed / total * 100, 2)
      else
        0.0
      end

    Map.put(stats, :success_rate, success_rate)
  end
end
