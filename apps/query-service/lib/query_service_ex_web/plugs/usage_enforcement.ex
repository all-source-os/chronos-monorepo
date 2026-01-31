defmodule QueryServiceExWeb.Plugs.UsageEnforcement do
  @moduledoc """
  Plug that enforces usage quotas for tenant operations.

  This plug checks if the tenant has exceeded their quota before
  allowing operations that consume usage (events, queries).

  Usage is tracked and enforced per billing period.
  """

  import Plug.Conn
  alias QueryServiceEx.Tenants
  alias QueryServiceEx.Tenants.Tenant

  require Logger

  @doc """
  Initialize the plug with the usage type to enforce.

  Options:
  - `:type` - The usage type to check: `:events` or `:queries`
  """
  def init(opts) do
    type = Keyword.get(opts, :type, :events)

    unless type in [:events, :queries] do
      raise ArgumentError, "UsageEnforcement :type must be :events or :queries"
    end

    %{type: type}
  end

  def call(conn, %{type: type}) do
    tenant = conn.assigns[:current_tenant]

    cond do
      is_nil(tenant) ->
        # No tenant context - let other plugs handle auth
        conn

      quota_exceeded?(tenant, type) ->
        send_quota_exceeded(conn, tenant, type)

      true ->
        conn
    end
  end

  defp quota_exceeded?(tenant, :events) do
    Tenant.events_quota_exceeded?(tenant)
  end

  defp quota_exceeded?(tenant, :queries) do
    Tenant.queries_quota_exceeded?(tenant)
  end

  defp send_quota_exceeded(conn, tenant, type) do
    usage_stats = Tenants.get_usage_stats(tenant.id)
    stats = Map.get(usage_stats, type)

    Logger.info(
      "[UsageEnforcement] Tenant #{tenant.id} exceeded #{type} quota: #{stats.used}/#{stats.quota}"
    )

    conn
    |> put_status(:payment_required)
    |> Phoenix.Controller.json(%{
      error: %{
        code: "quota_exceeded",
        message: "You have exceeded your #{type} quota for this billing period.",
        usage_type: type,
        used: stats.used,
        quota: stats.quota,
        percentage: stats.percentage,
        reset_at: usage_stats.reset_at,
        upgrade_url: "/billing/upgrade"
      }
    })
    |> halt()
  end
end
