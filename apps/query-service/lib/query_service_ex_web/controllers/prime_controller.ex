defmodule QueryServiceExWeb.PrimeController do
  @moduledoc """
  Gateway for Prime's declarative projection + per-field provenance primitives.

  Proxies Core's internal `/api/v1/prime/*` projection routes so SDK/REST
  callers reach the same capabilities the MCP tools expose. Access is gated
  by the `:tenant_scoped` pipeline; Core itself stays internal-only.
  """

  use Phoenix.Controller, formats: [:json]

  alias QueryServiceEx.Infrastructure.Adapters.RustCoreClient
  alias QueryServiceEx.Prime.GraphFold

  # Upper bound on prime.* events folded per graph read. The graph is
  # materialized server-side from the tenant's events; this caps the read so a
  # pathological tenant can't OOM the gateway. The `?limit` on nodes is applied
  # after the fold.
  @event_scan_limit 10_000

  @doc """
  GET /api/v1/prime/graph — full materialized Prime knowledge graph for the
  authenticated tenant.

  Synced tenant memory (`prime.*` events from allsource-prime →
  control-plane → `/api/v1/events`) lives **tenant-scoped in the MAIN event
  store**, not in Core's single-tenant embedded Prime store. So we build the
  hosted graph by reading this tenant's `prime.*` events (the SAME tenant-scoped
  main-store read the Memory tab uses) and folding them into the full-graph
  contract — instead of forwarding to Core's embedded `/api/v1/prime/graph`,
  which would return empty for real tenants.

  The tenant is derived from the authenticated caller (`conn.assigns`), NEVER
  from client-supplied params — the same path the events endpoint uses — so this
  can never read another tenant's memory.

  Optional query params: `node_type`, `limit` (applied to the materialized
  node set, mirroring 012's contract).
  """
  def graph(conn, params) do
    tenant_id = get_tenant_id!(conn)
    consistency = conn.assigns[:consistency]

    query = %{event_type_prefix: "prime.", limit: @event_scan_limit}

    case RustCoreClient.query_events(tenant_id, query, consistency: consistency) do
      {:ok, events} when is_list(events) ->
        opts =
          []
          |> maybe_opt(:node_type, params["node_type"])
          |> maybe_opt(:limit, params["limit"])

        json(conn, GraphFold.fold(events, opts))

      {:error, reason} ->
        conn |> put_status(:bad_gateway) |> json(%{error: to_string(reason)})
    end
  end

  @doc "GET /api/v1/prime/projections — list registered projection definitions"
  def index(conn, _params) do
    case RustCoreClient.list_prime_projections() do
      {:ok, projections} ->
        json(conn, %{data: projections, count: length_of(projections)})

      {:error, reason} ->
        conn |> put_status(:bad_gateway) |> json(%{error: to_string(reason)})
    end
  end

  @doc "POST /api/v1/prime/projections — define or replace a projection"
  def create(conn, params) do
    case RustCoreClient.create_prime_projection(projection_params(params)) do
      {:ok, created} ->
        conn |> put_status(:created) |> json(%{data: created})

      {:error, reason} ->
        conn |> put_status(:unprocessable_entity) |> json(%{error: to_string(reason)})
    end
  end

  @doc "POST /api/v1/prime/nodes/:id/project — fold a node into a snapshot"
  def project(conn, %{"id" => node_id}) do
    case RustCoreClient.project_node(node_id) do
      {:ok, snapshot} ->
        json(conn, %{data: snapshot})

      {:error, reason} ->
        conn |> put_status(:bad_gateway) |> json(%{error: to_string(reason)})
    end
  end

  @doc "GET /api/v1/prime/nodes/:id/fields/:field/provenance — per-field source"
  def provenance(conn, %{"id" => node_id, "field" => field}) do
    case RustCoreClient.node_field_provenance(node_id, field) do
      {:ok, prov} ->
        json(conn, %{data: prov})

      {:error, :not_found} ->
        conn
        |> put_status(:not_found)
        |> json(%{error: "no provenance for field '#{field}' on #{node_id}"})

      {:error, reason} ->
        conn |> put_status(:bad_gateway) |> json(%{error: to_string(reason)})
    end
  end

  # Core's define endpoint reads {entity_type, field_policies}. Pass them
  # through untouched; Core validates the merge-policy strings.
  defp projection_params(params) do
    %{
      "entity_type" => params["entity_type"],
      "field_policies" => params["field_policies"] || %{}
    }
  end

  defp length_of(list) when is_list(list), do: length(list)
  defp length_of(_), do: 0

  defp maybe_opt(opts, _key, nil), do: opts
  defp maybe_opt(opts, _key, ""), do: opts
  defp maybe_opt(opts, key, value), do: Keyword.put(opts, key, value)

  # Derive tenant from the authenticated caller; raise (caught by the
  # :tenant_scoped pipeline / fallback) if absent — a missing tenant is a
  # security violation, never a silent full-graph read.
  defp get_tenant_id!(conn) do
    case conn.assigns[:tenant_id] do
      tenant_id when is_binary(tenant_id) ->
        tenant_id

      _ ->
        raise "Tenant context required but not present - security violation"
    end
  end
end
