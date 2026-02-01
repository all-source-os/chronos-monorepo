defmodule QueryServiceExWeb.Plugs.TenantContextTest do
  @moduledoc """
  Tests for the TenantContext plug.

  Verifies tenant isolation is properly enforced:
  - Users can only access their own tenant's data
  - Subscription status is validated
  - Tenant context is properly set for downstream operations
  """
  use QueryServiceEx.DataCase

  import Plug.Conn
  import Plug.Test

  alias QueryServiceEx.Accounts
  alias QueryServiceEx.Accounts.Guardian
  alias QueryServiceEx.Tenants
  alias QueryServiceExWeb.Plugs.TenantContext

  @moduletag :database

  setup do
    # Create two tenants for isolation testing
    {:ok, tenant_a} =
      Tenants.create_tenant(%{
        name: "Tenant A",
        slug: "tenant-a-#{System.unique_integer([:positive])}"
      })

    {:ok, tenant_b} =
      Tenants.create_tenant(%{
        name: "Tenant B",
        slug: "tenant-b-#{System.unique_integer([:positive])}"
      })

    # Create users for each tenant with required OAuth fields
    {:ok, user_a} =
      Accounts.create_user(%{
        email: "user-a-#{System.unique_integer([:positive])}@example.com",
        name: "User A",
        provider: "google",
        google_id: "google_a_#{System.unique_integer([:positive])}",
        tenant_id: tenant_a.id
      })

    {:ok, user_b} =
      Accounts.create_user(%{
        email: "user-b-#{System.unique_integer([:positive])}@example.com",
        name: "User B",
        provider: "google",
        google_id: "google_b_#{System.unique_integer([:positive])}",
        tenant_id: tenant_b.id
      })

    {:ok, tenant_a: tenant_a, tenant_b: tenant_b, user_a: user_a, user_b: user_b}
  end

  describe "call/2 tenant isolation" do
    test "sets correct tenant context for user", %{tenant_a: tenant_a, user_a: user_a} do
      conn =
        conn(:get, "/api/events")
        |> assign_user(user_a)

      result = TenantContext.call(conn, TenantContext.init([]))

      refute result.halted
      assert result.assigns[:current_tenant].id == tenant_a.id
      assert result.assigns[:tenant_id] == tenant_a.id
      assert result.private[:chronos_tenant_id] == tenant_a.id
    end

    test "user from tenant A gets tenant A context", %{tenant_a: tenant_a, user_a: user_a} do
      conn =
        conn(:get, "/api/events")
        |> assign_user(user_a)

      result = TenantContext.call(conn, TenantContext.init([]))

      assert result.assigns[:tenant_id] == tenant_a.id
    end

    test "user from tenant B gets tenant B context", %{tenant_b: tenant_b, user_b: user_b} do
      conn =
        conn(:get, "/api/events")
        |> assign_user(user_b)

      result = TenantContext.call(conn, TenantContext.init([]))

      assert result.assigns[:tenant_id] == tenant_b.id
    end

    test "tenant contexts are isolated between users", %{
      tenant_a: tenant_a,
      tenant_b: tenant_b,
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

      # Verify each user gets their own tenant
      assert conn_a.assigns[:tenant_id] == tenant_a.id
      assert conn_b.assigns[:tenant_id] == tenant_b.id

      # Verify tenants are different
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
      # Create user struct with nil tenant_id
      user = %{id: Ecto.UUID.generate(), tenant_id: nil, email: "no-tenant@example.com"}

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

  describe "call/2 with deleted tenant" do
    test "returns error when tenant no longer exists", %{user_a: user_a, tenant_a: tenant_a} do
      # Delete the tenant
      Tenants.delete_tenant(tenant_a)

      conn =
        conn(:get, "/api/events")
        |> assign_user(user_a)
        |> put_private(:phoenix_format, "json")

      result = TenantContext.call(conn, TenantContext.init([]))

      assert result.halted
      assert result.status == 404

      response = Jason.decode!(result.resp_body)
      assert response["error"]["code"] == "tenant_not_found"
    end
  end

  describe "call/2 subscription validation" do
    test "allows trialing subscription", %{user_a: user_a} do
      conn =
        conn(:get, "/api/events")
        |> assign_user(user_a)

      result = TenantContext.call(conn, TenantContext.init([]))

      refute result.halted
    end

    test "allows active subscription", %{user_a: user_a, tenant_a: tenant_a} do
      {:ok, _} = Tenants.update_subscription(tenant_a, %{subscription_status: :active})

      conn =
        conn(:get, "/api/events")
        |> assign_user(user_a)

      result = TenantContext.call(conn, TenantContext.init([]))

      refute result.halted
    end

    test "blocks cancelled subscription", %{user_a: user_a, tenant_a: tenant_a} do
      {:ok, _} = Tenants.update_subscription(tenant_a, %{subscription_status: :cancelled})

      conn =
        conn(:get, "/api/events")
        |> assign_user(user_a)
        |> put_private(:phoenix_format, "json")

      result = TenantContext.call(conn, TenantContext.init([]))

      assert result.halted
      assert result.status == 402

      response = Jason.decode!(result.resp_body)
      assert response["error"]["code"] == "subscription_required"
    end

    test "blocks expired subscription", %{user_a: user_a, tenant_a: tenant_a} do
      {:ok, _} = Tenants.update_subscription(tenant_a, %{subscription_status: :expired})

      conn =
        conn(:get, "/api/events")
        |> assign_user(user_a)
        |> put_private(:phoenix_format, "json")

      result = TenantContext.call(conn, TenantContext.init([]))

      assert result.halted
      assert result.status == 402
    end

    test "blocks past_due subscription", %{user_a: user_a, tenant_a: tenant_a} do
      {:ok, _} = Tenants.update_subscription(tenant_a, %{subscription_status: :past_due})

      conn =
        conn(:get, "/api/events")
        |> assign_user(user_a)
        |> put_private(:phoenix_format, "json")

      result = TenantContext.call(conn, TenantContext.init([]))

      assert result.halted
      assert result.status == 402
    end
  end

  describe "tenant_id consistency" do
    test "tenant_id in assigns matches current_tenant.id", %{user_a: user_a, tenant_a: tenant_a} do
      conn =
        conn(:get, "/api/events")
        |> assign_user(user_a)

      result = TenantContext.call(conn, TenantContext.init([]))

      assert result.assigns[:tenant_id] == result.assigns[:current_tenant].id
      assert result.assigns[:tenant_id] == tenant_a.id
    end

    test "private chronos_tenant_id matches assigns", %{user_a: user_a} do
      conn =
        conn(:get, "/api/events")
        |> assign_user(user_a)

      result = TenantContext.call(conn, TenantContext.init([]))

      assert result.private[:chronos_tenant_id] == result.assigns[:tenant_id]
    end
  end

  # Helper to simulate Guardian setting current_resource
  defp assign_user(conn, user) do
    conn
    |> Guardian.Plug.put_current_resource(user)
  end
end
