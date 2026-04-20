defmodule QueryServiceExWeb.AuditLogController do
  @moduledoc """
  Controller for customer-facing audit log entries.

  Provides a paginated, filterable list of actions taken on a tenant.
  Retention is enforced client-side based on subscription tier:
  - Pro (growth): last 90 days
  - Team (enterprise): last 1 year
  """

  use Phoenix.Controller, formats: [:json]

  alias QueryServiceEx.AuditLog
  alias QueryServiceEx.DevMode
  alias QueryServiceEx.Infrastructure.Adapters.RustCoreClient
  alias QueryServiceEx.TenantCache

  action_fallback(QueryServiceExWeb.FallbackController)

  @retention_days %{
    "free" => 7,
    "pro" => 30,
    "growth" => 90,
    "enterprise" => 365
  }

  @doc """
  Lists audit log entries for the authenticated tenant.

  GET /api/tenant/audit-logs
  Query params:
    - action: filter by action type (e.g. "api_key.created")
    - limit: max entries (default 50, max 200)
    - offset: pagination offset (default 0)
  """
  def index(conn, params) do
    with {:ok, tenant} <- get_tenant(conn) do
      tenant_id = tenant["id"]
      tier = get_tier(tenant)
      retention = Map.get(@retention_days, tier, 90)

      limit = parse_int(params["limit"], 50, 1, 200)
      offset = parse_int(params["offset"], 0, 0, 10_000)

      opts = [limit: limit, offset: offset]

      opts =
        case params["action"] do
          action when is_binary(action) and action != "" ->
            if action in AuditLog.valid_actions() do
              Keyword.put(opts, :action, action)
            else
              opts
            end

          _ ->
            opts
        end

      case AuditLog.list(tenant_id, opts) do
        {:ok, entries} ->
          cutoff =
            DateTime.utc_now()
            |> DateTime.add(-retention * 86_400, :second)
            |> DateTime.to_iso8601()

          filtered =
            Enum.filter(entries, fn entry ->
              (entry["timestamp"] || "") >= cutoff
            end)

          conn
          |> put_status(:ok)
          |> json(%{
            data: %{
              entries: filtered,
              retention_days: retention,
              actions: AuditLog.valid_actions()
            }
          })

        {:error, reason} ->
          conn
          |> put_status(:internal_server_error)
          |> json(%{
            error: %{
              code: "audit_log_error",
              message: "Failed to fetch audit logs: #{inspect(reason)}"
            }
          })
      end
    end
  end

  defp get_tenant(conn) do
    if DevMode.auth_disabled?() do
      tenant = conn.assigns[:current_tenant] || DevMode.dev_tenant()
      {:ok, tenant}
    else
      user = conn.assigns[:current_user]
      tenant_id = user["tenant_id"] || user[:tenant_id]

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
  end

  defp get_tier(tenant) do
    metadata = tenant["metadata"] || %{}
    subscription = metadata["subscription"] || %{}
    subscription["tier"] || "free"
  end

  defp parse_int(nil, default, _min, _max), do: default

  defp parse_int(value, default, min, max) when is_binary(value) do
    case Integer.parse(value) do
      {n, ""} -> n |> Kernel.max(min) |> Kernel.min(max)
      _ -> default
    end
  end

  defp parse_int(value, _default, min, max) when is_integer(value) do
    value |> Kernel.max(min) |> Kernel.min(max)
  end

  defp parse_int(_, default, _min, _max), do: default
end
