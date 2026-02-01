defmodule QueryServiceEx.UsageMeter do
  @moduledoc """
  Comprehensive usage metering system for tracking tenant resource consumption.

  This module provides:
  - Real-time usage tracking with telemetry events
  - Usage history for audit trails
  - Alert threshold notifications
  - Atomic usage increments with race condition protection
  """

  import Ecto.Query
  alias QueryServiceEx.Billing.HybridPricing
  alias QueryServiceEx.Repo
  alias QueryServiceEx.Tenants
  alias QueryServiceEx.Tenants.Tenant
  alias QueryServiceEx.UsageMeter.UsageRecord

  require Logger

  @usage_types [:events, :queries]
  @alert_thresholds [50, 75, 90, 100]

  # -------------------------------------------------------------------
  # Usage Recording
  # -------------------------------------------------------------------

  @doc """
  Records event usage for a tenant and returns the updated tenant.

  Emits telemetry events for monitoring and alerting.
  Uses atomic increment to prevent race conditions.

  ## Options
  - `:count` - Number of events to record (default: 1)
  - `:metadata` - Additional metadata to store with the usage record
  """
  def record_events(tenant_id, opts \\ []) do
    count = Keyword.get(opts, :count, 1)
    metadata = Keyword.get(opts, :metadata, %{})

    record_usage(tenant_id, :events, count, metadata)
  end

  @doc """
  Records query usage for a tenant and returns the updated tenant.

  Emits telemetry events for monitoring and alerting.
  Uses atomic increment to prevent race conditions.

  ## Options
  - `:count` - Number of queries to record (default: 1)
  - `:metadata` - Additional metadata to store with the usage record
  """
  def record_queries(tenant_id, opts \\ []) do
    count = Keyword.get(opts, :count, 1)
    metadata = Keyword.get(opts, :metadata, %{})

    record_usage(tenant_id, :queries, count, metadata)
  end

  @doc """
  Records usage of a specific type for a tenant.

  Returns `{:ok, tenant}` on success, `{:error, reason}` on failure.

  When usage exceeds quota and overage billing is enabled, automatically
  reports the overage to the billing provider.
  """
  def record_usage(tenant_id, usage_type, count, metadata \\ %{})
      when usage_type in @usage_types and is_integer(count) and count > 0 do
    start_time = System.monotonic_time()

    result =
      Repo.transaction(fn ->
        # Get tenant before increment to calculate overage
        tenant_before = Tenants.get_tenant!(tenant_id)
        previous_usage = get_usage(tenant_before, usage_type)
        quota = get_quota(tenant_before, usage_type)

        # Atomic increment using UPDATE with RETURNING
        tenant = atomic_increment(tenant_id, usage_type, count)

        # Create usage record for audit trail
        create_usage_record(tenant_id, usage_type, count, metadata)

        # Check and emit alert if threshold crossed
        check_usage_thresholds(tenant, usage_type)

        # Handle overage billing if enabled
        maybe_record_overage(tenant, usage_type, previous_usage, quota, count)

        tenant
      end)

    # Emit telemetry
    duration = System.monotonic_time() - start_time

    case result do
      {:ok, tenant} ->
        emit_usage_recorded(tenant, usage_type, count, duration)
        {:ok, tenant}

      {:error, reason} ->
        emit_usage_error(tenant_id, usage_type, reason, duration)
        {:error, reason}
    end
  end

  # -------------------------------------------------------------------
  # Usage History & Analytics
  # -------------------------------------------------------------------

  @doc """
  Gets usage history for a tenant within a time range.

  ## Options
  - `:from` - Start datetime (default: 30 days ago)
  - `:to` - End datetime (default: now)
  - `:type` - Filter by usage type (:events, :queries, or nil for all)
  - `:limit` - Maximum records to return (default: 1000)
  """
  def get_usage_history(tenant_id, opts \\ []) do
    from_date = Keyword.get(opts, :from, DateTime.add(DateTime.utc_now(), -30, :day))
    to_date = Keyword.get(opts, :to, DateTime.utc_now())
    usage_type = Keyword.get(opts, :type)
    limit = Keyword.get(opts, :limit, 1000)

    query =
      from(r in UsageRecord,
        where: r.tenant_id == ^tenant_id,
        where: r.recorded_at >= ^from_date,
        where: r.recorded_at <= ^to_date,
        order_by: [desc: r.recorded_at],
        limit: ^limit
      )

    query =
      if usage_type do
        where(query, [r], r.usage_type == ^to_string(usage_type))
      else
        query
      end

    Repo.all(query)
  end

  @doc """
  Gets aggregated usage statistics for a tenant within a time range.

  Returns a map with totals and breakdowns by day/hour.
  """
  def get_usage_analytics(tenant_id, opts \\ []) do
    from_date = Keyword.get(opts, :from, DateTime.add(DateTime.utc_now(), -30, :day))
    to_date = Keyword.get(opts, :to, DateTime.utc_now())

    # Get current stats from tenant
    current_stats = Tenants.get_usage_stats(tenant_id)

    # Get historical totals
    totals = get_usage_totals(tenant_id, from_date, to_date)

    # Get daily breakdown
    daily = get_daily_breakdown(tenant_id, from_date, to_date)

    %{
      current: current_stats,
      period: %{
        from: from_date,
        to: to_date,
        totals: totals,
        daily: daily
      }
    }
  end

  @doc """
  Gets the current usage percentage for a specific type.
  """
  def get_usage_percentage(tenant_id, usage_type) when usage_type in @usage_types do
    tenant = Tenants.get_tenant!(tenant_id)

    case usage_type do
      :events -> calculate_percentage(tenant.events_used, tenant.events_quota)
      :queries -> calculate_percentage(tenant.queries_used, tenant.queries_quota)
    end
  end

  @doc """
  Checks if a tenant is approaching their quota limit.

  Returns `{:ok, percentage}` if under threshold, or
  `{:warning, percentage, threshold}` if at or above a threshold.
  """
  def check_quota_status(tenant_id, usage_type) when usage_type in @usage_types do
    percentage = get_usage_percentage(tenant_id, usage_type)

    # Find the highest threshold that has been reached/crossed
    crossed_thresholds =
      Enum.filter(@alert_thresholds, fn threshold -> percentage >= threshold end)

    case Enum.max(crossed_thresholds, fn -> nil end) do
      nil -> {:ok, percentage}
      threshold -> {:warning, percentage, threshold}
    end
  end

  # -------------------------------------------------------------------
  # Alert Thresholds
  # -------------------------------------------------------------------

  @doc """
  Returns the alert threshold percentages.
  """
  def alert_thresholds, do: @alert_thresholds

  @doc """
  Checks all tenants for usage approaching limits.

  Returns a list of `{tenant_id, usage_type, percentage, threshold}` tuples
  for tenants at or above the minimum threshold.
  """
  def check_all_tenants_usage(min_threshold \\ 75) do
    tenants = Tenants.list_tenants()
    Enum.flat_map(tenants, &check_tenant_usage(&1, min_threshold))
  end

  defp check_tenant_usage(tenant, min_threshold) do
    @usage_types
    |> Enum.map(&check_usage_type(tenant.id, &1, min_threshold))
    |> Enum.reject(&is_nil/1)
  end

  defp check_usage_type(tenant_id, type, min_threshold) do
    percentage = get_usage_percentage(tenant_id, type)

    if percentage >= min_threshold do
      # Find the highest threshold that has been reached/crossed
      crossed_thresholds = Enum.filter(@alert_thresholds, fn t -> percentage >= t end)
      threshold = Enum.max(crossed_thresholds, fn -> nil end)
      {tenant_id, type, percentage, threshold}
    end
  end

  # -------------------------------------------------------------------
  # Private Functions
  # -------------------------------------------------------------------

  defp atomic_increment(tenant_id, :events, count) do
    {1, [tenant]} =
      from(t in Tenant,
        where: t.id == ^tenant_id,
        select: t
      )
      |> Repo.update_all(inc: [events_used: count])

    tenant
  end

  defp atomic_increment(tenant_id, :queries, count) do
    {1, [tenant]} =
      from(t in Tenant,
        where: t.id == ^tenant_id,
        select: t
      )
      |> Repo.update_all(inc: [queries_used: count])

    tenant
  end

  defp create_usage_record(tenant_id, usage_type, count, metadata) do
    %UsageRecord{}
    |> UsageRecord.changeset(%{
      tenant_id: tenant_id,
      usage_type: to_string(usage_type),
      count: count,
      metadata: metadata,
      recorded_at: DateTime.utc_now()
    })
    |> Repo.insert!()
  end

  defp check_usage_thresholds(tenant, usage_type) do
    {used, quota} =
      case usage_type do
        :events -> {tenant.events_used, tenant.events_quota}
        :queries -> {tenant.queries_used, tenant.queries_quota}
      end

    percentage = calculate_percentage(used, quota)

    # Find crossed threshold
    crossed_threshold =
      @alert_thresholds
      |> Enum.filter(fn threshold -> percentage >= threshold end)
      |> Enum.max(fn -> nil end)

    if crossed_threshold do
      emit_threshold_alert(tenant, usage_type, percentage, crossed_threshold)
    end
  end

  defp calculate_percentage(_used, -1), do: 0.0

  defp calculate_percentage(used, quota) when is_integer(quota) and quota > 0 do
    Float.round(used / quota * 100, 1)
  end

  defp calculate_percentage(_used, _quota), do: 0.0

  defp get_usage(tenant, :events), do: tenant.events_used
  defp get_usage(tenant, :queries), do: tenant.queries_used

  defp get_quota(tenant, :events), do: tenant.events_quota
  defp get_quota(tenant, :queries), do: tenant.queries_quota

  defp maybe_record_overage(tenant, usage_type, previous_usage, quota, count) do
    # Skip if overage billing not enabled or unlimited quota
    if Tenant.overage_enabled?(tenant) and is_integer(quota) do
      new_usage = previous_usage + count

      cond do
        # Already over quota before this usage - all new usage is overage
        previous_usage >= quota ->
          record_overage_async(tenant.id, usage_type, count)

        # Crossed quota with this usage - only count the overage portion
        new_usage > quota ->
          overage_amount = new_usage - quota
          record_overage_async(tenant.id, usage_type, overage_amount)

        # Still within quota - no overage
        true ->
          :ok
      end
    else
      :ok
    end
  end

  defp record_overage_async(tenant_id, usage_type, count) do
    # Record overage asynchronously to not block the main transaction
    Task.start(fn ->
      case HybridPricing.record_overage(tenant_id, usage_type, count) do
        {:ok, _} ->
          Logger.debug(
            "[UsageMeter] Recorded #{count} #{usage_type} overage for tenant #{tenant_id}"
          )

        {:error, reason} ->
          Logger.warning(
            "[UsageMeter] Failed to record overage: #{inspect(reason)}",
            tenant_id: tenant_id,
            usage_type: usage_type,
            count: count
          )
      end
    end)

    :ok
  end

  defp get_usage_totals(tenant_id, from_date, to_date) do
    from(r in UsageRecord,
      where: r.tenant_id == ^tenant_id,
      where: r.recorded_at >= ^from_date,
      where: r.recorded_at <= ^to_date,
      group_by: r.usage_type,
      select: {r.usage_type, sum(r.count)}
    )
    |> Repo.all()
    |> Map.new(fn {type, total} -> {String.to_atom(type), total || 0} end)
  end

  defp get_daily_breakdown(tenant_id, from_date, to_date) do
    from(r in UsageRecord,
      where: r.tenant_id == ^tenant_id,
      where: r.recorded_at >= ^from_date,
      where: r.recorded_at <= ^to_date,
      group_by: [fragment("DATE(?)", r.recorded_at), r.usage_type],
      select: {fragment("DATE(?)", r.recorded_at), r.usage_type, sum(r.count)},
      order_by: [asc: fragment("DATE(?)", r.recorded_at)]
    )
    |> Repo.all()
    |> Enum.group_by(
      fn {date, _type, _count} -> date end,
      fn {_date, type, count} -> {String.to_atom(type), count || 0} end
    )
    |> Enum.map(fn {date, type_counts} -> {date, Map.new(type_counts)} end)
    |> Map.new()
  end

  # -------------------------------------------------------------------
  # Telemetry Events
  # -------------------------------------------------------------------

  defp emit_usage_recorded(tenant, usage_type, count, duration) do
    :telemetry.execute(
      [:query_service_ex, :usage_meter, :recorded],
      %{count: count, duration: duration},
      %{
        tenant_id: tenant.id,
        usage_type: usage_type,
        events_used: tenant.events_used,
        events_quota: tenant.events_quota,
        queries_used: tenant.queries_used,
        queries_quota: tenant.queries_quota
      }
    )

    Logger.debug(
      "[UsageMeter] Recorded #{count} #{usage_type} for tenant #{tenant.id}",
      tenant_id: tenant.id,
      usage_type: usage_type,
      count: count
    )
  end

  defp emit_usage_error(tenant_id, usage_type, reason, duration) do
    :telemetry.execute(
      [:query_service_ex, :usage_meter, :error],
      %{duration: duration},
      %{
        tenant_id: tenant_id,
        usage_type: usage_type,
        reason: reason
      }
    )

    Logger.error(
      "[UsageMeter] Failed to record #{usage_type} for tenant #{tenant_id}: #{inspect(reason)}",
      tenant_id: tenant_id,
      usage_type: usage_type,
      reason: reason
    )
  end

  defp emit_threshold_alert(tenant, usage_type, percentage, threshold) do
    :telemetry.execute(
      [:query_service_ex, :usage_meter, :threshold_alert],
      %{percentage: percentage, threshold: threshold},
      %{
        tenant_id: tenant.id,
        tenant_name: tenant.name,
        usage_type: usage_type,
        subscription_tier: tenant.subscription_tier
      }
    )

    Logger.warning(
      "[UsageMeter] Tenant #{tenant.id} reached #{threshold}% of #{usage_type} quota (#{percentage}%)",
      tenant_id: tenant.id,
      usage_type: usage_type,
      percentage: percentage,
      threshold: threshold
    )
  end
end
