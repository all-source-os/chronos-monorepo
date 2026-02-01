defmodule QueryServiceEx.Application.UseCases.ProjectionServer do
  @moduledoc """
  GenServer for managing projection state and processing events.

  Each projection runs in its own process with isolated state.
  Uses OTP supervision for fault tolerance and automatic recovery.

  ## Features
  - Real-time event processing
  - State persistence with configurable backends
  - Hot code reloading for projection updates
  - Supervision tree integration
  - Telemetry instrumentation

  ## State Management
  - In-memory state for fast access
  - Periodic snapshots to persistent storage
  - Crash recovery from last snapshot

  ## Example

      # Start a projection server
      {:ok, pid} = ProjectionServer.start_link(
        projection: user_stats_projection,
        entity_id: "user-123",
        state_store: MyApp.PostgresStateStore
      )

      # Apply an event
      ProjectionServer.apply_event(pid, event)

      # Get current state
      ProjectionServer.get_state(pid)
  """

  use GenServer
  require Logger

  alias QueryServiceEx.Domain.Entities.Projection

  @type state :: %{
          projection: Projection.Definition.t(),
          entity_id: String.t(),
          projection_state: any(),
          state_store: module() | nil,
          last_snapshot_at: DateTime.t() | nil,
          events_since_snapshot: non_neg_integer(),
          snapshot_interval: pos_integer()
        }

  @default_snapshot_interval 100

  ## Client API

  @doc """
  Start a projection server.

  ## Options
    * `:projection` - ProjectionDefinition struct (required)
    * `:entity_id` - Entity ID for this projection (required)
    * `:state_store` - Module implementing state persistence (optional)
    * `:snapshot_interval` - Number of events between snapshots (default: 100)
    * `:name` - Process name for registration (optional)
  """
  def start_link(opts) do
    {name_opt, opts} = Keyword.pop(opts, :name)

    case name_opt do
      nil -> GenServer.start_link(__MODULE__, opts)
      name -> GenServer.start_link(__MODULE__, opts, name: name)
    end
  end

  @doc """
  Apply a single event to the projection.

  Returns the updated state.
  """
  def apply_event(server, event) do
    GenServer.call(server, {:apply_event, event})
  end

  @doc """
  Apply multiple events to the projection.

  More efficient than calling apply_event/2 multiple times.
  """
  def apply_events(server, events) when is_list(events) do
    GenServer.call(server, {:apply_events, events})
  end

  @doc "Get the current projection state"
  def get_state(server) do
    GenServer.call(server, :get_state)
  end

  @doc "Get projection metadata (name, version, etc.)"
  def get_metadata(server) do
    GenServer.call(server, :get_metadata)
  end

  @doc "Force a snapshot to be taken"
  def snapshot(server) do
    GenServer.call(server, :snapshot)
  end

  @doc "Reset projection to initial state"
  def reset(server) do
    GenServer.call(server, :reset)
  end

  @doc "Stop the projection server gracefully"
  def stop(server) do
    GenServer.stop(server, :normal)
  end

  ## Server Callbacks

  @impl true
  def init(opts) do
    projection = Keyword.fetch!(opts, :projection)
    entity_id = Keyword.fetch!(opts, :entity_id)
    state_store = Keyword.get(opts, :state_store)
    snapshot_interval = Keyword.get(opts, :snapshot_interval, @default_snapshot_interval)

    if Projection.valid_projection?(projection) do
      # Try to restore from snapshot
      initial_state =
        case restore_snapshot(projection, entity_id, state_store) do
          {:ok, snapshot_state} ->
            Logger.info("Restored projection #{projection.name} for #{entity_id} from snapshot")
            snapshot_state

          {:error, _reason} ->
            Logger.info(
              "Starting projection #{projection.name} for #{entity_id} with initial state"
            )

            Projection.initialize_state(projection)
        end

      state = %{
        projection: projection,
        entity_id: entity_id,
        projection_state: initial_state,
        state_store: state_store,
        last_snapshot_at: DateTime.utc_now(),
        events_since_snapshot: 0,
        snapshot_interval: snapshot_interval
      }

      # Emit telemetry
      :telemetry.execute(
        [:projection_server, :started],
        %{count: 1},
        %{projection: projection.name, entity_id: entity_id}
      )

      {:ok, state}
    else
      {:stop, {:invalid_projection, projection}}
    end
  end

  @impl true
  def handle_call({:apply_event, event}, _from, state) do
    new_projection_state = Projection.apply_event(state.projection, state.projection_state, event)

    new_state = %{
      state
      | projection_state: new_projection_state,
        events_since_snapshot: state.events_since_snapshot + 1
    }

    # Check if we should snapshot
    new_state =
      if should_snapshot?(new_state) do
        case take_snapshot(new_state) do
          :ok ->
            %{new_state | events_since_snapshot: 0, last_snapshot_at: DateTime.utc_now()}

          {:error, reason} ->
            Logger.error("Failed to snapshot: #{inspect(reason)}")
            new_state
        end
      else
        new_state
      end

    # Emit telemetry
    :telemetry.execute(
      [:projection_server, :event_applied],
      %{count: 1},
      %{projection: state.projection.name, entity_id: state.entity_id}
    )

    {:reply, new_projection_state, new_state}
  end

  @impl true
  def handle_call({:apply_events, events}, _from, state) do
    new_projection_state =
      Projection.apply_events(state.projection, state.projection_state, events)

    new_state = %{
      state
      | projection_state: new_projection_state,
        events_since_snapshot: state.events_since_snapshot + length(events)
    }

    # Check if we should snapshot
    new_state =
      if should_snapshot?(new_state) do
        case take_snapshot(new_state) do
          :ok ->
            %{new_state | events_since_snapshot: 0, last_snapshot_at: DateTime.utc_now()}

          {:error, reason} ->
            Logger.error("Failed to snapshot: #{inspect(reason)}")
            new_state
        end
      else
        new_state
      end

    # Emit telemetry
    :telemetry.execute(
      [:projection_server, :events_applied],
      %{count: length(events)},
      %{projection: state.projection.name, entity_id: state.entity_id}
    )

    {:reply, new_projection_state, new_state}
  end

  @impl true
  def handle_call(:get_state, _from, state) do
    {:reply, state.projection_state, state}
  end

  @impl true
  def handle_call(:get_metadata, _from, state) do
    metadata = %{
      projection_name: state.projection.name,
      projection_version: state.projection.version,
      entity_id: state.entity_id,
      events_since_snapshot: state.events_since_snapshot,
      last_snapshot_at: state.last_snapshot_at
    }

    {:reply, metadata, state}
  end

  @impl true
  def handle_call(:snapshot, _from, state) do
    case take_snapshot(state) do
      :ok ->
        new_state = %{
          state
          | events_since_snapshot: 0,
            last_snapshot_at: DateTime.utc_now()
        }

        {:reply, :ok, new_state}

      {:error, _reason} = error ->
        {:reply, error, state}
    end
  end

  @impl true
  def handle_call(:reset, _from, state) do
    initial_state = Projection.initialize_state(state.projection)

    new_state = %{
      state
      | projection_state: initial_state,
        events_since_snapshot: 0,
        last_snapshot_at: DateTime.utc_now()
    }

    Logger.info("Reset projection #{state.projection.name} for #{state.entity_id}")

    {:reply, :ok, new_state}
  end

  @impl true
  def terminate(reason, state) do
    Logger.info(
      "Stopping projection server for #{state.projection.name}/#{state.entity_id}: #{inspect(reason)}"
    )

    # Take final snapshot before terminating
    take_snapshot(state)

    :telemetry.execute(
      [:projection_server, :stopped],
      %{count: 1},
      %{projection: state.projection.name, entity_id: state.entity_id, reason: reason}
    )

    :ok
  end

  ## Private Helpers

  defp should_snapshot?(%{events_since_snapshot: count, snapshot_interval: interval}) do
    count >= interval
  end

  defp take_snapshot(%{state_store: nil}), do: :ok

  defp take_snapshot(%{state_store: state_store} = state) do
    snapshot =
      Projection.snapshot_state(
        state.projection.name,
        state.entity_id,
        state.projection_state,
        state.projection.version
      )

    case state_store.save_snapshot(snapshot) do
      :ok ->
        Logger.debug("Saved snapshot for #{state.projection.name}/#{state.entity_id}")
        :ok

      {:error, reason} = error ->
        Logger.error("Failed to save snapshot: #{inspect(reason)}")
        error
    end
  end

  defp restore_snapshot(_projection, _entity_id, nil), do: {:error, :no_state_store}

  defp restore_snapshot(projection, entity_id, state_store) do
    case state_store.load_snapshot(projection.name, entity_id) do
      {:ok, snapshot} ->
        {:ok, Projection.restore_from_snapshot(snapshot)}

      {:error, _reason} = error ->
        error
    end
  end
end
