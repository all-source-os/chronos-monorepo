defmodule QueryServiceExWeb.BillingController do
  @moduledoc """
  Controller for billing operations via LemonSqueezy.

  Provides endpoints for:
  - Creating checkout sessions for new subscriptions
  - Generating customer portal URLs for managing subscriptions
  """

  use Phoenix.Controller, formats: [:json]

  alias QueryServiceEx.Accounts.Guardian
  alias QueryServiceEx.Billing.HybridPricing
  alias QueryServiceEx.Billing.LemonSqueezy
  alias QueryServiceEx.Tenants

  require Logger

  action_fallback(QueryServiceExWeb.FallbackController)

  @doc """
  Creates a checkout session for a subscription plan.

  POST /api/billing/checkout

  Params:
  - variant_id: LemonSqueezy product variant ID for the plan
  """
  def checkout(conn, %{"variant_id" => variant_id}) do
    user = Guardian.Plug.current_resource(conn)
    tenant = Tenants.get_tenant!(user.tenant_id)
    correlation_id = conn.assigns[:correlation_id] || "unknown"

    Logger.info(
      "[BillingController] Creating checkout for tenant #{tenant.id}, variant #{variant_id}",
      correlation_id: correlation_id,
      tenant_id: tenant.id,
      variant_id: variant_id
    )

    checkout_params = %{
      email: user.email,
      name: user.name,
      tenant_id: tenant.id,
      variant_id: variant_id
    }

    case LemonSqueezy.create_checkout(checkout_params) do
      {:ok, checkout_url} ->
        conn
        |> put_status(:ok)
        |> json(%{data: %{checkout_url: checkout_url}})

      {:error, reason} ->
        Logger.error("[BillingController] Checkout creation failed: #{inspect(reason)}",
          correlation_id: correlation_id,
          tenant_id: tenant.id
        )

        conn
        |> put_status(:unprocessable_entity)
        |> json(%{
          error: %{code: "checkout_failed", message: "Failed to create checkout session"}
        })
    end
  end

  def checkout(conn, _params) do
    conn
    |> put_status(:bad_request)
    |> json(%{error: %{code: "missing_variant_id", message: "variant_id is required"}})
  end

  @doc """
  Returns a URL to the LemonSqueezy customer portal for managing subscriptions.

  GET /api/billing/portal
  """
  def portal(conn, _params) do
    user = Guardian.Plug.current_resource(conn)
    tenant = Tenants.get_tenant!(user.tenant_id)
    correlation_id = conn.assigns[:correlation_id] || "unknown"

    case tenant.lemon_squeezy_customer_id do
      nil ->
        # No customer yet - they haven't subscribed
        conn
        |> put_status(:ok)
        |> json(%{
          data: %{
            has_subscription: false,
            message: "No active subscription. Start a subscription to access the billing portal."
          }
        })

      customer_id ->
        Logger.info("[BillingController] Generating portal URL for customer #{customer_id}",
          correlation_id: correlation_id,
          tenant_id: tenant.id,
          customer_id: customer_id
        )

        case LemonSqueezy.get_customer_portal_url(customer_id) do
          {:ok, portal_url} ->
            conn
            |> put_status(:ok)
            |> json(%{data: %{portal_url: portal_url, has_subscription: true}})

          {:error, reason} ->
            Logger.error("[BillingController] Portal URL generation failed: #{inspect(reason)}",
              correlation_id: correlation_id,
              tenant_id: tenant.id
            )

            conn
            |> put_status(:unprocessable_entity)
            |> json(%{error: %{code: "portal_failed", message: "Failed to generate portal URL"}})
        end
    end
  end

  @doc """
  Returns the current overage usage and projected charges.

  GET /api/billing/overage
  """
  def overage(conn, _params) do
    user = Guardian.Plug.current_resource(conn)
    tenant = Tenants.get_tenant!(user.tenant_id)

    summary = HybridPricing.get_overage_summary(tenant.id)

    conn
    |> put_status(:ok)
    |> json(%{data: summary})
  end

  @doc """
  Enables overage billing for the tenant.

  POST /api/billing/overage/enable

  Params:
  - events_rate: Rate in cents per event over quota (optional)
  - queries_rate: Rate in cents per query over quota (optional)
  - events_item_id: LemonSqueezy subscription item ID for events (optional)
  - queries_item_id: LemonSqueezy subscription item ID for queries (optional)
  """
  def enable_overage(conn, params) do
    user = Guardian.Plug.current_resource(conn)
    tenant = Tenants.get_tenant!(user.tenant_id)
    correlation_id = conn.assigns[:correlation_id] || "unknown"

    Logger.info("[BillingController] Enabling overage billing for tenant #{tenant.id}",
      correlation_id: correlation_id,
      tenant_id: tenant.id
    )

    opts =
      [
        events_rate: params["events_rate"],
        queries_rate: params["queries_rate"],
        events_item_id: params["events_item_id"],
        queries_item_id: params["queries_item_id"]
      ]
      |> Enum.reject(fn {_k, v} -> is_nil(v) end)

    case HybridPricing.enable_overage(tenant.id, opts) do
      {:ok, updated_tenant} ->
        summary = HybridPricing.get_overage_summary(updated_tenant.id)

        conn
        |> put_status(:ok)
        |> json(%{
          data: %{
            message: "Overage billing enabled",
            overage: summary
          }
        })

      {:error, changeset} ->
        Logger.error("[BillingController] Failed to enable overage: #{inspect(changeset)}",
          correlation_id: correlation_id,
          tenant_id: tenant.id
        )

        conn
        |> put_status(:unprocessable_entity)
        |> json(%{error: %{code: "enable_failed", message: "Failed to enable overage billing"}})
    end
  end

  @doc """
  Disables overage billing for the tenant.

  POST /api/billing/overage/disable
  """
  def disable_overage(conn, _params) do
    user = Guardian.Plug.current_resource(conn)
    tenant = Tenants.get_tenant!(user.tenant_id)
    correlation_id = conn.assigns[:correlation_id] || "unknown"

    Logger.info("[BillingController] Disabling overage billing for tenant #{tenant.id}",
      correlation_id: correlation_id,
      tenant_id: tenant.id
    )

    case HybridPricing.disable_overage(tenant.id) do
      {:ok, _tenant} ->
        conn
        |> put_status(:ok)
        |> json(%{data: %{message: "Overage billing disabled"}})

      {:error, _changeset} ->
        conn
        |> put_status(:unprocessable_entity)
        |> json(%{error: %{code: "disable_failed", message: "Failed to disable overage billing"}})
    end
  end

  @doc """
  Returns projected charges for the current billing period.

  GET /api/billing/projected-charges
  """
  def projected_charges(conn, _params) do
    user = Guardian.Plug.current_resource(conn)
    tenant = Tenants.get_tenant!(user.tenant_id)

    charges = HybridPricing.calculate_overage_charges(tenant)
    usage_stats = Tenants.get_usage_stats(tenant.id)

    conn
    |> put_status(:ok)
    |> json(%{
      data: %{
        subscription_tier: tenant.subscription_tier,
        overage_enabled: tenant.overage_enabled,
        usage: usage_stats,
        overage_charges: charges,
        billing_period: %{
          reset_at: tenant.usage_reset_at
        }
      }
    })
  end
end
