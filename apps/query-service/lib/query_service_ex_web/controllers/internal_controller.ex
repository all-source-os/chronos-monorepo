defmodule QueryServiceExWeb.InternalController do
  @moduledoc """
  Internal endpoints called by infrastructure components (e.g., allsource-sentinel, Control Plane).

  These endpoints are on the internal network only. Do not expose them publicly.
  The `tenant-updated` endpoint requires a shared `INTERNAL_API_KEY` for authentication.
  """

  use Phoenix.Controller, formats: [:json]

  alias QueryServiceEx.TenantCache

  require Logger

  @doc """
  Update the leader URL at runtime.

  Called by the sentinel process during failover to notify the Query Service
  that a new Core leader has been promoted. Updates the `:core_write_url`
  application config so subsequent writes are routed to the new leader
  without requiring a restart.

  ## Request body

      {"leader_url": "http://new-leader:3900"}
  """
  def update_leader(conn, %{"leader_url" => leader_url}) when is_binary(leader_url) do
    previous_url = Application.get_env(:query_service_ex, :core_write_url)

    Application.put_env(:query_service_ex, :core_write_url, leader_url)

    Logger.warning(
      "[InternalController] Leader changed: #{inspect(previous_url)} -> #{inspect(leader_url)}"
    )

    conn
    |> put_status(:ok)
    |> json(%{status: "updated", leader_url: leader_url})
  end

  def update_leader(conn, _params) do
    conn
    |> put_status(:bad_request)
    |> json(%{error: "missing or invalid leader_url"})
  end

  @valid_actions ~w(suspended activated updated deleted)

  @doc """
  Invalidate the tenant cache entry when tenant state changes.

  Called by the Control Plane when a tenant is suspended, activated, updated,
  or deleted. Requires `INTERNAL_API_KEY` authentication (via InternalApiKey plug).

  Always returns 200 for idempotency — even if the tenant wasn't cached.

  ## Request body

      {"tenant_id": "uuid", "action": "suspended|activated|updated|deleted"}
  """
  def tenant_updated(conn, %{"tenant_id" => tenant_id, "action" => action})
      when is_binary(tenant_id) and action in @valid_actions do
    was_cached = TenantCache.cached?(tenant_id)
    TenantCache.invalidate(tenant_id)

    Logger.info(
      "[InternalController] Tenant cache invalidated: tenant_id=#{tenant_id} action=#{action} was_cached=#{was_cached}"
    )

    conn
    |> put_status(:ok)
    |> json(%{status: "ok", tenant_id: tenant_id, action: action, was_cached: was_cached})
  end

  def tenant_updated(conn, _params) do
    conn
    |> put_status(:bad_request)
    |> json(%{error: "missing or invalid tenant_id/action"})
  end
end
