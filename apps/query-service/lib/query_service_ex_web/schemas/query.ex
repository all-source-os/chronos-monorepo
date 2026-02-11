defmodule QueryServiceExWeb.Schemas.Query do
  @moduledoc """
  OpenAPI schemas for query endpoints.
  """

  require OpenApiSpex

  alias OpenApiSpex.Schema

  defmodule Predicate do
    @moduledoc "Query predicate"
    OpenApiSpex.schema(%{
      title: "Predicate",
      description: "Query predicate for filtering events",
      type: :object,
      oneOf: [
        %Schema{
          type: :object,
          description: "Comparison predicate",
          properties: %{
            op: %Schema{
              type: :string,
              enum: ["eq", "gt", "lt", "gte", "lte", "between", "in", "not_in"],
              description: "Comparison operator"
            },
            field: %Schema{type: :string, description: "Field to compare"},
            value: %Schema{description: "Value to compare against"}
          },
          required: [:op, :field, :value]
        },
        %Schema{
          type: :object,
          description: "Logical AND predicate",
          properties: %{
            and: %Schema{type: :array, description: "Predicates to AND together"}
          },
          required: [:and]
        },
        %Schema{
          type: :object,
          description: "Logical OR predicate",
          properties: %{
            or: %Schema{type: :array, description: "Predicates to OR together"}
          },
          required: [:or]
        }
      ],
      example: %{
        op: "eq",
        field: "event_type",
        value: "order.placed"
      }
    })
  end

  defmodule SimpleQueryRequest do
    @moduledoc "Simple query request"
    OpenApiSpex.schema(%{
      title: "SimpleQueryRequest",
      description: "Simple query with basic filters",
      type: :object,
      properties: %{
        entity_id: %Schema{type: :string, description: "Filter by entity ID"},
        event_type: %Schema{type: :string, description: "Filter by event type"},
        limit: %Schema{type: :integer, minimum: 1, maximum: 1000, default: 100},
        offset: %Schema{type: :integer, minimum: 0, default: 0}
      },
      example: %{
        entity_id: "order-123",
        event_type: "order.placed",
        limit: 100
      }
    })
  end

  defmodule DslQueryRequest do
    @moduledoc "DSL query request"
    OpenApiSpex.schema(%{
      title: "DslQueryRequest",
      description: "Advanced query using the Query DSL",
      type: :object,
      properties: %{
        from: %Schema{
          type: :string,
          enum: ["events"],
          description: "Source to query from",
          default: "events"
        },
        where: Predicate,
        limit: %Schema{type: :integer, minimum: 1, maximum: 1000, default: 100},
        offset: %Schema{type: :integer, minimum: 0, default: 0}
      },
      required: [:from],
      example: %{
        from: "events",
        where: %{
          and: [
            %{op: "eq", field: "event_type", value: "order.placed"},
            %{op: "gt", field: "payload.total", value: 100}
          ]
        },
        limit: 50
      }
    })
  end

  defmodule QueryRequest do
    @moduledoc "Query request (simple or DSL)"
    OpenApiSpex.schema(%{
      title: "QueryRequest",
      description: "Query request - supports both simple filters and advanced DSL",
      type: :object,
      oneOf: [SimpleQueryRequest, DslQueryRequest],
      example: %{
        from: "events",
        where: %{op: "eq", field: "event_type", value: "order.placed"},
        limit: 100
      }
    })
  end

  defmodule QuerySummary do
    @moduledoc "Query execution summary"
    OpenApiSpex.schema(%{
      title: "QuerySummary",
      description: "Summary of the executed query",
      type: :object,
      properties: %{
        from: %Schema{type: :string, description: "Source queried"},
        has_filter: %Schema{type: :boolean, description: "Whether filters were applied"},
        limit: %Schema{type: :integer, description: "Result limit"},
        offset: %Schema{type: :integer, description: "Result offset"},
        type: %Schema{type: :string, enum: ["simple", "dsl"]}
      }
    })
  end

  defmodule QueryResponse do
    @moduledoc "Query execution response"
    OpenApiSpex.schema(%{
      title: "QueryResponse",
      description: "Response from query execution",
      type: :object,
      properties: %{
        data: %Schema{
          type: :array,
          items: QueryServiceExWeb.Schemas.Events.Event,
          description: "Matching events"
        },
        count: %Schema{type: :integer, description: "Number of results"},
        query: QuerySummary
      },
      required: [:data, :count, :query],
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
        count: 1,
        query: %{
          from: "events",
          has_filter: true,
          limit: 100,
          offset: 0
        }
      }
    })
  end
end
