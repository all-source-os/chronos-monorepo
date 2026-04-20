defmodule QueryServiceExWeb.Plugs.TenantContextTest do
  @moduledoc """
  Tests for the TenantContext plug.

  Verifies tenant isolation is properly enforced using TenantCache (ETS)
  backed by Core tenant data — no PostgreSQL dependency for tenant lookups.
  """
  use ExUnit.Case, async: true

  import Plug.Conn
  import Plug.Test

  alias QueryServiceEx.TenantCache
  alias QueryServiceExWeb.Plugs.TenantContext

  setup do
    # Run in enterprise mode so TenantContext resolves tenants instead of defaulting to community
    previous_edition = Application.get_env(:query_service_ex, :edition)
    Application.put_env(:query_service_ex, :edition, :enterprise)

    on_exit(fn ->
      Application.put_env(:query_service_ex, :edition, previous_edition || :community)
    end)

    # Ensure TenantCache ETS table exists (started by supervision tree)
    # Clear it before each test for isolation
    TenantCache.clear()

    tenant_a_id = generate_uuid()
    tenant_b_id = generate_uuid()

    # Core-format tenant maps (as returned by GET /api/v1/tenants/:id)
    tenant_a = %{
      "id" => tenant_a_id,
      "name" => "Tenant A",
      "status" => "active",
      "metadata" => %{
        "subscription" => %{
          "status" => "active",
          "tier" => "pro",
          "trial_ends_at" => nil,
          "subscription_ends_at" => nil
        },
        "quotas" => %{
          "events_quota" => 1_000_000,
          "queries_quota" => 100_000,
          "events_used" => 0,
          "queries_used" => 0
        }
      }
    }

    tenant_b = %{
      "id" => tenant_b_id,
      "name" => "Tenant B",
      "status" => "active",
      "metadata" => %{
        "subscription" => %{
          "status" => "trialing",
          "tier" => "free",
          "trial_ends_at" => "2026-03-01T00:00:00Z",
          "subscription_ends_at" => nil
        },
        "quotas" => %{
          "events_quota" => 10_000,
          "queries_quota" => 1_000,
          "events_used" => 0,
          "queries_used" => 0
        }
      }
    }

    # Pre-populate cache with Core tenant data
    TenantCache.put(tenant_a_id, tenant_a)
    TenantCache.put(tenant_b_id, tenant_b)

    user_a = %{id: generate_uuid(), tenant_id: tenant_a_id, email: "a@example.com"}
    user_b = %{id: generate_uuid(), tenant_id: tenant_b_id, email: "b@example.com"}

    {:ok,
     tenant_a: tenant_a,
     tenant_b: tenant_b,
     tenant_a_id: tenant_a_id,
     tenant_b_id: tenant_b_id,
     user_a: user_a,
     user_b: user_b}
  end

  describe "call/2 tenant isolation" do
    test "sets correct tenant context for user", %{tenant_a: tenant_a, user_a: user_a} do
      conn =
        conn(:get, "/api/events")
        |> assign_user(user_a)

      result = TenantContext.call(conn, TenantContext.init([]))

      refute result.halted
      assert result.assigns[:current_tenant]["id"] == tenant_a["id"]
      assert result.assigns[:tenant_id] == tenant_a["id"]
      assert result.private[:allsource_tenant_id] == tenant_a["id"]
    end

    test "user from tenant A gets tenant A context", %{tenant_a_id: tid, user_a: user_a} do
      conn =
        conn(:get, "/api/events")
        |> assign_user(user_a)

      result = TenantContext.call(conn, TenantContext.init([]))

      assert result.assigns[:tenant_id] == tid
    end

    test "user from tenant B gets tenant B context", %{tenant_b_id: tid, user_b: user_b} do
      conn =
        conn(:get, "/api/events")
        |> assign_user(user_b)

      result = TenantContext.call(conn, TenantContext.init([]))

      assert result.assigns[:tenant_id] == tid
    end

    test "tenant contexts are isolated between users", %{
      tenant_a_id: tid_a,
      tenant_b_id: tid_b,
      user_a: user_a,
      user_b: user_b
    } do
      conn_a =
        conn(:get, "/api/events")
        |> assign_user(user_a)
        |> TenantContext.call(TenantContext.init([]))

      conn_b =
        conn(:get, "/api/events")
        |> assign_user(user_b)
        |> TenantContext.call(TenantContext.init([]))

      assert conn_a.assigns[:tenant_id] == tid_a
      assert conn_b.assigns[:tenant_id] == tid_b
      refute conn_a.assigns[:tenant_id] == conn_b.assigns[:tenant_id]
    end
  end

  describe "call/2 with no user" do
    test "passes through when no user assigned" do
      conn = conn(:get, "/api/events")

      result = TenantContext.call(conn, TenantContext.init([]))

      refute result.halted
      assert is_nil(result.assigns[:current_tenant])
    end
  end

  describe "call/2 with user without tenant" do
    test "returns error when user has no tenant_id" do
      user = %{id: generate_uuid(), tenant_id: nil, email: "no-tenant@example.com"}

      conn =
        conn(:get, "/api/events")
        |> assign_user(user)
        |> put_private(:phoenix_format, "json")

      result = TenantContext.call(conn, TenantContext.init([]))

      assert result.halted
      assert result.status == 422

      response = Jason.decode!(result.resp_body)
      assert response["error"]["code"] == "no_tenant"
    end
  end

  describe "call/2 with missing tenant" do
    test "returns error when tenant is not in cache or Core" do
      user = %{id: generate_uuid(), tenant_id: generate_uuid(), email: "orphan@example.com"}

      conn =
        conn(:get, "/api/events")
        |> assign_user(user)
        |> put_private(:phoenix_format, "json")

      result = TenantContext.call(conn, TenantContext.init([]))

      assert result.halted
      assert result.status == 404

      response = Jason.decode!(result.resp_body)
      assert response["error"]["code"] == "tenant_not_found"
    end
  end

  describe "call/2 subscription validation" do
    test "allows active subscription", %{user_a: user_a} do
      conn =
        conn(:get, "/api/events")
        |> assign_user(user_a)

      result = TenantContext.call(conn, TenantContext.init([]))

      refute result.halted
    end

    test "allows trialing subscription", %{user_b: user_b} do
      conn =
        conn(:get, "/api/events")
        |> assign_user(user_b)

      result = TenantContext.call(conn, TenantContext.init([]))

      refute result.halted
    end

    test "blocks cancelled subscription" do
      tenant_id = generate_uuid()

      tenant = %{
        "id" => tenant_id,
        "name" => "Cancelled Tenant",
        "status" => "active",
        "metadata" => %{
          "subscription" => %{"status" => "cancelled", "tier" => "pro"}
        }
      }

      TenantCache.put(tenant_id, tenant)
      user = %{id: generate_uuid(), tenant_id: tenant_id, email: "cancelled@example.com"}

      conn =
        conn(:get, "/api/events")
        |> assign_user(user)
        |> put_private(:phoenix_format, "json")

      result = TenantContext.call(conn, TenantContext.init([]))

      assert result.halted
      assert result.status == 402

      response = Jason.decode!(result.resp_body)
      assert response["error"]["code"] == "subscription_required"
    end

    test "blocks expired subscription" do
      tenant_id = generate_uuid()

      tenant = %{
        "id" => tenant_id,
        "name" => "Expired Tenant",
        "status" => "active",
        "metadata" => %{
          "subscription" => %{"status" => "expired", "tier" => "free"}
        }
      }

      TenantCache.put(tenant_id, tenant)
      user = %{id: generate_uuid(), tenant_id: tenant_id, email: "expired@example.com"}

      conn =
        conn(:get, "/api/events")
        |> assign_user(user)
        |> put_private(:phoenix_format, "json")

      result = TenantContext.call(conn, TenantContext.init([]))

      assert result.halted
      assert result.status == 402
    end

    test "blocks past_due subscription" do
      tenant_id = generate_uuid()

      tenant = %{
        "id" => tenant_id,
        "name" => "Past Due Tenant",
        "status" => "active",
        "metadata" => %{
          "subscription" => %{"status" => "past_due", "tier" => "pro"}
        }
      }

      TenantCache.put(tenant_id, tenant)
      user = %{id: generate_uuid(), tenant_id: tenant_id, email: "pastdue@example.com"}

      conn =
        conn(:get, "/api/events")
        |> assign_user(user)
        |> put_private(:phoenix_format, "json")

      result = TenantContext.call(conn, TenantContext.init([]))

      assert result.halted
      assert result.status == 402
    end
  end

  describe "call/2 with Core tenant without subscription metadata" do
    test "treats tenant without subscription metadata as active" do
      tenant_id = generate_uuid()

      # Core tenant with no subscription metadata (freshly created, no billing yet)
      tenant = %{
        "id" => tenant_id,
        "name" => "New Tenant",
        "status" => "active",
        "metadata" => %{}
      }

      TenantCache.put(tenant_id, tenant)
      user = %{id: generate_uuid(), tenant_id: tenant_id, email: "new@example.com"}

      conn =
        conn(:get, "/api/events")
        |> assign_user(user)

      result = TenantContext.call(conn, TenantContext.init([]))

      # Defaults to "active" when no subscription metadata present
      refute result.halted
    end
  end

  describe "tenant_id consistency" do
    test "tenant_id in assigns matches current_tenant id", %{user_a: user_a, tenant_a_id: tid} do
      conn =
        conn(:get, "/api/events")
        |> assign_user(user_a)

      result = TenantContext.call(conn, TenantContext.init([]))

      assert result.assigns[:tenant_id] == result.assigns[:current_tenant]["id"]
      assert result.assigns[:tenant_id] == tid
    end

    test "private allsource_tenant_id matches assigns", %{user_a: user_a} do
      conn =
        conn(:get, "/api/events")
        |> assign_user(user_a)

      result = TenantContext.call(conn, TenantContext.init([]))

      assert result.private[:allsource_tenant_id] == result.assigns[:tenant_id]
    end
  end

  describe "call/2 dev mode (AUTH_DISABLED)" do
    setup do
      # Enable dev mode for these tests
      System.put_env("AUTH_DISABLED", "true")

      on_exit(fn ->
        System.delete_env("AUTH_DISABLED")
      end)
    end

    test "sets dev tenant context with string-keyed map" do
      conn = conn(:get, "/api/events")

      result = TenantContext.call(conn, TenantContext.init([]))

      refute result.halted
      assert result.assigns[:current_tenant]["id"] == "00000000-0000-0000-0000-000000000002"
      assert result.assigns[:tenant_id] == "00000000-0000-0000-0000-000000000002"
      assert result.private[:allsource_tenant_id] == "00000000-0000-0000-0000-000000000002"
    end

    test "dev tenant has active subscription status" do
      conn = conn(:get, "/api/events")

      result = TenantContext.call(conn, TenantContext.init([]))

      tenant = result.assigns[:current_tenant]
      assert tenant["status"] == "active"
      assert get_in(tenant, ["metadata", "subscription", "status"]) == "active"
    end

    test "dev tenant context works without current_user assigned" do
      # In dev mode, no user is needed — the plug should not crash
      conn = conn(:get, "/api/events")

      result = TenantContext.call(conn, TenantContext.init([]))

      refute result.halted
      assert is_binary(result.assigns[:tenant_id])
    end
  end

  describe "subscription_required response includes metadata" do
    test "includes trial_ends_at and subscription_ends_at from metadata" do
      tenant_id = generate_uuid()

      tenant = %{
        "id" => tenant_id,
        "name" => "Cancelled With Dates",
        "status" => "active",
        "metadata" => %{
          "subscription" => %{
            "status" => "cancelled",
            "tier" => "pro",
            "trial_ends_at" => "2026-01-15T00:00:00Z",
            "subscription_ends_at" => "2026-02-15T00:00:00Z"
          }
        }
      }

      TenantCache.put(tenant_id, tenant)
      user = %{id: generate_uuid(), tenant_id: tenant_id, email: "dates@example.com"}

      conn =
        conn(:get, "/api/events")
        |> assign_user(user)
        |> put_private(:phoenix_format, "json")

      result = TenantContext.call(conn, TenantContext.init([]))

      assert result.halted
      response = Jason.decode!(result.resp_body)
      assert response["error"]["trial_ends_at"] == "2026-01-15T00:00:00Z"
      assert response["error"]["subscription_ends_at"] == "2026-02-15T00:00:00Z"
    end
  end

  defp assign_user(conn, user) do
    Plug.Conn.assign(conn, :current_user, user)
  end

  defp generate_uuid do
    <<a::32, b::16, c::16, d::16, e::48>> = :crypto.strong_rand_bytes(16)

    <<a::32, b::16, 4::4, c::12, 2::2, d::14, e::48>>
    |> Base.encode16(case: :lower)
    |> then(fn hex ->
      <<a::binary-8, b::binary-4, c::binary-4, d::binary-4, e::binary-12>> = hex
      "#{a}-#{b}-#{c}-#{d}-#{e}"
    end)
  end
end
