defmodule QueryServiceExWeb.Schemas.Events do
  @moduledoc """
  OpenAPI schemas for event endpoints.
  """

  require OpenApiSpex

  alias OpenApiSpex.Schema

  defmodule Event do
    @moduledoc "Event entity"
    OpenApiSpex.schema(%{
      title: "Event",
      description: "A domain event",
      type: :object,
      properties: %{
        id: %Schema{type: :string, format: :uuid, description: "Event ID"},
        entity_id: %Schema{type: :string, description: "Entity this event belongs to"},
        event_type: %Schema{type: :string, description: "Type of the event"},
        payload: %Schema{
          type: :object,
          additionalProperties: true,
          description: "Event payload data"
        },
        timestamp: %Schema{
          type: :string,
          format: :"date-time",
          description: "When the event occurred"
        },
        version: %Schema{type: :integer, description: "Event version in the stream"}
      },
      required: [:id, :entity_id, :event_type, :payload, :timestamp],
      example: %{
        id: "123e4567-e89b-12d3-a456-426614174000",
        entity_id: "order-123",
        event_type: "order.placed",
        payload: %{
          customer_id: "cust-456",
          total: 99.99,
          items: [%{sku: "WIDGET-1", quantity: 2}]
        },
        timestamp: "2026-02-10T12:00:00Z",
        version: 1
      }
    })
  end

  defmodule CreateEventRequest do
    @moduledoc "Request to create a new event"
    OpenApiSpex.schema(%{
      title: "CreateEventRequest",
      description: "Request body for creating a new event",
      type: :object,
      properties: %{
        entity_id: %Schema{
          type: :string,
          description: "Entity this event belongs to",
          minLength: 1,
          maxLength: 255
        },
        event_type: %Schema{
          type: :string,
          description: "Type of the event",
          minLength: 1,
          maxLength: 255
        },
        payload: %Schema{
          type: :object,
          additionalProperties: true,
          description: "Event payload data"
        }
      },
      required: [:entity_id, :event_type],
      example: %{
        entity_id: "order-123",
        event_type: "order.placed",
        payload: %{
          customer_id: "cust-456",
          total: 99.99
        }
      }
    })
  end

  defmodule CreateBatchRequest do
    @moduledoc "Request to create multiple events"
    OpenApiSpex.schema(%{
      title: "CreateBatchRequest",
      description: "Request body for creating multiple events",
      type: :object,
      properties: %{
        events: %Schema{
          type: :array,
          items: CreateEventRequest,
          minItems: 1,
          maxItems: 1000,
          description: "List of events to create"
        }
      },
      required: [:events],
      example: %{
        events: [
          %{entity_id: "order-123", event_type: "order.placed", payload: %{total: 99.99}},
          %{entity_id: "order-123", event_type: "order.confirmed", payload: %{}}
        ]
      }
    })
  end

  defmodule EventResponse do
    @moduledoc "Single event response"
    OpenApiSpex.schema(%{
      title: "EventResponse",
      description: "Response containing a single event",
      type: :object,
      properties: %{
        data: Event
      },
      required: [:data]
    })
  end

  defmodule EventListResponse do
    @moduledoc "List of events response"
    OpenApiSpex.schema(%{
      title: "EventListResponse",
      description: "Response containing a list of events",
      type: :object,
      properties: %{
        data: %Schema{
          type: :array,
          items: Event,
          description: "List of events"
        },
        count: %Schema{type: :integer, description: "Number of events returned"}
      },
      required: [:data, :count],
      example: %{
        data: [
          %{
            id: "123e4567-e89b-12d3-a456-426614174000",
            entity_id: "order-123",
            event_type: "order.placed",
            payload: %{total: 99.99},
            timestamp: "2026-02-10T12:00:00Z"
          }
        ],
        count: 1
      }
    })
  end

  defmodule EntityEventsResponse do
    @moduledoc "Events for a specific entity"
    OpenApiSpex.schema(%{
      title: "EntityEventsResponse",
      description: "Response containing events for a specific entity",
      type: :object,
      properties: %{
        data: %Schema{type: :array, items: Event},
        count: %Schema{type: :integer},
        entity_id: %Schema{type: :string, description: "The entity ID"}
      },
      required: [:data, :count, :entity_id]
    })
  end

  defmodule TypeEventsResponse do
    @moduledoc "Events of a specific type"
    OpenApiSpex.schema(%{
      title: "TypeEventsResponse",
      description: "Response containing events of a specific type",
      type: :object,
      properties: %{
        data: %Schema{type: :array, items: Event},
        count: %Schema{type: :integer},
        event_type: %Schema{type: :string, description: "The event type"}
      },
      required: [:data, :count, :event_type]
    })
  end

  defmodule BatchCreateResponse do
    @moduledoc "Batch creation response"
    OpenApiSpex.schema(%{
      title: "BatchCreateResponse",
      description: "Response after creating multiple events",
      type: :object,
      properties: %{
        data: %Schema{type: :array, items: Event},
        count: %Schema{type: :integer, description: "Number of events created"}
      },
      required: [:data, :count]
    })
  end
end
