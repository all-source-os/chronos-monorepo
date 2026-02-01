defmodule QueryServiceEx.ApiKeys.ApiKey do
  @moduledoc """
  Schema for customer API keys.

  API keys allow customers to authenticate programmatically without
  using OAuth tokens. Keys are scoped to tenants and can have specific
  permissions (scopes).

  Security:
  - Only the SHA-256 hash of the key is stored
  - The raw key is returned only once at creation time
  - Keys can be rotated (revoke old + create new) without support
  """

  use Ecto.Schema
  import Ecto.Changeset

  @primary_key {:id, :binary_id, autogenerate: true}
  @foreign_key_type :binary_id

  @available_scopes ~w(events:read events:write queries:execute projections:read projections:write schemas:read schemas:write)

  schema "api_keys" do
    field(:name, :string)
    field(:description, :string)
    field(:key_hash, :string)
    field(:key_prefix, :string)
    field(:scopes, {:array, :string}, default: [])
    field(:last_used_at, :utc_datetime)
    field(:expires_at, :utc_datetime)
    field(:revoked_at, :utc_datetime)

    belongs_to(:tenant, QueryServiceEx.Tenants.Tenant)
    belongs_to(:created_by_user, QueryServiceEx.Accounts.User, foreign_key: :created_by_user_id)

    timestamps(type: :utc_datetime)
  end

  @doc """
  Returns the list of available scopes for API keys.
  """
  def available_scopes, do: @available_scopes

  @doc """
  Changeset for creating a new API key.

  Note: key_hash and key_prefix must be set by the context module
  after generating the raw key.
  """
  def create_changeset(api_key, attrs) do
    api_key
    |> cast(attrs, [
      :name,
      :description,
      :scopes,
      :expires_at,
      :tenant_id,
      :created_by_user_id,
      :key_hash,
      :key_prefix
    ])
    |> validate_required([:name, :tenant_id, :key_hash, :key_prefix])
    |> validate_length(:name, min: 1, max: 100)
    |> validate_length(:description, max: 500)
    |> validate_scopes()
    |> validate_expires_at()
    |> unique_constraint([:tenant_id, :name],
      name: :api_keys_tenant_name_active_unique,
      message: "an API key with this name already exists"
    )
    |> unique_constraint(:key_hash)
    |> foreign_key_constraint(:tenant_id)
    |> foreign_key_constraint(:created_by_user_id)
  end

  @doc """
  Changeset for updating API key metadata (name, description, scopes).
  """
  def update_changeset(api_key, attrs) do
    api_key
    |> cast(attrs, [:name, :description, :scopes])
    |> validate_length(:name, min: 1, max: 100)
    |> validate_length(:description, max: 500)
    |> validate_scopes()
    |> unique_constraint([:tenant_id, :name],
      name: :api_keys_tenant_name_active_unique,
      message: "an API key with this name already exists"
    )
  end

  @doc """
  Changeset for revoking an API key.
  """
  def revoke_changeset(api_key) do
    api_key
    |> change(%{revoked_at: DateTime.utc_now() |> DateTime.truncate(:second)})
  end

  @doc """
  Changeset for updating last_used_at timestamp.
  """
  def touch_changeset(api_key) do
    api_key
    |> change(%{last_used_at: DateTime.utc_now() |> DateTime.truncate(:second)})
  end

  @doc """
  Checks if the API key is currently active (not revoked or expired).
  """
  def active?(%__MODULE__{revoked_at: revoked_at, expires_at: expires_at}) do
    not_revoked = is_nil(revoked_at)
    not_expired = is_nil(expires_at) or DateTime.compare(expires_at, DateTime.utc_now()) == :gt

    not_revoked and not_expired
  end

  @doc """
  Checks if the API key has a specific scope.
  """
  def has_scope?(%__MODULE__{scopes: scopes}, required_scope) do
    required_scope in scopes
  end

  @doc """
  Checks if the API key has all the specified scopes.
  """
  def has_all_scopes?(%__MODULE__{scopes: scopes}, required_scopes) do
    Enum.all?(required_scopes, &(&1 in scopes))
  end

  # Private helpers

  defp validate_scopes(changeset) do
    validate_change(changeset, :scopes, fn :scopes, scopes ->
      invalid_scopes = Enum.reject(scopes, &(&1 in @available_scopes))

      if Enum.empty?(invalid_scopes) do
        []
      else
        [{:scopes, "contains invalid scopes: #{Enum.join(invalid_scopes, ", ")}"}]
      end
    end)
  end

  defp validate_expires_at(changeset) do
    validate_change(changeset, :expires_at, fn :expires_at, expires_at ->
      if DateTime.compare(expires_at, DateTime.utc_now()) == :gt do
        []
      else
        [{:expires_at, "must be in the future"}]
      end
    end)
  end
end
