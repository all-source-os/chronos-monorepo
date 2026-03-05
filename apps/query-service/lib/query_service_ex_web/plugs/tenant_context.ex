defmodule QueryServiceExWeb.Plugs.TenantContext do
  @moduledoc """
  Plug that sets the tenant context for authenticated requests.

  This plug extracts the tenant from the authenticated user and loads it
  from TenantCache (backed by Core) — not from PostgreSQL.

  It validates that the tenant has an active subscription (or is in trial)
  using metadata from Core's tenant record.

  When AUTH_DISABLED=true (dev mode), uses the dev tenant context instead.
  """

  import Plug.Conn
  alias QueryServiceEx.DevMode
  alias QueryServiceEx.Infrastructure.Adapters.RustCoreClient
  alias QueryServiceEx.TenantCache

  require Logger

  def init(opts), do: opts

  def call(conn, _opts) do
    if DevMode.auth_disabled?() do
      set_dev_tenant_context(conn)
    else
      set_user_tenant_context(conn)
    end
  end

  defp set_dev_tenant_context(conn) do
    dev_tenant = DevMode.dev_tenant()
    tid = tenant_id(dev_tenant)

    conn
    |> assign(:current_tenant, dev_tenant)
    |> assign(:tenant_id, tid)
    |> put_private(:allsource_tenant_id, tid)
  end

  defp set_user_tenant_context(conn) do
    user = conn.assigns[:current_user]

    cond do
      is_nil(user) ->
        conn

      is_nil(user.tenant_id) ->
        Logger.warning("[TenantContext] User #{user.id} has no tenant_id")
        send_error(conn, :unprocessable_entity, "no_tenant", "User has no associated workspace")

      true ->
        set_tenant_context(conn, user)
    end
  end

  defp set_tenant_context(conn, user) do
    tenant = TenantCache.fetch(user.tenant_id, fn -> load_tenant_from_core(user.tenant_id) end)

    cond do
      is_nil(tenant) ->
        # Lazy auto-provisioning: the JWT is valid but the tenant doesn't exist
        # in Core (e.g., after Core restart losing in-memory tenants, or race condition).
        # Create the tenant using JWT claims so the request can proceed.
        case auto_provision_tenant(user) do
          {:ok, new_tenant} ->
            Logger.info(
              "[TenantContext] Auto-provisioned tenant #{user.tenant_id} for user #{user.id}"
            )

            TenantCache.put(user.tenant_id, new_tenant)
            tid = tenant_id(new_tenant)

            conn
            |> assign(:current_tenant, new_tenant)
            |> assign(:tenant_id, tid)
            |> put_private(:allsource_tenant_id, tid)

          {:error, reason} ->
            Logger.warning(
              "[TenantContext] Tenant #{user.tenant_id} not found and auto-provision failed: #{inspect(reason)}"
            )

            send_error(conn, :not_found, "tenant_not_found", "Workspace not found")
        end

      not subscription_active?(tenant) ->
        status = get_subscription_status(tenant)

        Logger.info(
          "[TenantContext] Tenant #{tenant_id(tenant)} subscription not active: #{status}"
        )

        send_subscription_required(conn, tenant)

      true ->
        tid = tenant_id(tenant)

        conn
        |> assign(:current_tenant, tenant)
        |> assign(:tenant_id, tid)
        |> put_private(:allsource_tenant_id, tid)
    end
  end

  defp load_tenant_from_core(tenant_id) do
    case RustCoreClient.get_tenant(tenant_id) do
      {:ok, tenant} -> tenant
      {:error, _reason} -> nil
    end
  end

  defp auto_provision_tenant(user) do
    name = Map.get(user, :name) || Map.get(user, :email) || "Workspace"
    is_demo = Map.get(user, :is_demo, false)

    RustCoreClient.create_tenant(user.tenant_id, name, %{
      is_demo: is_demo,
      tier: if(is_demo, do: "unlimited", else: "free")
    })
  end

  # Subscription status helpers that work with both Ecto Tenant structs and Core maps

  defp subscription_active?(tenant) do
    get_subscription_status(tenant) in (~w(trialing active)a ++ ["trialing", "active"])
  end

  defp get_subscription_status(%{subscription_status: status}) when not is_nil(status), do: status

  defp get_subscription_status(tenant) when is_map(tenant) do
    get_in(tenant, ["metadata", "subscription", "status"]) ||
      get_in(tenant, [:metadata, :subscription, :status]) ||
      tenant["status"] ||
      "active"
  end

  defp tenant_id(%{id: id}), do: id
  defp tenant_id(%{"id" => id}), do: id

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
    status = get_subscription_status(tenant)
    trial_ends_at = get_metadata_field(tenant, "trial_ends_at")
    subscription_ends_at = get_metadata_field(tenant, "subscription_ends_at")

    conn
    |> put_status(:payment_required)
    |> Phoenix.Controller.json(%{
      error: %{
        code: "subscription_required",
        message: "Your subscription has #{status}. Please update your billing.",
        subscription_status: status,
        trial_ends_at: trial_ends_at,
        subscription_ends_at: subscription_ends_at
      }
    })
    |> halt()
  end

  defp get_metadata_field(%{trial_ends_at: v}, "trial_ends_at"), do: v
  defp get_metadata_field(%{subscription_ends_at: v}, "subscription_ends_at"), do: v

  defp get_metadata_field(tenant, field) when is_map(tenant) do
    get_in(tenant, ["metadata", "subscription", field]) ||
      get_in(tenant, [:metadata, :subscription, String.to_existing_atom(field)])
  rescue
    ArgumentError -> nil
  end

  defp get_metadata_field(_, _), do: nil
end
