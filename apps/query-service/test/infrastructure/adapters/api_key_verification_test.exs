defmodule QueryServiceEx.Infrastructure.Adapters.ApiKeyVerificationTest do
  @moduledoc """
  Regression coverage for the gateway's local API-key validation.

  A customer dogfooding the TypeScript SDK against the hosted gateway hit 401
  "invalid API key" on every data endpoint with a valid, unexpired, correctly
  signed service-account key. Root cause: the gateway verified keys by calling
  Core's `/api/v1/auth/me`, whose validator decodes into a `Claims` struct with a
  required `iss` field and `set_issuer("allsource")`. Keys minted by
  `ApiKeyController.sign_api_key/3` omit `iss`, so Core rejected every one.

  The fix validates keys locally (HS256 + expiry), like the Bearer path. These
  tests pin that contract — most importantly, that a key WITHOUT an `iss` claim
  authenticates.
  """
  use ExUnit.Case, async: false

  alias QueryServiceEx.Infrastructure.Adapters.RustCoreClient

  # 32+ bytes, matches AuthHelpers.test_jwt_secret/0.
  @secret "test_jwt_secret_for_testing_purposes_only_at_least_32_bytes"

  setup do
    original = System.get_env("JWT_SECRET")
    System.put_env("JWT_SECRET", @secret)

    on_exit(fn ->
      if original,
        do: System.put_env("JWT_SECRET", original),
        else: System.delete_env("JWT_SECRET")
    end)

    :ok
  end

  # The exact claim shape ApiKeyController.sign_api_key/3 historically produced:
  # a service-account API key with NO `iss` claim.
  defp base_claims do
    now = System.system_time(:second)

    %{
      "sub" => "key-#{now}",
      "tenant_id" => "decebal-dobrica-at-gmail-com",
      "name" => "DecebalDobrica Blog",
      "role" => "serviceaccount",
      "is_api_key" => true,
      "iat" => now,
      "exp" => now + 3600
    }
  end

  defp sign(claims, secret \\ @secret) do
    jwk = JOSE.JWK.from_oct(secret)

    {_alg, token} =
      JOSE.JWT.sign(jwk, %{"alg" => "HS256"}, JOSE.JWT.from_map(claims))
      |> JOSE.JWS.compact()

    token
  end

  describe "decode_api_key_jwt/1" do
    test "accepts a valid serviceaccount key that has NO iss claim (the regression)" do
      token = sign(base_claims())

      assert {:ok, claims} = RustCoreClient.decode_api_key_jwt(token)
      assert claims["tenant_id"] == "decebal-dobrica-at-gmail-com"
      assert claims["role"] == "serviceaccount"
      assert claims["is_api_key"] == true
      refute Map.has_key?(claims, "iss")
    end

    test "still accepts a key that DOES carry iss=allsource (forward-compat)" do
      token = sign(Map.put(base_claims(), "iss", "allsource"))
      assert {:ok, _claims} = RustCoreClient.decode_api_key_jwt(token)
    end

    test "rejects a token signed with a different secret" do
      token = sign(base_claims(), "a_totally_different_secret_value_at_least_32b")
      assert {:error, :invalid_key} = RustCoreClient.decode_api_key_jwt(token)
    end

    test "reports expiry distinctly from invalidity" do
      token = sign(Map.put(base_claims(), "exp", System.system_time(:second) - 10))
      assert {:error, :key_expired} = RustCoreClient.decode_api_key_jwt(token)
    end

    test "rejects a token missing tenant_id" do
      token = sign(Map.delete(base_claims(), "tenant_id"))
      assert {:error, :invalid_key} = RustCoreClient.decode_api_key_jwt(token)
    end

    test "rejects a structurally invalid token without raising" do
      assert {:error, :invalid_key} = RustCoreClient.decode_api_key_jwt("not.a.jwt")
      assert {:error, :invalid_key} = RustCoreClient.decode_api_key_jwt("garbage")
    end

    test "rejects when JWT_SECRET is not configured" do
      System.delete_env("JWT_SECRET")
      token = sign(base_claims())
      assert {:error, :invalid_key} = RustCoreClient.decode_api_key_jwt(token)
    end
  end
end
