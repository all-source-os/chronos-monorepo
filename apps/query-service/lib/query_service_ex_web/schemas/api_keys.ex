defmodule QueryServiceExWeb.Schemas.ApiKeys do
  @moduledoc """
  OpenAPI schemas for API key endpoints.
  """

  require OpenApiSpex

  alias OpenApiSpex.Schema

  defmodule ApiKey do
    @moduledoc "API key entity"
    OpenApiSpex.schema(%{
      title: "ApiKey",
      description: "API key for programmatic access",
      type: :object,
      properties: %{
        id: %Schema{type: :string, format: :uuid},
        name: %Schema{type: :string, description: "Human-readable name"},
        description: %Schema{type: :string, nullable: true},
        key_prefix: %Schema{type: :string, description: "First 8 characters of the key"},
        scopes: %Schema{
          type: :array,
          items: %Schema{type: :string},
          description: "Granted permission scopes"
        },
        last_used_at: %Schema{type: :string, format: :"date-time", nullable: true},
        expires_at: %Schema{type: :string, format: :"date-time", nullable: true},
        created_at: %Schema{type: :string, format: :"date-time"},
        updated_at: %Schema{type: :string, format: :"date-time"}
      },
      required: [:id, :name, :key_prefix, :scopes, :created_at],
      example: %{
        id: "123e4567-e89b-12d3-a456-426614174000",
        name: "Production API Key",
        description: "Key for production backend services",
        key_prefix: "qs_live_",
        scopes: ["events:read", "events:write", "queries:execute"],
        last_used_at: "2026-02-10T11:30:00Z",
        expires_at: nil,
        created_at: "2026-01-01T00:00:00Z",
        updated_at: "2026-02-10T11:30:00Z"
      }
    })
  end

  defmodule ApiKeyWithSecret do
    @moduledoc "API key with the secret key value (only returned on creation)"
    OpenApiSpex.schema(%{
      title: "ApiKeyWithSecret",
      description: "API key including the secret key (only shown once)",
      allOf: [
        ApiKey,
        %Schema{
          type: :object,
          properties: %{
            key: %Schema{
              type: :string,
              description: "The full API key - save immediately, shown only once"
            }
          },
          required: [:key]
        }
      ],
      example: %{
        id: "123e4567-e89b-12d3-a456-426614174000",
        name: "Production API Key",
        key_prefix: "qs_live_",
        key: "qs_live_abc123def456ghi789jkl012mno345pqr678stu901vwx234",
        scopes: ["events:read", "events:write"],
        created_at: "2026-02-10T12:00:00Z"
      }
    })
  end

  defmodule CreateApiKeyRequest do
    @moduledoc "Request to create an API key"
    OpenApiSpex.schema(%{
      title: "CreateApiKeyRequest",
      description: "Request body for creating an API key",
      type: :object,
      properties: %{
        name: %Schema{
          type: :string,
          description: "Human-readable name for the key",
          minLength: 1,
          maxLength: 255
        },
        description: %Schema{
          type: :string,
          maxLength: 1000,
          nullable: true
        },
        scopes: %Schema{
          type: :array,
          items: %Schema{
            type: :string,
            enum: [
              "events:read",
              "events:write",
              "queries:execute",
              "projections:read",
              "projections:write",
              "schemas:read",
              "schemas:write"
            ]
          },
          description: "Permission scopes to grant"
        },
        expires_at: %Schema{
          type: :string,
          format: :"date-time",
          nullable: true,
          description: "Optional expiration date"
        }
      },
      required: [:name, :scopes],
      example: %{
        name: "Production API Key",
        description: "Key for production backend services",
        scopes: ["events:read", "events:write", "queries:execute"],
        expires_at: nil
      }
    })
  end

  defmodule UpdateApiKeyRequest do
    @moduledoc "Request to update an API key"
    OpenApiSpex.schema(%{
      title: "UpdateApiKeyRequest",
      description: "Request body for updating an API key",
      type: :object,
      properties: %{
        name: %Schema{type: :string, minLength: 1, maxLength: 255},
        description: %Schema{type: :string, maxLength: 1000, nullable: true},
        scopes: %Schema{
          type: :array,
          items: %Schema{type: :string}
        }
      },
      example: %{
        name: "Updated Key Name",
        scopes: ["events:read"]
      }
    })
  end

  defmodule ApiKeyResponse do
    @moduledoc "Single API key response"
    OpenApiSpex.schema(%{
      title: "ApiKeyResponse",
      description: "Response containing a single API key",
      type: :object,
      properties: %{
        data: ApiKey
      },
      required: [:data]
    })
  end

  defmodule ApiKeyListResponse do
    @moduledoc "List of API keys response"
    OpenApiSpex.schema(%{
      title: "ApiKeyListResponse",
      description: "Response containing a list of API keys",
      type: :object,
      properties: %{
        data: %Schema{type: :array, items: ApiKey}
      },
      required: [:data]
    })
  end

  defmodule ApiKeyCreatedResponse do
    @moduledoc "API key creation response"
    OpenApiSpex.schema(%{
      title: "ApiKeyCreatedResponse",
      description: "Response after creating an API key, includes secret",
      type: :object,
      properties: %{
        data: ApiKeyWithSecret,
        warning: %Schema{
          type: :string,
          description: "Warning about storing the key"
        }
      },
      required: [:data, :warning],
      example: %{
        data: %{
          id: "123e4567-e89b-12d3-a456-426614174000",
          name: "Production API Key",
          key_prefix: "qs_live_",
          key: "qs_live_abc123def456ghi789jkl012mno345pqr678stu901vwx234",
          scopes: ["events:read", "events:write"]
        },
        warning: "Save this key immediately. You won't be able to see it again."
      }
    })
  end

  defmodule Scope do
    @moduledoc "API key scope"
    OpenApiSpex.schema(%{
      title: "Scope",
      description: "API key permission scope",
      type: :object,
      properties: %{
        name: %Schema{type: :string},
        description: %Schema{type: :string}
      },
      required: [:name, :description],
      example: %{
        name: "events:read",
        description: "Read event data"
      }
    })
  end

  defmodule ScopesResponse do
    @moduledoc "List of available scopes"
    OpenApiSpex.schema(%{
      title: "ScopesResponse",
      description: "Response containing available API key scopes",
      type: :object,
      properties: %{
        data: %Schema{type: :array, items: Scope}
      },
      required: [:data],
      example: %{
        data: [
          %{name: "events:read", description: "Read event data"},
          %{name: "events:write", description: "Write/ingest events"},
          %{name: "queries:execute", description: "Execute queries"}
        ]
      }
    })
  end

  defmodule RevokeResponse do
    @moduledoc "API key revocation response"
    OpenApiSpex.schema(%{
      title: "RevokeResponse",
      description: "Response after revoking an API key",
      type: :object,
      properties: %{
        data: %Schema{
          type: :object,
          properties: %{
            message: %Schema{type: :string}
          },
          required: [:message]
        }
      },
      required: [:data],
      example: %{
        data: %{message: "API key revoked successfully"}
      }
    })
  end
end
