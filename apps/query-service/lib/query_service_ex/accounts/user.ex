defmodule QueryServiceEx.Accounts.User do
  @moduledoc """
  Schema for users authenticated via OAuth providers.

  Users belong to a tenant (workspace) and can authenticate via:
  - Google OAuth
  - GitHub OAuth
  """

  use Ecto.Schema
  import Ecto.Changeset

  @primary_key {:id, :binary_id, autogenerate: true}
  @foreign_key_type :binary_id

  @providers ~w(google github)

  schema "users" do
    field(:email, :string)
    field(:name, :string)
    field(:avatar_url, :string)
    field(:provider, :string, default: "google")

    # Google OAuth
    field(:google_id, :string)
    field(:google_token, :string)
    field(:google_refresh_token, :string)

    # GitHub OAuth
    field(:github_id, :string)
    field(:github_token, :string)
    field(:github_refresh_token, :string)

    # Tenant association
    belongs_to(:tenant, QueryServiceEx.Tenants.Tenant)

    timestamps(type: :utc_datetime)
  end

  @doc """
  Changeset for creating or updating a user from Google OAuth data.
  """
  def changeset(user, attrs) do
    user
    |> cast(attrs, [
      :email,
      :name,
      :avatar_url,
      :provider,
      :google_id,
      :google_token,
      :google_refresh_token,
      :github_id,
      :github_token,
      :github_refresh_token,
      :tenant_id
    ])
    |> validate_required([:email, :provider])
    |> validate_inclusion(:provider, @providers)
    |> validate_format(:email, ~r/^[^\s]+@[^\s]+$/, message: "must have the @ sign and no spaces")
    |> validate_oauth_provider()
    |> unique_constraint(:email)
    |> unique_constraint(:google_id)
    |> unique_constraint(:github_id)
    |> foreign_key_constraint(:tenant_id)
  end

  @doc """
  Changeset for updating tenant association.
  """
  def tenant_changeset(user, tenant_id) do
    user
    |> cast(%{tenant_id: tenant_id}, [:tenant_id])
    |> foreign_key_constraint(:tenant_id)
  end

  @doc """
  Returns the user's display name or email as fallback.
  """
  def display_name(%__MODULE__{name: name, email: email}) do
    name || email
  end

  @doc """
  Returns the OAuth provider used by this user.
  """
  def oauth_provider(%__MODULE__{provider: provider}), do: provider

  @doc """
  Returns the OAuth ID for the user's provider.
  """
  def oauth_id(%__MODULE__{provider: "google", google_id: id}), do: id
  def oauth_id(%__MODULE__{provider: "github", github_id: id}), do: id

  # Private helpers

  defp validate_oauth_provider(changeset) do
    provider = get_field(changeset, :provider)
    google_id = get_field(changeset, :google_id)
    github_id = get_field(changeset, :github_id)

    case provider do
      "google" when is_nil(google_id) or google_id == "" ->
        add_error(changeset, :google_id, "is required for Google authentication")

      "github" when is_nil(github_id) or github_id == "" ->
        add_error(changeset, :github_id, "is required for GitHub authentication")

      _ ->
        changeset
    end
  end
end
