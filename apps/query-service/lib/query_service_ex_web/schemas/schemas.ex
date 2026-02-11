defmodule QueryServiceExWeb.Schemas.Schemas do
  @moduledoc """
  OpenAPI schemas for event schema endpoints.
  """

  require OpenApiSpex

  alias OpenApiSpex.Schema

  defmodule EventSchema do
    @moduledoc "Event schema entity"
    OpenApiSpex.schema(%{
      title: "EventSchema",
      description: "JSON Schema for validating event payloads",
      type: :object,
      properties: %{
        event_type: %Schema{type: :string, description: "Event type this schema applies to"},
        version: %Schema{type: :integer, description: "Schema version"},
        schema: %Schema{
          type: :object,
          additionalProperties: true,
          description: "JSON Schema definition"
        },
        created_at: %Schema{type: :string, format: :"date-time"},
        updated_at: %Schema{type: :string, format: :"date-time"}
      },
      required: [:event_type, :version, :schema],
      example: %{
        event_type: "order.placed",
        version: 1,
        schema: %{
          type: "object",
          required: ["order_id", "total"],
          properties: %{
            order_id: %{type: "string"},
            total: %{type: "number", minimum: 0}
          }
        }
      }
    })
  end

  defmodule RegisterSchemaRequest do
    @moduledoc "Request to register an event schema"
    OpenApiSpex.schema(%{
      title: "RegisterSchemaRequest",
      description: "Request body for registering an event schema",
      type: :object,
      properties: %{
        event_type: %Schema{
          type: :string,
          description: "Event type this schema applies to",
          minLength: 1,
          maxLength: 255
        },
        version: %Schema{type: :integer, minimum: 1, default: 1},
        schema: %Schema{
          type: :object,
          additionalProperties: true,
          description: "JSON Schema definition"
        }
      },
      required: [:event_type, :schema],
      example: %{
        event_type: "order.placed",
        version: 1,
        schema: %{
          type: "object",
          required: ["order_id"],
          properties: %{
            order_id: %{type: "string"}
          }
        }
      }
    })
  end

  defmodule SchemaResponse do
    @moduledoc "Single schema response"
    OpenApiSpex.schema(%{
      title: "SchemaResponse",
      description: "Response containing a single event schema",
      type: :object,
      properties: %{
        data: EventSchema
      },
      required: [:data]
    })
  end

  defmodule SchemaListResponse do
    @moduledoc "List of schemas response"
    OpenApiSpex.schema(%{
      title: "SchemaListResponse",
      description: "Response containing a list of event schemas",
      type: :object,
      properties: %{
        data: %Schema{type: :array, items: EventSchema},
        count: %Schema{type: :integer}
      },
      required: [:data, :count]
    })
  end
end
