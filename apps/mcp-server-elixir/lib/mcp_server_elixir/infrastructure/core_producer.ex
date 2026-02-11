defmodule McpServerElixir.Infrastructure.CoreProducer do
  @moduledoc """
  GenStage producer that polls events from Core with cursor-based tracking.

  This producer fetches events from the Rust Core API and emits them downstream
  to Broadway processors. It maintains cursor position for restart recovery and
  implements automatic backpressure through GenStage demand.

  ## Features
  - Cursor-based event polling with persistent storage
  - Automatic backpressure via GenStage demand
  - Configurable poll interval and batch size
  - Graceful handling of Core unavailability
  - Telemetry instrumentation

  ## Configuration

      config :mcp_server_elixir,
        core_url: "http://localhost:3900",
        event_poll_interval_ms: 100,
        event_batch_size: 1000

  ## Example

      # Start producer (usually done via Broadway)
      {:ok, producer} = CoreProducer.start_link(name: MyProducer)

      # Get cursor position
      CoreProducer.get_cursor(MyProducer)

      # Reset cursor (e.g., for replay)
      CoreProducer.reset_cursor(MyProducer)
  """

  use GenStage
  require Logger

  alias McpServerElixir.Infrastructure.CoreClient

  @default_poll_interval_ms 100
  @default_batch_size 1000
  @cursor_persistence_table :core_producer_cursor
  @cursor_key :event_cursor

  # Client API

  @doc """
  Start the CoreProducer.

  ## Options
    * `:name` - Process name (default: __MODULE__)
    * `:poll_interval_ms` - Polling interval in milliseconds (default: 100)
    * `:batch_size` - Maximum events per poll (default: 1000)
    * `:cursor` - Initial cursor position (default: nil for beginning)
    * `:broadway` - Broadway module reference (for ack callbacks)
  """
  def start_link(opts \\ []) do
    name = Keyword.get(opts, :name, __MODULE__)
    GenStage.start_link(__MODULE__, opts, name: name)
  end

  @doc """
  Get the current cursor position.
  """
  def get_cursor(producer \\ __MODULE__) do
    GenStage.call(producer, :get_cursor)
  end

  @doc """
  Reset cursor to a specific position or the beginning.

  Returns `:ok` on success.
  """
  def reset_cursor(producer \\ __MODULE__, cursor \\ nil) do
    GenStage.call(producer, {:reset_cursor, cursor})
  end

  @doc """
  Get producer statistics.
  """
  def stats(producer \\ __MODULE__) do
    GenStage.call(producer, :stats)
  end

  @doc """
  Pause event production (for graceful shutdown or maintenance).
  """
  def pause(producer \\ __MODULE__) do
    GenStage.call(producer, :pause)
  end

  @doc """
  Resume event production.
  """
  def resume(producer \\ __MODULE__) do
    GenStage.call(producer, :resume)
  end

  # GenStage Callbacks

  @impl true
  def init(opts) do
    poll_interval =
      opts[:poll_interval_ms] ||
        Application.get_env(
          :mcp_server_elixir,
          :event_poll_interval_ms,
          @default_poll_interval_ms
        )

    batch_size =
      opts[:batch_size] ||
        Application.get_env(:mcp_server_elixir, :event_batch_size, @default_batch_size)

    # Initialize cursor persistence table
    create_cursor_table()

    # Load cursor from persistence or use provided cursor
    initial_cursor = opts[:cursor] || load_persisted_cursor()

    state = %{
      poll_interval_ms: poll_interval,
      batch_size: batch_size,
      cursor: initial_cursor,
      core_client: CoreClient.new(),
      pending_demand: 0,
      events_produced: 0,
      poll_count: 0,
      last_poll_at: nil,
      last_error: nil,
      core_available: true,
      paused: false,
      started_at: DateTime.utc_now()
    }

    Logger.info(
      "[CoreProducer] Started with #{poll_interval}ms interval, batch_size=#{batch_size}, cursor=#{inspect(initial_cursor)}"
    )

    # Schedule first poll
    schedule_poll(poll_interval)

    {:producer, state}
  end

  @impl true
  def handle_demand(demand, state) when demand > 0 do
    new_demand = state.pending_demand + demand

    new_state = %{state | pending_demand: new_demand}

    # If we have pending demand and not paused, trigger immediate poll
    if new_demand > 0 and not state.paused do
      send(self(), :poll)
    end

    {:noreply, [], new_state}
  end

  @impl true
  def handle_call(:get_cursor, _from, state) do
    {:reply, state.cursor, [], state}
  end

  @impl true
  def handle_call({:reset_cursor, cursor}, _from, state) do
    persist_cursor(cursor)
    new_state = %{state | cursor: cursor}
    Logger.info("[CoreProducer] Cursor reset to: #{inspect(cursor)}")
    {:reply, :ok, [], new_state}
  end

  @impl true
  def handle_call(:stats, _from, state) do
    stats = %{
      cursor: state.cursor,
      pending_demand: state.pending_demand,
      events_produced: state.events_produced,
      poll_count: state.poll_count,
      last_poll_at: state.last_poll_at,
      last_error: state.last_error,
      core_available: state.core_available,
      paused: state.paused,
      started_at: state.started_at,
      poll_interval_ms: state.poll_interval_ms,
      batch_size: state.batch_size
    }

    {:reply, stats, [], state}
  end

  @impl true
  def handle_call(:pause, _from, state) do
    Logger.info("[CoreProducer] Paused")
    {:reply, :ok, [], %{state | paused: true}}
  end

  @impl true
  def handle_call(:resume, _from, state) do
    Logger.info("[CoreProducer] Resumed")
    new_state = %{state | paused: false}

    if state.pending_demand > 0 do
      send(self(), :poll)
    end

    {:reply, :ok, [], new_state}
  end

  @impl true
  def handle_info(:poll, %{paused: true} = state) do
    schedule_poll(state.poll_interval_ms)
    {:noreply, [], state}
  end

  @impl true
  def handle_info(:poll, %{pending_demand: 0} = state) do
    schedule_poll(state.poll_interval_ms)
    {:noreply, [], state}
  end

  @impl true
  def handle_info(:poll, state) do
    {events, new_state} = poll_events(state)

    events_count = length(events)

    if events_count > 0 do
      Logger.debug("[CoreProducer] Emitting #{events_count} events, cursor=#{new_state.cursor}")
    end

    # Update pending demand
    new_state = %{new_state | pending_demand: max(0, new_state.pending_demand - events_count)}

    # Schedule next poll
    schedule_poll(state.poll_interval_ms)

    {:noreply, events, new_state}
  end

  @impl true
  def handle_info(_msg, state) do
    {:noreply, [], state}
  end

  # Private Functions

  defp schedule_poll(interval_ms) do
    Process.send_after(self(), :poll, interval_ms)
  end

  defp poll_events(state) do
    start_time = System.monotonic_time(:microsecond)
    limit = min(state.batch_size, state.pending_demand)

    query_params = build_query_params(state.cursor, limit)

    case CoreClient.query_events(state.core_client, query_params) do
      {:ok, response} ->
        events = response["events"] || []
        events_count = length(events)

        new_cursor = calculate_new_cursor(events, state.cursor)

        # Persist cursor periodically (every 10 polls or when cursor changes)
        if new_cursor != state.cursor do
          persist_cursor(new_cursor)
        end

        # Emit telemetry
        duration_us = System.monotonic_time(:microsecond) - start_time

        :telemetry.execute(
          [:mcp_server_elixir, :core_producer, :poll_completed],
          %{
            events_count: events_count,
            duration_us: duration_us,
            batch_size: limit
          },
          %{cursor: new_cursor}
        )

        # Log when Core becomes available again
        if not state.core_available do
          Logger.info("[CoreProducer] Core is now available")
        end

        # Convert events to Broadway messages
        messages = events_to_messages(events)

        new_state = %{
          state
          | cursor: new_cursor,
            events_produced: state.events_produced + events_count,
            poll_count: state.poll_count + 1,
            last_poll_at: DateTime.utc_now(),
            core_available: true,
            last_error: nil
        }

        {messages, new_state}

      {:error, reason} ->
        # Only log when transitioning from available to unavailable
        if state.core_available do
          Logger.warning("[CoreProducer] Core became unavailable: #{inspect(reason)}")
        end

        :telemetry.execute(
          [:mcp_server_elixir, :core_producer, :poll_failed],
          %{},
          %{error: reason}
        )

        new_state = %{
          state
          | poll_count: state.poll_count + 1,
            last_poll_at: DateTime.utc_now(),
            core_available: false,
            last_error: reason
        }

        {[], new_state}
    end
  end

  defp build_query_params(cursor, limit) do
    params = %{limit: limit}

    # Add cursor-based filtering
    # Assuming cursor is an event ID or timestamp
    if cursor do
      Map.put(params, :after_id, cursor)
    else
      params
    end
  end

  defp calculate_new_cursor([], current_cursor), do: current_cursor

  defp calculate_new_cursor(events, _current_cursor) do
    # Use the last event's ID as the new cursor
    last_event = List.last(events)
    last_event["id"]
  end

  defp events_to_messages(events) do
    Enum.map(events, fn event ->
      %Broadway.Message{
        data: event,
        acknowledger: {__MODULE__, :ack_id, event["id"]}
      }
    end)
  end

  # Broadway acknowledger callback
  def ack(:ack_id, _successful, _failed) do
    # For now, acknowledgment is a no-op since we track cursor position
    # This could be extended for at-least-once delivery guarantees
    :ok
  end

  # Cursor persistence functions

  defp create_cursor_table do
    :ets.new(@cursor_persistence_table, [:named_table, :set, :public])
  rescue
    ArgumentError ->
      # Table already exists
      :ok
  end

  defp persist_cursor(cursor) do
    :ets.insert(@cursor_persistence_table, {@cursor_key, cursor})
    :ok
  rescue
    ArgumentError -> :ok
  end

  defp load_persisted_cursor do
    case :ets.lookup(@cursor_persistence_table, @cursor_key) do
      [{@cursor_key, cursor}] -> cursor
      [] -> nil
    end
  rescue
    ArgumentError -> nil
  end
end
