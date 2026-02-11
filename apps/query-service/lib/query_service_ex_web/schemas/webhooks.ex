defmodule QueryServiceExWeb.Schemas.Webhooks do
  @moduledoc """
  OpenAPI schemas for webhook endpoints.
  """

  require OpenApiSpex

  alias OpenApiSpex.Schema

  defmodule LemonSqueezyWebhook do
    @moduledoc "LemonSqueezy webhook payload"
    OpenApiSpex.schema(%{
      title: "LemonSqueezyWebhook",
      description: "LemonSqueezy webhook event payload",
      type: :object,
      properties: %{
        meta: %Schema{
          type: :object,
          properties: %{
            event_name: %Schema{
              type: :string,
              description: "Event type",
              enum: [
                "subscription_created",
                "subscription_updated",
                "subscription_cancelled",
                "subscription_resumed",
                "subscription_expired",
                "subscription_paused",
                "subscription_unpaused",
                "subscription_payment_success",
                "subscription_payment_failed",
                "subscription_payment_recovered"
              ]
            },
            custom_data: %Schema{
              type: :object,
              properties: %{
                tenant_id: %Schema{type: :string}
              }
            }
          },
          required: [:event_name]
        },
        data: %Schema{
          type: :object,
          description: "Event-specific data",
          additionalProperties: true
        }
      },
      required: [:meta, :data],
      example: %{
        meta: %{
          event_name: "subscription_created",
          custom_data: %{tenant_id: "987fcdeb-51a2-43e1-b2c4-123456789000"}
        },
        data: %{
          type: "subscriptions",
          id: "12345",
          attributes: %{
            status: "active",
            variant_id: 123_456
          }
        }
      }
    })
  end

  defmodule WebhookResponse do
    @moduledoc "Webhook acknowledgment response"
    OpenApiSpex.schema(%{
      title: "WebhookResponse",
      description: "Response acknowledging webhook receipt",
      type: :object,
      properties: %{
        status: %Schema{type: :string, enum: ["ok"]},
        message: %Schema{type: :string}
      },
      required: [:status],
      example: %{
        status: "ok",
        message: "Webhook processed successfully"
      }
    })
  end

  defmodule WebhookError do
    @moduledoc "Webhook error response"
    OpenApiSpex.schema(%{
      title: "WebhookError",
      description: "Error response for webhook processing failure",
      type: :object,
      properties: %{
        error: %Schema{type: :string}
      },
      required: [:error],
      example: %{
        error: "Invalid signature"
      }
    })
  end
end
