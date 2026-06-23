defmodule QueryServiceEx.Application.Services.ProjectionSync do
  @moduledoc """
  GenServer for synchronizing projection state with Rust Core's DashMap.

  This module provides a two-tier caching strategy:
  - L1: Local ETS cache for sub-millisecond reads
  - L2: Core's DashMap (11.9 μs) as source of truth

  ## Features
  - Loads initial state from Core on startup
  - Periodic sync of dirty state to Core (configurable interval)
  - Event subscription for real-time updates
  - ETS cache for local reads
  - Graceful degradation when Core is unavailable
  - Automatic retry with backoff for failed syncs

  ## Usage

      # Start a projection sync process
      {:ok, pid} = ProjectionSync.start_link(
        projection_name: "user_snapshots",
        entity_id: "user-123"
      )

      # Get current state (from local ETS)
      state = ProjectionSync.get_state(pid)

      # Apply an event (marks as dirty)
      ProjectionSync.apply_event(pid, event)

  ## Configuration

      config :query_service_ex,
        projection_sync_interval_ms: 100,
        projection_sync_enabled: true,
        projection_sync_max_retries: 3,
        projection_sync_retry_backoff_ms: 1_000
  """

  use GenServer
  require Logger

  alias QueryServiceEx.Infrastructure.Adapters.RustCoreClient

  @default_sync_interval_ms 100
  @default_max_retries 3
  @default_retry_backoff_ms 1_000
  @ets_table :projection_state_cache

  # Client API

  @doc """
  Start a projection sync process.

  ## Options
    * `:projection_name` - Name of the projection (required)
    * `:entity_id` - Entity identifier (required)
    * `:initial_state` - Initial state if not found in Core (default: %{})
    * `:sync_interval_ms` - Sync interval in ms (default: 100)
    * `:project_fn` - Function to apply events to state (default: deep merge)
  """
  def start_link(opts) do
    projection_name = Keyword.fetch!(opts, :projection_name)
    entity_id = Keyword.fetch!(opts, :entity_id)
    name = opts[:name] || via_tuple(projection_name, entity_id)

    GenServer.start_link(__MODULE__, opts, name: name)
  end

  @doc """
  Get current projection state from local cache.
  """
  def get_state(pid_or_name) do
    GenServer.call(pid_or_name, :get_state)
  end

  @doc """
  Apply an event to the projection state.
  """
  def apply_event(pid_or_name, event) do
    GenServer.cast(pid_or_name, {:apply_event, event})
  end

  @doc """
  Force sync to Core immediately.
  """
  def force_sync(pid_or_name) do
    GenServer.call(pid_or_name, :force_sync)
  end

  @doc """
  Check if the projection has unsaved changes.
  """
  def dirty?(pid_or_name) do
    GenServer.call(pid_or_name, :is_dirty)
  end

  @doc """
  Get projection state directly from ETS cache (fastest).
  Returns nil if not found.
  """
  def get_state_from_cache(projection_name, entity_id) do
    case :ets.lookup(@ets_table, {projection_name, entity_id}) do
      [{_key, state}] -> state
      [] -> nil
    end
  rescue
    # Table doesn't exist
    ArgumentError -> nil
  end

  @doc """
  Initialize the ETS cache table. Called once at application startup.
  """
  def init_cache do
    if :ets.whereis(@ets_table) == :undefined do
      :ets.new(@ets_table, [:set, :public, :named_table, read_concurrency: true])
      Logger.info("[ProjectionSync] ETS cache initialized")
    end

    :ok
  end

  # Server Callbacks

  @impl GenServer
  def init(opts) do
    projection_name = Keyword.fetch!(opts, :projection_name)
    entity_id = Keyword.fetch!(opts, :entity_id)
    initial_state = Keyword.get(opts, :initial_state, %{})
    sync_interval = Keyword.get(opts, :sync_interval_ms, @default_sync_interval_ms)
    project_fn = Keyword.get(opts, :project_fn, &default_project_fn/2)

    max_retries =
      Keyword.get(
        opts,
        :max_retries,
        Application.get_env(:query_service_ex, :projection_sync_max_retries, @default_max_retries)
      )

    retry_backoff =
      Keyword.get(
        opts,
        :retry_backoff_ms,
        Application.get_env(
          :query_service_ex,
          :projection_sync_retry_backoff_ms,
          @default_retry_backoff_ms
        )
      )

    # Ensure ETS table exists
    init_cache()

    # Load initial state from Core (with graceful degradation)
    {state, core_available} =
      load_from_core_with_fallback(projection_name, entity_id, initial_state)

    # Store in ETS cache
    update_cache(projection_name, entity_id, state)

    # Subscribe to events for this entity.
    # ISOLATION_OK: internal projection builder consuming the event_pipeline
    # stream; not a user-facing subscription. Tenant-scope alongside event_pipeline
    # once ProjectionSync carries the tenant (tracked follow-up).
    Phoenix.PubSub.subscribe(QueryServiceEx.PubSub, "events:#{entity_id}")

    # Schedule periodic sync
    schedule_sync(sync_interval)

    Logger.debug("[ProjectionSync] Started for #{projection_name}:#{entity_id}",
      core_available: core_available
    )

    {:ok,
     %{
       projection_name: projection_name,
       entity_id: entity_id,
       state: state,
       dirty: false,
       sync_interval: sync_interval,
       project_fn: project_fn,
       sync_count: 0,
       events_applied: 0,
       failed_syncs: 0,
       max_retries: max_retries,
       retry_backoff_ms: retry_backoff,
       core_available: core_available,
       last_sync_error: nil
     }}
  end

  @impl GenServer
  def handle_call(:get_state, _from, state) do
    {:reply, state.state, state}
  end

  @impl GenServer
  def handle_call(:is_dirty, _from, state) do
    {:reply, state.dirty, state}
  end

  @impl GenServer
  def handle_call(:force_sync, _from, state) do
    {new_state, _success} = sync_to_core_with_retry(state)
    {:reply, :ok, %{new_state | dirty: false}}
  end

  @impl GenServer
  def handle_cast({:apply_event, event}, state) do
    new_projection_state = state.project_fn.(state.state, event)

    # Update ETS cache
    update_cache(state.projection_name, state.entity_id, new_projection_state)

    {:noreply,
     %{state | state: new_projection_state, dirty: true, events_applied: state.events_applied + 1}}
  end

  @impl GenServer
  def handle_info({:new_event, event}, state) do
    # Apply event received from PubSub
    new_projection_state = state.project_fn.(state.state, event)

    # Update ETS cache
    update_cache(state.projection_name, state.entity_id, new_projection_state)

    {:noreply,
     %{state | state: new_projection_state, dirty: true, events_applied: state.events_applied + 1}}
  end

  @impl GenServer
  def handle_info(:sync, %{dirty: true} = state) do
    start_time = System.monotonic_time()

    :telemetry.execute(
      [:query_service_ex, :projection, :sync_started],
      %{},
      %{projection_name: state.projection_name, entity_id: state.entity_id}
    )

    {new_state, sync_successful} = sync_to_core_with_retry(state)

    duration_ms =
      System.convert_time_unit(System.monotonic_time() - start_time, :native, :millisecond)

    if sync_successful do
      :telemetry.execute(
        [:query_service_ex, :projection, :sync_completed],
        %{duration_ms: duration_ms},
        %{projection_name: state.projection_name, entity_id: state.entity_id}
      )

      schedule_sync(state.sync_interval)

      {:noreply,
       %{
         new_state
         | dirty: false,
           sync_count: state.sync_count + 1,
           failed_syncs: 0,
           core_available: true
       }}
    else
      # Sync failed - schedule retry with backoff
      retry_interval = calculate_retry_interval(state)

      Logger.warning(
        "[ProjectionSync] Sync failed, scheduling retry in #{retry_interval}ms",
        projection_name: state.projection_name,
        entity_id: state.entity_id,
        failed_syncs: new_state.failed_syncs
      )

      schedule_sync(retry_interval)
      {:noreply, %{new_state | core_available: false}}
    end
  end

  @impl GenServer
  def handle_info(:sync, state) do
    schedule_sync(state.sync_interval)
    {:noreply, state}
  end

  @impl GenServer
  def terminate(_reason, state) do
    # Sync any pending changes before shutdown
    if state.dirty do
      sync_to_core_with_retry(state)
    end

    :ok
  end

  # Private Functions

  defp load_from_core_with_fallback(projection_name, entity_id, default_state) do
    case RustCoreClient.get_projection_state(projection_name, entity_id) do
      {:ok, state} ->
        Logger.debug(
          "[ProjectionSync] Loaded state from Core for #{projection_name}:#{entity_id}"
        )

        {state, true}

      {:error, :not_found} ->
        Logger.debug(
          "[ProjectionSync] No state in Core for #{projection_name}:#{entity_id}, using default"
        )

        {default_state, true}

      {:error, reason} ->
        Logger.warning(
          "[ProjectionSync] Failed to load from Core: #{inspect(reason)}, using default (degraded mode)"
        )

        :telemetry.execute(
          [:query_service_ex, :projection, :load_failed],
          %{},
          %{
            projection_name: projection_name,
            entity_id: entity_id,
            error: reason
          }
        )

        {default_state, false}
    end
  end

  defp sync_to_core_with_retry(state) do
    case RustCoreClient.save_projection_state(
           state.projection_name,
           state.entity_id,
           state.state
         ) do
      :ok ->
        Logger.debug(
          "[ProjectionSync] Synced to Core: #{state.projection_name}:#{state.entity_id}",
          projection_name: state.projection_name,
          entity_id: state.entity_id
        )

        {state, true}

      {:error, reason} ->
        new_failed_syncs = state.failed_syncs + 1

        if new_failed_syncs >= state.max_retries do
          Logger.error(
            "[ProjectionSync] Max retries (#{state.max_retries}) exceeded for sync to Core",
            projection_name: state.projection_name,
            entity_id: state.entity_id,
            error: inspect(reason)
          )

          :telemetry.execute(
            [:query_service_ex, :projection, :sync_exhausted],
            %{failed_syncs: new_failed_syncs},
            %{
              projection_name: state.projection_name,
              entity_id: state.entity_id,
              error: reason
            }
          )
        else
          Logger.warning(
            "[ProjectionSync] Failed to sync to Core (attempt #{new_failed_syncs}/#{state.max_retries}): #{inspect(reason)}",
            projection_name: state.projection_name,
            entity_id: state.entity_id,
            error: inspect(reason)
          )
        end

        :telemetry.execute(
          [:query_service_ex, :projection, :sync_failed],
          %{failed_syncs: new_failed_syncs},
          %{
            projection_name: state.projection_name,
            entity_id: state.entity_id,
            error: reason
          }
        )

        {%{state | failed_syncs: new_failed_syncs, last_sync_error: reason}, false}
    end
  end

  defp calculate_retry_interval(state) do
    # Exponential backoff with cap
    base = state.retry_backoff_ms
    # Cap at 5x backoff
    multiplier = min(state.failed_syncs + 1, 5)
    # Add up to 20% jitter
    jitter = :rand.uniform() * 0.2
    round(base * multiplier * (1 + jitter))
  end

  defp update_cache(projection_name, entity_id, value) do
    :ets.insert(@ets_table, {{projection_name, entity_id}, value})
  rescue
    ArgumentError ->
      # Table doesn't exist, create it
      init_cache()
      :ets.insert(@ets_table, {{projection_name, entity_id}, value})
  end

  defp schedule_sync(interval_ms) do
    Process.send_after(self(), :sync, interval_ms)
  end

  defp via_tuple(projection_name, entity_id) do
    {:via, Registry, {QueryServiceEx.ProjectionRegistry, {projection_name, entity_id}}}
  end

  defp default_project_fn(state, event) when is_map(state) and is_map(event) do
    # Deep merge event payload into state
    payload = event["payload"] || event[:payload] || %{}

    Map.merge(state, payload, fn
      _key, v1, v2 when is_map(v1) and is_map(v2) -> Map.merge(v1, v2)
      _key, _v1, v2 -> v2
    end)
  end

  defp default_project_fn(state, _event), do: state
end
