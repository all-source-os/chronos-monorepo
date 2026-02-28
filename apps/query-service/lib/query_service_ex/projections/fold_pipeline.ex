defmodule QueryServiceEx.Projections.FoldPipeline do
  @moduledoc """
  Snapshot-aware fold-on-read pipeline for server-side projections.

  Checks Core for existing snapshots before fetching events. When a snapshot
  exists, uses it as the initial accumulator and only fetches events after
  the snapshot timestamp — reducing fold cost from N events to a delta.
  """

  alias QueryServiceEx.Infrastructure.Adapters.RustCoreClient

  require Logger

  @default_snapshot_threshold 100

  @type fold_metadata :: %{
          event_count: non_neg_integer(),
          snapshot_used: boolean(),
          events_after_snapshot: non_neg_integer(),
          fold_duration_ms: float(),
          snapshot_created: boolean()
        }

  @type fold_result :: {:ok, %{state: [map()], metadata: fold_metadata()}} | {:error, term()}

  @doc """
  Fold events for a projection, using snapshots when available.

  ## Options
    * `:tenant_id` — required, tenant context for Core queries
    * `:consistency` — optional, forwarded to Core client
    * `:core_client` — optional, module implementing `list_snapshots/1` and `query_events/3` (default: RustCoreClient)
  """
  @spec fold(module(), String.t() | nil, keyword()) :: fold_result()
  def fold(projection_mod, entity_id, opts \\ []) do
    tenant_id = Keyword.fetch!(opts, :tenant_id)
    consistency = Keyword.get(opts, :consistency)
    core_client = Keyword.get(opts, :core_client, RustCoreClient)
    start_time = System.monotonic_time(:millisecond)

    {initial_states, since_timestamp} = load_snapshots(core_client, entity_id)

    with {:ok, events} <-
           fetch_events(
             core_client,
             tenant_id,
             projection_mod,
             entity_id,
             since_timestamp,
             consistency
           ) do
      projected = fold_events(events, projection_mod, initial_states)
      duration = System.monotonic_time(:millisecond) - start_time
      events_after_snapshot = length(events)

      snapshot_created =
        maybe_create_snapshots(
          core_client,
          projection_mod,
          events,
          projected,
          events_after_snapshot,
          opts
        )

      {:ok,
       %{
         state: projected,
         metadata: %{
           event_count: events_after_snapshot + snapshot_event_count(initial_states),
           snapshot_used: map_size(initial_states) > 0,
           events_after_snapshot: events_after_snapshot,
           fold_duration_ms: duration / 1.0,
           snapshot_created: snapshot_created
         }
       }}
    end
  end

  defp snapshot_threshold do
    Application.get_env(:query_service_ex, :snapshot_threshold, @default_snapshot_threshold)
  end

  defp maybe_create_snapshots(
         _core_client,
         _projection_mod,
         _events,
         _projected,
         events_after_snapshot,
         _opts
       )
       when events_after_snapshot <= 0 do
    false
  end

  defp maybe_create_snapshots(
         core_client,
         projection_mod,
         events,
         projected,
         events_after_snapshot,
         _opts
       ) do
    if events_after_snapshot > snapshot_threshold() do
      projection_name = projection_mod.entity_type()

      # Build entity_id -> state map from projected results
      entity_states =
        events
        |> Enum.map(& &1["entity_id"])
        |> Enum.uniq()
        |> Enum.map(fn eid ->
          state = Enum.find(projected, fn s -> s["id"] == eid end)
          {eid, state}
        end)
        |> Enum.reject(fn {_eid, state} -> is_nil(state) end)

      Task.start(fn ->
        for {entity_id, state} <- entity_states do
          try do
            core_client.create_snapshot(entity_id, "automatic")
            core_client.save_projection_state(projection_name, entity_id, state)
          rescue
            e ->
              Logger.warning(
                "Failed to create lazy snapshot for #{projection_name}/#{entity_id}: #{inspect(e)}"
              )
          catch
            kind, reason ->
              Logger.warning(
                "Failed to create lazy snapshot for #{projection_name}/#{entity_id}: #{inspect(kind)} #{inspect(reason)}"
              )
          end
        end
      end)

      true
    else
      false
    end
  end

  defp load_snapshots(core_client, entity_id) do
    case core_client.list_snapshots(entity_id) do
      {:ok, [_ | _] = snapshots} ->
        build_snapshot_accumulators(snapshots)

      {:ok, %{"snapshots" => [_ | _] = snapshots}} ->
        build_snapshot_accumulators(snapshots)

      _ ->
        {%{}, nil}
    end
  end

  defp build_snapshot_accumulators(snapshots) do
    by_entity =
      snapshots
      |> Enum.group_by(& &1["entity_id"])
      |> Enum.map(fn {eid, snaps} ->
        latest = Enum.max_by(snaps, & &1["as_of"], fn -> nil end)
        {eid, latest}
      end)
      |> Enum.reject(fn {_eid, snap} -> is_nil(snap) end)
      |> Map.new()

    initial_states =
      by_entity
      |> Enum.map(fn {eid, snap} -> {eid, snap["state"] || %{}} end)
      |> Map.new()

    oldest_as_of =
      by_entity
      |> Map.values()
      |> Enum.map(& &1["as_of"])
      |> Enum.reject(&is_nil/1)
      |> Enum.min(fn -> nil end)

    {initial_states, oldest_as_of}
  end

  defp fetch_events(
         core_client,
         tenant_id,
         projection_mod,
         entity_id,
         since_timestamp,
         consistency
       ) do
    entity_type = projection_mod.entity_type()

    query_params =
      %{event_type: entity_type}
      |> maybe_put(:entity_id, entity_id)
      |> maybe_put(:since, since_timestamp)

    core_client.query_events(tenant_id, query_params, consistency: consistency)
  end

  defp fold_events(events, projection_mod, initial_states) do
    events
    |> Enum.group_by(& &1["entity_id"])
    |> Enum.map(fn {eid, entity_events} ->
      initial = Map.get(initial_states, eid, projection_mod.initial_state())

      entity_events
      |> Enum.sort_by(& &1["timestamp"])
      |> Enum.reduce(initial, fn event, state ->
        projection_mod.apply_event(state, event)
      end)
    end)
    |> then(fn folded ->
      snapshot_only_entities =
        initial_states
        |> Enum.reject(fn {eid, _state} ->
          Enum.any?(events, &(&1["entity_id"] == eid))
        end)
        |> Enum.map(fn {_eid, state} -> state end)

      folded ++ snapshot_only_entities
    end)
  end

  defp snapshot_event_count(initial_states) when map_size(initial_states) == 0, do: 0
  defp snapshot_event_count(initial_states), do: map_size(initial_states)

  defp maybe_put(map, _key, nil), do: map
  defp maybe_put(map, key, value), do: Map.put(map, key, value)
end
