defmodule QueryServiceExWeb.PrimeController do
  @moduledoc """
  Gateway for Prime's declarative projection + per-field provenance primitives.

  Proxies Core's internal `/api/v1/prime/*` projection routes so SDK/REST
  callers reach the same capabilities the MCP tools expose. Access is gated
  by the `:tenant_scoped` pipeline; Core itself stays internal-only.
  """

  use Phoenix.Controller, formats: [:json]

  alias QueryServiceEx.Infrastructure.Adapters.RustCoreClient

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
end
