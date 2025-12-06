defmodule QueryServiceEx.Application.Services.CoreProducer do
  @moduledoc """
  GenStage producer that polls events from Rust Core for Broadway consumption.

  This producer fetches events from Core's HTTP API with cursor-based pagination,
  providing reliable event delivery with backpressure management.

  ## Features
  - Cursor-based polling with persistent cursor tracking
  - Configurable batch sizes and poll intervals
  - Automatic backpressure (respects demand)
  - ETS-based cursor persistence
  - Metrics via Telemetry

  ## Configuration

      config :query_service_ex,
        core_producer_poll_interval_ms: 100,
        core_producer_batch_size: 1000,
        core_producer_enabled: true

  ## Usage

      # Start as part of a Broadway pipeline
      defmodule MyPipeline do
        use Broadway

        def start_link(_opts) do
          Broadway.start_link(__MODULE__,
            name: __MODULE__,
            producer: [
              module: {QueryServiceEx.Application.Services.CoreProducer, []},
              concurrency: 1
            ],
            processors: [default: [concurrency: 10]]
          )
        end
      end
  """

  use GenStage
  require Logger

  alias QueryServiceEx.Infrastructure.Adapters.RustCoreClient

  @default_poll_interval_ms 100
  @default_batch_size 1000
  @cursor_table :core_producer_cursors
  @cursor_key :event_cursor

  # Client API

  @doc """
  Start the CoreProducer.

  ## Options
    * `:poll_interval_ms` - Polling interval in milliseconds (default: 100)
    * `:batch_size` - Maximum events per poll (default: 1000)
    * `:cursor` - Initial cursor timestamp (default: load from ETS or epoch)
    * `:name` - Process name
  """
  def start_link(opts \\ []) do
    GenStage.start_link(__MODULE__, opts, name: opts[:name])
  end

  @doc """
  Get the current cursor position.
  """
  def get_cursor(pid) do
    GenStage.call(pid, :get_cursor)
  end

  @doc """
  Reset cursor to a specific timestamp.
  """
  def reset_cursor(pid, timestamp) do
    GenStage.call(pid, {:reset_cursor, timestamp})
  end

  @doc """
  Get producer statistics.
  """
  def stats(pid) do
    GenStage.call(pid, :stats)
  end

  # GenStage Callbacks

  @impl GenStage
  def init(opts) do
    poll_interval = opts[:poll_interval_ms] ||
      Application.get_env(:query_service_ex, :core_producer_poll_interval_ms, @default_poll_interval_ms)

    batch_size = opts[:batch_size] ||
      Application.get_env(:query_service_ex, :core_producer_batch_size, @default_batch_size)

    # Initialize cursor table
    init_cursor_table()

    # Load or initialize cursor
    cursor = opts[:cursor] || load_cursor()

    state = %{
      poll_interval: poll_interval,
      batch_size: batch_size,
      cursor: cursor,
      demand: 0,
      events_fetched: 0,
      polls_count: 0,
      last_poll_at: nil,
      last_error: nil
    }

    # Schedule first poll
    schedule_poll(poll_interval)

    Logger.info("[CoreProducer] Started with cursor: #{cursor}, batch_size: #{batch_size}")

    {:producer, state}
  end

  @impl GenStage
  def handle_demand(new_demand, state) do
    new_state = %{state | demand: state.demand + new_demand}

    # If we have pending demand, try to fetch immediately
    if new_state.demand > 0 do
      fetch_and_dispatch(new_state)
    else
      {:noreply, [], new_state}
    end
  end

  @impl GenStage
  def handle_call(:get_cursor, _from, state) do
    {:reply, state.cursor, [], state}
  end

  @impl GenStage
  def handle_call({:reset_cursor, timestamp}, _from, state) do
    save_cursor(timestamp)
    {:reply, :ok, [], %{state | cursor: timestamp}}
  end

  @impl GenStage
  def handle_call(:stats, _from, state) do
    stats = %{
      cursor: state.cursor,
      demand: state.demand,
      events_fetched: state.events_fetched,
      polls_count: state.polls_count,
      last_poll_at: state.last_poll_at,
      last_error: state.last_error
    }
    {:reply, stats, [], state}
  end

  @impl GenStage
  def handle_info(:poll, state) do
    if state.demand > 0 do
      fetch_and_dispatch(state)
    else
      schedule_poll(state.poll_interval)
      {:noreply, [], state}
    end
  end

  # Private Functions

  defp fetch_and_dispatch(state) do
    fetch_count = min(state.demand, state.batch_size)

    case fetch_events(state.cursor, fetch_count) do
      {:ok, events} when length(events) > 0 ->
        # Convert to Broadway messages
        messages = Enum.map(events, &to_broadway_message/1)

        # Update cursor to last event timestamp
        new_cursor = get_last_timestamp(events) || state.cursor
        save_cursor(new_cursor)

        new_state = %{state |
          cursor: new_cursor,
          demand: state.demand - length(messages),
          events_fetched: state.events_fetched + length(messages),
          polls_count: state.polls_count + 1,
          last_poll_at: DateTime.utc_now(),
          last_error: nil
        }

        # Emit telemetry
        :telemetry.execute(
          [:query_service_ex, :core_producer, :events_fetched],
          %{count: length(messages)},
          %{cursor: new_cursor}
        )

        schedule_poll(state.poll_interval)
        {:noreply, messages, new_state}

      {:ok, []} ->
        # No new events
        new_state = %{state |
          polls_count: state.polls_count + 1,
          last_poll_at: DateTime.utc_now()
        }

        schedule_poll(state.poll_interval)
        {:noreply, [], new_state}

      {:error, reason} ->
        Logger.warning("[CoreProducer] Failed to fetch events: #{inspect(reason)}")

        new_state = %{state |
          polls_count: state.polls_count + 1,
          last_poll_at: DateTime.utc_now(),
          last_error: reason
        }

        # Back off on error
        schedule_poll(state.poll_interval * 2)
        {:noreply, [], new_state}
    end
  end

  defp fetch_events(cursor, limit) do
    params = %{
      since: cursor,
      limit: limit
    }

    RustCoreClient.query_events(params)
  end

  defp to_broadway_message(event) do
    %Broadway.Message{
      data: event,
      acknowledger: {Broadway.NoopAcknowledger, nil, nil}
    }
  end

  defp get_last_timestamp([]), do: nil
  defp get_last_timestamp(events) do
    events
    |> List.last()
    |> Map.get("timestamp", nil)
  end

  defp schedule_poll(interval) do
    Process.send_after(self(), :poll, interval)
  end

  defp init_cursor_table do
    if :ets.whereis(@cursor_table) == :undefined do
      :ets.new(@cursor_table, [:set, :public, :named_table])
    end
  rescue
    ArgumentError -> :ok  # Table already exists
  end

  defp load_cursor do
    case :ets.lookup(@cursor_table, @cursor_key) do
      [{@cursor_key, cursor}] -> cursor
      [] -> "1970-01-01T00:00:00Z"  # Epoch start
    end
  rescue
    ArgumentError -> "1970-01-01T00:00:00Z"
  end

  defp save_cursor(cursor) do
    :ets.insert(@cursor_table, {@cursor_key, cursor})
  rescue
    ArgumentError ->
      init_cursor_table()
      :ets.insert(@cursor_table, {@cursor_key, cursor})
  end
end
