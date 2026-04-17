defmodule QueryServiceExWeb.BillingControllerTest do
  use ExUnit.Case, async: true

  alias QueryServiceExWeb.BillingController

  describe "default_billing_state/1" do
    test "returns free tier with correct quotas" do
      state = BillingController.default_billing_state("tenant-test-1")

      assert state.tenant_id == "tenant-test-1"
      assert state.tier == "free"
      assert state.status == "active"
      assert state.billing_period == nil
      assert state.payment_provider == nil
      assert state.subscription_id == nil
      assert state.events_quota == 100_000
      assert state.queries_quota == 10_000
      assert state.events_used == 0
      assert state.queries_used == 0
      assert state.last_updated == nil
    end
  end

  describe "derive_state/2" do
    test "derives growth tier from billing event" do
      event = %{
        "payload" => %{
          "tier" => "growth",
          "status" => "active",
          "billing_period" => "monthly",
          "payment_provider" => "lemonsqueezy",
          "subscription_id" => "sub_123"
        },
        "timestamp" => "2026-03-26T12:00:00Z"
      }

      state = BillingController.derive_state("tenant-test-2", event)

      assert state.tenant_id == "tenant-test-2"
      assert state.tier == "growth"
      assert state.status == "active"
      assert state.billing_period == "monthly"
      assert state.payment_provider == "lemonsqueezy"
      assert state.subscription_id == "sub_123"
      assert state.events_quota == 10_000_000
      assert state.queries_quota == 1_000_000
      assert state.last_updated == "2026-03-26T12:00:00Z"
    end

    test "derives enterprise tier quotas" do
      event = %{
        "payload" => %{"tier" => "enterprise", "status" => "active"},
        "timestamp" => "2026-03-26T12:00:00Z"
      }

      state = BillingController.derive_state("t-3", event)

      assert state.tier == "enterprise"
      # Enterprise is unlimited (-1 sentinel).
      assert state.events_quota == -1
      assert state.queries_quota == -1
    end

    test "unknown tier gets free quotas" do
      event = %{
        "payload" => %{"tier" => "unknown", "status" => "active"},
        "timestamp" => "2026-03-26T12:00:00Z"
      }

      state = BillingController.derive_state("t-4", event)

      assert state.tier == "unknown"
      assert state.events_quota == 100_000
      assert state.queries_quota == 10_000
    end

    test "handles nil payload" do
      event = %{"payload" => nil, "timestamp" => "2026-03-26T12:00:00Z"}

      state = BillingController.derive_state("t-5", event)

      assert state.tier == "free"
      assert state.status == "active"
    end
  end
end
