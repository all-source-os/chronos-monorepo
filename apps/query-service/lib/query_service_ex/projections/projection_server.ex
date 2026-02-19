defmodule QueryServiceEx.Projections.ProjectionServer do
  @moduledoc """
  GenServer that maintains projected state for a single projection by
  subscribing to Core's event stream via PubSub.

  Events are folded incrementally (one at a time) as they arrive.
  Per-entity state is stored in an ETS table. Dirty entities are
  periodically synced to Core's DashMap via RustCoreClient.

  On init, hydrates state from Core projection state cache (cold start recovery).
  """

  use GenServer
  require Logger

  alias QueryServiceEx.Infrastructure.Adapters.RustCoreClient

  @default_sync_interval_ms 5_000

  @type state :: %{
          projection: module(),
          projection_name: String.t(),
          entity_type: String.t(),
          ets_table: atom(),
          dirty_entities: MapSet.t(String.t()),
          sync_interval: pos_integer(),
          core_client: module()
        }

  ## Client API

  @doc """
  Start a ProjectionServer for the given projection module.

  ## Options
    * `:projection` — required, module implementing `Projections.Behaviour`
    * `:name` — optional, process name for registration
    * `:sync_interval` — optional, ms between Core syncs (default: 5000)
    * `:core_client` — optional, module for Core API calls (default: RustCoreClient)
    * `:pubsub` — optional, PubSub module (default: QueryServiceEx.PubSub)
  """
  def start_link(opts) do
    {name_opt, opts} = Keyword.pop(opts, :name)

    case name_opt do
      nil -> GenServer.start_link(__MODULE__, opts)
      name -> GenServer.start_link(__MODULE__, opts, name: name)
    end
  end

  @doc "Get the projected state for a specific entity."
  def get_entity_state(server, entity_id) do
    GenServer.call(server, {:get_entity_state, entity_id})
  end

  @doc "Get all projected states from the ETS table."
  def get_all_states(server) do
    GenServer.call(server, :get_all_states)
  end

  ## Server Callbacks

  @impl true
  def init(opts) do
    projection = Keyword.fetch!(opts, :projection)
    sync_interval = Keyword.get(opts, :sync_interval, sync_interval_config())
    core_client = Keyword.get(opts, :core_client, RustCoreClient)
    pubsub = Keyword.get(opts, :pubsub, QueryServiceEx.PubSub)

    projection_name = resolve_projection_name(projection)
    entity_type = projection.entity_type()
    ets_table = ets_table_name(entity_type)

    # Create ETS table for this projection's entity states
    :ets.new(ets_table, [:named_table, :public, :set, read_concurrency: true])

    # Subscribe to PubSub topic for this entity type
    topic = "events:type:#{entity_type}"
    Phoenix.PubSub.subscribe(pubsub, topic)
    Logger.info("ProjectionServer[#{entity_type}] subscribed to #{topic}")

    state = %{
      projection: projection,
      projection_name: projection_name,
      entity_type: entity_type,
      ets_table: ets_table,
      dirty_entities: MapSet.new(),
      sync_interval: sync_interval,
      core_client: core_client
    }

    # Hydrate from Core on startup (async to not block init)
    send(self(), :hydrate)

    # Schedule first sync
    schedule_sync(sync_interval)

    {:ok, state}
  end

  @impl true
  def handle_info({:new_event, event}, state) do
    entity_id = event["entity_id"]

    if entity_id do
      current = ets_get(state.ets_table, entity_id, state.projection.initial_state())
      new_state = state.projection.apply_event(current, event)
      :ets.insert(state.ets_table, {entity_id, new_state})

      {:noreply, %{state | dirty_entities: MapSet.put(state.dirty_entities, entity_id)}}
    else
      {:noreply, state}
    end
  end

  @impl true
  def handle_info(:sync, state) do
    state =
      if MapSet.size(state.dirty_entities) > 0 do
        remaining = sync_dirty_entities(state)
        %{state | dirty_entities: remaining}
      else
        state
      end

    schedule_sync(state.sync_interval)
    {:noreply, state}
  end

  @impl true
  def handle_info(:hydrate, state) do
    hydrate_from_core(state)
    {:noreply, state}
  end

  @impl true
  def handle_call({:get_entity_state, entity_id}, _from, state) do
    result = ets_get(state.ets_table, entity_id, nil)
    {:reply, result, state}
  end

  @impl true
  def handle_call(:get_all_states, _from, state) do
    all =
      state.ets_table
      |> :ets.tab2list()
      |> Enum.map(fn {_eid, entity_state} -> entity_state end)

    {:reply, all, state}
  end

  ## Private Helpers

  defp sync_dirty_entities(state) do
    Enum.reduce(state.dirty_entities, MapSet.new(), fn entity_id, failed ->
      case ets_get(state.ets_table, entity_id, nil) do
        nil ->
          failed

        entity_state ->
          case state.core_client.save_projection_state(
                 state.projection_name,
                 entity_id,
                 entity_state
               ) do
            :ok ->
              failed

            {:error, reason} ->
              Logger.warning(
                "ProjectionServer[#{state.entity_type}] failed to sync #{entity_id}: #{inspect(reason)}"
              )

              MapSet.put(failed, entity_id)
          end
      end
    end)
  end

  defp hydrate_from_core(state) do
    case state.core_client.get_projection_state_summary(state.projection_name) do
      {:ok, %{"states" => states}} when is_list(states) ->
        for %{"entity_id" => eid, "state" => entity_state} <- states, not is_nil(entity_state) do
          :ets.insert(state.ets_table, {eid, entity_state})
        end

        Logger.info(
          "ProjectionServer[#{state.entity_type}] hydrated #{length(states)} entities from Core"
        )

      {:ok, states} when is_list(states) ->
        for %{"entity_id" => eid, "state" => entity_state} <- states, not is_nil(entity_state) do
          :ets.insert(state.ets_table, {eid, entity_state})
        end

        Logger.info(
          "ProjectionServer[#{state.entity_type}] hydrated #{length(states)} entities from Core"
        )

      {:error, reason} ->
        Logger.warning(
          "ProjectionServer[#{state.entity_type}] failed to hydrate from Core: #{inspect(reason)}. Will build state from events."
        )

      _ ->
        Logger.info(
          "ProjectionServer[#{state.entity_type}] no existing state in Core, starting fresh"
        )
    end
  end

  defp ets_get(table, key, default) do
    case :ets.lookup(table, key) do
      [{^key, value}] -> value
      [] -> default
    end
  end

  defp ets_table_name(entity_type) do
    :"projection_#{entity_type}"
  end

  defp schedule_sync(interval) do
    Process.send_after(self(), :sync, interval)
  end

  defp sync_interval_config do
    Application.get_env(:query_service_ex, :projection_sync_interval, @default_sync_interval_ms)
  end

  defp resolve_projection_name(projection_mod) do
    alias QueryServiceEx.Projections.Registry, as: ProjectionRegistry

    ProjectionRegistry.list()
    |> Enum.find(fn name ->
      ProjectionRegistry.get(name) == projection_mod
    end)
    |> case do
      nil -> projection_mod.entity_type()
      name -> name
    end
  end
end
