defmodule QueryServiceExWeb.Schemas.Health do
  @moduledoc """
  OpenAPI schemas for health check endpoints.
  """

  require OpenApiSpex

  alias OpenApiSpex.Schema

  defmodule LiveResponse do
    @moduledoc "Liveness probe response"
    OpenApiSpex.schema(%{
      title: "LiveResponse",
      description: "Liveness probe response indicating the service is running",
      type: :object,
      properties: %{
        status: %Schema{
          type: :string,
          enum: ["alive"],
          description: "Liveness status"
        },
        timestamp: %Schema{type: :string, format: :"date-time"}
      },
      required: [:status, :timestamp],
      example: %{
        status: "alive",
        timestamp: "2026-02-10T12:00:00Z"
      }
    })
  end

  defmodule ReadyResponse do
    @moduledoc "Readiness probe response"
    OpenApiSpex.schema(%{
      title: "ReadyResponse",
      description: "Readiness probe response with dependency status",
      type: :object,
      properties: %{
        status: %Schema{
          type: :string,
          enum: ["ready", "not_ready"],
          description: "Readiness status"
        },
        checks: %Schema{
          type: :object,
          properties: %{
            database: %Schema{
              type: :string,
              enum: ["healthy", "unhealthy", "degraded", "timeout"]
            },
            backend: %Schema{
              type: :string,
              enum: ["healthy", "unhealthy", "degraded", "timeout"]
            },
            websocket: %Schema{
              type: :string,
              enum: ["healthy", "unhealthy", "degraded", "timeout"]
            }
          }
        },
        timestamp: %Schema{type: :string, format: :"date-time"}
      },
      required: [:status, :checks, :timestamp],
      example: %{
        status: "ready",
        checks: %{
          database: "healthy",
          backend: "healthy",
          websocket: "healthy"
        },
        timestamp: "2026-02-10T12:00:00Z"
      }
    })
  end

  defmodule HealthResponse do
    @moduledoc "Combined health check response"
    OpenApiSpex.schema(%{
      title: "HealthResponse",
      description: "Combined health check response with all dependencies",
      type: :object,
      properties: %{
        service: %Schema{type: :string, description: "Service name"},
        status: %Schema{
          type: :string,
          enum: ["healthy", "unhealthy", "degraded"],
          description: "Overall health status"
        },
        checks: %Schema{
          type: :object,
          properties: %{
            database: %Schema{type: :string},
            backend: %Schema{type: :string},
            websocket: %Schema{type: :string}
          }
        },
        timestamp: %Schema{type: :string, format: :"date-time"},
        version: %Schema{type: :string, description: "Service version"}
      },
      required: [:service, :status, :checks, :timestamp, :version],
      example: %{
        service: "query_service_ex",
        status: "healthy",
        checks: %{
          database: "healthy",
          backend: "healthy",
          websocket: "healthy"
        },
        timestamp: "2026-02-10T12:00:00Z",
        version: "0.1.0"
      }
    })
  end
end
