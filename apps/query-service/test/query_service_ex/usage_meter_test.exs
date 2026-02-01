defmodule QueryServiceEx.UsageMeterTest do
  @moduledoc """
  Tests for the UsageMeter context.

  These tests require a running PostgreSQL database.
  Run with: mix test --include database
  """
  use QueryServiceEx.DataCase

  alias QueryServiceEx.Billing.HybridPricing
  alias QueryServiceEx.Tenants
  alias QueryServiceEx.UsageMeter
  alias QueryServiceEx.UsageMeter.UsageRecord

  @moduletag :database

  setup do
    {:ok, tenant} =
      Tenants.create_tenant(%{
        name: "Test Workspace",
        slug: "usage-meter-test-#{System.unique_integer([:positive])}"
      })

    {:ok, tenant: tenant}
  end

  describe "record_events/2" do
    test "increments events usage counter", %{tenant: tenant} do
      assert tenant.events_used == 0

      {:ok, updated} = UsageMeter.record_events(tenant.id, count: 5)

      assert updated.events_used == 5
    end

    test "creates usage record for audit trail", %{tenant: tenant} do
      {:ok, _updated} =
        UsageMeter.record_events(tenant.id,
          count: 10,
          metadata: %{entity_id: "test-123", event_type: "order.placed"}
        )

      records = Repo.all(from(r in UsageRecord, where: r.tenant_id == ^tenant.id))
      assert length(records) == 1

      record = hd(records)
      assert record.usage_type == "events"
      assert record.count == 10
      assert record.metadata["entity_id"] == "test-123"
      assert record.metadata["event_type"] == "order.placed"
    end

    test "accumulates usage across multiple calls", %{tenant: tenant} do
      {:ok, _} = UsageMeter.record_events(tenant.id, count: 100)
      {:ok, _} = UsageMeter.record_events(tenant.id, count: 50)
      {:ok, updated} = UsageMeter.record_events(tenant.id, count: 25)

      assert updated.events_used == 175

      records = Repo.all(from(r in UsageRecord, where: r.tenant_id == ^tenant.id))
      assert length(records) == 3
    end

    test "uses default count of 1", %{tenant: tenant} do
      {:ok, updated} = UsageMeter.record_events(tenant.id)

      assert updated.events_used == 1
    end
  end

  describe "record_queries/2" do
    test "increments queries usage counter", %{tenant: tenant} do
      assert tenant.queries_used == 0

      {:ok, updated} = UsageMeter.record_queries(tenant.id, count: 3)

      assert updated.queries_used == 3
    end

    test "creates usage record for audit trail", %{tenant: tenant} do
      {:ok, _updated} =
        UsageMeter.record_queries(tenant.id,
          count: 1,
          metadata: %{query_type: "dsl", result_count: 42}
        )

      records = Repo.all(from(r in UsageRecord, where: r.tenant_id == ^tenant.id))
      assert length(records) == 1

      record = hd(records)
      assert record.usage_type == "queries"
      assert record.count == 1
      assert record.metadata["query_type"] == "dsl"
      assert record.metadata["result_count"] == 42
    end
  end

  describe "get_usage_history/2" do
    test "returns usage records within time range", %{tenant: tenant} do
      {:ok, _} = UsageMeter.record_events(tenant.id, count: 10)
      {:ok, _} = UsageMeter.record_queries(tenant.id, count: 5)

      history = UsageMeter.get_usage_history(tenant.id)

      assert length(history) == 2
    end

    test "filters by usage type", %{tenant: tenant} do
      {:ok, _} = UsageMeter.record_events(tenant.id, count: 10)
      {:ok, _} = UsageMeter.record_events(tenant.id, count: 20)
      {:ok, _} = UsageMeter.record_queries(tenant.id, count: 5)

      events_history = UsageMeter.get_usage_history(tenant.id, type: :events)
      queries_history = UsageMeter.get_usage_history(tenant.id, type: :queries)

      assert length(events_history) == 2
      assert length(queries_history) == 1
    end

    test "respects limit option", %{tenant: tenant} do
      for _ <- 1..10 do
        {:ok, _} = UsageMeter.record_events(tenant.id, count: 1)
      end

      history = UsageMeter.get_usage_history(tenant.id, limit: 5)

      assert length(history) == 5
    end
  end

  describe "get_usage_analytics/2" do
    test "returns current stats and historical data", %{tenant: tenant} do
      {:ok, _} = UsageMeter.record_events(tenant.id, count: 500)
      {:ok, _} = UsageMeter.record_queries(tenant.id, count: 50)

      analytics = UsageMeter.get_usage_analytics(tenant.id)

      assert analytics.current.events.used == 500
      assert analytics.current.queries.used == 50

      assert Map.has_key?(analytics.period, :from)
      assert Map.has_key?(analytics.period, :to)
      assert Map.has_key?(analytics.period, :totals)
    end
  end

  describe "get_usage_percentage/2" do
    test "calculates correct percentage for events", %{tenant: tenant} do
      {:ok, _} = UsageMeter.record_events(tenant.id, count: 5000)

      percentage = UsageMeter.get_usage_percentage(tenant.id, :events)

      # Default quota is 10,000, so 5000 = 50%
      assert percentage == 50.0
    end

    test "calculates correct percentage for queries", %{tenant: tenant} do
      {:ok, _} = UsageMeter.record_queries(tenant.id, count: 250)

      percentage = UsageMeter.get_usage_percentage(tenant.id, :queries)

      # Default quota is 1,000, so 250 = 25%
      assert percentage == 25.0
    end
  end

  describe "check_quota_status/2" do
    test "returns ok when under threshold", %{tenant: tenant} do
      {:ok, _} = UsageMeter.record_events(tenant.id, count: 1000)

      assert {:ok, 10.0} = UsageMeter.check_quota_status(tenant.id, :events)
    end

    test "returns warning when at 50% threshold", %{tenant: tenant} do
      {:ok, _} = UsageMeter.record_events(tenant.id, count: 5000)

      assert {:warning, 50.0, 50} = UsageMeter.check_quota_status(tenant.id, :events)
    end

    test "returns warning when at 75% threshold", %{tenant: tenant} do
      {:ok, _} = UsageMeter.record_events(tenant.id, count: 7500)

      assert {:warning, 75.0, 75} = UsageMeter.check_quota_status(tenant.id, :events)
    end

    test "returns warning when at 90% threshold", %{tenant: tenant} do
      {:ok, _} = UsageMeter.record_events(tenant.id, count: 9500)

      assert {:warning, 95.0, 90} = UsageMeter.check_quota_status(tenant.id, :events)
    end

    test "returns warning when at 100% threshold", %{tenant: tenant} do
      {:ok, _} = UsageMeter.record_events(tenant.id, count: 10_000)

      assert {:warning, 100.0, 100} = UsageMeter.check_quota_status(tenant.id, :events)
    end
  end

  describe "alert_thresholds/0" do
    test "returns the configured thresholds" do
      thresholds = UsageMeter.alert_thresholds()

      assert thresholds == [50, 75, 90, 100]
    end
  end

  describe "check_all_tenants_usage/1" do
    test "returns tenants approaching quota limits" do
      # Create a tenant at high usage
      {:ok, high_usage_tenant} =
        Tenants.create_tenant(%{
          name: "High Usage",
          slug: "high-usage-#{System.unique_integer([:positive])}"
        })

      {:ok, _} = UsageMeter.record_events(high_usage_tenant.id, count: 8000)

      # Create a tenant at low usage
      {:ok, low_usage_tenant} =
        Tenants.create_tenant(%{
          name: "Low Usage",
          slug: "low-usage-#{System.unique_integer([:positive])}"
        })

      {:ok, _} = UsageMeter.record_events(low_usage_tenant.id, count: 100)

      alerts = UsageMeter.check_all_tenants_usage(75)

      # High usage tenant should be in alerts
      high_usage_alert =
        Enum.find(alerts, fn {tenant_id, _type, _pct, _thresh} ->
          tenant_id == high_usage_tenant.id
        end)

      assert high_usage_alert != nil
      {_id, type, percentage, threshold} = high_usage_alert
      assert type == :events
      assert percentage == 80.0
      assert threshold == 75

      # Low usage tenant should not be in alerts
      low_usage_alert =
        Enum.find(alerts, fn {tenant_id, _type, _pct, _thresh} ->
          tenant_id == low_usage_tenant.id
        end)

      assert low_usage_alert == nil
    end
  end

  describe "hybrid pricing integration" do
    test "tracks overage when usage exceeds quota with overage enabled", %{tenant: tenant} do
      # Enable overage billing
      {:ok, _} = HybridPricing.enable_overage(tenant.id)

      # Record usage that exceeds quota (10,000 default)
      {:ok, updated} = UsageMeter.record_events(tenant.id, count: 12_000)

      # Wait for async overage recording
      :timer.sleep(100)

      # Check overage was calculated correctly
      assert updated.events_used == 12_000
      overage = HybridPricing.calculate_overage(updated, :events)
      assert overage == 2_000
    end

    test "records incremental overage when already over quota", %{tenant: tenant} do
      {:ok, _} = HybridPricing.enable_overage(tenant.id)

      # First push over quota
      {:ok, _} = UsageMeter.record_events(tenant.id, count: 11_000)
      :timer.sleep(100)

      # Record more events (all should be overage)
      {:ok, updated} = UsageMeter.record_events(tenant.id, count: 500)
      :timer.sleep(100)

      assert updated.events_used == 11_500
      overage = HybridPricing.calculate_overage(updated, :events)
      assert overage == 1_500
    end

    test "does not track overage when billing is disabled", %{tenant: tenant} do
      # Overage billing not enabled (default)
      {:ok, updated} = UsageMeter.record_events(tenant.id, count: 12_000)

      :timer.sleep(100)

      # Overage should not be reported
      refreshed = Tenants.get_tenant!(tenant.id)
      assert refreshed.events_overage == 0
      assert refreshed.events_used == 12_000
    end

    test "partial overage when crossing quota threshold", %{tenant: tenant} do
      {:ok, _} = HybridPricing.enable_overage(tenant.id)

      # Record 9,500 events (under quota)
      {:ok, _} = UsageMeter.record_events(tenant.id, count: 9_500)

      # Record 1,000 more (crosses quota by 500)
      {:ok, updated} = UsageMeter.record_events(tenant.id, count: 1_000)
      :timer.sleep(100)

      assert updated.events_used == 10_500
      overage = HybridPricing.calculate_overage(updated, :events)
      assert overage == 500
    end
  end
end
