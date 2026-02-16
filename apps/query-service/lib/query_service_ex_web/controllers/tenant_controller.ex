defmodule QueryServiceExWeb.TenantController do
  @moduledoc """
  Controller for tenant management operations.

  Provides endpoints for viewing and managing tenant information,
  usage statistics, and settings. Tenant data is fetched from Core
  and cached locally via TenantCache.
  """

  use Phoenix.Controller, formats: [:json]
  use OpenApiSpex.ControllerSpecs

  alias QueryServiceEx.DevMode
  alias QueryServiceEx.Infrastructure.Adapters.RustCoreClient
  alias QueryServiceEx.TenantCache
  alias QueryServiceExWeb.Schemas.Common
  alias QueryServiceExWeb.Schemas.Tenant

  require Logger

  action_fallback(QueryServiceExWeb.FallbackController)

  tags(["Tenant"])

  operation(:show,
    summary: "Get tenant",
    description: "Returns the current user's tenant workspace information.",
    security: [%{"bearer_auth" => []}],
    responses: [
      ok: {"Tenant info", "application/json", Tenant.TenantResponse},
      not_found: {"Tenant not found", "application/json", Common.Error}
    ]
  )

  @doc """
  Returns the current user's tenant information.

  GET /api/tenant
  """
  def show(conn, _params) do
    if DevMode.auth_disabled?() do
      tenant = conn.assigns[:current_tenant] || DevMode.dev_tenant()

      conn
      |> put_status(:ok)
      |> json(%{data: serialize_tenant(tenant)})
    else
      user = conn.assigns[:current_user]
      tenant_id = user["tenant_id"] || user.tenant_id

      case fetch_tenant(tenant_id) do
        {:ok, tenant} ->
          conn
          |> put_status(:ok)
          |> json(%{data: serialize_tenant(tenant)})

        {:error, _reason} ->
          conn
          |> put_status(:not_found)
          |> json(%{error: %{code: "tenant_not_found", message: "Workspace not found"}})
      end
    end
  end

  operation(:update,
    summary: "Update tenant",
    description:
      "Updates the current user's tenant settings. Only name and settings can be updated.",
    security: [%{"bearer_auth" => []}],
    request_body:
      {"Tenant updates", "application/json", Tenant.UpdateTenantRequest, required: true},
    responses: [
      ok: {"Tenant updated", "application/json", Tenant.TenantResponse},
      unprocessable_entity: {"Validation error", "application/json", Common.Error}
    ]
  )

  @doc """
  Updates the current user's tenant settings.

  PUT /api/tenant

  Note: Tenant updates (name, settings) are now managed by the Control Plane.
  This endpoint returns the current cached tenant state. To update tenant
  settings, use the Control Plane's tenant update endpoint.
  """
  def update(conn, _params) do
    if DevMode.auth_disabled?() do
      # Dev/standalone mode: can't persist, return current tenant
      tenant = conn.assigns[:current_tenant] || DevMode.dev_tenant()

      conn
      |> put_status(:ok)
      |> json(%{data: serialize_tenant(tenant)})
    else
      user = conn.assigns[:current_user]
      tenant_id = user["tenant_id"] || user.tenant_id

      case fetch_tenant(tenant_id) do
        {:ok, tenant} ->
          conn
          |> put_status(:ok)
          |> json(%{data: serialize_tenant(tenant)})

        {:error, _reason} ->
          conn
          |> put_status(:not_found)
          |> json(%{error: %{code: "tenant_not_found", message: "Workspace not found"}})
      end
    end
  end

  operation(:usage,
    summary: "Get usage statistics",
    description: "Returns usage statistics (events and queries) for the current billing period.",
    security: [%{"bearer_auth" => []}],
    responses: [
      ok: {"Usage statistics", "application/json", Tenant.UsageResponse}
    ]
  )

  @doc """
  Returns usage statistics for the current tenant.

  GET /api/tenant/usage
  """
  def usage(conn, _params) do
    if DevMode.auth_disabled?() do
      tenant = conn.assigns[:current_tenant] || DevMode.dev_tenant()

      conn
      |> put_status(:ok)
      |> json(%{
        data: build_usage_response(tenant)
      })
    else
      user = conn.assigns[:current_user]
      tenant_id = user["tenant_id"] || user.tenant_id

      case fetch_tenant(tenant_id) do
        {:ok, tenant} ->
          conn
          |> put_status(:ok)
          |> json(%{data: build_usage_response(tenant)})

        {:error, _reason} ->
          conn
          |> put_status(:not_found)
          |> json(%{error: %{code: "tenant_not_found", message: "Workspace not found"}})
      end
    end
  end

  # -------------------------------------------------------------------
  # Private Helpers
  # -------------------------------------------------------------------

  @doc false
  def fetch_tenant(tenant_id) do
    TenantCache.fetch(tenant_id, fn ->
      case RustCoreClient.get_tenant(tenant_id) do
        {:ok, tenant} -> tenant
        {:error, _} -> nil
      end
    end)
    |> case do
      nil -> {:error, :not_found}
      tenant -> {:ok, tenant}
    end
  end

  defp serialize_tenant(tenant) when is_map(tenant) do
    # Handle Core-format tenant maps (string keys like "id", "name", etc.)
    metadata = tenant["metadata"] || %{}
    subscription = metadata["subscription"] || %{}
    quotas = metadata["quotas"] || %{}

    %{
      id: tenant["id"],
      name: tenant["name"],
      slug: tenant["slug"],
      status: tenant["status"],
      subscription_status: subscription["status"],
      subscription_tier: subscription["tier"],
      events_quota: quotas["events_quota"],
      queries_quota: quotas["queries_quota"],
      events_used: quotas["events_used"] || 0,
      queries_used: quotas["queries_used"] || 0,
      created_at: tenant["inserted_at"],
      updated_at: tenant["updated_at"]
    }
  end

  # credo:disable-for-next-line Credo.Check.Refactor.CyclomaticComplexity
  defp build_usage_response(tenant) when is_map(tenant) do
    metadata = tenant["metadata"] || %{}
    subscription = metadata["subscription"] || %{}
    quotas = metadata["quotas"] || %{}

    events_quota = quotas["events_quota"] || -1
    events_used = quotas["events_used"] || 0
    queries_quota = quotas["queries_quota"] || -1
    queries_used = quotas["queries_used"] || 0

    %{
      tenant_id: tenant["id"],
      subscription_tier: subscription["tier"],
      subscription_status: subscription["status"],
      events: %{
        used: events_used,
        quota: events_quota,
        remaining: remaining(events_quota, events_used)
      },
      queries: %{
        used: queries_used,
        quota: queries_quota,
        remaining: remaining(queries_quota, queries_used)
      },
      billing_period: %{
        reset_at: nil
      }
    }
  end

  defp remaining(-1, _used), do: -1
  defp remaining(quota, used) when is_integer(quota), do: max(0, quota - used)
end
