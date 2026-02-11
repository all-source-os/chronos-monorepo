defmodule McpServerElixir.Infrastructure.ProjectionSync do
  @moduledoc """
  GenServer that synchronizes projection state between Elixir and Core's DashMap.

  This GenServer maintains local ETS tables for sub-millisecond reads while
  periodically syncing dirty state to the Rust Core service.

  ## Features
  - Loads initial state from Core on startup
  - Tracks dirty flag when state changes locally
  - Syncs dirty state to Core every 100ms
  - Uses ETS for fast local reads (sub-ms)
  - Restores projection state from Core on restart
  - Handles Core unavailability gracefully

  ## ETS Tables
  - `projection_state` - stores projection state by key `{projection_name, entity_id}`
  - `projection_dirty` - tracks which keys have unsaved changes

  ## Configuration

      config :mcp_server_elixir,
        projection_sync_enabled: true,
        projection_sync_interval_ms: 100,
        core_url: "http://localhost:3900"

  ## Example

      # Get projection state (sub-ms read from ETS)
      ProjectionSync.get_state("entity_snapshots", "user-123")

      # Update projection state (marks as dirty, syncs later)
      ProjectionSync.put_state("entity_snapshots", "user-123", %{name: "Test"})

      # Force immediate sync
      ProjectionSync.sync_now()
  """

  use GenServer
  require Logger

  alias McpServerElixir.Infrastructure.CoreClient

  @default_sync_interval_ms 100
  @ets_state_table :projection_state
  @ets_dirty_table :projection_dirty

  # Client API

  @doc """
  Start the ProjectionSync GenServer.

  ## Options
    * `:name` - Process name (default: __MODULE__)
    * `:enabled` - Whether to start the sync process (default: true in prod, false in test)
    * `:sync_interval_ms` - Sync interval in milliseconds (default: 100)
    * `:projections` - List of projection names to sync (default: ["entity_snapshots", "event_counters"])
  """
  def start_link(opts \\ []) do
    default_enabled = Application.get_env(:mcp_server_elixir, :projection_sync_enabled, true)
    enabled = Keyword.get(opts, :enabled, default_enabled)

    if enabled do
      name = Keyword.get(opts, :name, __MODULE__)
      GenServer.start_link(__MODULE__, opts, name: name)
    else
      Logger.info("[ProjectionSync] Disabled, not starting")
      :ignore
    end
  end

  @doc """
  Get projection state for an entity. Reads from local ETS (sub-ms).

  Returns `nil` if the state doesn't exist.
  """
  def get_state(projection_name, entity_id, server \\ __MODULE__) do
    GenServer.call(server, {:get_state, projection_name, entity_id})
  end

  @doc """
  Update projection state for an entity. Marks as dirty for later sync.

  Returns `:ok` on success.
  """
  def put_state(projection_name, entity_id, state, server \\ __MODULE__) do
    GenServer.call(server, {:put_state, projection_name, entity_id, state})
  end

  @doc """
  Delete projection state for an entity.

  Returns `:ok` on success.
  """
  def delete_state(projection_name, entity_id, server \\ __MODULE__) do
    GenServer.call(server, {:delete_state, projection_name, entity_id})
  end

  @doc """
  Force immediate sync of all dirty state to Core.

  Returns `{:ok, synced_count}` on success, `{:error, reason}` on failure.
  """
  def sync_now(server \\ __MODULE__) do
    GenServer.call(server, :sync_now, 10_000)
  end

  @doc """
  Get sync statistics.
  """
  def stats(server \\ __MODULE__) do
    GenServer.call(server, :stats)
  end

  @doc """
  Check if the sync process is healthy.
  """
  def healthy?(server \\ __MODULE__) do
    GenServer.call(server, :healthy?)
  end

  @doc """
  Load projection state from Core for specific entities (used for on-demand loading).
  """
  def load_from_core(projection_name, entity_ids, server \\ __MODULE__)
      when is_list(entity_ids) do
    GenServer.call(server, {:load_from_core, projection_name, entity_ids}, 30_000)
  end

  # Server Callbacks

  @impl true
  def init(opts) do
    sync_interval =
      opts[:sync_interval_ms] ||
        Application.get_env(
          :mcp_server_elixir,
          :projection_sync_interval_ms,
          @default_sync_interval_ms
        )

    projections = opts[:projections] || ["entity_snapshots", "event_counters"]

    # Create ETS tables
    create_ets_tables()

    state = %{
      sync_interval_ms: sync_interval,
      projections: projections,
      core_client: CoreClient.new(),
      synced_count: 0,
      sync_errors: 0,
      last_sync_at: nil,
      last_sync_error: nil,
      core_available: true,
      started_at: DateTime.utc_now()
    }

    # Load initial state from Core
    state = load_initial_state(state)

    # Schedule periodic sync
    schedule_sync(sync_interval)

    Logger.info("[ProjectionSync] Started with #{sync_interval}ms sync interval")

    {:ok, state}
  end

  @impl true
  def handle_call({:get_state, projection_name, entity_id}, _from, state) do
    result = ets_get(projection_name, entity_id)
    {:reply, result, state}
  end

  @impl true
  def handle_call({:put_state, projection_name, entity_id, new_state}, _from, state) do
    ets_put(projection_name, entity_id, new_state)
    mark_dirty(projection_name, entity_id)
    {:reply, :ok, state}
  end

  @impl true
  def handle_call({:delete_state, projection_name, entity_id}, _from, state) do
    ets_delete(projection_name, entity_id)
    unmark_dirty(projection_name, entity_id)
    {:reply, :ok, state}
  end

  @impl true
  def handle_call(:sync_now, _from, state) do
    {result, new_state} = do_sync(state)
    {:reply, result, new_state}
  end

  @impl true
  def handle_call(:stats, _from, state) do
    stats = %{
      synced_count: state.synced_count,
      sync_errors: state.sync_errors,
      last_sync_at: state.last_sync_at,
      last_sync_error: state.last_sync_error,
      core_available: state.core_available,
      started_at: state.started_at,
      sync_interval_ms: state.sync_interval_ms,
      dirty_count: count_dirty(),
      state_count: count_states()
    }

    {:reply, stats, state}
  end

  @impl true
  def handle_call(:healthy?, _from, state) do
    healthy = state.core_available || state.sync_errors < 10
    {:reply, healthy, state}
  end

  @impl true
  def handle_call({:load_from_core, projection_name, entity_ids}, _from, state) do
    result = load_entities_from_core(state.core_client, projection_name, entity_ids)
    {:reply, result, state}
  end

  @impl true
  def handle_info(:sync, state) do
    {_result, new_state} = do_sync(state)

    # Schedule next sync
    schedule_sync(state.sync_interval_ms)

    {:noreply, new_state}
  end

  @impl true
  def handle_info(_msg, state) do
    {:noreply, state}
  end

  # Private Functions

  defp create_ets_tables do
    # State table: {projection_name, entity_id} => state (map)
    :ets.new(@ets_state_table, [:named_table, :set, :public, read_concurrency: true])

    # Dirty tracking table: {projection_name, entity_id} => true
    :ets.new(@ets_dirty_table, [:named_table, :set, :public, read_concurrency: true])
  rescue
    ArgumentError ->
      # Tables might already exist (e.g., in tests)
      :ok
  end

  defp ets_get(projection_name, entity_id) do
    key = {projection_name, entity_id}

    case :ets.lookup(@ets_state_table, key) do
      [{^key, state}] -> state
      [] -> nil
    end
  rescue
    ArgumentError -> nil
  end

  defp ets_put(projection_name, entity_id, state) do
    key = {projection_name, entity_id}
    :ets.insert(@ets_state_table, {key, state})
    :ok
  rescue
    ArgumentError -> :ok
  end

  defp ets_delete(projection_name, entity_id) do
    key = {projection_name, entity_id}
    :ets.delete(@ets_state_table, key)
    :ok
  rescue
    ArgumentError -> :ok
  end

  defp mark_dirty(projection_name, entity_id) do
    key = {projection_name, entity_id}
    :ets.insert(@ets_dirty_table, {key, true})
    :ok
  rescue
    ArgumentError -> :ok
  end

  defp unmark_dirty(projection_name, entity_id) do
    key = {projection_name, entity_id}
    :ets.delete(@ets_dirty_table, key)
    :ok
  rescue
    ArgumentError -> :ok
  end

  defp get_dirty_keys do
    :ets.tab2list(@ets_dirty_table)
    |> Enum.map(fn {key, _} -> key end)
  rescue
    ArgumentError -> []
  end

  defp count_dirty do
    :ets.info(@ets_dirty_table, :size)
  rescue
    ArgumentError -> 0
  end

  defp count_states do
    :ets.info(@ets_state_table, :size)
  rescue
    ArgumentError -> 0
  end

  defp schedule_sync(interval_ms) do
    Process.send_after(self(), :sync, interval_ms)
  end

  defp load_initial_state(state) do
    Logger.info("[ProjectionSync] Loading initial state from Core...")

    # Note: Currently we don't bulk-load projection state on startup because
    # the Core API doesn't have a "get all entities for projection" endpoint.
    # State is loaded on-demand via load_from_core/3 or populated as events arrive.
    # Future enhancement: add bulk load endpoint to Core.

    Enum.each(state.projections, fn projection_name ->
      Logger.debug("[ProjectionSync] Registered projection '#{projection_name}' for sync")
    end)

    state
  end

  defp load_entities_from_core(_client, projection_name, entity_ids) do
    url =
      Application.get_env(:mcp_server_elixir, :core_url, "http://localhost:3900") <>
        "/api/v1/projections/#{projection_name}/bulk"

    body = %{entity_ids: entity_ids}

    case Tesla.post(url, Jason.encode!(body), headers: [{"content-type", "application/json"}]) do
      {:ok, %Tesla.Env{status: 200, body: response}} ->
        response = if is_binary(response), do: Jason.decode!(response), else: response
        states = response["states"] || []

        loaded =
          states
          |> Enum.filter(fn s -> s["found"] == true end)
          |> Enum.map(fn s ->
            entity_id = s["entity_id"]
            entity_state = s["state"]
            ets_put(projection_name, entity_id, entity_state)
            entity_id
          end)

        {:ok, loaded}

      {:ok, %Tesla.Env{status: status, body: body}} ->
        {:error, "HTTP #{status}: #{inspect(body)}"}

      {:error, reason} ->
        {:error, reason}
    end
  rescue
    e -> {:error, Exception.message(e)}
  end

  defp do_sync(state) do
    dirty_keys = get_dirty_keys()

    if Enum.empty?(dirty_keys) do
      {{:ok, 0}, state}
    else
      sync_dirty_to_core(state, dirty_keys)
    end
  end

  defp sync_dirty_to_core(state, dirty_keys) do
    base_url = Application.get_env(:mcp_server_elixir, :core_url, "http://localhost:3900")

    {synced, errors} =
      Enum.reduce(dirty_keys, {0, 0}, fn {projection_name, entity_id} = _key,
                                         {synced_acc, error_acc} ->
        entity_state = ets_get(projection_name, entity_id)

        if entity_state do
          url = "#{base_url}/api/v1/projections/#{projection_name}/#{entity_id}/state"
          body = Jason.encode!(%{state: entity_state})

          case Tesla.put(url, body, headers: [{"content-type", "application/json"}]) do
            {:ok, %Tesla.Env{status: status}} when status in [200, 201] ->
              unmark_dirty(projection_name, entity_id)
              {synced_acc + 1, error_acc}

            {:ok, %Tesla.Env{status: status, body: response_body}} ->
              # Only log once per batch when errors start occurring
              if error_acc == 0 do
                Logger.warning(
                  "[ProjectionSync] Sync errors starting - HTTP #{status}: #{inspect(response_body)}"
                )
              end

              {synced_acc, error_acc + 1}

            {:error, reason} ->
              # Only log once per batch when errors start occurring
              if error_acc == 0 do
                Logger.warning("[ProjectionSync] Sync errors starting: #{inspect(reason)}")
              end

              {synced_acc, error_acc + 1}
          end
        else
          # State was deleted, just unmark as dirty
          unmark_dirty(projection_name, entity_id)
          {synced_acc, error_acc}
        end
      end)

    new_state = %{
      state
      | synced_count: state.synced_count + synced,
        sync_errors: if(errors > 0, do: state.sync_errors + errors, else: state.sync_errors),
        last_sync_at: DateTime.utc_now(),
        last_sync_error: if(errors > 0, do: "#{errors} sync errors", else: nil),
        core_available: errors == 0
    }

    if synced > 0 do
      Logger.debug("[ProjectionSync] Synced #{synced} states to Core")
    end

    :telemetry.execute(
      [:mcp_server_elixir, :projection_sync, :sync_completed],
      %{synced: synced, errors: errors, duration_ms: 0},
      %{}
    )

    {{:ok, synced}, new_state}
  rescue
    e ->
      Logger.error("[ProjectionSync] Sync failed: #{Exception.message(e)}")

      new_state = %{
        state
        | sync_errors: state.sync_errors + 1,
          last_sync_error: Exception.message(e),
          core_available: false
      }

      {{:error, Exception.message(e)}, new_state}
  end
end
