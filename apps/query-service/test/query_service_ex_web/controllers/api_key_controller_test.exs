defmodule QueryServiceExWeb.ApiKeyControllerTest do
  @moduledoc """
  Tests for ApiKeyController.

  These tests require a running PostgreSQL database.
  Run with: mix test --include database
  """
  use QueryServiceExWeb.ConnCase

  alias QueryServiceEx.Accounts
  alias QueryServiceEx.ApiKeys
  alias QueryServiceEx.Tenants
  alias QueryServiceEx.Accounts.Guardian

  @moduletag :database

  setup %{conn: conn} do
    {:ok, tenant} =
      Tenants.create_tenant(%{
        name: "Test Workspace",
        slug: "test-workspace-#{System.unique_integer([:positive])}"
      })

    {:ok, user} =
      Accounts.create_user(%{
        email: "test#{System.unique_integer([:positive])}@example.com",
        name: "Test User",
        provider: "google",
        google_id: "google_#{System.unique_integer([:positive])}",
        tenant_id: tenant.id
      })

    user = QueryServiceEx.Repo.preload(user, :tenant)

    {:ok, token, _claims} = Guardian.encode_and_sign(user)

    conn =
      conn
      |> put_req_header("authorization", "Bearer #{token}")
      |> put_req_header("accept", "application/json")

    {:ok, conn: conn, tenant: tenant, user: user}
  end

  describe "GET /api/api-keys" do
    test "returns empty list when no keys exist", %{conn: conn} do
      conn = get(conn, "/api/api-keys")

      assert json_response(conn, 200)["data"] == []
    end

    test "returns all active API keys for tenant", %{conn: conn, tenant: tenant, user: user} do
      {:ok, _} = ApiKeys.create_api_key(tenant.id, user.id, %{name: "Key 1"})
      {:ok, _} = ApiKeys.create_api_key(tenant.id, user.id, %{name: "Key 2"})

      conn = get(conn, "/api/api-keys")
      response = json_response(conn, 200)["data"]

      assert length(response) == 2
      assert Enum.any?(response, &(&1["name"] == "Key 1"))
      assert Enum.any?(response, &(&1["name"] == "Key 2"))
    end

    test "excludes revoked keys", %{conn: conn, tenant: tenant, user: user} do
      {:ok, _} = ApiKeys.create_api_key(tenant.id, user.id, %{name: "Active"})

      {:ok, {_raw_key, to_revoke}} =
        ApiKeys.create_api_key(tenant.id, user.id, %{name: "Revoked"})

      {:ok, _} = ApiKeys.revoke_api_key(to_revoke)

      conn = get(conn, "/api/api-keys")
      response = json_response(conn, 200)["data"]

      assert length(response) == 1
      assert hd(response)["name"] == "Active"
    end

    test "returns 401 without auth token", %{conn: conn} do
      conn =
        conn
        |> delete_req_header("authorization")
        |> get("/api/api-keys")

      assert json_response(conn, 401)
    end
  end

  describe "POST /api/api-keys" do
    test "creates API key and returns raw key", %{conn: conn} do
      params = %{
        "name" => "My New Key",
        "description" => "Test key for CI/CD"
      }

      conn = post(conn, "/api/api-keys", params)
      response = json_response(conn, 201)

      assert response["data"]["name"] == "My New Key"
      assert response["data"]["description"] == "Test key for CI/CD"
      assert response["data"]["key"] =~ ~r/^chronos_test_/
      assert response["data"]["key_prefix"] =~ ~r/^chronos_test_/
      assert response["warning"] =~ "Save this key"
    end

    test "creates API key with custom scopes", %{conn: conn} do
      params = %{
        "name" => "Limited Key",
        "scopes" => ["events:read", "queries:execute"]
      }

      conn = post(conn, "/api/api-keys", params)
      response = json_response(conn, 201)["data"]

      assert response["scopes"] == ["events:read", "queries:execute"]
    end

    test "creates API key with expiration", %{conn: conn} do
      expires = DateTime.utc_now() |> DateTime.add(30, :day) |> DateTime.to_iso8601()

      params = %{
        "name" => "Expiring Key",
        "expires_at" => expires
      }

      conn = post(conn, "/api/api-keys", params)
      response = json_response(conn, 201)["data"]

      assert response["expires_at"] != nil
    end

    test "returns validation error without name", %{conn: conn} do
      params = %{"description" => "No name"}

      conn = post(conn, "/api/api-keys", params)
      response = json_response(conn, 422)["error"]

      assert response["code"] == "validation_error"
      assert response["details"]["name"] != nil
    end

    test "returns validation error for duplicate name", %{conn: conn, tenant: tenant, user: user} do
      {:ok, _} = ApiKeys.create_api_key(tenant.id, user.id, %{name: "Duplicate"})

      params = %{"name" => "Duplicate"}

      conn = post(conn, "/api/api-keys", params)
      response = json_response(conn, 422)["error"]

      assert response["code"] == "validation_error"
    end
  end

  describe "GET /api/api-keys/:id" do
    test "returns API key details", %{conn: conn, tenant: tenant, user: user} do
      {:ok, {_raw_key, api_key}} =
        ApiKeys.create_api_key(tenant.id, user.id, %{name: "Fetch Me"})

      conn = get(conn, "/api/api-keys/#{api_key.id}")
      response = json_response(conn, 200)["data"]

      assert response["id"] == api_key.id
      assert response["name"] == "Fetch Me"
      # Should NOT include the raw key
      refute Map.has_key?(response, "key") or
               (Map.has_key?(response, "key") and response["key"] =~ ~r/^chronos_test_[^\.]+$/)
    end

    test "returns 404 for non-existent key", %{conn: conn} do
      conn = get(conn, "/api/api-keys/#{Ecto.UUID.generate()}")

      assert json_response(conn, 404)["error"]["code"] == "not_found"
    end

    test "returns 404 for revoked key", %{conn: conn, tenant: tenant, user: user} do
      {:ok, {_raw_key, api_key}} = ApiKeys.create_api_key(tenant.id, user.id, %{name: "Revoked"})
      {:ok, _} = ApiKeys.revoke_api_key(api_key)

      conn = get(conn, "/api/api-keys/#{api_key.id}")

      assert json_response(conn, 404)["error"]["code"] == "not_found"
    end
  end

  describe "PUT /api/api-keys/:id" do
    test "updates API key metadata", %{conn: conn, tenant: tenant, user: user} do
      {:ok, {_raw_key, api_key}} = ApiKeys.create_api_key(tenant.id, user.id, %{name: "Original"})

      params = %{
        "name" => "Updated Name",
        "description" => "Updated description"
      }

      conn = put(conn, "/api/api-keys/#{api_key.id}", params)
      response = json_response(conn, 200)["data"]

      assert response["name"] == "Updated Name"
      assert response["description"] == "Updated description"
    end

    test "updates scopes", %{conn: conn, tenant: tenant, user: user} do
      {:ok, {_raw_key, api_key}} = ApiKeys.create_api_key(tenant.id, user.id, %{name: "Key"})

      params = %{"scopes" => ["events:read"]}

      conn = put(conn, "/api/api-keys/#{api_key.id}", params)
      response = json_response(conn, 200)["data"]

      assert response["scopes"] == ["events:read"]
    end

    test "returns 404 for non-existent key", %{conn: conn} do
      params = %{"name" => "Updated"}

      conn = put(conn, "/api/api-keys/#{Ecto.UUID.generate()}", params)

      assert json_response(conn, 404)["error"]["code"] == "not_found"
    end
  end

  describe "POST /api/api-keys/:id/rotate" do
    test "rotates API key and returns new key", %{conn: conn, tenant: tenant, user: user} do
      {:ok, {old_raw_key, api_key}} =
        ApiKeys.create_api_key(tenant.id, user.id, %{
          name: "Rotate Me",
          description: "Important"
        })

      conn = post(conn, "/api/api-keys/#{api_key.id}/rotate")
      response = json_response(conn, 200)

      # New key is returned
      assert response["data"]["key"] =~ ~r/^chronos_test_/
      assert response["data"]["key"] != old_raw_key
      assert response["data"]["name"] == "Rotate Me"
      assert response["data"]["description"] == "Important"
      assert response["warning"] =~ "old key has been revoked"

      # Old key is now invalid
      assert {:error, :key_revoked} = ApiKeys.verify_api_key(old_raw_key)
    end

    test "returns 404 for non-existent key", %{conn: conn} do
      conn = post(conn, "/api/api-keys/#{Ecto.UUID.generate()}/rotate")

      assert json_response(conn, 404)["error"]["code"] == "not_found"
    end
  end

  describe "DELETE /api/api-keys/:id" do
    test "revokes API key", %{conn: conn, tenant: tenant, user: user} do
      {:ok, {raw_key, api_key}} = ApiKeys.create_api_key(tenant.id, user.id, %{name: "Delete Me"})

      conn = delete(conn, "/api/api-keys/#{api_key.id}")
      response = json_response(conn, 200)["data"]

      assert response["message"] =~ "revoked successfully"

      # Key is no longer valid
      assert {:error, :key_revoked} = ApiKeys.verify_api_key(raw_key)
    end

    test "returns 404 for non-existent key", %{conn: conn} do
      conn = delete(conn, "/api/api-keys/#{Ecto.UUID.generate()}")

      assert json_response(conn, 404)["error"]["code"] == "not_found"
    end
  end

  describe "GET /api/api-keys/scopes" do
    test "returns available scopes with descriptions", %{conn: conn} do
      conn = get(conn, "/api/api-keys/scopes")
      response = json_response(conn, 200)["data"]

      assert is_list(response)
      assert length(response) > 0

      scope = hd(response)
      assert Map.has_key?(scope, "name")
      assert Map.has_key?(scope, "description")
    end
  end
end
