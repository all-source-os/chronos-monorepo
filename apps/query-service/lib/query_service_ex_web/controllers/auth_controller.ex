defmodule QueryServiceExWeb.AuthController do
  @moduledoc """
  Controller for authentication endpoints.

  Provides JWT-based authentication info. OAuth has been removed —
  auth responsibility is handled externally. This controller provides:
  - `/me` — returns user info from JWT claims (no DB lookup)
  - `/logout` — stateless logout (client discards token)
  - `/dev-token` — development token for local testing
  """

  use Phoenix.Controller, formats: [:json]
  use OpenApiSpex.ControllerSpecs

  alias QueryServiceEx.DevMode
  alias QueryServiceExWeb.Schemas.Auth

  require Logger

  action_fallback(QueryServiceExWeb.FallbackController)

  tags(["Authentication"])

  operation(:me,
    summary: "Get current user",
    description:
      "Returns the current authenticated user's information from JWT claims. No DB lookup.",
    security: [%{"bearer_auth" => []}],
    responses: [
      ok: {"Current user info", "application/json", Auth.MeResponse},
      unauthorized: {"Not authenticated", "application/json", Auth.AuthErrorResponse}
    ]
  )

  @doc """
  Returns the current authenticated user's information from JWT claims.

  GET /api/auth/me
  """
  def me(conn, _params) do
    user = conn.assigns[:current_user]

    conn
    |> put_status(:ok)
    |> json(%{
      data: %{
        user: serialize_user(user)
      }
    })
  end

  operation(:logout,
    summary: "Logout",
    description: "Logs out the current user. Stateless — client should discard the JWT.",
    security: [%{"bearer_auth" => []}],
    responses: [
      ok: {"Logged out", "application/json", Auth.LogoutResponse}
    ]
  )

  @doc """
  Logs out the current user.

  POST /api/auth/logout

  With shared-secret JWTs, tokens are stateless and cannot be revoked server-side.
  The client should discard the token.
  """
  def logout(conn, _params) do
    correlation_id = conn.assigns[:correlation_id] || "unknown"

    Logger.info("[AuthController] User logged out",
      correlation_id: correlation_id
    )

    conn
    |> put_status(:ok)
    |> json(%{data: %{message: "Successfully logged out"}})
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
      ok: {"Development token", "application/json", Auth.DevTokenResponse},
      forbidden: {"Dev mode not enabled", "application/json", Auth.AuthErrorResponse}
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

      case encode_and_sign(dev_user, ttl_seconds: 86_400) do
        {:ok, token} ->
          Logger.info("[AuthController] Dev token issued",
            correlation_id: conn.assigns[:correlation_id] || "unknown"
          )

          dev_tenant = DevMode.dev_tenant()

          conn
          |> put_status(:ok)
          |> json(%{
            data: %{
              token: token,
              user: serialize_user(dev_user),
              tenant: serialize_tenant(dev_tenant),
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

  @default_ttl_seconds 3600

  defp encode_and_sign(user, opts) do
    case System.get_env("JWT_SECRET") do
      nil ->
        {:error, "JWT_SECRET not configured"}

      secret ->
        ttl = Keyword.get(opts, :ttl_seconds, @default_ttl_seconds)
        now = System.system_time(:second)

        claims = %{
          "sub" => to_string(user.id),
          "tenant_id" => user.tenant_id,
          "role" => "user",
          "iat" => now,
          "exp" => now + ttl
        }

        jwk = JOSE.JWK.from_oct(secret)
        jws = %{"alg" => "HS256"}
        jwt = JOSE.JWT.from_map(claims)

        {_alg, token} = JOSE.JWT.sign(jwk, jws, jwt) |> JOSE.JWS.compact()
        {:ok, token}
    end
  end

  defp serialize_user(user) do
    %{
      id: user[:id] || user["id"],
      email: user[:email] || user["email"],
      name: user[:name] || user["name"],
      tenant_id: user[:tenant_id] || user["tenant_id"]
    }
  end

  defp serialize_tenant(tenant) do
    metadata = tenant["metadata"] || %{}
    subscription = metadata["subscription"] || %{}

    %{
      id: tenant["id"] || tenant[:id],
      name: tenant["name"] || tenant[:name],
      slug: tenant["slug"] || tenant[:slug],
      subscription_status: subscription["status"],
      subscription_tier: subscription["tier"]
    }
  end
end
