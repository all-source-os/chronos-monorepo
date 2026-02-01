defmodule QueryServiceExWeb.Plugs.UsageEnforcement do
  @moduledoc """
  Plug that enforces usage quotas for tenant operations.

  This plug checks if the tenant has exceeded their quota before
  allowing operations that consume usage (events, queries).

  Usage is tracked and enforced per billing period.

  ## Hybrid Pricing Mode

  When overage billing is enabled for a tenant, the plug operates in
  "soft limit" mode - it allows the request but adds headers indicating
  the tenant is in overage. This enables the hybrid pricing model where
  customers pay base subscription + usage overage.
  """

  import Plug.Conn
  alias QueryServiceEx.Tenants
  alias QueryServiceEx.Tenants.Tenant
  alias QueryServiceEx.Billing.HybridPricing

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
        handle_quota_exceeded(conn, tenant, type)

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

  defp handle_quota_exceeded(conn, tenant, type) do
    if Tenant.overage_enabled?(tenant) do
      # Soft limit mode: allow request but add overage headers
      allow_with_overage_warning(conn, tenant, type)
    else
      # Hard limit mode: block request
      send_quota_exceeded(conn, tenant, type)
    end
  end

  defp allow_with_overage_warning(conn, tenant, type) do
    usage_stats = Tenants.get_usage_stats(tenant.id)
    stats = Map.get(usage_stats, type)
    overage_summary = HybridPricing.get_overage_summary(tenant.id)

    Logger.info(
      "[UsageEnforcement] Tenant #{tenant.id} in overage for #{type}: #{stats.used}/#{stats.quota} (overage billing enabled)"
    )

    conn
    |> put_resp_header("x-usage-overage", "true")
    |> put_resp_header("x-usage-type", to_string(type))
    |> put_resp_header("x-usage-used", to_string(stats.used))
    |> put_resp_header("x-usage-quota", to_string(stats.quota))
    |> put_resp_header("x-overage-units", to_string(overage_summary[type].overage))
    |> put_resp_header(
      "x-projected-charge-cents",
      to_string(overage_summary.total_projected_cents)
    )
    |> assign(:in_overage, true)
    |> assign(:overage_type, type)
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
        upgrade_url: "/billing/upgrade",
        enable_overage_url: "/billing/enable-overage"
      }
    })
    |> halt()
  end
end
