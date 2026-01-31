defmodule QueryServiceEx.Billing.LemonSqueezyTest do
  use ExUnit.Case, async: true

  alias QueryServiceEx.Billing.LemonSqueezy

  describe "when not configured" do
    test "create_checkout returns not_configured error" do
      params = %{
        email: "test@example.com",
        name: "Test User",
        tenant_id: "123",
        variant_id: "456"
      }

      # Without API key configured, should return error
      assert {:error, :not_configured} = LemonSqueezy.create_checkout(params)
    end

    test "get_customer_portal_url returns not_configured error" do
      assert {:error, :not_configured} = LemonSqueezy.get_customer_portal_url("cust_123")
    end

    test "get_subscription returns not_configured error" do
      assert {:error, :not_configured} = LemonSqueezy.get_subscription("sub_123")
    end

    test "cancel_subscription returns not_configured error" do
      assert {:error, :not_configured} = LemonSqueezy.cancel_subscription("sub_123")
    end

    test "resume_subscription returns not_configured error" do
      assert {:error, :not_configured} = LemonSqueezy.resume_subscription("sub_123")
    end

    test "report_usage returns not_configured error" do
      assert {:error, :not_configured} = LemonSqueezy.report_usage("item_123", 100)
    end
  end
end
