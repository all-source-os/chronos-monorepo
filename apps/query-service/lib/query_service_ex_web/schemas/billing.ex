defmodule QueryServiceExWeb.Schemas.Billing do
  @moduledoc """
  OpenAPI schemas for billing endpoints.
  """

  require OpenApiSpex

  alias OpenApiSpex.Schema

  defmodule CheckoutRequest do
    @moduledoc "Checkout request"
    OpenApiSpex.schema(%{
      title: "CheckoutRequest",
      description: "Request body for creating a checkout session",
      type: :object,
      properties: %{
        variant_id: %Schema{
          type: :string,
          description: "LemonSqueezy product variant ID for the plan"
        }
      },
      required: [:variant_id],
      example: %{
        variant_id: "123456"
      }
    })
  end

  defmodule CheckoutResponse do
    @moduledoc "Checkout response"
    OpenApiSpex.schema(%{
      title: "CheckoutResponse",
      description: "Response containing checkout URL",
      type: :object,
      properties: %{
        data: %Schema{
          type: :object,
          properties: %{
            checkout_url: %Schema{type: :string, format: :uri}
          },
          required: [:checkout_url]
        }
      },
      required: [:data],
      example: %{
        data: %{
          checkout_url: "https://checkout.lemonsqueezy.com/checkout/..."
        }
      }
    })
  end

  defmodule PortalResponse do
    @moduledoc "Billing portal response"
    OpenApiSpex.schema(%{
      title: "PortalResponse",
      description: "Response containing billing portal URL",
      type: :object,
      properties: %{
        data: %Schema{
          type: :object,
          properties: %{
            portal_url: %Schema{type: :string, format: :uri, nullable: true},
            has_subscription: %Schema{type: :boolean},
            message: %Schema{type: :string, nullable: true}
          },
          required: [:has_subscription]
        }
      },
      required: [:data],
      example: %{
        data: %{
          portal_url: "https://app.lemonsqueezy.com/...",
          has_subscription: true
        }
      }
    })
  end

  defmodule OverageResponse do
    @moduledoc "Overage usage response"
    OpenApiSpex.schema(%{
      title: "OverageResponse",
      description: "Response containing overage usage details",
      type: :object,
      properties: %{
        data: %Schema{
          type: :object,
          properties: %{
            overage_enabled: %Schema{type: :boolean},
            events: %Schema{
              type: :object,
              properties: %{
                used: %Schema{type: :integer},
                quota: %Schema{type: :integer},
                overage: %Schema{type: :integer},
                rate_cents: %Schema{type: :integer, nullable: true}
              }
            },
            queries: %Schema{
              type: :object,
              properties: %{
                used: %Schema{type: :integer},
                quota: %Schema{type: :integer},
                overage: %Schema{type: :integer},
                rate_cents: %Schema{type: :integer, nullable: true}
              }
            },
            estimated_charges_cents: %Schema{type: :integer}
          }
        }
      },
      required: [:data]
    })
  end

  defmodule EnableOverageRequest do
    @moduledoc "Enable overage request"
    OpenApiSpex.schema(%{
      title: "EnableOverageRequest",
      description: "Request to enable overage billing",
      type: :object,
      properties: %{
        events_rate: %Schema{
          type: :integer,
          description: "Rate in cents per event over quota",
          nullable: true
        },
        queries_rate: %Schema{
          type: :integer,
          description: "Rate in cents per query over quota",
          nullable: true
        },
        events_item_id: %Schema{
          type: :string,
          description: "LemonSqueezy subscription item ID for events",
          nullable: true
        },
        queries_item_id: %Schema{
          type: :string,
          description: "LemonSqueezy subscription item ID for queries",
          nullable: true
        }
      },
      example: %{
        events_rate: 1,
        queries_rate: 5
      }
    })
  end

  defmodule EnableOverageResponse do
    @moduledoc "Enable overage response"
    OpenApiSpex.schema(%{
      title: "EnableOverageResponse",
      description: "Response after enabling overage billing",
      type: :object,
      properties: %{
        data: %Schema{
          type: :object,
          properties: %{
            message: %Schema{type: :string},
            overage: %Schema{type: :object}
          },
          required: [:message, :overage]
        }
      },
      required: [:data],
      example: %{
        data: %{
          message: "Overage billing enabled",
          overage: %{overage_enabled: true, estimated_charges_cents: 0}
        }
      }
    })
  end

  defmodule DisableOverageResponse do
    @moduledoc "Disable overage response"
    OpenApiSpex.schema(%{
      title: "DisableOverageResponse",
      description: "Response after disabling overage billing",
      type: :object,
      properties: %{
        data: %Schema{
          type: :object,
          properties: %{
            message: %Schema{type: :string}
          },
          required: [:message]
        }
      },
      required: [:data],
      example: %{
        data: %{message: "Overage billing disabled"}
      }
    })
  end

  defmodule ProjectedChargesResponse do
    @moduledoc "Projected charges response"
    OpenApiSpex.schema(%{
      title: "ProjectedChargesResponse",
      description: "Response containing projected charges for billing period",
      type: :object,
      properties: %{
        data: %Schema{
          type: :object,
          properties: %{
            subscription_tier: %Schema{type: :string},
            overage_enabled: %Schema{type: :boolean},
            usage: %Schema{type: :object},
            overage_charges: %Schema{
              type: :object,
              properties: %{
                events_overage: %Schema{type: :integer},
                queries_overage: %Schema{type: :integer},
                events_charges_cents: %Schema{type: :integer},
                queries_charges_cents: %Schema{type: :integer},
                total_charges_cents: %Schema{type: :integer}
              }
            },
            billing_period: %Schema{
              type: :object,
              properties: %{
                reset_at: %Schema{type: :string, format: :"date-time"}
              }
            }
          }
        }
      },
      required: [:data]
    })
  end
end
