defmodule QueryServiceEx.TenantsTest do
  @moduledoc """
  Tests for the Tenants context.

  These tests require a running PostgreSQL database.
  Run with: mix test --include database
  """
  use QueryServiceEx.DataCase

  alias QueryServiceEx.Tenants
  alias QueryServiceEx.Tenants.Tenant

  @moduletag :database

  describe "create_tenant/1" do
    test "creates a tenant with valid attributes" do
      attrs = %{
        name: "Test Workspace",
        slug: "test-workspace"
      }

      assert {:ok, tenant} = Tenants.create_tenant(attrs)
      assert tenant.name == "Test Workspace"
      assert tenant.slug == "test-workspace"
      assert tenant.subscription_status == :trialing
      assert tenant.subscription_tier == :free
      assert tenant.events_quota == 10_000
      assert tenant.queries_quota == 1_000
      assert tenant.events_used == 0
      assert tenant.queries_used == 0
      assert tenant.trial_ends_at != nil
    end

    test "requires name and slug" do
      assert {:error, changeset} = Tenants.create_tenant(%{})
      assert "can't be blank" in errors_on(changeset).name
      assert "can't be blank" in errors_on(changeset).slug
    end

    test "validates slug format" do
      attrs = %{name: "Test", slug: "Invalid Slug!"}
      assert {:error, changeset} = Tenants.create_tenant(attrs)

      assert "must be lowercase alphanumeric with hyphens, 2+ characters" in errors_on(changeset).slug
    end

    test "prevents duplicate slugs" do
      attrs = %{name: "Test", slug: "unique-slug"}
      assert {:ok, _} = Tenants.create_tenant(attrs)
      assert {:error, changeset} = Tenants.create_tenant(attrs)
      assert "has already been taken" in errors_on(changeset).slug
    end
  end

  describe "get_tenant/1" do
    test "returns tenant when found" do
      {:ok, created} = Tenants.create_tenant(%{name: "Test", slug: "test-get"})
      tenant = Tenants.get_tenant(created.id)
      assert tenant.id == created.id
    end

    test "returns nil when not found" do
      assert Tenants.get_tenant(Ecto.UUID.generate()) == nil
    end
  end

  describe "get_tenant_by_slug/1" do
    test "returns tenant by slug" do
      {:ok, created} = Tenants.create_tenant(%{name: "Test", slug: "slug-lookup"})
      tenant = Tenants.get_tenant_by_slug("slug-lookup")
      assert tenant.id == created.id
    end
  end

  describe "update_subscription/2" do
    test "updates subscription details" do
      {:ok, tenant} = Tenants.create_tenant(%{name: "Test", slug: "sub-test"})

      assert {:ok, updated} =
               Tenants.update_subscription(tenant, %{
                 lemon_squeezy_customer_id: "cust_123",
                 lemon_squeezy_subscription_id: "sub_456",
                 subscription_status: :active,
                 subscription_tier: :pro
               })

      assert updated.lemon_squeezy_customer_id == "cust_123"
      assert updated.lemon_squeezy_subscription_id == "sub_456"
      assert updated.subscription_status == :active
      assert updated.subscription_tier == :pro
      # Pro tier quotas
      assert updated.events_quota == 1_000_000
      assert updated.queries_quota == 100_000
    end
  end

  describe "usage metering" do
    test "increments events usage" do
      {:ok, tenant} = Tenants.create_tenant(%{name: "Test", slug: "usage-events"})
      assert tenant.events_used == 0

      assert {:ok, updated} = Tenants.increment_events_usage(tenant.id, 100)
      assert updated.events_used == 100

      assert {:ok, updated} = Tenants.increment_events_usage(tenant.id, 50)
      assert updated.events_used == 150
    end

    test "increments queries usage" do
      {:ok, tenant} = Tenants.create_tenant(%{name: "Test", slug: "usage-queries"})
      assert tenant.queries_used == 0

      assert {:ok, updated} = Tenants.increment_queries_usage(tenant.id, 10)
      assert updated.queries_used == 10
    end

    test "returns error when events quota exceeded" do
      {:ok, tenant} = Tenants.create_tenant(%{name: "Test", slug: "quota-exceeded"})

      # Set usage to quota limit
      {:ok, tenant} = Tenants.increment_events_usage(tenant.id, 10_000)

      assert {:error, :quota_exceeded} = Tenants.increment_events_usage(tenant.id, 1)
    end

    test "gets usage stats" do
      {:ok, tenant} = Tenants.create_tenant(%{name: "Test", slug: "usage-stats"})
      {:ok, _} = Tenants.increment_events_usage(tenant.id, 5000)
      {:ok, _} = Tenants.increment_queries_usage(tenant.id, 500)

      stats = Tenants.get_usage_stats(tenant.id)

      assert stats.events.used == 5000
      assert stats.events.quota == 10_000
      assert stats.events.remaining == 5000
      assert stats.events.percentage == 50.0

      assert stats.queries.used == 500
      assert stats.queries.quota == 1_000
      assert stats.queries.remaining == 500
      assert stats.queries.percentage == 50.0
    end

    test "resets usage counters" do
      {:ok, tenant} = Tenants.create_tenant(%{name: "Test", slug: "usage-reset"})
      {:ok, _} = Tenants.increment_events_usage(tenant.id, 5000)
      {:ok, _} = Tenants.increment_queries_usage(tenant.id, 500)

      {:ok, reset_tenant} = Tenants.reset_usage(tenant.id)

      assert reset_tenant.events_used == 0
      assert reset_tenant.queries_used == 0
      assert reset_tenant.usage_reset_at != nil
    end
  end

  describe "generate_slug/1" do
    test "generates slug from name" do
      slug = Tenants.generate_slug("My Company")
      assert slug =~ ~r/^my-company/
    end

    test "handles special characters" do
      slug = Tenants.generate_slug("Test!@#$%^&*()Company")
      assert slug =~ ~r/^test-company/
    end

    test "ensures uniqueness by adding suffix" do
      {:ok, _} = Tenants.create_tenant(%{name: "Test", slug: "unique-name"})

      slug = Tenants.generate_slug("Unique Name")
      assert slug != "unique-name"
      assert slug =~ ~r/^unique-name-[a-z0-9]+$/
    end
  end

  describe "subscription status checks" do
    test "subscription_active? returns true for trialing" do
      {:ok, tenant} = Tenants.create_tenant(%{name: "Test", slug: "active-trial"})
      assert Tenant.subscription_active?(tenant) == true
    end

    test "subscription_active? returns true for active" do
      {:ok, tenant} = Tenants.create_tenant(%{name: "Test", slug: "active-sub"})
      {:ok, tenant} = Tenants.update_subscription(tenant, %{subscription_status: :active})
      assert Tenant.subscription_active?(tenant) == true
    end

    test "subscription_active? returns false for cancelled" do
      {:ok, tenant} = Tenants.create_tenant(%{name: "Test", slug: "cancelled-sub"})
      {:ok, tenant} = Tenants.update_subscription(tenant, %{subscription_status: :cancelled})
      assert Tenant.subscription_active?(tenant) == false
    end

    test "in_trial? returns true when in trial period" do
      {:ok, tenant} = Tenants.create_tenant(%{name: "Test", slug: "in-trial"})
      assert Tenant.in_trial?(tenant) == true
    end

    test "in_trial? returns false when trial expired" do
      {:ok, tenant} = Tenants.create_tenant(%{name: "Test", slug: "expired-trial"})
      past_date = DateTime.utc_now() |> DateTime.add(-1, :day)

      {:ok, tenant} =
        Tenants.update_subscription(tenant, %{
          trial_ends_at: past_date
        })

      assert Tenant.in_trial?(tenant) == false
    end
  end
end
