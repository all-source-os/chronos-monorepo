defmodule QueryServiceExWeb.Schemas.Metrics do
  @moduledoc """
  OpenAPI schemas for metrics endpoints.
  """

  require OpenApiSpex

  alias OpenApiSpex.Schema

  defmodule MetricsResponse do
    @moduledoc "JSON metrics response"
    OpenApiSpex.schema(%{
      title: "MetricsResponse",
      description: "System metrics in JSON format",
      type: :object,
      properties: %{
        service: %Schema{type: :string, description: "Service name"},
        timestamp: %Schema{type: :string, format: :"date-time"},
        elixir: %Schema{
          type: :object,
          description: "Elixir runtime metrics",
          properties: %{
            processes: %Schema{type: :integer, description: "Number of Erlang processes"},
            memory: %Schema{
              type: :object,
              properties: %{
                total_mb: %Schema{type: :number, format: :float},
                processes_mb: %Schema{type: :number, format: :float},
                atom_mb: %Schema{type: :number, format: :float},
                binary_mb: %Schema{type: :number, format: :float},
                ets_mb: %Schema{type: :number, format: :float}
              }
            },
            uptime_seconds: %Schema{type: :integer},
            schedulers: %Schema{type: :integer}
          }
        },
        backend: %Schema{
          type: :object,
          description: "Backend metrics or error status",
          additionalProperties: true
        }
      },
      required: [:service, :timestamp, :elixir, :backend],
      example: %{
        service: "query_service_ex",
        timestamp: "2026-02-10T12:00:00Z",
        elixir: %{
          processes: 256,
          memory: %{
            total_mb: 128.5,
            processes_mb: 64.2,
            atom_mb: 1.2,
            binary_mb: 32.1,
            ets_mb: 4.5
          },
          uptime_seconds: 3600,
          schedulers: 8
        },
        backend: %{
          events_total: 1_000_000,
          ingestion_rate: 726_000
        }
      }
    })
  end
end
