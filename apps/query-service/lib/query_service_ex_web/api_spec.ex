defmodule QueryServiceExWeb.ApiSpec do
  @moduledoc """
  OpenAPI specification for the Query Service API.

  This module defines the OpenAPI 3.0 specification including:
  - API metadata (title, version, description)
  - Server information
  - Security schemes (Bearer JWT)
  - Tags for grouping operations

  ## Usage

  The specification is available at:
  - GET /api/openapi - Raw JSON specification
  - GET /api/docs - Interactive Swagger UI

  ## Security

  Most endpoints require authentication via Bearer JWT token:

      Authorization: Bearer <your_jwt_token>

  Public endpoints (health, metrics, OAuth callbacks) do not require authentication.
  """

  alias OpenApiSpex.{Components, Info, OpenApi, Paths, SecurityScheme, Server, Tag}

  @behaviour OpenApi

  @impl OpenApi
  def spec do
    %OpenApi{
      info: %Info{
        title: "Query Service API",
        version: Application.spec(:query_service_ex, :vsn) |> to_string(),
        description: """
        The Query Service provides a high-performance API for event sourcing and CQRS.

        ## Features

        - **Event Storage**: Store and retrieve domain events
        - **Query DSL**: Powerful query language for filtering events
        - **Projections**: Create and manage materialized views
        - **Real-time Updates**: WebSocket subscriptions for live data
        - **Multi-tenancy**: Isolated workspaces with usage-based billing

        ## Authentication

        Most endpoints require a valid JWT token obtained via OAuth (Google or GitHub).
        Include the token in the `Authorization` header:

        ```
        Authorization: Bearer <your_jwt_token>
        ```

        ## Rate Limiting

        API requests are rate-limited per tenant. Default limits:
        - 1000 requests/minute for read operations
        - 100 requests/minute for write operations

        ## Support

        For API issues, contact support@allsource.dev or visit our documentation.
        """,
        contact: %OpenApiSpex.Contact{
          name: "AllSource Support",
          email: "support@allsource.dev"
        },
        license: %OpenApiSpex.License{
          name: "MIT"
        }
      },
      servers: servers(),
      paths: Paths.from_router(QueryServiceExWeb.Router),
      components: %Components{
        securitySchemes: %{
          "bearer_auth" => %SecurityScheme{
            type: "http",
            scheme: "bearer",
            bearerFormat: "JWT",
            description: "JWT token obtained from OAuth callback"
          },
          "api_key" => %SecurityScheme{
            type: "apiKey",
            name: "X-API-Key",
            in: "header",
            description: "API key for programmatic access"
          }
        }
      },
      tags: [
        %Tag{
          name: "Health",
          description: "Health check and liveness probes"
        },
        %Tag{
          name: "Metrics",
          description: "System metrics and observability"
        },
        %Tag{
          name: "Authentication",
          description: "OAuth authentication and user management"
        },
        %Tag{
          name: "Events",
          description: "Event storage and retrieval"
        },
        %Tag{
          name: "Query",
          description: "Query DSL execution"
        },
        %Tag{
          name: "Projections",
          description: "Projection management and state queries"
        },
        %Tag{
          name: "Schemas",
          description: "Event schema registration and validation"
        },
        %Tag{
          name: "Tenant",
          description: "Tenant workspace management"
        },
        %Tag{
          name: "Onboarding",
          description: "Customer onboarding flow"
        },
        %Tag{
          name: "API Keys",
          description: "API key self-service management"
        },
        %Tag{
          name: "Billing",
          description: "Subscription and billing management"
        },
        %Tag{
          name: "Webhooks",
          description: "External service webhooks"
        }
      ]
    }
    |> OpenApiSpex.resolve_schema_modules()
  end

  defp servers do
    env = Application.get_env(:query_service_ex, :environment, :dev)

    case env do
      :prod ->
        [
          %Server{url: "https://api.allsource.dev", description: "Production"},
          %Server{url: "https://staging-api.allsource.dev", description: "Staging"}
        ]

      _ ->
        [
          %Server{url: "http://localhost:3902", description: "Local development"}
        ]
    end
  end
end
