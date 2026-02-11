defmodule QueryServiceExWeb.Plugs.TenantContext do
  @moduledoc """
  Plug that sets the tenant context for authenticated requests.

  This plug extracts the tenant from the authenticated user and stores it
  in the connection assigns for use by downstream plugs and controllers.

  It also validates that the tenant has an active subscription (or is in trial).

  When AUTH_DISABLED=true (dev mode), uses the dev tenant context instead.
  """

  import Plug.Conn
  alias QueryServiceEx.Accounts.Guardian
  alias QueryServiceEx.DevMode
  alias QueryServiceEx.Tenants
  alias QueryServiceEx.Tenants.Tenant

  require Logger

  def init(opts), do: opts

  def call(conn, _opts) do
    # Check for dev mode first
    if DevMode.auth_disabled?() do
      set_dev_tenant_context(conn)
    else
      set_user_tenant_context(conn)
    end
  end

  defp set_dev_tenant_context(conn) do
    dev_tenant = DevMode.dev_tenant()

    conn
    |> assign(:current_tenant, dev_tenant)
    |> assign(:tenant_id, dev_tenant.id)
    |> put_private(:chronos_tenant_id, dev_tenant.id)
  end

  defp set_user_tenant_context(conn) do
    user = Guardian.Plug.current_resource(conn)

    cond do
      is_nil(user) ->
        # No authenticated user - let auth pipeline handle it
        conn

      is_nil(user.tenant_id) ->
        # User has no tenant - this shouldn't happen but handle gracefully
        Logger.warning("[TenantContext] User #{user.id} has no tenant_id")
        send_error(conn, :unprocessable_entity, "no_tenant", "User has no associated workspace")

      true ->
        set_tenant_context(conn, user)
    end
  end

  defp set_tenant_context(conn, user) do
    tenant = Tenants.get_tenant(user.tenant_id)

    cond do
      is_nil(tenant) ->
        Logger.warning("[TenantContext] Tenant #{user.tenant_id} not found for user #{user.id}")
        send_error(conn, :not_found, "tenant_not_found", "Workspace not found")

      not Tenant.subscription_active?(tenant) ->
        Logger.info(
          "[TenantContext] Tenant #{tenant.id} subscription not active: #{tenant.subscription_status}"
        )

        send_subscription_required(conn, tenant)

      true ->
        conn
        |> assign(:current_tenant, tenant)
        |> assign(:tenant_id, tenant.id)
        |> put_private(:chronos_tenant_id, tenant.id)
    end
  end

  defp send_error(conn, status, code, message) do
    conn
    |> put_status(status)
    |> Phoenix.Controller.json(%{
      error: %{
        code: code,
        message: message
      }
    })
    |> halt()
  end

  defp send_subscription_required(conn, tenant) do
    conn
    |> put_status(:payment_required)
    |> Phoenix.Controller.json(%{
      error: %{
        code: "subscription_required",
        message:
          "Your subscription has #{tenant.subscription_status}. Please update your billing.",
        subscription_status: tenant.subscription_status,
        trial_ends_at: tenant.trial_ends_at,
        subscription_ends_at: tenant.subscription_ends_at
      }
    })
    |> halt()
  end
end
