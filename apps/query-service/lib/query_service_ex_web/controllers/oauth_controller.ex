defmodule QueryServiceExWeb.OAuthController do
  @moduledoc """
  OAuth2 flow controller for Google and GitHub authentication.

  Handles the redirect-based OAuth flow:
  1. GET /api/auth/:provider — redirects user to provider's authorization page
  2. GET /api/auth/:provider/callback — exchanges code for token, creates/finds user, issues JWT

  On success, redirects to the frontend callback with a JWT token.
  On failure, redirects to the frontend login page with an error code.
  """

  use Phoenix.Controller, formats: [:json]

  require Logger

  @frontend_url System.get_env("FRONTEND_URL") || "http://localhost:3000"

  # -------------------------------------------------------------------
  # Provider Authorization Redirect
  # -------------------------------------------------------------------

  @doc """
  Redirects user to the OAuth provider's authorization page.

  GET /api/auth/google
  GET /api/auth/github
  """
  def authorize(conn, %{"provider" => provider}) do
    frontend_url = frontend_url()

    case build_authorization_url(provider) do
      {:ok, url} ->
        redirect(conn, external: url)

      {:error, :unconfigured} ->
        redirect(conn, external: "#{frontend_url}/login?error=auth_failed")

      {:error, _reason} ->
        redirect(conn, external: "#{frontend_url}/login?error=auth_failed")
    end
  end

  # -------------------------------------------------------------------
  # Provider Callback (code exchange)
  # -------------------------------------------------------------------

  @doc """
  Handles the OAuth callback from the provider.

  GET /api/auth/google/callback?code=...
  GET /api/auth/github/callback?code=...
  """
  def callback(conn, %{"provider" => provider, "code" => code}) do
    frontend_url = frontend_url()

    with {:ok, provider_token} <- exchange_code(provider, code),
         {:ok, user_info} <- fetch_user_info(provider, provider_token),
         {:ok, jwt, is_new_user} <- find_or_create_user_and_sign(provider, user_info) do
      callback_url =
        "#{frontend_url}/api/auth/callback?token=#{jwt}&new_user=#{is_new_user}"

      redirect(conn, external: callback_url)
    else
      {:error, reason} ->
        Logger.error("[OAuthController] OAuth callback failed",
          provider: provider,
          error: inspect(reason)
        )

        redirect(conn, external: "#{frontend_url}/login?error=auth_failed")
    end
  end

  def callback(conn, %{"provider" => _provider, "error" => error}) do
    frontend_url = frontend_url()

    Logger.warning("[OAuthController] OAuth provider returned error",
      error: error
    )

    error_code = if error == "access_denied", do: "access_denied", else: "auth_failed"
    redirect(conn, external: "#{frontend_url}/login?error=#{error_code}")
  end

  def callback(conn, %{"provider" => _provider}) do
    frontend_url = frontend_url()
    redirect(conn, external: "#{frontend_url}/login?error=auth_failed")
  end

  # -------------------------------------------------------------------
  # Private: Build Authorization URL
  # -------------------------------------------------------------------

  defp build_authorization_url("google") do
    client_id = oauth_config(:google_client_id)
    redirect_uri = oauth_callback_url("google")

    if client_id do
      params =
        URI.encode_query(%{
          client_id: client_id,
          redirect_uri: redirect_uri,
          response_type: "code",
          scope: "openid email profile",
          access_type: "offline",
          prompt: "consent"
        })

      {:ok, "https://accounts.google.com/o/oauth2/v2/auth?#{params}"}
    else
      {:error, :unconfigured}
    end
  end

  defp build_authorization_url("github") do
    client_id = oauth_config(:github_client_id)
    redirect_uri = oauth_callback_url("github")

    if client_id do
      params =
        URI.encode_query(%{
          client_id: client_id,
          redirect_uri: redirect_uri,
          scope: "user:email"
        })

      {:ok, "https://github.com/login/oauth/authorize?#{params}"}
    else
      {:error, :unconfigured}
    end
  end

  defp build_authorization_url(_provider), do: {:error, :unknown_provider}

  # -------------------------------------------------------------------
  # Private: Exchange Code for Token
  # -------------------------------------------------------------------

  defp exchange_code("google", code) do
    client_id = oauth_config(:google_client_id)
    client_secret = oauth_config(:google_client_secret)
    redirect_uri = oauth_callback_url("google")

    body =
      URI.encode_query(%{
        code: code,
        client_id: client_id,
        client_secret: client_secret,
        redirect_uri: redirect_uri,
        grant_type: "authorization_code"
      })

    case Tesla.post(
           Tesla.client([{Tesla.Middleware.JSON, []}]),
           "https://oauth2.googleapis.com/token",
           body,
           headers: [{"content-type", "application/x-www-form-urlencoded"}]
         ) do
      {:ok, %{status: 200, body: %{"access_token" => token}}} ->
        {:ok, token}

      {:ok, %{body: body}} ->
        {:error, {:token_exchange, body}}

      {:error, reason} ->
        {:error, {:token_exchange, reason}}
    end
  end

  defp exchange_code("github", code) do
    client_id = oauth_config(:github_client_id)
    client_secret = oauth_config(:github_client_secret)

    body =
      Jason.encode!(%{
        code: code,
        client_id: client_id,
        client_secret: client_secret
      })

    case Tesla.post(
           Tesla.client([{Tesla.Middleware.JSON, []}]),
           "https://github.com/login/oauth/access_token",
           body,
           headers: [
             {"content-type", "application/json"},
             {"accept", "application/json"}
           ]
         ) do
      {:ok, %{status: 200, body: %{"access_token" => token}}} ->
        {:ok, token}

      {:ok, %{body: body}} ->
        {:error, {:token_exchange, body}}

      {:error, reason} ->
        {:error, {:token_exchange, reason}}
    end
  end

  defp exchange_code(_provider, _code), do: {:error, :unknown_provider}

  # -------------------------------------------------------------------
  # Private: Fetch User Info
  # -------------------------------------------------------------------

  defp fetch_user_info("google", token) do
    case Tesla.get(
           Tesla.client([{Tesla.Middleware.JSON, []}]),
           "https://www.googleapis.com/oauth2/v2/userinfo",
           headers: [{"authorization", "Bearer #{token}"}]
         ) do
      {:ok, %{status: 200, body: body}} ->
        {:ok,
         %{
           provider: "google",
           provider_id: body["id"],
           email: body["email"],
           name: body["name"] || body["email"]
         }}

      {:ok, %{body: body}} ->
        {:error, {:user_info, body}}

      {:error, reason} ->
        {:error, {:user_info, reason}}
    end
  end

  defp fetch_user_info("github", token) do
    client = Tesla.client([{Tesla.Middleware.JSON, []}])
    headers = [{"authorization", "Bearer #{token}"}]

    with {:ok, %{status: 200, body: user_body}} <-
           Tesla.get(client, "https://api.github.com/user", headers: headers),
         {:ok, email} <- get_github_email(client, headers, user_body) do
      {:ok,
       %{
         provider: "github",
         provider_id: to_string(user_body["id"]),
         email: email,
         name: user_body["name"] || user_body["login"]
       }}
    else
      {:ok, %{body: body}} -> {:error, {:user_info, body}}
      {:error, reason} -> {:error, {:user_info, reason}}
    end
  end

  defp fetch_user_info(_provider, _token), do: {:error, :unknown_provider}

  defp get_github_email(_client, _headers, %{"email" => email}) when is_binary(email) do
    {:ok, email}
  end

  defp get_github_email(client, headers, _user_body) do
    # GitHub user endpoint may not return email, fetch from emails API
    case Tesla.get(client, "https://api.github.com/user/emails", headers: headers) do
      {:ok, %{status: 200, body: emails}} when is_list(emails) ->
        primary = Enum.find(emails, fn e -> e["primary"] && e["verified"] end)
        verified = Enum.find(emails, fn e -> e["verified"] end)
        email_entry = primary || verified || List.first(emails)

        if email_entry do
          {:ok, email_entry["email"]}
        else
          {:error, :no_email}
        end

      _ ->
        {:error, :no_email}
    end
  end

  # -------------------------------------------------------------------
  # Private: Find/Create User + Sign JWT
  # -------------------------------------------------------------------

  defp find_or_create_user_and_sign(provider, user_info) do
    # Call Control Plane to find or create OAuth user + tenant
    control_plane_url = control_plane_url()

    body = %{
      provider: provider,
      provider_id: user_info.provider_id,
      email: user_info.email,
      name: user_info.name
    }

    case Tesla.post(
           Tesla.client([{Tesla.Middleware.JSON, []}]),
           "#{control_plane_url}/api/v1/auth/oauth",
           Jason.encode!(body),
           headers: [{"content-type", "application/json"}]
         ) do
      {:ok, %{status: status, body: resp_body}} when status in [200, 201] ->
        token = resp_body["token"]
        is_new_user = resp_body["new_user"] || status == 201

        if token do
          {:ok, token, is_new_user}
        else
          # Control Plane returned user info but no token — sign locally
          sign_jwt(resp_body, is_new_user)
        end

      {:ok, %{body: body}} ->
        {:error, {:control_plane, body}}

      {:error, reason} ->
        # If Control Plane is not available, sign JWT locally with user info
        Logger.warning("[OAuthController] Control Plane unreachable, signing JWT locally",
          error: inspect(reason),
          provider: provider
        )

        sign_jwt_for_oauth_user(user_info)
    end
  end

  defp sign_jwt(resp_body, is_new_user) do
    user_id = resp_body["user_id"] || resp_body["id"]
    tenant_id = resp_body["tenant_id"]

    case encode_jwt(%{user_id: user_id, tenant_id: tenant_id}) do
      {:ok, token} -> {:ok, token, is_new_user}
      {:error, _} = err -> err
    end
  end

  defp sign_jwt_for_oauth_user(user_info) do
    # Generate a deterministic user ID from provider + provider_id
    user_id = "oauth:#{user_info.provider}:#{user_info.provider_id}"

    case encode_jwt(%{user_id: user_id, tenant_id: nil, email: user_info.email}) do
      {:ok, token} -> {:ok, token, true}
      {:error, _} = err -> err
    end
  end

  defp encode_jwt(claims_data) do
    case System.get_env("JWT_SECRET") do
      nil ->
        {:error, "JWT_SECRET not configured"}

      secret ->
        now = System.system_time(:second)

        claims = %{
          "sub" => to_string(claims_data.user_id),
          "tenant_id" => claims_data[:tenant_id],
          "email" => claims_data[:email],
          "role" => "user",
          "iat" => now,
          "exp" => now + 604_800
        }

        jwk = JOSE.JWK.from_oct(secret)
        jws = %{"alg" => "HS256"}
        jwt = JOSE.JWT.from_map(claims)

        {_alg, token} = JOSE.JWT.sign(jwk, jws, jwt) |> JOSE.JWS.compact()
        {:ok, token}
    end
  end

  # -------------------------------------------------------------------
  # Private: Configuration Helpers
  # -------------------------------------------------------------------

  defp oauth_config(key) do
    Application.get_env(:query_service_ex, :oauth, %{})[key] ||
      System.get_env(key |> Atom.to_string() |> String.upcase())
  end

  defp oauth_callback_url(provider) do
    base_url = System.get_env("OAUTH_CALLBACK_BASE_URL") || api_base_url()
    "#{base_url}/api/auth/#{provider}/callback"
  end

  defp api_base_url do
    host = System.get_env("PHX_HOST") || "localhost"
    port = System.get_env("PORT") || "3902"
    scheme = if System.get_env("PHX_HOST"), do: "https", else: "http"
    "#{scheme}://#{host}:#{port}"
  end

  defp frontend_url do
    System.get_env("FRONTEND_URL") || @frontend_url
  end

  defp control_plane_url do
    Application.get_env(:query_service_ex, :control_plane_url) ||
      System.get_env("CONTROL_PLANE_URL") || "http://localhost:3901"
  end
end
