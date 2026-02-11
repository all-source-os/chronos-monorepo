defmodule QueryServiceExWeb.WebhookController do
  @moduledoc """
  Controller for handling webhook events from external services.

  Currently supports:
  - LemonSqueezy billing webhooks
  """

  use Phoenix.Controller, formats: [:json]
  use OpenApiSpex.ControllerSpecs

  alias QueryServiceEx.Tenants
  alias QueryServiceExWeb.Schemas.Webhooks

  require Logger

  tags(["Webhooks"])

  operation(:lemonsqueezy,
    summary: "LemonSqueezy webhook",
    description: """
    Handles incoming webhooks from LemonSqueezy for subscription events.

    Events handled:
    - subscription_created
    - subscription_updated
    - subscription_cancelled
    - subscription_resumed
    - subscription_expired
    - subscription_payment_success
    - subscription_payment_failed
    """,
    request_body:
      {"LemonSqueezy webhook payload", "application/json", Webhooks.LemonSqueezyWebhook,
       required: true},
    responses: [
      ok: {"Webhook received", "application/json", Webhooks.WebhookResponse},
      unauthorized: {"Invalid signature", "application/json", Webhooks.WebhookError},
      unprocessable_entity: {"Processing failed", "application/json", Webhooks.WebhookError}
    ]
  )

  @doc """
  Handles incoming webhooks from LemonSqueezy.

  POST /api/webhooks/lemonsqueezy

  LemonSqueezy sends webhooks for:
  - subscription_created
  - subscription_updated
  - subscription_cancelled
  - subscription_resumed
  - subscription_expired
  - subscription_payment_success
  - subscription_payment_failed
  """
  def lemonsqueezy(conn, params) do
    correlation_id = conn.assigns[:correlation_id] || "unknown"
    raw_body = conn.assigns[:raw_body] || ""
    signature = get_req_header(conn, "x-signature") |> List.first()

    Logger.info("[WebhookController] Received LemonSqueezy webhook",
      correlation_id: correlation_id,
      event_name: params["meta"]["event_name"]
    )

    with :ok <- verify_signature(raw_body, signature),
         :ok <- process_webhook(params, correlation_id) do
      conn
      |> put_status(:ok)
      |> json(%{received: true})
    else
      {:error, :invalid_signature} ->
        Logger.warning("[WebhookController] Invalid webhook signature",
          correlation_id: correlation_id
        )

        conn
        |> put_status(:unauthorized)
        |> json(%{error: "Invalid signature"})

      {:error, reason} ->
        Logger.error("[WebhookController] Webhook processing failed: #{inspect(reason)}",
          correlation_id: correlation_id
        )

        conn
        |> put_status(:unprocessable_entity)
        |> json(%{error: "Processing failed"})
    end
  end

  # -------------------------------------------------------------------
  # Signature Verification
  # -------------------------------------------------------------------

  defp verify_signature(_body, nil), do: {:error, :invalid_signature}

  defp verify_signature(body, signature) do
    webhook_secret = Application.get_env(:query_service_ex, :lemon_squeezy)[:webhook_secret]

    if is_nil(webhook_secret) do
      Logger.warning("[WebhookController] No webhook secret configured - skipping verification")
      :ok
    else
      expected_signature =
        :crypto.mac(:hmac, :sha256, webhook_secret, body)
        |> Base.encode16(case: :lower)

      if Plug.Crypto.secure_compare(expected_signature, signature) do
        :ok
      else
        {:error, :invalid_signature}
      end
    end
  end

  # -------------------------------------------------------------------
  # Webhook Processing
  # -------------------------------------------------------------------

  defp process_webhook(%{"meta" => %{"event_name" => event_name}, "data" => data}, correlation_id) do
    Logger.info("[WebhookController] Processing event: #{event_name}",
      correlation_id: correlation_id
    )

    case event_name do
      "subscription_created" ->
        handle_subscription_created(data, correlation_id)

      "subscription_updated" ->
        handle_subscription_updated(data, correlation_id)

      "subscription_cancelled" ->
        handle_subscription_cancelled(data, correlation_id)

      "subscription_resumed" ->
        handle_subscription_resumed(data, correlation_id)

      "subscription_expired" ->
        handle_subscription_expired(data, correlation_id)

      "subscription_payment_success" ->
        handle_payment_success(data, correlation_id)

      "subscription_payment_failed" ->
        handle_payment_failed(data, correlation_id)

      _ ->
        Logger.info("[WebhookController] Ignoring unhandled event: #{event_name}",
          correlation_id: correlation_id
        )

        :ok
    end
  end

  defp process_webhook(_params, _correlation_id), do: {:error, :invalid_payload}

  # -------------------------------------------------------------------
  # Event Handlers
  # -------------------------------------------------------------------

  defp handle_subscription_created(data, correlation_id) do
    attrs = data["attributes"]
    tenant_id = get_in(attrs, ["custom_data", "tenant_id"])

    Logger.info("[WebhookController] Subscription created for tenant #{tenant_id}",
      correlation_id: correlation_id,
      subscription_id: data["id"],
      status: attrs["status"]
    )

    if tenant_id do
      tenant = Tenants.get_tenant(tenant_id)

      if tenant do
        Tenants.update_subscription(tenant, %{
          lemon_squeezy_customer_id: to_string(attrs["customer_id"]),
          lemon_squeezy_subscription_id: to_string(data["id"]),
          subscription_status: map_status(attrs["status"]),
          subscription_tier: map_tier(attrs["variant_id"])
        })
      else
        Logger.warning("[WebhookController] Tenant not found: #{tenant_id}",
          correlation_id: correlation_id
        )
      end
    end

    :ok
  end

  defp handle_subscription_updated(data, correlation_id) do
    attrs = data["attributes"]
    subscription_id = to_string(data["id"])

    Logger.info("[WebhookController] Subscription updated: #{subscription_id}",
      correlation_id: correlation_id,
      status: attrs["status"]
    )

    tenant = Tenants.get_tenant_by_subscription_id(subscription_id)

    if tenant do
      Tenants.update_subscription(tenant, %{
        subscription_status: map_status(attrs["status"]),
        subscription_tier: map_tier(attrs["variant_id"]),
        subscription_ends_at: parse_datetime(attrs["ends_at"])
      })
    end

    :ok
  end

  defp handle_subscription_cancelled(data, correlation_id) do
    subscription_id = to_string(data["id"])

    Logger.info("[WebhookController] Subscription cancelled: #{subscription_id}",
      correlation_id: correlation_id
    )

    tenant = Tenants.get_tenant_by_subscription_id(subscription_id)

    if tenant do
      attrs = data["attributes"]

      Tenants.update_subscription(tenant, %{
        subscription_status: :cancelled,
        subscription_ends_at: parse_datetime(attrs["ends_at"])
      })
    end

    :ok
  end

  defp handle_subscription_resumed(data, correlation_id) do
    subscription_id = to_string(data["id"])

    Logger.info("[WebhookController] Subscription resumed: #{subscription_id}",
      correlation_id: correlation_id
    )

    tenant = Tenants.get_tenant_by_subscription_id(subscription_id)

    if tenant do
      Tenants.update_subscription(tenant, %{
        subscription_status: :active
      })
    end

    :ok
  end

  defp handle_subscription_expired(data, correlation_id) do
    subscription_id = to_string(data["id"])

    Logger.info("[WebhookController] Subscription expired: #{subscription_id}",
      correlation_id: correlation_id
    )

    tenant = Tenants.get_tenant_by_subscription_id(subscription_id)

    if tenant do
      Tenants.update_subscription(tenant, %{
        subscription_status: :expired
      })
    end

    :ok
  end

  defp handle_payment_success(data, correlation_id) do
    subscription_id = to_string(data["attributes"]["subscription_id"])

    Logger.info("[WebhookController] Payment succeeded for subscription: #{subscription_id}",
      correlation_id: correlation_id
    )

    # Reset usage counters on successful payment (new billing period)
    tenant = Tenants.get_tenant_by_subscription_id(subscription_id)

    if tenant do
      Tenants.reset_usage(tenant.id)
    end

    :ok
  end

  defp handle_payment_failed(data, correlation_id) do
    subscription_id = to_string(data["attributes"]["subscription_id"])

    Logger.warning("[WebhookController] Payment failed for subscription: #{subscription_id}",
      correlation_id: correlation_id
    )

    tenant = Tenants.get_tenant_by_subscription_id(subscription_id)

    if tenant do
      Tenants.update_subscription(tenant, %{
        subscription_status: :past_due
      })
    end

    :ok
  end

  # -------------------------------------------------------------------
  # Helpers
  # -------------------------------------------------------------------

  defp map_status("active"), do: :active
  defp map_status("on_trial"), do: :trialing
  defp map_status("past_due"), do: :past_due
  defp map_status("cancelled"), do: :cancelled
  defp map_status("expired"), do: :expired
  defp map_status(_), do: :active

  # Map variant IDs to tiers (configure these based on your LemonSqueezy products)
  defp map_tier(variant_id) do
    tier_map = Application.get_env(:query_service_ex, :lemon_squeezy)[:variant_tiers] || %{}

    case Map.get(tier_map, to_string(variant_id)) do
      nil -> :starter
      tier -> tier
    end
  end

  defp parse_datetime(nil), do: nil

  defp parse_datetime(datetime_string) when is_binary(datetime_string) do
    case DateTime.from_iso8601(datetime_string) do
      {:ok, datetime, _offset} -> DateTime.truncate(datetime, :second)
      _ -> nil
    end
  end

  defp parse_datetime(_), do: nil
end
