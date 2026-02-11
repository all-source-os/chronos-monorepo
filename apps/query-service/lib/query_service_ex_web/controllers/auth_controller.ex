defmodule QueryServiceExWeb.AuthController do
  @moduledoc """
  Controller for OAuth authentication flows.

  Handles OAuth callbacks from Google and GitHub, issues JWT tokens
  for authenticated users, and auto-creates tenant workspaces.

  Also provides a dev-token endpoint for local development when
  AUTH_DISABLED is set.
  """

  use Phoenix.Controller, formats: [:json]
  use OpenApiSpex.ControllerSpecs

  plug(Ueberauth, [] when action not in [:dev_token])

  alias QueryServiceEx.Accounts
  alias QueryServiceEx.Accounts.Guardian
  alias QueryServiceEx.DevMode
  alias QueryServiceExWeb.Schemas.Auth

  require Logger

  action_fallback(QueryServiceExWeb.FallbackController)

  tags(["Authentication"])

  operation(:request,
    summary: "Initiate OAuth flow",
    description: "Redirects to the OAuth provider for authentication.",
    parameters: [
      provider: [
        in: :path,
        schema: %OpenApiSpex.Schema{type: :string, enum: ["google", "github"]},
        required: true,
        description: "OAuth provider (google or github)"
      ]
    ],
    responses: [
      found: {"Redirect to OAuth provider", "text/html", nil}
    ]
  )

  @doc """
  Initiates the OAuth flow by redirecting to the provider.

  GET /api/auth/:provider (google, github)
  """
  def request(conn, _params) do
    # Ueberauth plug handles the redirect
    conn
  end

  operation(:callback,
    summary: "OAuth callback",
    description: "Handles OAuth callback from providers and returns JWT token.",
    parameters: [
      provider: [
        in: :path,
        schema: %OpenApiSpex.Schema{type: :string, enum: ["google", "github"]},
        required: true,
        description: "OAuth provider (google or github)"
      ]
    ],
    responses: [
      ok: {"Authentication successful", "application/json", Auth.AuthCallbackResponse},
      unauthorized: {"Authentication failed", "application/json", Auth.OAuthFailureResponse},
      unprocessable_entity:
        {"User creation failed", "application/json", Auth.OAuthFailureResponse}
    ]
  )

  @doc """
  Handles OAuth callback from providers.

  Handles:
  - GET /api/auth/google/callback
  - GET /api/auth/github/callback
  """
  def callback(conn, params)

  def callback(%{assigns: %{ueberauth_failure: failure}} = conn, %{"provider" => provider}) do
    correlation_id = conn.assigns[:correlation_id] || "unknown"

    Logger.warning("[AuthController] #{provider} OAuth failure: #{inspect(failure)}",
      correlation_id: correlation_id,
      provider: provider
    )

    conn
    |> put_status(:unauthorized)
    |> json(%{
      error: %{
        code: "oauth_failed",
        message: "Authentication failed",
        provider: provider
      }
    })
  end

  def callback(%{assigns: %{ueberauth_auth: auth}} = conn, %{"provider" => "google"}) do
    handle_oauth_callback(conn, auth, :google, &Accounts.find_or_create_from_google/1)
  end

  def callback(%{assigns: %{ueberauth_auth: auth}} = conn, %{"provider" => "github"}) do
    handle_oauth_callback(conn, auth, :github, &Accounts.find_or_create_from_github/1)
  end

  operation(:me,
    summary: "Get current user",
    description: "Returns the current authenticated user's information including tenant.",
    security: [%{"bearer_auth" => []}],
    responses: [
      ok: {"Current user info", "application/json", Auth.MeResponse},
      unauthorized: {"Not authenticated", "application/json", Auth.OAuthFailureResponse}
    ]
  )

  @doc """
  Returns the current authenticated user's information including tenant.

  GET /api/auth/me
  """
  def me(conn, _params) do
    user = Guardian.Plug.current_resource(conn)

    conn
    |> put_status(:ok)
    |> json(%{
      data: %{
        user: serialize_user(user),
        tenant: serialize_tenant(user.tenant)
      }
    })
  end

  operation(:logout,
    summary: "Logout",
    description: "Logs out the current user by revoking the JWT token.",
    security: [%{"bearer_auth" => []}],
    responses: [
      ok: {"Logged out", "application/json", Auth.LogoutResponse}
    ]
  )

  @doc """
  Logs out the current user by revoking the token.

  POST /api/auth/logout
  """
  def logout(conn, _params) do
    correlation_id = conn.assigns[:correlation_id] || "unknown"
    token = Guardian.Plug.current_token(conn)

    case Guardian.revoke(token) do
      {:ok, _claims} ->
        Logger.info("[AuthController] User logged out",
          correlation_id: correlation_id
        )

        conn
        |> put_status(:ok)
        |> json(%{data: %{message: "Successfully logged out"}})

      {:error, reason} ->
        Logger.warning("[AuthController] Logout failed: #{inspect(reason)}",
          correlation_id: correlation_id
        )

        # Still return success - token might already be invalid
        conn
        |> put_status(:ok)
        |> json(%{data: %{message: "Successfully logged out"}})
    end
  end

  # -------------------------------------------------------------------
  # Dev Token (for local development)
  # -------------------------------------------------------------------

  operation(:dev_token,
    summary: "Get development token",
    description: """
    Returns a JWT token for local development use.

    **Only available when AUTH_DISABLED=true is set.**

    This endpoint allows tools and scripts to obtain a valid JWT token
    without going through OAuth, useful for MCP integrations and CLI tools.
    """,
    responses: [
      ok: {"Development token", "application/json", Auth.AuthCallbackResponse},
      forbidden: {"Dev mode not enabled", "application/json", Auth.OAuthFailureResponse}
    ]
  )

  @doc """
  Issues a development JWT token for local testing.

  GET /api/auth/dev-token

  Only available when AUTH_DISABLED=true is set in the environment.
  Returns a token that can be used to authenticate API requests.
  """
  def dev_token(conn, _params) do
    if DevMode.auth_disabled?() do
      dev_user = DevMode.dev_user()

      case Guardian.encode_and_sign(dev_user, %{}, ttl: {24, :hours}) do
        {:ok, token, _claims} ->
          Logger.info("[AuthController] Dev token issued",
            correlation_id: conn.assigns[:correlation_id] || "unknown"
          )

          conn
          |> put_status(:ok)
          |> json(%{
            data: %{
              token: token,
              user: serialize_user(dev_user),
              tenant: serialize_tenant(dev_user.tenant),
              warning: "This is a development token. Do not use in production."
            }
          })

        {:error, reason} ->
          Logger.error("[AuthController] Failed to generate dev token: #{inspect(reason)}")

          conn
          |> put_status(:internal_server_error)
          |> json(%{
            error: %{
              code: "token_generation_failed",
              message: "Failed to generate development token"
            }
          })
      end
    else
      conn
      |> put_status(:forbidden)
      |> json(%{
        error: %{
          code: "dev_mode_disabled",
          message: "Development token endpoint is only available when AUTH_DISABLED=true"
        }
      })
    end
  end

  # -------------------------------------------------------------------
  # Private Helpers
  # -------------------------------------------------------------------

  defp handle_oauth_callback(conn, auth, provider, find_or_create_fn) do
    correlation_id = conn.assigns[:correlation_id] || "unknown"
    email = auth.info.email || "unknown"

    Logger.info("[AuthController] #{provider} OAuth callback for: #{email}",
      correlation_id: correlation_id,
      provider: provider
    )

    with {:ok, user} <- find_or_create_fn.(auth),
         {:ok, token, _claims} <- Guardian.encode_and_sign(user) do
      Logger.info("[AuthController] User authenticated: #{user.id}",
        correlation_id: correlation_id,
        user_id: user.id,
        tenant_id: user.tenant_id,
        provider: provider
      )

      conn
      |> put_status(:ok)
      |> json(%{
        data: %{
          token: token,
          user: serialize_user(user),
          tenant: serialize_tenant(user.tenant)
        }
      })
    else
      {:error, changeset} ->
        Logger.error("[AuthController] Failed to create user: #{inspect(changeset)}",
          correlation_id: correlation_id,
          provider: provider
        )

        conn
        |> put_status(:unprocessable_entity)
        |> json(%{
          error: %{
            code: "user_creation_failed",
            message: "Failed to create user account"
          }
        })
    end
  end

  defp serialize_user(user) do
    %{
      id: user.id,
      email: user.email,
      name: user.name,
      avatar_url: user.avatar_url,
      provider: user.provider,
      tenant_id: user.tenant_id
    }
  end

  defp serialize_tenant(nil), do: nil

  defp serialize_tenant(tenant) do
    %{
      id: tenant.id,
      name: tenant.name,
      slug: tenant.slug,
      subscription_status: tenant.subscription_status,
      subscription_tier: tenant.subscription_tier,
      trial_ends_at: tenant.trial_ends_at
    }
  end
end
