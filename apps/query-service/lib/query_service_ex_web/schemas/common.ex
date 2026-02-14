defmodule QueryServiceExWeb.Schemas.Common do
  @moduledoc """
  Common OpenAPI schemas shared across multiple endpoints.
  """

  require OpenApiSpex

  alias OpenApiSpex.Schema

  defmodule Error do
    @moduledoc "Standard error response"
    OpenApiSpex.schema(%{
      title: "Error",
      description: "Standard error response",
      type: :object,
      properties: %{
        error: %Schema{
          type: :object,
          properties: %{
            code: %Schema{type: :string, description: "Error code"},
            message: %Schema{type: :string, description: "Human-readable error message"},
            details: %Schema{
              type: :object,
              additionalProperties: true,
              description: "Additional error details"
            }
          },
          required: [:code, :message]
        }
      },
      required: [:error],
      example: %{
        error: %{
          code: "validation_error",
          message: "Validation failed",
          details: %{name: ["can't be blank"]}
        }
      }
    })
  end

  defmodule SimpleError do
    @moduledoc "Simple error response with just a message"
    OpenApiSpex.schema(%{
      title: "SimpleError",
      description: "Simple error response",
      type: :object,
      properties: %{
        error: %Schema{type: :string, description: "Error message"}
      },
      required: [:error],
      example: %{error: "Resource not found"}
    })
  end

  defmodule Timestamp do
    @moduledoc "ISO 8601 timestamp"
    OpenApiSpex.schema(%{
      title: "Timestamp",
      description: "ISO 8601 formatted timestamp",
      type: :string,
      format: :"date-time",
      example: "2026-02-10T12:00:00Z"
    })
  end

  defmodule UUID do
    @moduledoc "UUID identifier"
    OpenApiSpex.schema(%{
      title: "UUID",
      description: "UUID v4 identifier",
      type: :string,
      format: :uuid,
      example: "123e4567-e89b-12d3-a456-426614174000"
    })
  end

  defmodule Pagination do
    @moduledoc "Pagination parameters"
    OpenApiSpex.schema(%{
      title: "Pagination",
      description: "Pagination parameters",
      type: :object,
      properties: %{
        limit: %Schema{
          type: :integer,
          minimum: 1,
          maximum: 1000,
          default: 100,
          description: "Maximum number of results"
        },
        offset: %Schema{
          type: :integer,
          minimum: 0,
          default: 0,
          description: "Number of results to skip"
        }
      }
    })
  end

  defmodule HALLink do
    @moduledoc "HAL hypermedia link"
    OpenApiSpex.schema(%{
      title: "HALLink",
      description: "A HAL hypermedia link",
      type: :object,
      properties: %{
        href: %Schema{type: :string, description: "Link URL"},
        title: %Schema{type: :string, description: "Human-readable link title"},
        templated: %Schema{type: :boolean, description: "Whether the href is a URI template"}
      },
      required: [:href],
      example: %{href: "/api/events", title: "Events"}
    })
  end

  defmodule HALLinks do
    @moduledoc "HAL _links map — keys are relation names (self, tenant, billing, etc.)"
    OpenApiSpex.schema(%{
      title: "HALLinks",
      description:
        "HAL hypermedia links. Keys are relation names (e.g. self, tenant, billing). " <>
          "Values are HALLink objects.",
      type: :object,
      additionalProperties: HALLink,
      example: %{
        "self" => %{href: "/api/events"},
        "tenant" => %{href: "/api/tenant", title: "Tenant"}
      }
    })
  end

  defmodule SimpleSuccess do
    @moduledoc "Simple success response"
    OpenApiSpex.schema(%{
      title: "SimpleSuccess",
      description: "Simple success response",
      type: :object,
      properties: %{
        success: %Schema{type: :boolean, description: "Success indicator"},
        message: %Schema{type: :string, description: "Success message"}
      },
      required: [:success],
      example: %{success: true, message: "Operation completed successfully"}
    })
  end
end
