defmodule QueryServiceExWeb.ProjectionController do
  @moduledoc """
  Tenant-scoped, QS-owned per-tenant projections.

  Projections are a per-tenant, Query-Service-owned feature
  (`docs/proposals/PER_TENANT_PROJECTIONS.md`): a tenant enables curated
  *templates* from a catalog, and QS folds the tenant's (already isolated) event
  stream into a per-tenant read-model. The durable enabled set lives in Core
  tenant metadata as an opaque blob; Core's global projection engine is NOT
  touched and is never surfaced here.

  Every action is tenant-scoped and **fails closed**: no tenant on the conn → a
  security violation is raised (never cross-tenant data). This mirrors
  `EventController.get_tenant_id!/1`.
  """

  use Phoenix.Controller, formats: [:json]
  use OpenApiSpex.ControllerSpecs

  require Logger

  alias QueryServiceEx.Projections.Catalog
  alias QueryServiceEx.Projections.Enablement
  alias QueryServiceEx.Projections.TenantProjections
  alias QueryServiceExWeb.Schemas.Common

  action_fallback(QueryServiceExWeb.FallbackController)

  tags(["Projections"])

  operation(:index,
    summary: "List enabled projections",
    description: "List the authenticated tenant's enabled projections with status.",
    security: [%{"bearer_auth" => []}],
    responses: [
      ok: {"Enabled projections", "application/json", Common.SimpleSuccess},
      bad_request: {"Bad request", "application/json", Common.SimpleError}
    ]
  )

  @doc """
  List the tenant's ENABLED projections (with status). This is the count the
  dashboard reads. Shape: `{"projections": [...], "total": N}`.
  """
  def index(conn, _params) do
    tenant_id = get_tenant_id!(conn)

    case Enablement.list(tenant_id) do
      {:ok, projections} ->
        json(conn, %{projections: projections, total: length(projections)})

      {:error, reason} ->
        Logger.warning("[ProjectionController] index failed: #{inspect(reason)}",
          tenant_id: tenant_id
        )

        conn
        |> put_status(:bad_request)
        |> json(%{error: to_string_reason(reason)})
    end
  end

  operation(:templates,
    summary: "List projection templates",
    description: "List the curated projection template catalog.",
    security: [%{"bearer_auth" => []}],
    responses: [
      ok: {"Template catalog", "application/json", Common.SimpleSuccess}
    ]
  )

  @doc """
  List the curated template catalog (platform-defined, not tenant-scoped data —
  but still served only to authenticated tenants).
  """
  def templates(conn, _params) do
    _tenant_id = get_tenant_id!(conn)

    templates =
      Enum.map(Catalog.list(), fn t ->
        %{name: t.name, title: t.title, description: t.description}
      end)

    json(conn, %{templates: templates, total: length(templates)})
  end

  operation(:create,
    summary: "Enable a projection",
    description: "Enable a projection template for the tenant (returns status building).",
    security: [%{"bearer_auth" => []}],
    responses: [
      created: {"Projection enabled", "application/json", Common.SimpleSuccess},
      not_found: {"Unknown template", "application/json", Common.SimpleError},
      unprocessable_entity: {"Validation error", "application/json", Common.SimpleError}
    ]
  )

  @doc """
  Enable a projection from the catalog for the tenant.

  Body: `{"template": "event-count"}`. Returns the projection with status
  `"building"` while the background backfill runs. Unknown templates → 422.
  """
  def create(conn, params) do
    tenant_id = get_tenant_id!(conn)
    template = params["template"] || params["name"]

    cond do
      not is_binary(template) or template == "" ->
        conn
        |> put_status(:unprocessable_entity)
        |> json(%{error: "template is required"})

      not Catalog.valid?(template) ->
        conn
        |> put_status(:not_found)
        |> json(%{error: "Unknown projection template: #{template}"})

      true ->
        case Enablement.enable(tenant_id, template) do
          {:ok, projection} ->
            conn
            |> put_status(:created)
            |> json(%{projection: projection})

          {:error, :unknown_template} ->
            conn
            |> put_status(:not_found)
            |> json(%{error: "Unknown projection template: #{template}"})

          {:error, reason} ->
            conn
            |> put_status(:bad_request)
            |> json(%{error: to_string_reason(reason)})
        end
    end
  end

  operation(:delete,
    summary: "Disable a projection",
    description: "Disable a projection: remove it from the enabled set and drop its state.",
    security: [%{"bearer_auth" => []}],
    parameters: [
      name: [in: :path, type: :string, description: "Projection name", required: true]
    ],
    responses: [
      ok: {"Projection disabled", "application/json", Common.SimpleSuccess},
      bad_request: {"Bad request", "application/json", Common.SimpleError}
    ]
  )

  @doc """
  Disable a projection: remove it from the enabled set + drop the tenant's state.
  """
  def delete(conn, %{"name" => name}) do
    tenant_id = get_tenant_id!(conn)

    case Enablement.disable(tenant_id, name) do
      {:ok, _name} ->
        json(conn, %{deleted: name})

      {:error, reason} ->
        conn
        |> put_status(:bad_request)
        |> json(%{error: to_string_reason(reason)})
    end
  end

  operation(:get_state,
    summary: "Get projection state",
    description: "Get the tenant-scoped folded state for a projection entity.",
    security: [%{"bearer_auth" => []}],
    parameters: [
      name: [in: :path, type: :string, description: "Projection name", required: true],
      entity_id: [in: :query, type: :string, description: "Entity id", required: false]
    ],
    responses: [
      ok: {"Projection state", "application/json", Common.SimpleSuccess},
      not_found: {"Not enabled / no state", "application/json", Common.SimpleError}
    ]
  )

  @doc """
  Get tenant-scoped folded state for a projection.

  Requires `?entity_id=...`. Returns 404 if the projection is not enabled for the
  tenant or there is no state for that entity.
  """
  def get_state(conn, %{"name" => name} = params) do
    tenant_id = get_tenant_id!(conn)
    entity_id = params["entity_id"] || Catalog.tenant_key()

    with {:ok, enabled} <- Enablement.enabled_set(tenant_id),
         true <- name in enabled,
         {:ok, state} <- TenantProjections.get_state(tenant_id, name, entity_id) do
      json(conn, %{
        projection: name,
        entity_id: entity_id,
        status: status_string(tenant_id, name),
        state: state
      })
    else
      false ->
        conn
        |> put_status(:not_found)
        |> json(%{error: "Projection not enabled for tenant: #{name}"})

      {:error, :not_found} ->
        conn
        |> put_status(:not_found)
        |> json(%{error: "No state for entity"})

      {:error, reason} ->
        conn
        |> put_status(:bad_request)
        |> json(%{error: to_string_reason(reason)})
    end
  end

  # Gets tenant_id from connection, raises if not present (security check).
  # Mirrors EventController.get_tenant_id!/1 — fail closed.
  defp get_tenant_id!(conn) do
    case conn.assigns[:tenant_id] do
      tenant_id when is_binary(tenant_id) ->
        tenant_id

      _ ->
        raise "Tenant context required but not present - security violation"
    end
  end

  defp status_string(tenant_id, name) do
    case TenantProjections.status(tenant_id, name) do
      nil -> "building"
      status -> Atom.to_string(status)
    end
  end

  defp to_string_reason(reason) when is_binary(reason), do: reason
  defp to_string_reason(reason), do: inspect(reason)
end
