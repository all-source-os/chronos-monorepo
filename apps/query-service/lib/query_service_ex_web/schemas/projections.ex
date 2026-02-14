defmodule QueryServiceExWeb.Schemas.Projections do
  @moduledoc """
  OpenAPI schemas for projection endpoints.
  """

  require OpenApiSpex

  alias OpenApiSpex.Schema

  defmodule Projection do
    @moduledoc "Projection entity"
    OpenApiSpex.schema(%{
      title: "Projection",
      description: "A projection that materializes events into a view",
      type: :object,
      properties: %{
        name: %Schema{type: :string, description: "Unique projection name"},
        version: %Schema{type: :integer, description: "Projection version"},
        initial_state: %Schema{
          type: :object,
          additionalProperties: true,
          description: "Initial state when projection is created"
        },
        definition: %Schema{type: :string, description: "Projection definition/handler code"},
        created_at: %Schema{type: :string, format: :"date-time"},
        updated_at: %Schema{type: :string, format: :"date-time"}
      },
      required: [:name, :version],
      example: %{
        name: "order_totals",
        version: 1,
        initial_state: %{total: 0, count: 0},
        definition: "fn event, state -> ... end"
      }
    })
  end

  defmodule CreateProjectionRequest do
    @moduledoc "Request to create a projection"
    OpenApiSpex.schema(%{
      title: "CreateProjectionRequest",
      description: "Request body for creating a new projection",
      type: :object,
      properties: %{
        name: %Schema{
          type: :string,
          description: "Unique projection name",
          minLength: 1,
          maxLength: 255,
          pattern: "^[a-z][a-z0-9_]*$"
        },
        version: %Schema{type: :integer, minimum: 1, default: 1},
        initial_state: %Schema{
          type: :object,
          additionalProperties: true,
          description: "Initial projection state"
        },
        definition: %Schema{type: :string, description: "Projection handler definition"}
      },
      required: [:name],
      example: %{
        name: "order_totals",
        version: 1,
        initial_state: %{total: 0, count: 0},
        definition: "fn event, state -> Map.update(state, :count, 1, &(&1 + 1)) end"
      }
    })
  end

  defmodule ProjectionResponse do
    @moduledoc "Single projection response"
    OpenApiSpex.schema(%{
      title: "ProjectionResponse",
      description: "Response containing a single projection with HAL links",
      type: :object,
      properties: %{
        data: Projection,
        _links: QueryServiceExWeb.Schemas.Common.HALLinks
      },
      required: [:data]
    })
  end

  defmodule ProjectionListResponse do
    @moduledoc "List of projections response"
    OpenApiSpex.schema(%{
      title: "ProjectionListResponse",
      description: "Response containing a list of projections with HAL links",
      type: :object,
      properties: %{
        data: %Schema{type: :array, items: Projection},
        count: %Schema{type: :integer},
        _links: QueryServiceExWeb.Schemas.Common.HALLinks
      },
      required: [:data, :count],
      example: %{
        data: [
          %{name: "order_totals", version: 1},
          %{name: "user_activity", version: 2}
        ],
        count: 2
      }
    })
  end

  defmodule ProjectionState do
    @moduledoc "Projection state"
    OpenApiSpex.schema(%{
      title: "ProjectionState",
      description: "Current state of a projection",
      type: :object,
      properties: %{
        name: %Schema{type: :string},
        version: %Schema{type: :integer},
        state: %Schema{
          type: :object,
          additionalProperties: true,
          description: "Current projection state"
        },
        last_event_id: %Schema{type: :string, description: "Last processed event ID"},
        last_updated_at: %Schema{type: :string, format: :"date-time"}
      },
      required: [:name, :state]
    })
  end

  defmodule ProjectionStateResponse do
    @moduledoc "Projection state response"
    OpenApiSpex.schema(%{
      title: "ProjectionStateResponse",
      description: "Response containing projection state with HAL links",
      type: :object,
      properties: %{
        data: ProjectionState,
        _links: QueryServiceExWeb.Schemas.Common.HALLinks
      },
      required: [:data]
    })
  end
end
