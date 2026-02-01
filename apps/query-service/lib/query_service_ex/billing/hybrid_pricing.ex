defmodule QueryServiceEx.Billing.HybridPricing do
  @moduledoc """
  Hybrid pricing model: base subscription + usage overage.

  This module handles:
  - Calculating overage charges when usage exceeds base quota
  - Reporting overage usage to LemonSqueezy for metered billing
  - Tracking and projecting overage costs
  """

  import Ecto.Query
  alias QueryServiceEx.Repo
  alias QueryServiceEx.Tenants
  alias QueryServiceEx.Tenants.Tenant
  alias QueryServiceEx.Billing.LemonSqueezy

  require Logger

  @usage_types [:events, :queries]

  @doc """
  Calculates the current overage for a tenant and usage type.

  Returns 0 if usage is within quota, or the amount over quota.
  Enterprise tier (unlimited quota) always returns 0.
  """
  def calculate_overage(%Tenant{} = tenant, :events) do
    case tenant.events_quota do
      :unlimited -> 0
      quota -> max(0, tenant.events_used - quota)
    end
  end

  def calculate_overage(%Tenant{} = tenant, :queries) do
    case tenant.queries_quota do
      :unlimited -> 0
      quota -> max(0, tenant.queries_used - quota)
    end
  end

  @doc """
  Calculates the projected overage charge (in cents) for a tenant.

  Accepts either a `%Tenant{}` struct or a tenant ID string.

  Returns a map with charges by usage type and total.
  """
  def calculate_overage_charges(%Tenant{} = tenant) do
    events_overage = calculate_overage(tenant, :events)
    queries_overage = calculate_overage(tenant, :queries)

    events_charge = events_overage * tenant.overage_rate_events
    queries_charge = queries_overage * tenant.overage_rate_queries

    %{
      events: %{
        overage_units: events_overage,
        rate_cents: tenant.overage_rate_events,
        charge_cents: events_charge
      },
      queries: %{
        overage_units: queries_overage,
        rate_cents: tenant.overage_rate_queries,
        charge_cents: queries_charge
      },
      total_cents: events_charge + queries_charge
    }
  end

  def calculate_overage_charges(tenant_id) when is_binary(tenant_id) do
    tenant = Tenants.get_tenant!(tenant_id)
    calculate_overage_charges(tenant)
  end

  @doc """
  Records overage usage and reports it to the billing provider.

  This should be called when usage exceeds quota and overage billing is enabled.
  Reports the incremental overage to LemonSqueezy for metered billing.

  Returns `{:ok, tenant}` on success, `{:error, reason}` on failure.
  """
  def record_overage(tenant_id, usage_type, overage_count)
      when usage_type in @usage_types and is_integer(overage_count) and overage_count > 0 do
    tenant = Tenants.get_tenant!(tenant_id)

    unless Tenant.overage_enabled?(tenant) do
      {:error, :overage_not_enabled}
    else
      case report_to_billing_provider(tenant, usage_type, overage_count) do
        :ok ->
          update_overage_tracking(tenant, usage_type, overage_count)

        {:error, reason} ->
          Logger.error(
            "[HybridPricing] Failed to report overage: #{inspect(reason)}",
            tenant_id: tenant_id,
            usage_type: usage_type,
            overage_count: overage_count
          )

          {:error, reason}
      end
    end
  end

  @doc """
  Gets a summary of overage usage and charges for a tenant.
  """
  def get_overage_summary(tenant_id) do
    tenant = Tenants.get_tenant!(tenant_id)
    charges = calculate_overage_charges(tenant)

    %{
      overage_enabled: tenant.overage_enabled,
      events: %{
        used: tenant.events_used,
        quota: tenant.events_quota,
        overage: charges.events.overage_units,
        reported_overage: tenant.events_overage,
        rate_cents: tenant.overage_rate_events,
        projected_charge_cents: charges.events.charge_cents
      },
      queries: %{
        used: tenant.queries_used,
        quota: tenant.queries_quota,
        overage: charges.queries.overage_units,
        reported_overage: tenant.queries_overage,
        rate_cents: tenant.overage_rate_queries,
        projected_charge_cents: charges.queries.charge_cents
      },
      total_projected_cents: charges.total_cents,
      last_reported_at: tenant.overage_reported_at
    }
  end

  @doc """
  Enables overage billing for a tenant with the specified rates.

  Options:
  - `:events_rate` - Rate in cents per event over quota (default: 100)
  - `:queries_rate` - Rate in cents per query over quota (default: 1000)
  - `:events_item_id` - LemonSqueezy subscription item ID for events metering
  - `:queries_item_id` - LemonSqueezy subscription item ID for queries metering
  """
  def enable_overage(tenant_id, opts \\ []) do
    tenant = Tenants.get_tenant!(tenant_id)

    attrs = %{
      overage_enabled: true,
      overage_rate_events: Keyword.get(opts, :events_rate, Tenant.default_overage_rates().events),
      overage_rate_queries:
        Keyword.get(opts, :queries_rate, Tenant.default_overage_rates().queries),
      lemon_squeezy_events_item_id: Keyword.get(opts, :events_item_id),
      lemon_squeezy_queries_item_id: Keyword.get(opts, :queries_item_id)
    }

    tenant
    |> Tenant.overage_changeset(attrs)
    |> Repo.update()
  end

  @doc """
  Disables overage billing for a tenant.
  """
  def disable_overage(tenant_id) do
    tenant = Tenants.get_tenant!(tenant_id)

    tenant
    |> Tenant.overage_changeset(%{overage_enabled: false})
    |> Repo.update()
  end

  @doc """
  Syncs unreported overage to the billing provider.

  This is useful for batch processing or recovering from failed reports.
  """
  def sync_overage(tenant_id) do
    tenant = Tenants.get_tenant!(tenant_id)

    if Tenant.overage_enabled?(tenant) do
      # Calculate current overage vs what's been reported
      current_events_overage = calculate_overage(tenant, :events)
      current_queries_overage = calculate_overage(tenant, :queries)

      unreported_events = max(0, current_events_overage - tenant.events_overage)
      unreported_queries = max(0, current_queries_overage - tenant.queries_overage)

      results = []

      results =
        if unreported_events > 0 do
          [record_overage(tenant_id, :events, unreported_events) | results]
        else
          results
        end

      results =
        if unreported_queries > 0 do
          [record_overage(tenant_id, :queries, unreported_queries) | results]
        else
          results
        end

      errors = Enum.filter(results, fn result -> match?({:error, _}, result) end)

      if Enum.empty?(errors) do
        {:ok, %{events_synced: unreported_events, queries_synced: unreported_queries}}
      else
        {:error, errors}
      end
    else
      {:error, :overage_not_enabled}
    end
  end

  @doc """
  Resets overage tracking for a tenant (called at billing period start).
  """
  def reset_overage(tenant_id) do
    tenant = Tenants.get_tenant!(tenant_id)

    tenant
    |> Tenant.usage_changeset(%{
      events_overage: 0,
      queries_overage: 0,
      overage_reported_at: nil
    })
    |> Repo.update()
  end

  @doc """
  Gets all tenants with unreported overage.
  """
  def get_tenants_with_unreported_overage do
    from(t in Tenant,
      where: t.overage_enabled == true,
      where: t.events_used > t.events_quota and t.events_overage < t.events_used - t.events_quota,
      or_where:
        t.queries_used > t.queries_quota and t.queries_overage < t.queries_used - t.queries_quota
    )
    |> Repo.all()
  end

  # -------------------------------------------------------------------
  # Private Functions
  # -------------------------------------------------------------------

  defp report_to_billing_provider(tenant, :events, count) do
    case tenant.lemon_squeezy_events_item_id do
      nil ->
        Logger.debug("[HybridPricing] No events item ID configured, skipping report")
        :ok

      item_id ->
        case LemonSqueezy.report_usage(item_id, count) do
          {:ok, _} -> :ok
          {:error, reason} -> {:error, reason}
        end
    end
  end

  defp report_to_billing_provider(tenant, :queries, count) do
    case tenant.lemon_squeezy_queries_item_id do
      nil ->
        Logger.debug("[HybridPricing] No queries item ID configured, skipping report")
        :ok

      item_id ->
        case LemonSqueezy.report_usage(item_id, count) do
          {:ok, _} -> :ok
          {:error, reason} -> {:error, reason}
        end
    end
  end

  defp update_overage_tracking(tenant, :events, count) do
    new_overage = tenant.events_overage + count

    tenant
    |> Tenant.usage_changeset(%{
      events_overage: new_overage,
      overage_reported_at: DateTime.utc_now()
    })
    |> Repo.update()
  end

  defp update_overage_tracking(tenant, :queries, count) do
    new_overage = tenant.queries_overage + count

    tenant
    |> Tenant.usage_changeset(%{
      queries_overage: new_overage,
      overage_reported_at: DateTime.utc_now()
    })
    |> Repo.update()
  end
end
