defmodule QueryServiceExWeb.AuthController do
  @moduledoc """
  Controller for OAuth authentication flows.

  Handles OAuth callbacks from Google and GitHub, issues JWT tokens
  for authenticated users, and auto-creates tenant workspaces.
  """

  use Phoenix.Controller, formats: [:json]

  plug(Ueberauth)

  alias QueryServiceEx.Accounts
  alias QueryServiceEx.Accounts.Guardian

  require Logger

  action_fallback(QueryServiceExWeb.FallbackController)

  @doc """
  Initiates the OAuth flow by redirecting to the provider.

  GET /api/auth/:provider (google, github)
  """
  def request(conn, _params) do
    # Ueberauth plug handles the redirect
    conn
  end

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
