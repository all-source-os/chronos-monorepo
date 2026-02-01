defmodule QueryServiceExWeb.Plugs.UsageEnforcementTest do
  @moduledoc """
  Tests for the UsageEnforcement plug.

  Verifies that subscription limits are enforced for free tier users
  and overage billing works correctly for paid tiers.
  """
  use QueryServiceEx.DataCase

  import Plug.Conn
  import Plug.Test

  alias QueryServiceEx.Billing.HybridPricing
  alias QueryServiceEx.Tenants
  alias QueryServiceEx.Tenants.Tenant
  alias QueryServiceEx.UsageMeter
  alias QueryServiceExWeb.Plugs.UsageEnforcement

  @moduletag :database

  setup do
    {:ok, tenant} =
      Tenants.create_tenant(%{
        name: "Enforcement Test",
        slug: "enforcement-test-#{System.unique_integer([:positive])}"
      })

    {:ok, tenant: tenant}
  end

  describe "init/1" do
    test "accepts :events type" do
      opts = UsageEnforcement.init(type: :events)
      assert opts == %{type: :events}
    end

    test "accepts :queries type" do
      opts = UsageEnforcement.init(type: :queries)
      assert opts == %{type: :queries}
    end

    test "defaults to :events type" do
      opts = UsageEnforcement.init([])
      assert opts == %{type: :events}
    end

    test "raises on invalid type" do
      assert_raise ArgumentError, ~r/must be :events or :queries/, fn ->
        UsageEnforcement.init(type: :invalid)
      end
    end
  end

  describe "call/2 with no tenant context" do
    test "passes through when no tenant is assigned" do
      conn =
        conn(:get, "/api/events")
        |> assign(:current_tenant, nil)

      opts = UsageEnforcement.init(type: :events)
      result = UsageEnforcement.call(conn, opts)

      refute result.halted
    end
  end

  describe "call/2 with events quota - hard limit mode" do
    test "allows request when under quota", %{tenant: tenant} do
      {:ok, tenant} = UsageMeter.record_events(tenant.id, count: 5000)

      conn =
        conn(:post, "/api/events")
        |> assign(:current_tenant, tenant)

      opts = UsageEnforcement.init(type: :events)
      result = UsageEnforcement.call(conn, opts)

      refute result.halted
    end

    test "blocks request when quota exceeded", %{tenant: tenant} do
      # Use up all quota (10,000 events for free tier)
      {:ok, tenant} = UsageMeter.record_events(tenant.id, count: 10_000)

      conn =
        conn(:post, "/api/events")
        |> assign(:current_tenant, tenant)
        |> put_private(:phoenix_format, "json")

      opts = UsageEnforcement.init(type: :events)
      result = UsageEnforcement.call(conn, opts)

      assert result.halted
      assert result.status == 402

      response = Jason.decode!(result.resp_body)
      assert response["error"]["code"] == "quota_exceeded"
      assert response["error"]["usage_type"] == "events"
      assert response["error"]["used"] == 10_000
      assert response["error"]["quota"] == 10_000
    end

    test "includes upgrade URL in error response", %{tenant: tenant} do
      {:ok, tenant} = UsageMeter.record_events(tenant.id, count: 10_000)

      conn =
        conn(:post, "/api/events")
        |> assign(:current_tenant, tenant)
        |> put_private(:phoenix_format, "json")

      opts = UsageEnforcement.init(type: :events)
      result = UsageEnforcement.call(conn, opts)

      response = Jason.decode!(result.resp_body)
      assert response["error"]["upgrade_url"] == "/billing/upgrade"
      assert response["error"]["enable_overage_url"] == "/billing/enable-overage"
    end
  end

  describe "call/2 with queries quota - hard limit mode" do
    test "allows request when under quota", %{tenant: tenant} do
      {:ok, tenant} = UsageMeter.record_queries(tenant.id, count: 500)

      conn =
        conn(:post, "/api/query")
        |> assign(:current_tenant, tenant)

      opts = UsageEnforcement.init(type: :queries)
      result = UsageEnforcement.call(conn, opts)

      refute result.halted
    end

    test "blocks request when quota exceeded", %{tenant: tenant} do
      # Use up all quota (1,000 queries for free tier)
      {:ok, tenant} = UsageMeter.record_queries(tenant.id, count: 1_000)

      conn =
        conn(:post, "/api/query")
        |> assign(:current_tenant, tenant)
        |> put_private(:phoenix_format, "json")

      opts = UsageEnforcement.init(type: :queries)
      result = UsageEnforcement.call(conn, opts)

      assert result.halted
      assert result.status == 402

      response = Jason.decode!(result.resp_body)
      assert response["error"]["code"] == "quota_exceeded"
      assert response["error"]["usage_type"] == "queries"
    end
  end

  describe "call/2 with overage billing enabled - soft limit mode" do
    test "allows request with overage headers when quota exceeded", %{tenant: tenant} do
      # Enable overage billing
      {:ok, _} = HybridPricing.enable_overage(tenant.id)

      # Exceed quota
      {:ok, tenant} = UsageMeter.record_events(tenant.id, count: 12_000)

      conn =
        conn(:post, "/api/events")
        |> assign(:current_tenant, tenant)

      opts = UsageEnforcement.init(type: :events)
      result = UsageEnforcement.call(conn, opts)

      # Should NOT be halted (soft limit)
      refute result.halted

      # Should have overage headers
      assert get_resp_header(result, "x-usage-overage") == ["true"]
      assert get_resp_header(result, "x-usage-type") == ["events"]
      assert get_resp_header(result, "x-usage-used") == ["12000"]
      assert get_resp_header(result, "x-usage-quota") == ["10000"]

      # Should have conn assigns for downstream use
      assert result.assigns[:in_overage] == true
      assert result.assigns[:overage_type] == :events
    end

    test "includes projected charge in overage headers", %{tenant: tenant} do
      # Enable with custom rate (50 cents per event over)
      {:ok, _} = HybridPricing.enable_overage(tenant.id, events_rate: 50)

      # 2000 events over quota = 100,000 cents = $1,000
      {:ok, tenant} = UsageMeter.record_events(tenant.id, count: 12_000)

      conn =
        conn(:post, "/api/events")
        |> assign(:current_tenant, tenant)

      opts = UsageEnforcement.init(type: :events)
      result = UsageEnforcement.call(conn, opts)

      # Should have projected charge header
      projected_charge = get_resp_header(result, "x-projected-charge-cents")
      assert projected_charge == ["100000"]
    end
  end

  describe "enterprise tier with unlimited quota" do
    test "allows requests without quota check" do
      {:ok, enterprise_tenant} =
        Tenants.create_tenant(%{
          name: "Enterprise",
          slug: "enterprise-#{System.unique_integer([:positive])}"
        })

      {:ok, enterprise_tenant} =
        Tenants.update_subscription(enterprise_tenant, %{
          subscription_tier: :enterprise
        })

      # Record massive usage
      {:ok, enterprise_tenant} = UsageMeter.record_events(enterprise_tenant.id, count: 10_000_000)

      conn =
        conn(:post, "/api/events")
        |> assign(:current_tenant, enterprise_tenant)

      opts = UsageEnforcement.init(type: :events)
      result = UsageEnforcement.call(conn, opts)

      refute result.halted
    end
  end

  describe "subscription tier quotas" do
    test "free tier has 10k events and 1k queries quota", %{tenant: tenant} do
      assert tenant.events_quota == 10_000
      assert tenant.queries_quota == 1_000
    end

    test "starter tier has 100k events and 10k queries quota" do
      {:ok, tenant} =
        Tenants.create_tenant(%{
          name: "Starter",
          slug: "starter-#{System.unique_integer([:positive])}"
        })

      {:ok, tenant} =
        Tenants.update_subscription(tenant, %{
          subscription_tier: :starter
        })

      assert tenant.events_quota == 100_000
      assert tenant.queries_quota == 10_000
    end

    test "pro tier has 1M events and 100k queries quota" do
      {:ok, tenant} =
        Tenants.create_tenant(%{
          name: "Pro",
          slug: "pro-#{System.unique_integer([:positive])}"
        })

      {:ok, tenant} =
        Tenants.update_subscription(tenant, %{
          subscription_tier: :pro
        })

      assert tenant.events_quota == 1_000_000
      assert tenant.queries_quota == 100_000
    end

    test "enterprise tier has unlimited quota" do
      {:ok, tenant} =
        Tenants.create_tenant(%{
          name: "Enterprise",
          slug: "enterprise-quota-#{System.unique_integer([:positive])}"
        })

      {:ok, tenant} =
        Tenants.update_subscription(tenant, %{
          subscription_tier: :enterprise
        })

      # -1 represents unlimited quota
      assert tenant.events_quota == -1
      assert tenant.queries_quota == -1
    end
  end

  describe "Tenant quota helper functions" do
    test "events_quota_exceeded? returns false when under quota", %{tenant: tenant} do
      {:ok, tenant} = UsageMeter.record_events(tenant.id, count: 5000)
      refute Tenant.events_quota_exceeded?(tenant)
    end

    test "events_quota_exceeded? returns true when at or over quota", %{tenant: tenant} do
      {:ok, tenant} = UsageMeter.record_events(tenant.id, count: 10_000)
      assert Tenant.events_quota_exceeded?(tenant)
    end

    test "queries_quota_exceeded? returns false when under quota", %{tenant: tenant} do
      {:ok, tenant} = UsageMeter.record_queries(tenant.id, count: 500)
      refute Tenant.queries_quota_exceeded?(tenant)
    end

    test "queries_quota_exceeded? returns true when at or over quota", %{tenant: tenant} do
      {:ok, tenant} = UsageMeter.record_queries(tenant.id, count: 1_000)
      assert Tenant.queries_quota_exceeded?(tenant)
    end

    test "unlimited quota is never exceeded" do
      {:ok, tenant} =
        Tenants.create_tenant(%{
          name: "Enterprise",
          slug: "enterprise-unlimited-#{System.unique_integer([:positive])}"
        })

      {:ok, tenant} =
        Tenants.update_subscription(tenant, %{
          subscription_tier: :enterprise
        })

      {:ok, tenant} = UsageMeter.record_events(tenant.id, count: 100_000_000)

      refute Tenant.events_quota_exceeded?(tenant)
      refute Tenant.queries_quota_exceeded?(tenant)
    end
  end
end
