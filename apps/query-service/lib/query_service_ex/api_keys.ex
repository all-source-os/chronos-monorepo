defmodule QueryServiceEx.ApiKeys do
  @moduledoc """
  The ApiKeys context handles customer API key management.

  Provides functions for:
  - Creating and revoking API keys
  - Verifying API keys for authentication
  - Rotating keys without support intervention
  - Listing and managing keys for a tenant
  """

  import Ecto.Query
  alias QueryServiceEx.ApiKeys.ApiKey
  alias QueryServiceEx.Repo

  require Logger

  # Key format: chronos_<env>_<random>
  # Example: chronos_live_a1b2c3d4e5f6g7h8i9j0k1l2m3n4o5p6
  @key_prefix "chronos"
  @key_length 32

  # -------------------------------------------------------------------
  # API Key CRUD
  # -------------------------------------------------------------------

  @doc """
  Creates a new API key for a tenant.

  Returns `{:ok, {raw_key, api_key}}` where `raw_key` is the secret that
  should be shown to the user once. The raw key is NOT stored - only its
  hash is persisted.

  ## Options
    * `:name` - Required. A user-friendly name for the key
    * `:description` - Optional. Description of what the key is used for
    * `:scopes` - Optional. List of permission scopes (defaults to all)
    * `:expires_at` - Optional. Expiration datetime
  """
  def create_api_key(tenant_id, user_id, attrs) do
    {raw_key, key_hash, key_prefix} = generate_key()

    attrs =
      attrs
      |> Map.put(:tenant_id, tenant_id)
      |> Map.put(:created_by_user_id, user_id)
      |> Map.put(:key_hash, key_hash)
      |> Map.put(:key_prefix, key_prefix)
      |> maybe_set_default_scopes()

    case %ApiKey{}
         |> ApiKey.create_changeset(attrs)
         |> Repo.insert() do
      {:ok, api_key} ->
        Logger.info("API key created",
          tenant_id: tenant_id,
          api_key_id: api_key.id,
          name: api_key.name
        )

        {:ok, {raw_key, api_key}}

      {:error, changeset} ->
        {:error, changeset}
    end
  end

  @doc """
  Gets an API key by ID, scoped to a tenant.

  Returns `nil` if not found or belongs to different tenant.
  """
  def get_api_key(id, tenant_id) do
    from(k in ApiKey,
      where: k.id == ^id,
      where: k.tenant_id == ^tenant_id,
      where: is_nil(k.revoked_at)
    )
    |> Repo.one()
  end

  @doc """
  Gets an API key by ID, scoped to a tenant.

  Raises `Ecto.NoResultsError` if not found.
  """
  def get_api_key!(id, tenant_id) do
    from(k in ApiKey,
      where: k.id == ^id,
      where: k.tenant_id == ^tenant_id,
      where: is_nil(k.revoked_at)
    )
    |> Repo.one!()
  end

  @doc """
  Lists all active (non-revoked) API keys for a tenant.
  """
  def list_api_keys(tenant_id) do
    from(k in ApiKey,
      where: k.tenant_id == ^tenant_id,
      where: is_nil(k.revoked_at),
      order_by: [desc: k.inserted_at]
    )
    |> Repo.all()
  end

  @doc """
  Updates an API key's metadata (name, description, scopes).

  The key itself cannot be changed - use `rotate_api_key/2` for that.
  """
  def update_api_key(%ApiKey{} = api_key, attrs) do
    api_key
    |> ApiKey.update_changeset(attrs)
    |> Repo.update()
  end

  @doc """
  Revokes an API key (soft delete).

  The key will no longer be valid for authentication.
  """
  def revoke_api_key(%ApiKey{} = api_key) do
    api_key
    |> ApiKey.revoke_changeset()
    |> Repo.update()
    |> tap(fn
      {:ok, _} ->
        Logger.info("API key revoked",
          tenant_id: api_key.tenant_id,
          api_key_id: api_key.id,
          name: api_key.name
        )

      _ ->
        :ok
    end)
  end

  @doc """
  Revokes an API key by ID, scoped to tenant.
  """
  def revoke_api_key(id, tenant_id) do
    case get_api_key(id, tenant_id) do
      nil -> {:error, :not_found}
      api_key -> revoke_api_key(api_key)
    end
  end

  @doc """
  Rotates an API key - revokes the old one and creates a new one
  with the same settings.

  Returns `{:ok, {raw_key, new_api_key}}` on success.
  """
  def rotate_api_key(%ApiKey{} = api_key, user_id) do
    Repo.transaction(fn ->
      with {:ok, _revoked} <- revoke_api_key(api_key),
           {:ok, {raw_key, new_api_key}} <- create_rotated_key(api_key, user_id) do
        log_key_rotation(api_key, new_api_key)
        {raw_key, new_api_key}
      else
        {:error, reason} -> Repo.rollback(reason)
      end
    end)
  end

  defp create_rotated_key(api_key, user_id) do
    attrs = %{
      name: api_key.name,
      description: api_key.description,
      scopes: api_key.scopes,
      expires_at: api_key.expires_at
    }

    create_api_key(api_key.tenant_id, user_id, attrs)
  end

  defp log_key_rotation(old_key, new_key) do
    Logger.info("API key rotated",
      tenant_id: old_key.tenant_id,
      old_api_key_id: old_key.id,
      new_api_key_id: new_key.id,
      name: old_key.name
    )
  end

  # -------------------------------------------------------------------
  # Authentication
  # -------------------------------------------------------------------

  @doc """
  Verifies an API key and returns the associated tenant and key record.

  Returns `{:ok, {tenant, api_key}}` if valid, or `{:error, reason}` if not.

  Also updates the `last_used_at` timestamp on successful verification.
  """
  def verify_api_key(raw_key) do
    key_hash = hash_key(raw_key)

    case Repo.get_by(ApiKey, key_hash: key_hash) do
      nil -> {:error, :invalid_key}
      api_key -> validate_api_key(api_key)
    end
  end

  defp validate_api_key(api_key) do
    cond do
      not is_nil(api_key.revoked_at) ->
        {:error, :key_revoked}

      api_key_expired?(api_key) ->
        {:error, :key_expired}

      true ->
        touch_api_key_async(api_key)
        tenant = QueryServiceEx.Tenants.get_tenant(api_key.tenant_id)
        {:ok, {tenant, api_key}}
    end
  end

  defp api_key_expired?(%{expires_at: nil}), do: false

  defp api_key_expired?(%{expires_at: expires_at}) do
    DateTime.compare(expires_at, DateTime.utc_now()) != :gt
  end

  defp touch_api_key_async(api_key) do
    spawn(fn ->
      try do
        touch_api_key(api_key)
      rescue
        _ -> :ok
      end
    end)
  end

  @doc """
  Checks if an API key has the required scope.
  """
  def check_scope(%ApiKey{} = api_key, required_scope) do
    if ApiKey.has_scope?(api_key, required_scope) do
      :ok
    else
      {:error, :insufficient_scope}
    end
  end

  @doc """
  Checks if an API key has all required scopes.
  """
  def check_scopes(%ApiKey{} = api_key, required_scopes) do
    if ApiKey.has_all_scopes?(api_key, required_scopes) do
      :ok
    else
      {:error, :insufficient_scope}
    end
  end

  # -------------------------------------------------------------------
  # Maintenance
  # -------------------------------------------------------------------

  @doc """
  Gets all API keys expiring within the specified number of days.
  """
  def get_expiring_keys(days \\ 7) do
    cutoff = DateTime.utc_now() |> DateTime.add(days, :day)

    from(k in ApiKey,
      where: is_nil(k.revoked_at),
      where: not is_nil(k.expires_at),
      where: k.expires_at <= ^cutoff,
      where: k.expires_at > ^DateTime.utc_now(),
      preload: [:tenant]
    )
    |> Repo.all()
  end

  @doc """
  Counts active API keys for a tenant.
  """
  def count_api_keys(tenant_id) do
    from(k in ApiKey,
      where: k.tenant_id == ^tenant_id,
      where: is_nil(k.revoked_at),
      select: count(k.id)
    )
    |> Repo.one()
  end

  # -------------------------------------------------------------------
  # Private Helpers
  # -------------------------------------------------------------------

  defp generate_key do
    env = Application.get_env(:query_service_ex, :env, :dev)
    env_prefix = if env == :prod, do: "live", else: "test"

    random_part =
      :crypto.strong_rand_bytes(@key_length)
      |> Base.url_encode64(padding: false)
      |> String.slice(0, @key_length)

    raw_key = "#{@key_prefix}_#{env_prefix}_#{random_part}"
    key_hash = hash_key(raw_key)
    # Show first 12 chars for identification: chronos_live_xxxx
    key_prefix = String.slice(raw_key, 0, 17) <> "..."

    {raw_key, key_hash, key_prefix}
  end

  defp hash_key(key) do
    :crypto.hash(:sha256, key)
    |> Base.encode16(case: :lower)
  end

  defp maybe_set_default_scopes(%{scopes: scopes} = attrs)
       when is_list(scopes) and scopes != [] do
    attrs
  end

  defp maybe_set_default_scopes(attrs) do
    Map.put(attrs, :scopes, ApiKey.available_scopes())
  end

  defp touch_api_key(%ApiKey{} = api_key) do
    api_key
    |> ApiKey.touch_changeset()
    |> Repo.update()
  end
end
