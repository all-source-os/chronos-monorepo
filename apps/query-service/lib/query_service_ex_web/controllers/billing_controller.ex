defmodule QueryServiceExWeb.BillingController do
  @moduledoc """
  Controller for billing operations via LemonSqueezy.

  Provides endpoints for:
  - Creating checkout sessions for new subscriptions
  - Generating customer portal URLs for managing subscriptions
  """

  use Phoenix.Controller, formats: [:json]

  alias QueryServiceEx.Accounts.Guardian
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
end
