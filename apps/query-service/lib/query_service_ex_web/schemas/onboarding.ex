defmodule QueryServiceExWeb.Schemas.Onboarding do
  @moduledoc """
  OpenAPI schemas for onboarding endpoints.
  """

  require OpenApiSpex

  alias OpenApiSpex.Schema

  defmodule OnboardingStep do
    @moduledoc "Onboarding step details"
    OpenApiSpex.schema(%{
      title: "OnboardingStep",
      description: "Details about an onboarding step",
      type: :object,
      properties: %{
        id: %Schema{type: :string, description: "Step identifier"},
        order: %Schema{type: :integer, description: "Step order (1-based)"},
        title: %Schema{type: :string},
        description: %Schema{type: :string},
        completed: %Schema{type: :boolean},
        current: %Schema{type: :boolean, description: "Whether this is the current step"},
        docs_url: %Schema{type: :string, format: :uri, nullable: true}
      },
      required: [:id, :order, :title, :completed, :current],
      example: %{
        id: "create_api_key",
        order: 1,
        title: "Create an API Key",
        description: "Generate your first API key to authenticate requests",
        completed: false,
        current: true,
        docs_url: "https://docs.allsource.dev/getting-started/api-keys"
      }
    })
  end

  defmodule OnboardingStatus do
    @moduledoc "Onboarding status"
    OpenApiSpex.schema(%{
      title: "OnboardingStatus",
      description: "Current onboarding status for a tenant",
      type: :object,
      properties: %{
        tenant_id: %Schema{type: :string, format: :uuid},
        completed: %Schema{type: :boolean, description: "Whether onboarding is complete"},
        skipped: %Schema{type: :boolean, description: "Whether onboarding was skipped"},
        current_step: %Schema{type: :string, nullable: true},
        completion_percentage: %Schema{
          type: :number,
          format: :float,
          minimum: 0,
          maximum: 100
        },
        completed_at: %Schema{type: :string, format: :"date-time", nullable: true},
        skipped_at: %Schema{type: :string, format: :"date-time", nullable: true},
        steps: %Schema{type: :array, items: OnboardingStep}
      },
      required: [:tenant_id, :completed, :skipped, :steps],
      example: %{
        tenant_id: "987fcdeb-51a2-43e1-b2c4-123456789000",
        completed: false,
        skipped: false,
        current_step: "create_api_key",
        completion_percentage: 25.0,
        completed_at: nil,
        skipped_at: nil,
        steps: [
          %{
            id: "create_api_key",
            order: 1,
            title: "Create an API Key",
            completed: true,
            current: false
          },
          %{
            id: "ingest_event",
            order: 2,
            title: "Ingest Your First Event",
            completed: false,
            current: true
          }
        ]
      }
    })
  end

  defmodule OnboardingStatusResponse do
    @moduledoc "Onboarding status response"
    OpenApiSpex.schema(%{
      title: "OnboardingStatusResponse",
      description: "Response containing onboarding status",
      type: :object,
      properties: %{
        data: OnboardingStatus
      },
      required: [:data]
    })
  end

  defmodule StepsResponse do
    @moduledoc "Onboarding steps response"
    OpenApiSpex.schema(%{
      title: "StepsResponse",
      description: "Response containing all onboarding step details",
      type: :object,
      properties: %{
        data: %Schema{type: :array, items: OnboardingStep}
      },
      required: [:data]
    })
  end

  defmodule InvalidStepError do
    @moduledoc "Invalid step error"
    OpenApiSpex.schema(%{
      title: "InvalidStepError",
      description: "Error response for invalid onboarding step",
      type: :object,
      properties: %{
        error: %Schema{
          type: :object,
          properties: %{
            code: %Schema{type: :string, enum: ["invalid_step"]},
            message: %Schema{type: :string},
            valid_steps: %Schema{type: :array, items: %Schema{type: :string}}
          },
          required: [:code, :message, :valid_steps]
        }
      },
      required: [:error],
      example: %{
        error: %{
          code: "invalid_step",
          message: "Invalid onboarding step: invalid_step_name",
          valid_steps: ["create_api_key", "ingest_event", "query_events", "create_projection"]
        }
      }
    })
  end
end
