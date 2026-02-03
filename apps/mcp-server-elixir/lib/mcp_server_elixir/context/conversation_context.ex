defmodule McpServerElixir.Context.ConversationContext do
  @moduledoc """
  Manages multi-turn conversational context for AI agents interacting with AllSource.

  This GenServer maintains QuerySession state per session ID, enabling:
  - Query refinement across turns (e.g., "Show users" → "Filter to premium tier")
  - Context accumulation for iterative exploration
  - Session timeout and automatic cleanup

  ## Example Usage

  ```
  # Turn 1: Initial query
  "Show users created yesterday"
  → Stores: entity_type: "user", time_filter: {since: "yesterday"}

  # Turn 2: Refinement
  "Filter to premium tier"
  → Merges: entity_type: "user", time_filter: {since: "yesterday"}, filters: {tier: "premium"}

  # Turn 3: Comparison
  "Compare with last month"
  → Builds comparison query using stored context
  ```
  """

  use GenServer
  require Logger

  @default_timeout_ms :timer.minutes(30)
  @cleanup_interval_ms :timer.minutes(5)

  # Session structure
  defmodule QuerySession do
    @moduledoc """
    Represents the accumulated context for a conversation session.
    """
    defstruct [
      :session_id,
      :created_at,
      :last_accessed,
      # Core query context
      entity_ids: [],
      entity_type: nil,
      event_types: [],
      # Time filters
      time_since: nil,
      time_until: nil,
      time_as_of: nil,
      # Search context
      semantic_query: nil,
      keywords: nil,
      # Metadata filters
      filters: %{},
      # Query history for this session
      query_history: [],
      # Last query result summary (for context)
      last_result_summary: nil
    ]

    @type t :: %__MODULE__{
            session_id: String.t(),
            created_at: DateTime.t(),
            last_accessed: DateTime.t(),
            entity_ids: [String.t()],
            entity_type: String.t() | nil,
            event_types: [String.t()],
            time_since: String.t() | nil,
            time_until: String.t() | nil,
            time_as_of: String.t() | nil,
            semantic_query: String.t() | nil,
            keywords: String.t() | nil,
            filters: map(),
            query_history: [map()],
            last_result_summary: map() | nil
          }
  end

  # Client API

  @doc """
  Starts the ConversationContext GenServer.
  """
  def start_link(opts \\ []) do
    name = Keyword.get(opts, :name, __MODULE__)
    timeout_ms = Keyword.get(opts, :timeout_ms, @default_timeout_ms)
    GenServer.start_link(__MODULE__, %{timeout_ms: timeout_ms}, name: name)
  end

  @doc """
  Creates or retrieves a session for the given session_id.
  Returns the current QuerySession state.
  """
  def get_or_create_session(server \\ __MODULE__, session_id) do
    GenServer.call(server, {:get_or_create_session, session_id})
  end

  @doc """
  Gets the current session state without creating a new one.
  Returns `{:ok, session}` or `{:error, :not_found}`.
  """
  def get_session(server \\ __MODULE__, session_id) do
    GenServer.call(server, {:get_session, session_id})
  end

  @doc """
  Updates a session with new query context.
  The context map can include any of the QuerySession fields.
  """
  def update_session(server \\ __MODULE__, session_id, context) do
    GenServer.call(server, {:update_session, session_id, context})
  end

  @doc """
  Builds a merged query based on the current session context and new refinements.
  This is the main entry point for multi-turn query building.

  ## Parameters
  - `session_id`: The session identifier
  - `refinement`: A map describing the query refinement (new filters, entities, etc.)

  ## Returns
  `{:ok, query_params}` - The merged query parameters ready for tool execution
  """
  def build_query(server \\ __MODULE__, session_id, refinement \\ %{}) do
    GenServer.call(server, {:build_query, session_id, refinement})
  end

  @doc """
  Records the result summary of a query execution for future context.
  """
  def record_result(server \\ __MODULE__, session_id, result_summary) do
    GenServer.call(server, {:record_result, session_id, result_summary})
  end

  @doc """
  Clears a specific session.
  """
  def clear_session(server \\ __MODULE__, session_id) do
    GenServer.call(server, {:clear_session, session_id})
  end

  @doc """
  Lists all active sessions (for debugging/monitoring).
  """
  def list_sessions(server \\ __MODULE__) do
    GenServer.call(server, :list_sessions)
  end

  @doc """
  Gets session statistics.
  """
  def get_stats(server \\ __MODULE__) do
    GenServer.call(server, :get_stats)
  end

  # Server Callbacks

  @impl true
  def init(%{timeout_ms: timeout_ms}) do
    # Schedule periodic cleanup
    schedule_cleanup()

    {:ok,
     %{
       sessions: %{},
       timeout_ms: timeout_ms,
       created_at: DateTime.utc_now()
     }}
  end

  @impl true
  def handle_call({:get_or_create_session, session_id}, _from, state) do
    {session, new_state} = ensure_session(state, session_id)
    {:reply, {:ok, session}, new_state}
  end

  @impl true
  def handle_call({:get_session, session_id}, _from, state) do
    case Map.get(state.sessions, session_id) do
      nil ->
        {:reply, {:error, :not_found}, state}

      session ->
        updated_session = %{session | last_accessed: DateTime.utc_now()}
        new_sessions = Map.put(state.sessions, session_id, updated_session)
        {:reply, {:ok, updated_session}, %{state | sessions: new_sessions}}
    end
  end

  @impl true
  def handle_call({:update_session, session_id, context}, _from, state) do
    {session, state} = ensure_session(state, session_id)
    updated_session = merge_context(session, context)
    new_sessions = Map.put(state.sessions, session_id, updated_session)
    {:reply, {:ok, updated_session}, %{state | sessions: new_sessions}}
  end

  @impl true
  def handle_call({:build_query, session_id, refinement}, _from, state) do
    {session, state} = ensure_session(state, session_id)

    # Merge refinement into session
    merged_session = merge_context(session, refinement)

    # Build query parameters from merged context
    query_params = session_to_query_params(merged_session)

    # Record this query in history
    history_entry = %{
      timestamp: DateTime.utc_now(),
      refinement: refinement,
      query_params: query_params
    }

    final_session = %{
      merged_session
      | query_history: [history_entry | Enum.take(merged_session.query_history, 9)]
    }

    new_sessions = Map.put(state.sessions, session_id, final_session)
    {:reply, {:ok, query_params}, %{state | sessions: new_sessions}}
  end

  @impl true
  def handle_call({:record_result, session_id, result_summary}, _from, state) do
    case Map.get(state.sessions, session_id) do
      nil ->
        {:reply, {:error, :not_found}, state}

      session ->
        updated_session = %{
          session
          | last_result_summary: result_summary,
            last_accessed: DateTime.utc_now()
        }

        new_sessions = Map.put(state.sessions, session_id, updated_session)
        {:reply, :ok, %{state | sessions: new_sessions}}
    end
  end

  @impl true
  def handle_call({:clear_session, session_id}, _from, state) do
    new_sessions = Map.delete(state.sessions, session_id)
    {:reply, :ok, %{state | sessions: new_sessions}}
  end

  @impl true
  def handle_call(:list_sessions, _from, state) do
    session_summaries =
      state.sessions
      |> Enum.map(fn {id, session} ->
        %{
          session_id: id,
          created_at: session.created_at,
          last_accessed: session.last_accessed,
          query_count: length(session.query_history),
          has_context: has_context?(session)
        }
      end)

    {:reply, session_summaries, state}
  end

  @impl true
  def handle_call(:get_stats, _from, state) do
    stats = %{
      active_sessions: map_size(state.sessions),
      server_uptime_seconds: DateTime.diff(DateTime.utc_now(), state.created_at),
      timeout_minutes: div(state.timeout_ms, 60_000)
    }

    {:reply, stats, state}
  end

  @impl true
  def handle_info(:cleanup_sessions, state) do
    now = DateTime.utc_now()
    timeout_seconds = div(state.timeout_ms, 1000)

    expired_ids =
      state.sessions
      |> Enum.filter(fn {_id, session} ->
        DateTime.diff(now, session.last_accessed) > timeout_seconds
      end)
      |> Enum.map(fn {id, _} -> id end)

    if length(expired_ids) > 0 do
      Logger.info("ConversationContext: Cleaning up #{length(expired_ids)} expired sessions")
    end

    new_sessions = Map.drop(state.sessions, expired_ids)

    schedule_cleanup()
    {:noreply, %{state | sessions: new_sessions}}
  end

  # Private Helpers

  defp ensure_session(state, session_id) do
    case Map.get(state.sessions, session_id) do
      nil ->
        now = DateTime.utc_now()

        session = %QuerySession{
          session_id: session_id,
          created_at: now,
          last_accessed: now
        }

        new_sessions = Map.put(state.sessions, session_id, session)
        {session, %{state | sessions: new_sessions}}

      existing ->
        updated = %{existing | last_accessed: DateTime.utc_now()}
        new_sessions = Map.put(state.sessions, session_id, updated)
        {updated, %{state | sessions: new_sessions}}
    end
  end

  defp merge_context(session, context) when is_map(context) do
    now = DateTime.utc_now()

    # Handle special merge logic for different fields
    merged =
      Enum.reduce(context, session, fn {key, value}, acc ->
        case key do
          # Entity IDs accumulate unless explicitly replaced
          :entity_ids when is_list(value) ->
            if Map.get(context, :replace_entity_ids, false) do
              %{acc | entity_ids: value}
            else
              %{acc | entity_ids: Enum.uniq(acc.entity_ids ++ value)}
            end

          :entity_id when is_binary(value) ->
            if Map.get(context, :replace_entity_ids, false) do
              %{acc | entity_ids: [value]}
            else
              %{acc | entity_ids: Enum.uniq(acc.entity_ids ++ [value])}
            end

          # Event types accumulate
          :event_types when is_list(value) ->
            %{acc | event_types: Enum.uniq(acc.event_types ++ value)}

          :event_type when is_binary(value) ->
            %{acc | event_types: Enum.uniq(acc.event_types ++ [value])}

          # Filters merge
          :filters when is_map(value) ->
            %{acc | filters: Map.merge(acc.filters, value)}

          # Time filters replace (latest wins)
          :time_since ->
            %{acc | time_since: value}

          :time_until ->
            %{acc | time_until: value}

          :time_as_of ->
            %{acc | time_as_of: value}

          # Convenience aliases for time filters
          :since ->
            %{acc | time_since: value}

          :until ->
            %{acc | time_until: value}

          :as_of ->
            %{acc | time_as_of: value}

          # Search context replaces
          :semantic_query ->
            %{acc | semantic_query: value}

          :keywords ->
            %{acc | keywords: value}

          # Entity type replaces
          :entity_type ->
            %{acc | entity_type: value}

          # Skip internal flags
          :replace_entity_ids ->
            acc

          # For any other keys, try to set them if they exist in the struct
          _ ->
            if Map.has_key?(acc, key) do
              Map.put(acc, key, value)
            else
              acc
            end
        end
      end)

    %{merged | last_accessed: now}
  end

  defp session_to_query_params(session) do
    params = %{}

    # Add entity context
    params =
      case session.entity_ids do
        [single_id] -> Map.put(params, "entity_id", single_id)
        ids when length(ids) > 1 -> Map.put(params, "entity_ids", ids)
        _ -> params
      end

    # Add event type context
    params =
      case session.event_types do
        [single_type] -> Map.put(params, "event_type", single_type)
        types when length(types) > 1 -> Map.put(params, "event_types", types)
        _ -> params
      end

    # Add time filters
    params = if session.time_since, do: Map.put(params, "since", session.time_since), else: params
    params = if session.time_until, do: Map.put(params, "until", session.time_until), else: params
    params = if session.time_as_of, do: Map.put(params, "as_of", session.time_as_of), else: params

    # Add search context
    params =
      if session.semantic_query,
        do: Map.put(params, "semantic_query", session.semantic_query),
        else: params

    params = if session.keywords, do: Map.put(params, "keywords", session.keywords), else: params

    # Add filters if present (convert atom keys to strings for JSON compatibility)
    params =
      if map_size(session.filters) > 0 do
        string_filters = stringify_keys(session.filters)
        Map.put(params, "filters", string_filters)
      else
        params
      end

    params
  end

  defp has_context?(session) do
    length(session.entity_ids) > 0 or
      session.entity_type != nil or
      length(session.event_types) > 0 or
      session.time_since != nil or
      session.time_until != nil or
      session.semantic_query != nil or
      map_size(session.filters) > 0
  end

  defp schedule_cleanup do
    Process.send_after(self(), :cleanup_sessions, @cleanup_interval_ms)
  end

  defp stringify_keys(map) when is_map(map) do
    Map.new(map, fn
      {k, v} when is_atom(k) -> {Atom.to_string(k), stringify_keys(v)}
      {k, v} -> {k, stringify_keys(v)}
    end)
  end

  defp stringify_keys(list) when is_list(list) do
    Enum.map(list, &stringify_keys/1)
  end

  defp stringify_keys(value), do: value
end
