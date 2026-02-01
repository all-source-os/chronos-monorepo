defmodule QueryServiceEx.Accounts do
  @moduledoc """
  The Accounts context handles user authentication and management.

  Supports multiple OAuth providers:
  - Google OAuth
  - GitHub OAuth

  Auto-creates a tenant for new users on first sign-in.
  """

  import Ecto.Query
  alias QueryServiceEx.Accounts.User
  alias QueryServiceEx.Repo
  alias QueryServiceEx.Tenants
  alias QueryServiceEx.Tenants.Tenant

  require Logger

  # -------------------------------------------------------------------
  # User Retrieval
  # -------------------------------------------------------------------

  @doc """
  Gets a user by ID.

  Returns `nil` if the user does not exist.
  """
  def get_user(id) do
    Repo.get(User, id)
    |> maybe_preload_tenant()
  end

  @doc """
  Gets a user by ID.

  Raises `Ecto.NoResultsError` if the user does not exist.
  """
  def get_user!(id) do
    Repo.get!(User, id)
    |> Repo.preload(:tenant)
  end

  @doc """
  Gets a user by email.

  Returns `nil` if the user does not exist.
  """
  def get_user_by_email(email) do
    Repo.get_by(User, email: email)
    |> maybe_preload_tenant()
  end

  @doc """
  Gets a user by Google ID.

  Returns `nil` if the user does not exist.
  """
  def get_user_by_google_id(google_id) do
    Repo.get_by(User, google_id: google_id)
    |> maybe_preload_tenant()
  end

  @doc """
  Gets a user by GitHub ID.

  Returns `nil` if the user does not exist.
  """
  def get_user_by_github_id(github_id) do
    Repo.get_by(User, github_id: github_id)
    |> maybe_preload_tenant()
  end

  @doc """
  Gets all users for a tenant.
  """
  def list_users_by_tenant(tenant_id) do
    from(u in User, where: u.tenant_id == ^tenant_id)
    |> Repo.all()
  end

  # -------------------------------------------------------------------
  # OAuth Authentication
  # -------------------------------------------------------------------

  @doc """
  Creates or updates a user from Google OAuth data.

  If the user doesn't exist, creates a new tenant for them.
  If the user exists, updates their OAuth tokens.
  """
  def find_or_create_from_google(auth) do
    attrs = %{
      email: auth.info.email,
      name: auth.info.name,
      avatar_url: auth.info.image,
      google_id: auth.uid,
      google_token: get_credential(auth, :token),
      google_refresh_token: get_credential(auth, :refresh_token),
      provider: "google"
    }

    case get_user_by_google_id(auth.uid) do
      nil ->
        # Try to find by email (user might have signed up with GitHub first)
        case get_user_by_email(auth.info.email) do
          nil ->
            create_user_with_tenant(attrs)

          existing_user ->
            # Link Google account to existing user
            link_google_account(existing_user, attrs)
        end

      user ->
        update_user(user, attrs)
    end
  end

  @doc """
  Creates or updates a user from GitHub OAuth data.

  If the user doesn't exist, creates a new tenant for them.
  If the user exists, updates their OAuth tokens.
  """
  def find_or_create_from_github(auth) do
    attrs = %{
      email: get_github_email(auth),
      name: auth.info.name || auth.info.nickname,
      avatar_url: auth.info.image,
      github_id: to_string(auth.uid),
      github_token: get_credential(auth, :token),
      github_refresh_token: get_credential(auth, :refresh_token),
      provider: "github"
    }

    case get_user_by_github_id(to_string(auth.uid)) do
      nil ->
        # Try to find by email (user might have signed up with Google first)
        case get_user_by_email(attrs.email) do
          nil ->
            create_user_with_tenant(attrs)

          existing_user ->
            # Link GitHub account to existing user
            link_github_account(existing_user, attrs)
        end

      user ->
        update_user(user, attrs)
    end
  end

  # -------------------------------------------------------------------
  # User Management
  # -------------------------------------------------------------------

  @doc """
  Creates a new user.
  """
  def create_user(attrs) do
    %User{}
    |> User.changeset(attrs)
    |> Repo.insert()
  end

  @doc """
  Creates a new user and their tenant workspace.

  This is used for first-time sign-ups to auto-provision a workspace.
  """
  def create_user_with_tenant(attrs) do
    Repo.transaction(fn ->
      tenant_name = derive_tenant_name(attrs)
      tenant_slug = Tenants.generate_slug(tenant_name)

      with {:ok, tenant} <- Tenants.create_tenant(%{name: tenant_name, slug: tenant_slug}),
           {:ok, user} <- create_user(Map.put(attrs, :tenant_id, tenant.id)) do
        Repo.preload(user, :tenant)
      else
        {:error, changeset} -> Repo.rollback(changeset)
      end
    end)
  end

  @doc """
  Updates an existing user.
  """
  def update_user(%User{} = user, attrs) do
    user
    |> User.changeset(attrs)
    |> Repo.update()
    |> case do
      {:ok, user} -> {:ok, Repo.preload(user, :tenant)}
      error -> error
    end
  end

  @doc """
  Lists all users.
  """
  def list_users do
    Repo.all(User)
    |> Repo.preload(:tenant)
  end

  @doc """
  Deletes a user.
  """
  def delete_user(%User{} = user) do
    Repo.delete(user)
  end

  @doc """
  Associates a user with a tenant.
  """
  def assign_to_tenant(%User{} = user, %Tenant{} = tenant) do
    user
    |> User.tenant_changeset(tenant.id)
    |> Repo.update()
  end

  # -------------------------------------------------------------------
  # Account Linking
  # -------------------------------------------------------------------

  @doc """
  Links a Google account to an existing user.
  """
  def link_google_account(%User{} = user, attrs) do
    user
    |> User.changeset(%{
      google_id: attrs.google_id,
      google_token: attrs.google_token,
      google_refresh_token: attrs.google_refresh_token
    })
    |> Repo.update()
    |> case do
      {:ok, user} -> {:ok, Repo.preload(user, :tenant)}
      error -> error
    end
  end

  @doc """
  Links a GitHub account to an existing user.
  """
  def link_github_account(%User{} = user, attrs) do
    user
    |> User.changeset(%{
      github_id: attrs.github_id,
      github_token: attrs.github_token,
      github_refresh_token: attrs.github_refresh_token
    })
    |> Repo.update()
    |> case do
      {:ok, user} -> {:ok, Repo.preload(user, :tenant)}
      error -> error
    end
  end

  # -------------------------------------------------------------------
  # Private Helpers
  # -------------------------------------------------------------------

  defp get_credential(auth, key) do
    case auth.credentials do
      %{^key => value} when not is_nil(value) -> value
      _ -> nil
    end
  end

  defp get_github_email(auth) do
    # GitHub may have primary email in different locations
    cond do
      auth.info.email && auth.info.email != "" ->
        auth.info.email

      # Try to get from extra info
      auth.extra && auth.extra.raw_info && auth.extra.raw_info["email"] ->
        auth.extra.raw_info["email"]

      true ->
        # Fallback to GitHub username-based email
        "#{auth.info.nickname}@users.noreply.github.com"
    end
  end

  defp derive_tenant_name(attrs) do
    cond do
      attrs[:name] && attrs[:name] != "" ->
        "#{attrs[:name]}'s Workspace"

      attrs[:email] ->
        email_prefix = attrs[:email] |> String.split("@") |> List.first()
        "#{email_prefix}'s Workspace"

      true ->
        "My Workspace"
    end
  end

  defp maybe_preload_tenant(nil), do: nil
  defp maybe_preload_tenant(user), do: Repo.preload(user, :tenant)
end
