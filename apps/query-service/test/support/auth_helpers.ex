defmodule QueryServiceEx.AuthHelpers do
  @moduledoc """
  Test helpers for authentication-related tests.
  """

  @test_jwt_secret "test_jwt_secret_for_testing_purposes_only_at_least_32_bytes"

  def test_jwt_secret, do: @test_jwt_secret

  @doc """
  Creates a test user map and returns {user, token}.
  No database required.
  """
  def create_test_user_with_token(attrs \\ %{}) do
    user = %{
      id: attrs[:id] || "user-#{:rand.uniform(100_000)}",
      email: attrs[:email] || "test#{:rand.uniform(100_000)}@example.com",
      name: attrs[:name] || "Test User",
      tenant_id: attrs[:tenant_id] || "tenant-#{:rand.uniform(100_000)}",
      role: attrs[:role] || "user"
    }

    token = encode_jwt(user)
    {user, token}
  end

  @doc """
  Generates a JWT token for the given user map using JOSE.
  """
  def encode_jwt(user, opts \\ []) do
    ttl = Keyword.get(opts, :ttl_seconds, 3600)
    now = System.system_time(:second)

    user_id = Map.get(user, :id) || Map.get(user, "id")
    tenant_id = Map.get(user, :tenant_id) || Map.get(user, "tenant_id")
    email = Map.get(user, :email) || Map.get(user, "email")
    name = Map.get(user, :name) || Map.get(user, "name")

    claims = %{
      "sub" => to_string(user_id),
      "tenant_id" => tenant_id,
      "email" => email,
      "name" => name,
      "role" => "user",
      "iat" => now,
      "exp" => now + ttl
    }

    jwk = JOSE.JWK.from_oct(@test_jwt_secret)
    jws = %{"alg" => "HS256"}
    jwt = JOSE.JWT.from_map(claims)

    {_alg, token} = JOSE.JWT.sign(jwk, jws, jwt) |> JOSE.JWS.compact()
    token
  end

  def with_auth_token(conn, token) do
    Plug.Conn.put_req_header(conn, "authorization", "Bearer #{token}")
  end
end
