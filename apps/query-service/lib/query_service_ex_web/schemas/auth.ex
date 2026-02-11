defmodule QueryServiceExWeb.Schemas.Auth do
  @moduledoc """
  OpenAPI schemas for authentication endpoints.
  """

  require OpenApiSpex

  alias OpenApiSpex.Schema

  defmodule User do
    @moduledoc "User entity"
    OpenApiSpex.schema(%{
      title: "User",
      description: "Authenticated user",
      type: :object,
      properties: %{
        id: %Schema{type: :string, format: :uuid},
        email: %Schema{type: :string, format: :email},
        name: %Schema{type: :string},
        avatar_url: %Schema{type: :string, format: :uri, nullable: true},
        provider: %Schema{type: :string, enum: ["google", "github"]},
        tenant_id: %Schema{type: :string, format: :uuid}
      },
      required: [:id, :email, :provider, :tenant_id],
      example: %{
        id: "123e4567-e89b-12d3-a456-426614174000",
        email: "user@example.com",
        name: "John Doe",
        avatar_url: "https://lh3.googleusercontent.com/...",
        provider: "google",
        tenant_id: "987fcdeb-51a2-43e1-b2c4-123456789000"
      }
    })
  end

  defmodule Tenant do
    @moduledoc "Tenant entity (summary)"
    OpenApiSpex.schema(%{
      title: "Tenant",
      description: "Tenant workspace summary",
      type: :object,
      properties: %{
        id: %Schema{type: :string, format: :uuid},
        name: %Schema{type: :string},
        slug: %Schema{type: :string},
        subscription_status: %Schema{
          type: :string,
          enum: ["trialing", "active", "past_due", "cancelled", "expired"]
        },
        subscription_tier: %Schema{
          type: :string,
          enum: ["free", "starter", "growth", "enterprise"]
        },
        trial_ends_at: %Schema{type: :string, format: :"date-time", nullable: true}
      },
      required: [:id, :name, :slug],
      example: %{
        id: "987fcdeb-51a2-43e1-b2c4-123456789000",
        name: "Acme Corp",
        slug: "acme-corp",
        subscription_status: "active",
        subscription_tier: "growth",
        trial_ends_at: nil
      }
    })
  end

  defmodule AuthCallbackResponse do
    @moduledoc "OAuth callback success response"
    OpenApiSpex.schema(%{
      title: "AuthCallbackResponse",
      description: "Response after successful OAuth authentication",
      type: :object,
      properties: %{
        data: %Schema{
          type: :object,
          properties: %{
            token: %Schema{type: :string, description: "JWT access token"},
            user: User,
            tenant: Tenant
          },
          required: [:token, :user, :tenant]
        }
      },
      required: [:data],
      example: %{
        data: %{
          token: "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...",
          user: %{
            id: "123e4567-e89b-12d3-a456-426614174000",
            email: "user@example.com",
            name: "John Doe",
            provider: "google"
          },
          tenant: %{
            id: "987fcdeb-51a2-43e1-b2c4-123456789000",
            name: "Acme Corp",
            slug: "acme-corp"
          }
        }
      }
    })
  end

  defmodule MeResponse do
    @moduledoc "Current user response"
    OpenApiSpex.schema(%{
      title: "MeResponse",
      description: "Response containing current authenticated user",
      type: :object,
      properties: %{
        data: %Schema{
          type: :object,
          properties: %{
            user: User,
            tenant: Tenant
          },
          required: [:user]
        }
      },
      required: [:data]
    })
  end

  defmodule LogoutResponse do
    @moduledoc "Logout response"
    OpenApiSpex.schema(%{
      title: "LogoutResponse",
      description: "Response after successful logout",
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
        data: %{message: "Successfully logged out"}
      }
    })
  end

  defmodule OAuthFailureResponse do
    @moduledoc "OAuth failure response"
    OpenApiSpex.schema(%{
      title: "OAuthFailureResponse",
      description: "Response when OAuth authentication fails",
      type: :object,
      properties: %{
        error: %Schema{
          type: :object,
          properties: %{
            code: %Schema{type: :string},
            message: %Schema{type: :string},
            provider: %Schema{type: :string}
          },
          required: [:code, :message, :provider]
        }
      },
      required: [:error],
      example: %{
        error: %{
          code: "oauth_failed",
          message: "Authentication failed",
          provider: "google"
        }
      }
    })
  end
end
