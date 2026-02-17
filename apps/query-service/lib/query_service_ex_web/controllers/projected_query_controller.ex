defmodule QueryServiceExWeb.ProjectedQueryController do
  @moduledoc """
  Controller for projected queries with dual read path.

  When a ProjectionServer is running for the requested projection and
  consistency is not `:strong`, reads from ETS (sub-millisecond).
  Otherwise falls back to fold-on-read via FoldPipeline.
  """

  use Phoenix.Controller, formats: [:json]

  require Logger

  alias QueryServiceEx.Projections.FoldPipeline
  alias QueryServiceEx.Projections.ProjectionServer
  alias QueryServiceEx.Projections.Registry

  action_fallback(QueryServiceExWeb.FallbackController)

  @default_page 1
  @default_page_size 50

  def execute(conn, params) do
    tenant_id = get_tenant_id!(conn)
    consistency = conn.assigns[:consistency]

    with {:ok, projection_name, projection_mod} <- resolve_projection(params),
         {:ok, projected, metadata} <- read_projected(projection_name, projection_mod, tenant_id, consistency) do
      filtered = apply_filters(projected, params["filters"] || %{}, projection_mod)
      {page, result} = paginate(filtered, params)

      json(conn, %{
        data: result,
        count: length(result),
        total: length(filtered),
        projection: projection_name,
        folded_from: metadata,
        page: page.number,
        page_size: page.size
      })
    end
  end

  defp read_projected(projection_name, projection_mod, tenant_id, consistency) do
    if consistency != :strong && projection_server_running?(projection_name) do
      {states, metadata} = read_from_ets(projection_name)
      {:ok, states, metadata}
    else
      fold_on_read(projection_mod, tenant_id, consistency)
    end
  end

  defp projection_server_running?(projection_name) do
    via = {:via, Elixir.Registry, {QueryServiceEx.ProjectionRegistry, {:projection_server, projection_name}}}

    case GenServer.whereis(via) do
      nil -> false
      pid when is_pid(pid) -> Process.alive?(pid)
    end
  end

  defp read_from_ets(projection_name) do
    via = {:via, Elixir.Registry, {QueryServiceEx.ProjectionRegistry, {:projection_server, projection_name}}}
    start_time = System.monotonic_time(:millisecond)
    states = ProjectionServer.get_all_states(via)
    duration = System.monotonic_time(:millisecond) - start_time

    Logger.debug("ProjectedQuery[#{projection_name}] ETS read: #{length(states)} entities in #{duration}ms")

    metadata = %{
      source: "ets",
      fold_duration_ms: duration / 1.0,
      event_count: 0,
      snapshot_used: false,
      events_after_snapshot: 0,
      snapshot_created: false
    }

    {states, metadata}
  end

  defp fold_on_read(projection_mod, tenant_id, consistency) do
    case FoldPipeline.fold(projection_mod, nil,
           tenant_id: tenant_id,
           consistency: consistency
         ) do
      {:ok, %{state: projected, metadata: metadata}} ->
        {:ok, projected, Map.put(metadata, :source, "fold_on_read")}

      {:error, reason} ->
        {:error, reason}
    end
  end

  defp resolve_projection(%{"projection" => name}) when is_binary(name) do
    case Registry.get(name) do
      nil ->
        {:error, :bad_request,
         "Unknown projection '#{name}'. Available: #{inspect(Registry.list())}"}

      mod ->
        {:ok, name, mod}
    end
  end

  defp resolve_projection(_params) do
    {:error, :bad_request, "Missing required field 'projection'"}
  end

  defp apply_filters(projected, filters, projection_mod) when map_size(filters) == 0 do
    _ = projection_mod
    projected
  end

  defp apply_filters(projected, filters, projection_mod) do
    allowed = MapSet.new(projection_mod.filterable_fields())

    valid_filters =
      filters
      |> Enum.filter(fn {key, _val} -> MapSet.member?(allowed, key) end)
      |> Map.new()

    Enum.filter(projected, fn state ->
      Enum.all?(valid_filters, fn {key, value} ->
        Map.get(state, key) == value
      end)
    end)
  end

  defp paginate(items, params) do
    page_number = parse_int(params["page"], @default_page)
    page_size = parse_int(params["page_size"], @default_page_size)

    result =
      items
      |> maybe_sort(params["sort_by"], params["sort_order"])
      |> Enum.drop((page_number - 1) * page_size)
      |> Enum.take(page_size)

    {%{number: page_number, size: page_size}, result}
  end

  defp maybe_sort(items, nil, _order), do: items

  defp maybe_sort(items, sort_by, sort_order) do
    sorter =
      case sort_order do
        "desc" -> &>=/2
        _ -> &<=/2
      end

    Enum.sort_by(items, &Map.get(&1, sort_by), sorter)
  end

  defp parse_int(nil, default), do: default
  defp parse_int(val, _default) when is_integer(val), do: val

  defp parse_int(val, default) when is_binary(val) do
    case Integer.parse(val) do
      {n, _} -> n
      :error -> default
    end
  end

  defp parse_int(_, default), do: default

  defp get_tenant_id!(conn) do
    case conn.assigns[:tenant_id] do
      nil ->
        raise "Tenant context required but not present - security violation"

      tenant_id when is_binary(tenant_id) ->
        tenant_id
    end
  end
end
