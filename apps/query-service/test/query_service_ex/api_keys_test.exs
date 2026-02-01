defmodule QueryServiceEx.ApiKeysTest do
  @moduledoc """
  Tests for the ApiKeys context.

  These tests require a running PostgreSQL database.
  Run with: mix test --include database
  """
  use QueryServiceEx.DataCase

  alias QueryServiceEx.Accounts
  alias QueryServiceEx.ApiKeys
  alias QueryServiceEx.ApiKeys.ApiKey
  alias QueryServiceEx.Tenants

  @moduletag :database

  setup do
    slug = "test-api-keys-#{System.unique_integer([:positive])}"
    {:ok, tenant} = Tenants.create_tenant(%{name: "Test Workspace", slug: slug})

    {:ok, user} =
      Accounts.create_user(%{
        email: "test@example.com",
        name: "Test User",
        provider: "google",
        google_id: "google_#{System.unique_integer([:positive])}",
        tenant_id: tenant.id
      })

    {:ok, tenant: tenant, user: user}
  end

  describe "create_api_key/3" do
    test "creates an API key with valid attributes", %{tenant: tenant, user: user} do
      attrs = %{name: "My API Key", description: "Test key"}

      assert {:ok, {raw_key, api_key}} = ApiKeys.create_api_key(tenant.id, user.id, attrs)
      assert raw_key =~ ~r/^chronos_test_/
      assert api_key.name == "My API Key"
      assert api_key.description == "Test key"
      assert api_key.tenant_id == tenant.id
      assert api_key.created_by_user_id == user.id
      assert api_key.key_prefix =~ ~r/^chronos_test_/
      assert api_key.scopes == ApiKey.available_scopes()
      assert is_nil(api_key.revoked_at)
    end

    test "creates API key with custom scopes", %{tenant: tenant, user: user} do
      attrs = %{name: "Limited Key", scopes: ["events:read", "queries:execute"]}

      assert {:ok, {_raw_key, api_key}} = ApiKeys.create_api_key(tenant.id, user.id, attrs)
      assert api_key.scopes == ["events:read", "queries:execute"]
    end

    test "creates API key with expiration", %{tenant: tenant, user: user} do
      expires_at = DateTime.utc_now() |> DateTime.add(30, :day) |> DateTime.truncate(:second)
      attrs = %{name: "Expiring Key", expires_at: expires_at}

      assert {:ok, {_raw_key, api_key}} = ApiKeys.create_api_key(tenant.id, user.id, attrs)
      assert api_key.expires_at == expires_at
    end

    test "requires name", %{tenant: tenant, user: user} do
      attrs = %{description: "No name"}

      assert {:error, changeset} = ApiKeys.create_api_key(tenant.id, user.id, attrs)
      assert "can't be blank" in errors_on(changeset).name
    end

    test "prevents duplicate names within tenant", %{tenant: tenant, user: user} do
      attrs = %{name: "Unique Name"}

      assert {:ok, _} = ApiKeys.create_api_key(tenant.id, user.id, attrs)
      assert {:error, changeset} = ApiKeys.create_api_key(tenant.id, user.id, attrs)
      errors = errors_on(changeset)
      # Composite unique constraint error appears under first field (:tenant_id)
      assert "an API key with this name already exists" in errors.tenant_id
    end

    test "validates invalid scopes", %{tenant: tenant, user: user} do
      attrs = %{name: "Bad Scopes", scopes: ["invalid:scope"]}

      assert {:error, changeset} = ApiKeys.create_api_key(tenant.id, user.id, attrs)
      assert errors_on(changeset).scopes != nil
    end

    test "validates expires_at must be in the future", %{tenant: tenant, user: user} do
      past = DateTime.utc_now() |> DateTime.add(-1, :day)
      attrs = %{name: "Past Expiry", expires_at: past}

      assert {:error, changeset} = ApiKeys.create_api_key(tenant.id, user.id, attrs)
      assert "must be in the future" in errors_on(changeset).expires_at
    end
  end

  describe "get_api_key/2" do
    test "returns API key when found and belongs to tenant", %{tenant: tenant, user: user} do
      {:ok, {_raw_key, created}} =
        ApiKeys.create_api_key(tenant.id, user.id, %{name: "Test Key"})

      api_key = ApiKeys.get_api_key(created.id, tenant.id)
      assert api_key.id == created.id
    end

    test "returns nil when not found", %{tenant: tenant} do
      assert ApiKeys.get_api_key(Ecto.UUID.generate(), tenant.id) == nil
    end

    test "returns nil when belongs to different tenant", %{user: user} do
      {:ok, other_tenant} = Tenants.create_tenant(%{name: "Other", slug: "other-tenant"})

      {:ok, {_raw_key, api_key}} =
        ApiKeys.create_api_key(other_tenant.id, user.id, %{name: "Other Key"})

      # Try to get with wrong tenant
      {:ok, wrong_tenant} = Tenants.create_tenant(%{name: "Wrong", slug: "wrong-tenant"})
      assert ApiKeys.get_api_key(api_key.id, wrong_tenant.id) == nil
    end

    test "returns nil for revoked keys", %{tenant: tenant, user: user} do
      {:ok, {_raw_key, api_key}} =
        ApiKeys.create_api_key(tenant.id, user.id, %{name: "To Revoke"})

      {:ok, _} = ApiKeys.revoke_api_key(api_key)

      assert ApiKeys.get_api_key(api_key.id, tenant.id) == nil
    end
  end

  describe "list_api_keys/1" do
    test "returns all active keys for tenant", %{tenant: tenant, user: user} do
      {:ok, _} = ApiKeys.create_api_key(tenant.id, user.id, %{name: "Key 1"})
      {:ok, _} = ApiKeys.create_api_key(tenant.id, user.id, %{name: "Key 2"})
      {:ok, _} = ApiKeys.create_api_key(tenant.id, user.id, %{name: "Key 3"})

      keys = ApiKeys.list_api_keys(tenant.id)
      assert length(keys) == 3
    end

    test "excludes revoked keys", %{tenant: tenant, user: user} do
      {:ok, _} = ApiKeys.create_api_key(tenant.id, user.id, %{name: "Active Key"})

      {:ok, {_raw_key, to_revoke}} =
        ApiKeys.create_api_key(tenant.id, user.id, %{name: "Revoked Key"})

      {:ok, _} = ApiKeys.revoke_api_key(to_revoke)

      keys = ApiKeys.list_api_keys(tenant.id)
      assert length(keys) == 1
      assert hd(keys).name == "Active Key"
    end

    test "returns empty list for tenant with no keys", %{tenant: _tenant} do
      {:ok, empty_tenant} = Tenants.create_tenant(%{name: "Empty", slug: "empty-tenant"})
      assert ApiKeys.list_api_keys(empty_tenant.id) == []
    end
  end

  describe "update_api_key/2" do
    test "updates name and description", %{tenant: tenant, user: user} do
      {:ok, {_raw_key, api_key}} = ApiKeys.create_api_key(tenant.id, user.id, %{name: "Original"})

      assert {:ok, updated} =
               ApiKeys.update_api_key(api_key, %{
                 "name" => "Updated",
                 "description" => "New description"
               })

      assert updated.name == "Updated"
      assert updated.description == "New description"
    end

    test "updates scopes", %{tenant: tenant, user: user} do
      {:ok, {_raw_key, api_key}} = ApiKeys.create_api_key(tenant.id, user.id, %{name: "Key"})

      assert {:ok, updated} =
               ApiKeys.update_api_key(api_key, %{"scopes" => ["events:read"]})

      assert updated.scopes == ["events:read"]
    end
  end

  describe "revoke_api_key/1" do
    test "revokes an API key", %{tenant: tenant, user: user} do
      {:ok, {_raw_key, api_key}} =
        ApiKeys.create_api_key(tenant.id, user.id, %{name: "To Revoke"})

      assert is_nil(api_key.revoked_at)

      assert {:ok, revoked} = ApiKeys.revoke_api_key(api_key)
      assert not is_nil(revoked.revoked_at)
    end
  end

  describe "revoke_api_key/2" do
    test "revokes API key by ID", %{tenant: tenant, user: user} do
      {:ok, {_raw_key, api_key}} = ApiKeys.create_api_key(tenant.id, user.id, %{name: "Key"})

      assert {:ok, _} = ApiKeys.revoke_api_key(api_key.id, tenant.id)
    end

    test "returns error for non-existent key", %{tenant: tenant} do
      assert {:error, :not_found} = ApiKeys.revoke_api_key(Ecto.UUID.generate(), tenant.id)
    end
  end

  describe "rotate_api_key/2" do
    test "revokes old key and creates new one", %{tenant: tenant, user: user} do
      {:ok, {_old_raw_key, old_key}} =
        ApiKeys.create_api_key(tenant.id, user.id, %{
          name: "Rotate Me",
          description: "Important key",
          scopes: ["events:read"]
        })

      assert {:ok, {new_raw_key, new_key}} = ApiKeys.rotate_api_key(old_key, user.id)

      # New key has same metadata
      assert new_key.name == "Rotate Me"
      assert new_key.description == "Important key"
      assert new_key.scopes == ["events:read"]
      assert new_key.id != old_key.id
      assert new_raw_key =~ ~r/^chronos_test_/

      # Old key is revoked
      old = Repo.get(ApiKey, old_key.id)
      assert not is_nil(old.revoked_at)
    end
  end

  describe "verify_api_key/1" do
    test "returns tenant and key for valid key", %{tenant: tenant, user: user} do
      {:ok, {raw_key, api_key}} = ApiKeys.create_api_key(tenant.id, user.id, %{name: "Valid Key"})

      assert {:ok, {returned_tenant, returned_key}} = ApiKeys.verify_api_key(raw_key)
      assert returned_tenant.id == tenant.id
      assert returned_key.id == api_key.id
    end

    test "returns error for invalid key", %{} do
      assert {:error, :invalid_key} = ApiKeys.verify_api_key("chronos_test_invalid123")
    end

    test "returns error for revoked key", %{tenant: tenant, user: user} do
      {:ok, {raw_key, api_key}} = ApiKeys.create_api_key(tenant.id, user.id, %{name: "Revoked"})
      {:ok, _} = ApiKeys.revoke_api_key(api_key)

      assert {:error, :key_revoked} = ApiKeys.verify_api_key(raw_key)
    end

    test "returns error for expired key", %{tenant: tenant, user: user} do
      # Create key with very short expiration
      {:ok, {raw_key, _api_key}} =
        ApiKeys.create_api_key(tenant.id, user.id, %{
          name: "Expiring",
          expires_at: DateTime.utc_now() |> DateTime.add(1, :second)
        })

      # Wait for expiration with retry loop to handle CI timing variability
      result =
        Enum.reduce_while(1..20, :not_expired, fn _attempt, _acc ->
          Process.sleep(200)

          case ApiKeys.verify_api_key(raw_key) do
            {:error, :key_expired} -> {:halt, {:error, :key_expired}}
            _ -> {:cont, :not_expired}
          end
        end)

      assert {:error, :key_expired} = result
    end
  end

  describe "check_scope/2" do
    test "returns :ok when key has scope", %{tenant: tenant, user: user} do
      {:ok, {_raw_key, api_key}} =
        ApiKeys.create_api_key(tenant.id, user.id, %{
          name: "Scoped",
          scopes: ["events:read", "events:write"]
        })

      assert :ok = ApiKeys.check_scope(api_key, "events:read")
    end

    test "returns error when key lacks scope", %{tenant: tenant, user: user} do
      {:ok, {_raw_key, api_key}} =
        ApiKeys.create_api_key(tenant.id, user.id, %{
          name: "Limited",
          scopes: ["events:read"]
        })

      assert {:error, :insufficient_scope} = ApiKeys.check_scope(api_key, "events:write")
    end
  end

  describe "check_scopes/2" do
    test "returns :ok when key has all scopes", %{tenant: tenant, user: user} do
      {:ok, {_raw_key, api_key}} =
        ApiKeys.create_api_key(tenant.id, user.id, %{
          name: "Multi-scope",
          scopes: ["events:read", "events:write", "queries:execute"]
        })

      assert :ok = ApiKeys.check_scopes(api_key, ["events:read", "queries:execute"])
    end

    test "returns error when missing any scope", %{tenant: tenant, user: user} do
      {:ok, {_raw_key, api_key}} =
        ApiKeys.create_api_key(tenant.id, user.id, %{
          name: "Partial",
          scopes: ["events:read"]
        })

      assert {:error, :insufficient_scope} =
               ApiKeys.check_scopes(api_key, ["events:read", "events:write"])
    end
  end

  describe "count_api_keys/1" do
    test "counts active keys for tenant", %{tenant: tenant, user: user} do
      assert ApiKeys.count_api_keys(tenant.id) == 0

      {:ok, _} = ApiKeys.create_api_key(tenant.id, user.id, %{name: "Key 1"})
      {:ok, _} = ApiKeys.create_api_key(tenant.id, user.id, %{name: "Key 2"})

      assert ApiKeys.count_api_keys(tenant.id) == 2

      # Revoke one
      {:ok, {_raw_key, to_revoke}} = ApiKeys.create_api_key(tenant.id, user.id, %{name: "Key 3"})
      {:ok, _} = ApiKeys.revoke_api_key(to_revoke)

      assert ApiKeys.count_api_keys(tenant.id) == 2
    end
  end

  describe "ApiKey.active?/1" do
    test "returns true for active key", %{tenant: tenant, user: user} do
      {:ok, {_raw_key, api_key}} = ApiKeys.create_api_key(tenant.id, user.id, %{name: "Active"})
      assert ApiKey.active?(api_key) == true
    end

    test "returns false for revoked key", %{tenant: tenant, user: user} do
      {:ok, {_raw_key, api_key}} = ApiKeys.create_api_key(tenant.id, user.id, %{name: "Revoked"})
      {:ok, revoked} = ApiKeys.revoke_api_key(api_key)
      assert ApiKey.active?(revoked) == false
    end

    test "returns false for expired key", %{tenant: tenant, user: user} do
      past = DateTime.utc_now() |> DateTime.add(-1, :day)

      api_key = %ApiKey{
        expires_at: past,
        revoked_at: nil
      }

      assert ApiKey.active?(api_key) == false
    end
  end
end
