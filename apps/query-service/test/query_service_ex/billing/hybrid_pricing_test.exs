defmodule QueryServiceEx.Billing.HybridPricingTest do
  @moduledoc """
  Tests for the HybridPricing module.

  These tests verify the hybrid pricing model functionality:
  - Overage calculation
  - Charge estimation
  - Enabling/disabling overage billing
  """
  use QueryServiceEx.DataCase

  alias QueryServiceEx.Billing.HybridPricing
  alias QueryServiceEx.Tenants
  alias QueryServiceEx.Tenants.Tenant
  alias QueryServiceEx.UsageMeter

  @moduletag :database

  setup do
    {:ok, tenant} =
      Tenants.create_tenant(%{
        name: "Hybrid Pricing Test",
        slug: "hybrid-pricing-test-#{System.unique_integer([:positive])}"
      })

    {:ok, tenant: tenant}
  end

  describe "calculate_overage/2" do
    test "returns 0 when usage is under quota", %{tenant: tenant} do
      # Default quota is 10,000 events
      {:ok, tenant} = UsageMeter.record_events(tenant.id, count: 5000)

      assert HybridPricing.calculate_overage(tenant, :events) == 0
    end

    test "returns 0 when usage equals quota", %{tenant: tenant} do
      {:ok, tenant} = UsageMeter.record_events(tenant.id, count: 10_000)

      assert HybridPricing.calculate_overage(tenant, :events) == 0
    end

    test "returns correct overage when usage exceeds quota", %{tenant: tenant} do
      {:ok, tenant} = UsageMeter.record_events(tenant.id, count: 12_000)

      assert HybridPricing.calculate_overage(tenant, :events) == 2_000
    end

    test "calculates overage for queries", %{tenant: tenant} do
      # Default queries quota is 1,000
      {:ok, tenant} = UsageMeter.record_queries(tenant.id, count: 1_500)

      assert HybridPricing.calculate_overage(tenant, :queries) == 500
    end

    test "returns 0 for enterprise tier with unlimited quota" do
      {:ok, enterprise_tenant} =
        Tenants.create_tenant(%{
          name: "Enterprise",
          slug: "enterprise-#{System.unique_integer([:positive])}"
        })

      {:ok, enterprise_tenant} =
        Tenants.update_subscription(enterprise_tenant, %{
          subscription_tier: :enterprise
        })

      {:ok, enterprise_tenant} = UsageMeter.record_events(enterprise_tenant.id, count: 10_000_000)

      assert HybridPricing.calculate_overage(enterprise_tenant, :events) == 0
    end
  end

  describe "calculate_overage_charges/1" do
    test "returns zero charges when under quota", %{tenant: tenant} do
      {:ok, _} = UsageMeter.record_events(tenant.id, count: 5000)
      {:ok, _} = UsageMeter.record_queries(tenant.id, count: 500)

      charges = HybridPricing.calculate_overage_charges(tenant.id)

      assert charges.events.overage_units == 0
      assert charges.events.charge_cents == 0
      assert charges.queries.overage_units == 0
      assert charges.queries.charge_cents == 0
      assert charges.total_cents == 0
    end

    test "calculates correct charges for events overage", %{tenant: tenant} do
      {:ok, _} = HybridPricing.enable_overage(tenant.id, events_rate: 50)
      {:ok, tenant} = UsageMeter.record_events(tenant.id, count: 12_000)

      charges = HybridPricing.calculate_overage_charges(tenant)

      assert charges.events.overage_units == 2_000
      assert charges.events.rate_cents == 50
      assert charges.events.charge_cents == 100_000
    end

    test "calculates correct charges for queries overage", %{tenant: tenant} do
      {:ok, _} = HybridPricing.enable_overage(tenant.id, queries_rate: 200)
      {:ok, tenant} = UsageMeter.record_queries(tenant.id, count: 1_200)

      charges = HybridPricing.calculate_overage_charges(tenant)

      assert charges.queries.overage_units == 200
      assert charges.queries.rate_cents == 200
      assert charges.queries.charge_cents == 40_000
    end

    test "calculates combined total correctly", %{tenant: tenant} do
      {:ok, _} = HybridPricing.enable_overage(tenant.id, events_rate: 10, queries_rate: 100)
      {:ok, _} = UsageMeter.record_events(tenant.id, count: 11_000)
      {:ok, tenant} = UsageMeter.record_queries(tenant.id, count: 1_100)

      charges = HybridPricing.calculate_overage_charges(tenant)

      # 1000 events over * 10 cents = 10,000 cents
      # 100 queries over * 100 cents = 10,000 cents
      assert charges.events.charge_cents == 10_000
      assert charges.queries.charge_cents == 10_000
      assert charges.total_cents == 20_000
    end
  end

  describe "enable_overage/2" do
    test "enables overage billing with default rates", %{tenant: tenant} do
      {:ok, updated} = HybridPricing.enable_overage(tenant.id)

      assert updated.overage_enabled == true
      assert updated.overage_rate_events == 100
      assert updated.overage_rate_queries == 1000
    end

    test "enables overage billing with custom rates", %{tenant: tenant} do
      {:ok, updated} =
        HybridPricing.enable_overage(tenant.id,
          events_rate: 25,
          queries_rate: 500
        )

      assert updated.overage_enabled == true
      assert updated.overage_rate_events == 25
      assert updated.overage_rate_queries == 500
    end

    test "sets subscription item IDs", %{tenant: tenant} do
      {:ok, updated} =
        HybridPricing.enable_overage(tenant.id,
          events_item_id: "item_events_123",
          queries_item_id: "item_queries_456"
        )

      assert updated.lemon_squeezy_events_item_id == "item_events_123"
      assert updated.lemon_squeezy_queries_item_id == "item_queries_456"
    end
  end

  describe "disable_overage/1" do
    test "disables overage billing", %{tenant: tenant} do
      {:ok, _} = HybridPricing.enable_overage(tenant.id)
      {:ok, updated} = HybridPricing.disable_overage(tenant.id)

      assert updated.overage_enabled == false
    end
  end

  describe "get_overage_summary/1" do
    test "returns complete summary", %{tenant: tenant} do
      {:ok, _} = HybridPricing.enable_overage(tenant.id, events_rate: 50, queries_rate: 100)
      {:ok, _} = UsageMeter.record_events(tenant.id, count: 11_000)
      {:ok, _} = UsageMeter.record_queries(tenant.id, count: 1_200)

      summary = HybridPricing.get_overage_summary(tenant.id)

      assert summary.overage_enabled == true
      assert summary.events.used == 11_000
      assert summary.events.quota == 10_000
      assert summary.events.overage == 1_000
      assert summary.events.rate_cents == 50
      assert summary.events.projected_charge_cents == 50_000

      assert summary.queries.used == 1_200
      assert summary.queries.quota == 1_000
      assert summary.queries.overage == 200
      assert summary.queries.rate_cents == 100
      assert summary.queries.projected_charge_cents == 20_000

      assert summary.total_projected_cents == 70_000
    end
  end

  describe "record_overage/3" do
    test "fails when overage is not enabled", %{tenant: tenant} do
      assert {:error, :overage_not_enabled} =
               HybridPricing.record_overage(tenant.id, :events, 100)
    end

    test "updates overage tracking when enabled", %{tenant: tenant} do
      {:ok, _} = HybridPricing.enable_overage(tenant.id)

      {:ok, updated} = HybridPricing.record_overage(tenant.id, :events, 100)

      assert updated.events_overage == 100
      assert updated.overage_reported_at != nil
    end

    test "accumulates overage correctly", %{tenant: tenant} do
      {:ok, _} = HybridPricing.enable_overage(tenant.id)

      {:ok, _} = HybridPricing.record_overage(tenant.id, :events, 100)
      {:ok, updated} = HybridPricing.record_overage(tenant.id, :events, 50)

      assert updated.events_overage == 150
    end

    test "tracks queries overage separately", %{tenant: tenant} do
      {:ok, _} = HybridPricing.enable_overage(tenant.id)

      {:ok, _} = HybridPricing.record_overage(tenant.id, :events, 100)
      {:ok, updated} = HybridPricing.record_overage(tenant.id, :queries, 50)

      assert updated.events_overage == 0
      assert updated.queries_overage == 50
    end
  end

  describe "reset_overage/1" do
    test "resets overage tracking", %{tenant: tenant} do
      {:ok, _} = HybridPricing.enable_overage(tenant.id)
      {:ok, _} = HybridPricing.record_overage(tenant.id, :events, 100)
      {:ok, _} = HybridPricing.record_overage(tenant.id, :queries, 50)

      {:ok, reset} = HybridPricing.reset_overage(tenant.id)

      assert reset.events_overage == 0
      assert reset.queries_overage == 0
      assert reset.overage_reported_at == nil
    end
  end

  describe "Tenant.overage_enabled?/1" do
    test "returns false when overage is disabled", %{tenant: tenant} do
      assert Tenant.overage_enabled?(tenant) == false
    end

    test "returns true when overage is enabled", %{tenant: tenant} do
      {:ok, updated} = HybridPricing.enable_overage(tenant.id)

      assert Tenant.overage_enabled?(updated) == true
    end
  end

  describe "Tenant.overage_rate/2" do
    test "returns the events overage rate", %{tenant: tenant} do
      {:ok, updated} = HybridPricing.enable_overage(tenant.id, events_rate: 75)

      assert Tenant.overage_rate(updated, :events) == 75
    end

    test "returns the queries overage rate", %{tenant: tenant} do
      {:ok, updated} = HybridPricing.enable_overage(tenant.id, queries_rate: 250)

      assert Tenant.overage_rate(updated, :queries) == 250
    end
  end
end
