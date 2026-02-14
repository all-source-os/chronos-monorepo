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
      description: "Response containing a single event with HAL links",
      type: :object,
      properties: %{
        data: Event,
        _links: QueryServiceExWeb.Schemas.Common.HALLinks
      },
      required: [:data]
    })
  end

  defmodule EventListResponse do
    @moduledoc "List of events response"
    OpenApiSpex.schema(%{
      title: "EventListResponse",
      description: "Response containing a list of events with HAL links",
      type: :object,
      properties: %{
        data: %Schema{
          type: :array,
          items: Event,
          description: "List of events"
        },
        count: %Schema{type: :integer, description: "Number of events returned"},
        _links: QueryServiceExWeb.Schemas.Common.HALLinks
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
      description: "Response containing events for a specific entity with HAL links",
      type: :object,
      properties: %{
        data: %Schema{type: :array, items: Event},
        count: %Schema{type: :integer},
        entity_id: %Schema{type: :string, description: "The entity ID"},
        _links: QueryServiceExWeb.Schemas.Common.HALLinks
      },
      required: [:data, :count, :entity_id]
    })
  end

  defmodule TypeEventsResponse do
    @moduledoc "Events of a specific type"
    OpenApiSpex.schema(%{
      title: "TypeEventsResponse",
      description: "Response containing events of a specific type with HAL links",
      type: :object,
      properties: %{
        data: %Schema{type: :array, items: Event},
        count: %Schema{type: :integer},
        event_type: %Schema{type: :string, description: "The event type"},
        _links: QueryServiceExWeb.Schemas.Common.HALLinks
      },
      required: [:data, :count, :event_type]
    })
  end

  defmodule BatchCreateResponse do
    @moduledoc "Batch creation response"
    OpenApiSpex.schema(%{
      title: "BatchCreateResponse",
      description: "Response after creating multiple events with HAL links",
      type: :object,
      properties: %{
        data: %Schema{type: :array, items: Event},
        count: %Schema{type: :integer, description: "Number of events created"},
        _links: QueryServiceExWeb.Schemas.Common.HALLinks
      },
      required: [:data, :count]
    })
  end

  defmodule StreamInfo do
    @moduledoc "Stream (entity) information"
    OpenApiSpex.schema(%{
      title: "StreamInfo",
      description: "Information about a stream (entity_id)",
      type: :object,
      properties: %{
        stream_id: %Schema{type: :string, description: "The stream identifier (entity_id)"},
        event_count: %Schema{type: :integer, description: "Total events in this stream"},
        last_event_at: %Schema{
          type: :string,
          format: :"date-time",
          nullable: true,
          description: "Timestamp of the last event"
        }
      },
      required: [:stream_id, :event_count],
      example: %{
        stream_id: "order-123",
        event_count: 42,
        last_event_at: "2026-02-10T15:30:00Z"
      }
    })
  end

  defmodule StreamsResponse do
    @moduledoc "List of streams response"
    OpenApiSpex.schema(%{
      title: "StreamsResponse",
      description: "Response containing a list of streams (entity_ids) with HAL links",
      type: :object,
      properties: %{
        data: %Schema{
          type: :array,
          items: StreamInfo,
          description: "List of streams with metadata"
        },
        count: %Schema{type: :integer, description: "Number of streams returned"},
        total: %Schema{type: :integer, description: "Total number of streams"},
        _links: QueryServiceExWeb.Schemas.Common.HALLinks
      },
      required: [:data, :count, :total],
      example: %{
        data: [
          %{stream_id: "order-123", event_count: 42, last_event_at: "2026-02-10T15:30:00Z"},
          %{stream_id: "user-456", event_count: 15, last_event_at: "2026-02-10T14:20:00Z"}
        ],
        count: 2,
        total: 150
      }
    })
  end

  defmodule EventTypeInfo do
    @moduledoc "Event type information"
    OpenApiSpex.schema(%{
      title: "EventTypeInfo",
      description: "Information about an event type",
      type: :object,
      properties: %{
        event_type: %Schema{type: :string, description: "The event type name"},
        event_count: %Schema{type: :integer, description: "Total events of this type"},
        last_event_at: %Schema{
          type: :string,
          format: :"date-time",
          nullable: true,
          description: "Timestamp of the last event of this type"
        }
      },
      required: [:event_type, :event_count],
      example: %{
        event_type: "order.placed",
        event_count: 1500,
        last_event_at: "2026-02-10T15:45:00Z"
      }
    })
  end

  defmodule EventTypesResponse do
    @moduledoc "List of event types response"
    OpenApiSpex.schema(%{
      title: "EventTypesResponse",
      description: "Response containing a list of event types with HAL links",
      type: :object,
      properties: %{
        data: %Schema{
          type: :array,
          items: EventTypeInfo,
          description: "List of event types with metadata"
        },
        count: %Schema{type: :integer, description: "Number of event types returned"},
        total: %Schema{type: :integer, description: "Total number of event types"},
        _links: QueryServiceExWeb.Schemas.Common.HALLinks
      },
      required: [:data, :count, :total],
      example: %{
        data: [
          %{event_type: "order.placed", event_count: 1500, last_event_at: "2026-02-10T15:45:00Z"},
          %{event_type: "user.created", event_count: 850, last_event_at: "2026-02-10T15:30:00Z"}
        ],
        count: 2,
        total: 25
      }
    })
  end
end
