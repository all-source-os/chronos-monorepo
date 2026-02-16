defmodule QueryServiceExWeb.TeamController do
  @moduledoc """
  Controller for team member management.

  Provides endpoints for inviting, listing, removing, and updating
  team members. Team membership events are stored in Core's event store
  and projected in-memory via TeamStore (ETS).
  Seat limits are enforced per subscription tier.
  """

  use Phoenix.Controller, formats: [:json]

  alias QueryServiceEx.DevMode
  alias QueryServiceEx.Infrastructure.Adapters.RustCoreClient
  alias QueryServiceEx.TeamStore
  alias QueryServiceEx.TenantCache

  require Logger

  action_fallback(QueryServiceExWeb.FallbackController)

  @valid_roles ~w(admin member viewer)

  # -------------------------------------------------------------------
  # List Members
  # -------------------------------------------------------------------

  @doc """
  Lists all team members and pending invitations.

  GET /api/team/members
  """
  def index(conn, _params) do
    with {:ok, tenant} <- get_tenant(conn) do
      tenant_id = tenant["id"]
      members = TeamStore.list_members(tenant_id)
      tier = get_tier(tenant)
      seat_limit = TeamStore.seat_limit(tier)

      # Include the owner as the first member
      owner = build_owner(tenant)
      all_members = [owner | members]

      conn
      |> put_status(:ok)
      |> json(%{
        data: %{
          members: all_members,
          seat_limit: seat_limit,
          seats_used: length(all_members)
        }
      })
    end
  end

  # -------------------------------------------------------------------
  # Invite Member
  # -------------------------------------------------------------------

  @doc """
  Invites a new member to the team by email.

  POST /api/team/invite
  Body: {"email": "user@example.com", "role": "member"}
  """
  def invite(conn, params) do
    with {:ok, tenant} <- get_tenant(conn),
         {:ok, email} <- validate_email(params["email"]),
         {:ok, role} <- validate_role(params["role"]),
         :ok <- check_seat_limit(tenant),
         :ok <- check_not_duplicate(tenant, email) do
      user = conn.assigns[:current_user]
      invited_by = user["email"] || user[:email] || "unknown"
      tenant_id = tenant["id"]

      case TeamStore.invite_member(tenant_id, email, role, invited_by) do
        {:ok, invitation} ->
          conn
          |> put_status(:created)
          |> json(%{data: invitation})

        {:error, reason} ->
          conn
          |> put_status(:internal_server_error)
          |> json(%{
            error: %{code: "invite_failed", message: "Failed to send invite: #{inspect(reason)}"}
          })
      end
    end
  end

  # -------------------------------------------------------------------
  # Remove Member
  # -------------------------------------------------------------------

  @doc """
  Removes a member from the team.

  DELETE /api/team/members/:user_id
  """
  def remove(conn, %{"user_id" => user_id}) do
    with {:ok, tenant} <- get_tenant(conn),
         :ok <- check_not_self_remove(conn, user_id) do
      tenant_id = tenant["id"]

      case TeamStore.remove_member(tenant_id, user_id) do
        :ok ->
          conn
          |> put_status(:ok)
          |> json(%{data: %{message: "Member removed"}})

        {:error, :not_found} ->
          conn
          |> put_status(:not_found)
          |> json(%{error: %{code: "member_not_found", message: "Team member not found"}})

        {:error, reason} ->
          conn
          |> put_status(:internal_server_error)
          |> json(%{
            error: %{
              code: "remove_failed",
              message: "Failed to remove member: #{inspect(reason)}"
            }
          })
      end
    end
  end

  # -------------------------------------------------------------------
  # Update Member Role
  # -------------------------------------------------------------------

  @doc """
  Updates a team member's role.

  PUT /api/team/members/:user_id/role
  Body: {"role": "admin"}
  """
  def update_role(conn, %{"user_id" => user_id} = params) do
    with {:ok, tenant} <- get_tenant(conn),
         {:ok, role} <- validate_role(params["role"]) do
      tenant_id = tenant["id"]

      case TeamStore.update_role(tenant_id, user_id, role) do
        {:ok, member} ->
          conn
          |> put_status(:ok)
          |> json(%{data: member})

        {:error, :not_found} ->
          conn
          |> put_status(:not_found)
          |> json(%{error: %{code: "member_not_found", message: "Team member not found"}})

        {:error, reason} ->
          conn
          |> put_status(:internal_server_error)
          |> json(%{
            error: %{code: "update_failed", message: "Failed to update role: #{inspect(reason)}"}
          })
      end
    end
  end

  # -------------------------------------------------------------------
  # Private Helpers
  # -------------------------------------------------------------------

  defp get_tenant(conn) do
    if DevMode.auth_disabled?() do
      tenant = conn.assigns[:current_tenant] || DevMode.dev_tenant()
      {:ok, tenant}
    else
      user = conn.assigns[:current_user]
      tenant_id = user["tenant_id"] || user[:tenant_id]

      TenantCache.fetch(tenant_id, fn ->
        case RustCoreClient.get_tenant(tenant_id) do
          {:ok, tenant} -> tenant
          {:error, _} -> nil
        end
      end)
      |> case do
        nil -> {:error, :not_found}
        tenant -> {:ok, tenant}
      end
    end
  end

  defp get_tier(tenant) do
    metadata = tenant["metadata"] || %{}
    subscription = metadata["subscription"] || %{}
    subscription["tier"] || "free"
  end

  defp build_owner(tenant) do
    %{
      "id" => "owner",
      "user_id" => tenant["owner_id"] || tenant["id"],
      "email" => tenant["owner_email"] || "owner@#{tenant["slug"] || "workspace"}.com",
      "name" => tenant["owner_name"] || tenant["name"] || "Owner",
      "role" => "owner",
      "joined_at" => tenant["inserted_at"] || DateTime.utc_now() |> DateTime.to_iso8601(),
      "status" => "active"
    }
  end

  defp validate_email(nil) do
    {:error, :bad_request, "Email is required"}
  end

  defp validate_email(email) when is_binary(email) do
    if String.match?(email, ~r/^[^\s@]+@[^\s@]+\.[^\s@]+$/) do
      {:ok, String.downcase(email)}
    else
      {:error, :bad_request, "Invalid email address"}
    end
  end

  defp validate_email(_), do: {:error, :bad_request, "Invalid email address"}

  defp validate_role(nil) do
    {:error, :bad_request, "Role is required"}
  end

  defp validate_role(role) when role in @valid_roles, do: {:ok, role}

  defp validate_role(_) do
    {:error, :bad_request, "Role must be one of: admin, member, viewer"}
  end

  defp check_seat_limit(tenant) do
    tenant_id = tenant["id"]
    members = TeamStore.list_members(tenant_id)
    tier = get_tier(tenant)
    limit = TeamStore.seat_limit(tier)
    # +1 for the owner who is always counted
    current_count = length(members) + 1

    if current_count >= limit do
      {:error, :bad_request,
       "Seat limit reached (#{limit} seats for #{tier} plan). Upgrade to add more members."}
    else
      :ok
    end
  end

  defp check_not_duplicate(tenant, email) do
    tenant_id = tenant["id"]
    members = TeamStore.list_members(tenant_id)
    owner = build_owner(tenant)

    all = [owner | members]

    if Enum.any?(all, fn m -> m["email"] == email end) do
      {:error, :bad_request, "This email is already a team member"}
    else
      :ok
    end
  end

  defp check_not_self_remove(conn, user_id) do
    user = conn.assigns[:current_user]
    current_user_id = user["id"] || user[:id] || user["sub"]

    if current_user_id == user_id do
      {:error, :bad_request, "Cannot remove yourself from the team"}
    else
      :ok
    end
  end
end
