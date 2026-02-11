defmodule QueryServiceExWeb.TenantController do
  @moduledoc """
  Controller for tenant management operations.

  Provides endpoints for viewing and managing tenant information,
  usage statistics, and settings.
  """

  use Phoenix.Controller, formats: [:json]
  use OpenApiSpex.ControllerSpecs

  alias QueryServiceEx.Accounts.Guardian
  alias QueryServiceEx.Tenants
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
    user = Guardian.Plug.current_resource(conn)
    tenant = Tenants.get_tenant(user.tenant_id)

    case tenant do
      nil ->
        conn
        |> put_status(:not_found)
        |> json(%{error: %{code: "tenant_not_found", message: "Workspace not found"}})

      tenant ->
        conn
        |> put_status(:ok)
        |> json(%{data: serialize_tenant(tenant)})
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
  """
  def update(conn, params) do
    user = Guardian.Plug.current_resource(conn)
    tenant = Tenants.get_tenant!(user.tenant_id)

    # Only allow updating name and settings
    update_attrs = Map.take(params, ["name", "settings"])

    case Tenants.update_tenant(tenant, update_attrs) do
      {:ok, updated_tenant} ->
        conn
        |> put_status(:ok)
        |> json(%{data: serialize_tenant(updated_tenant)})

      {:error, changeset} ->
        conn
        |> put_status(:unprocessable_entity)
        |> json(%{error: format_changeset_errors(changeset)})
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
    user = Guardian.Plug.current_resource(conn)
    tenant = Tenants.get_tenant!(user.tenant_id)
    usage_stats = Tenants.get_usage_stats(user.tenant_id)

    conn
    |> put_status(:ok)
    |> json(%{
      data: %{
        tenant_id: tenant.id,
        subscription_tier: tenant.subscription_tier,
        subscription_status: tenant.subscription_status,
        events: usage_stats.events,
        queries: usage_stats.queries,
        billing_period: %{
          reset_at: usage_stats.reset_at
        }
      }
    })
  end

  # -------------------------------------------------------------------
  # Private Helpers
  # -------------------------------------------------------------------

  defp serialize_tenant(tenant) do
    %{
      id: tenant.id,
      name: tenant.name,
      slug: tenant.slug,
      subscription_status: tenant.subscription_status,
      subscription_tier: tenant.subscription_tier,
      trial_ends_at: tenant.trial_ends_at,
      subscription_ends_at: tenant.subscription_ends_at,
      events_quota: tenant.events_quota,
      queries_quota: tenant.queries_quota,
      events_used: tenant.events_used,
      queries_used: tenant.queries_used,
      settings: tenant.settings,
      created_at: tenant.inserted_at,
      updated_at: tenant.updated_at
    }
  end

  defp format_changeset_errors(changeset) do
    errors =
      Ecto.Changeset.traverse_errors(changeset, fn {msg, opts} ->
        Enum.reduce(opts, msg, fn {key, value}, acc ->
          String.replace(acc, "%{#{key}}", to_string(value))
        end)
      end)

    %{
      code: "validation_error",
      message: "Validation failed",
      details: errors
    }
  end
end
