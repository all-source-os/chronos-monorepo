defmodule QueryServiceExWeb.WebhookController do
  @moduledoc """
  Controller for handling webhook events from external services.

  Currently supports:
  - LemonSqueezy billing webhooks

  Note: Subscription management is now handled by the Control Plane (Go service).
  This controller verifies signatures, logs webhook events, and acknowledges them.
  """

  use Phoenix.Controller, formats: [:json]
  use OpenApiSpex.ControllerSpecs

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

  Subscription management is owned by the Control Plane.
  This endpoint verifies the signature, logs the event, and acknowledges receipt.
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
  # Webhook Processing (log-only, Control Plane owns subscriptions)
  # -------------------------------------------------------------------

  defp process_webhook(%{"meta" => %{"event_name" => event_name}, "data" => data}, correlation_id) do
    attrs = data["attributes"] || %{}

    Logger.info("[WebhookController] Processing event: #{event_name}",
      correlation_id: correlation_id,
      subscription_id: data["id"],
      status: map_status(attrs["status"]),
      tier: map_tier(attrs["variant_id"])
    )

    case event_name do
      "subscription_created" ->
        tenant_id = get_in(attrs, ["custom_data", "tenant_id"])

        Logger.info("[WebhookController] Subscription created",
          correlation_id: correlation_id,
          tenant_id: tenant_id,
          subscription_id: data["id"],
          status: attrs["status"]
        )

      "subscription_updated" ->
        Logger.info("[WebhookController] Subscription updated",
          correlation_id: correlation_id,
          subscription_id: data["id"],
          status: attrs["status"]
        )

      "subscription_cancelled" ->
        Logger.info("[WebhookController] Subscription cancelled",
          correlation_id: correlation_id,
          subscription_id: data["id"],
          ends_at: attrs["ends_at"]
        )

      "subscription_resumed" ->
        Logger.info("[WebhookController] Subscription resumed",
          correlation_id: correlation_id,
          subscription_id: data["id"]
        )

      "subscription_expired" ->
        Logger.info("[WebhookController] Subscription expired",
          correlation_id: correlation_id,
          subscription_id: data["id"]
        )

      "subscription_payment_success" ->
        Logger.info("[WebhookController] Payment succeeded",
          correlation_id: correlation_id,
          subscription_id: attrs["subscription_id"]
        )

      "subscription_payment_failed" ->
        Logger.warning("[WebhookController] Payment failed",
          correlation_id: correlation_id,
          subscription_id: attrs["subscription_id"]
        )

      _ ->
        Logger.info("[WebhookController] Ignoring unhandled event: #{event_name}",
          correlation_id: correlation_id
        )
    end

    :ok
  end

  defp process_webhook(_params, _correlation_id), do: {:error, :invalid_payload}

  # -------------------------------------------------------------------
  # Helpers (kept for structured logging)
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
end
