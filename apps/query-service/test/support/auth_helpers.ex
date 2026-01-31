defmodule QueryServiceEx.AuthHelpers do
  @moduledoc """
  Test helpers for authentication-related tests.
  """

  alias QueryServiceEx.Accounts.Guardian

  @doc """
  Creates a mock Ueberauth auth struct for testing.
  """
  def mock_google_auth(attrs \\ %{}) do
    %Ueberauth.Auth{
      uid: attrs[:uid] || "google_#{:rand.uniform(100_000)}",
      provider: :google,
      info: %Ueberauth.Auth.Info{
        email: attrs[:email] || "test@example.com",
        name: attrs[:name] || "Test User",
        image: attrs[:avatar_url] || "https://example.com/avatar.jpg"
      },
      credentials: %Ueberauth.Auth.Credentials{
        token: attrs[:token] || "mock_google_token",
        refresh_token: attrs[:refresh_token] || "mock_refresh_token",
        expires: true,
        expires_at: System.system_time(:second) + 3600
      }
    }
  end

  @doc """
  Creates a test user and returns a valid JWT token.
  """
  def create_user_with_token(attrs \\ %{}) do
    user_attrs = %{
      email: attrs[:email] || "test@example.com",
      name: attrs[:name] || "Test User",
      avatar_url: attrs[:avatar_url] || "https://example.com/avatar.jpg",
      google_id: attrs[:google_id] || "google_#{:rand.uniform(100_000)}",
      google_token: attrs[:google_token] || "mock_token",
      google_refresh_token: attrs[:google_refresh_token]
    }

    {:ok, user} = QueryServiceEx.Accounts.create_user(user_attrs)
    {:ok, token, _claims} = Guardian.encode_and_sign(user)

    {user, token}
  end

  @doc """
  Adds an authorization header with the given token to a connection.
  """
  def with_auth_token(conn, token) do
    Plug.Conn.put_req_header(conn, "authorization", "Bearer #{token}")
  end
end
