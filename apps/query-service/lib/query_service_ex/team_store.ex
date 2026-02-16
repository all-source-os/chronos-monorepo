defmodule QueryServiceEx.TeamStore do
  @moduledoc """
  ETS-backed store for team membership data.

  Team membership is managed as events in Core's event store.
  This module maintains an in-memory projection of the current
  team state, rebuilt from events on startup and updated on mutations.

  Events stored in Core:
  - team.member_invited
  - team.member_removed
  - team.member_role_updated
  """

  use GenServer

  alias QueryServiceEx.Infrastructure.Adapters.RustCoreClient

  require Logger

  @table :team_store
  @team_entity_prefix "team:"

  # Seat limits per subscription tier
  @seat_limits %{
    "free" => 1,
    "starter" => 1,
    "growth" => 3,
    "enterprise" => 10
  }

  # -------------------------------------------------------------------
  # Client API
  # -------------------------------------------------------------------

  def start_link(opts \\ []) do
    GenServer.start_link(__MODULE__, opts, name: __MODULE__)
  end

  @doc "Get all members for a tenant"
  def list_members(tenant_id) do
    case :ets.lookup(@table, {:members, tenant_id}) do
      [{_, members}] -> members
      [] -> []
    end
  end

  @doc "Get pending invitations for a tenant"
  def list_invitations(tenant_id) do
    case :ets.lookup(@table, {:invitations, tenant_id}) do
      [{_, invitations}] -> invitations
      [] -> []
    end
  end

  @doc "Get seat limit for a subscription tier"
  def seat_limit(tier), do: Map.get(@seat_limits, tier, 1)

  @doc "Invite a new team member"
  def invite_member(tenant_id, email, role, invited_by) do
    GenServer.call(__MODULE__, {:invite, tenant_id, email, role, invited_by})
  end

  @doc "Remove a team member"
  def remove_member(tenant_id, member_id) do
    GenServer.call(__MODULE__, {:remove, tenant_id, member_id})
  end

  @doc "Update a team member's role"
  def update_role(tenant_id, member_id, role) do
    GenServer.call(__MODULE__, {:update_role, tenant_id, member_id, role})
  end

  @doc "Load team data for a tenant from Core events"
  def load_tenant(tenant_id) do
    GenServer.cast(__MODULE__, {:load_tenant, tenant_id})
  end

  # -------------------------------------------------------------------
  # GenServer Callbacks
  # -------------------------------------------------------------------

  @impl true
  def init(_opts) do
    :ets.new(@table, [:named_table, :set, :public, read_concurrency: true])
    {:ok, %{}}
  end

  @impl true
  def handle_call({:invite, tenant_id, email, role, invited_by}, _from, state) do
    invitation_id = generate_uuid()
    now = DateTime.utc_now() |> DateTime.to_iso8601()

    event = %{
      entity_id: "#{@team_entity_prefix}#{tenant_id}",
      event_type: "team.member_invited",
      payload: %{
        invitation_id: invitation_id,
        email: email,
        role: role,
        invited_by: invited_by,
        invited_at: now
      }
    }

    case RustCoreClient.create_event(tenant_id, event) do
      {:ok, _} ->
        invitation = %{
          "id" => invitation_id,
          "email" => email,
          "role" => role,
          "invited_by" => invited_by,
          "invited_at" => now,
          "status" => "pending"
        }

        # Update ETS
        invitations = list_invitations(tenant_id)
        :ets.insert(@table, {{:invitations, tenant_id}, invitations ++ [invitation]})

        members = list_members(tenant_id)

        member = %{
          "id" => invitation_id,
          "user_id" => invitation_id,
          "email" => email,
          "name" => email,
          "role" => role,
          "joined_at" => now,
          "status" => "pending"
        }

        :ets.insert(@table, {{:members, tenant_id}, members ++ [member]})

        {:reply, {:ok, invitation}, state}

      {:error, reason} ->
        {:reply, {:error, reason}, state}
    end
  end

  @impl true
  def handle_call({:remove, tenant_id, member_id}, _from, state) do
    members = list_members(tenant_id)

    case Enum.find(members, fn m -> m["id"] == member_id || m["user_id"] == member_id end) do
      nil ->
        {:reply, {:error, :not_found}, state}

      member ->
        event = %{
          entity_id: "#{@team_entity_prefix}#{tenant_id}",
          event_type: "team.member_removed",
          payload: %{
            member_id: member_id,
            email: member["email"],
            removed_at: DateTime.utc_now() |> DateTime.to_iso8601()
          }
        }

        case RustCoreClient.create_event(tenant_id, event) do
          {:ok, _} ->
            updated =
              Enum.reject(members, fn m ->
                m["id"] == member_id || m["user_id"] == member_id
              end)

            :ets.insert(@table, {{:members, tenant_id}, updated})

            # Also remove from invitations
            invitations = list_invitations(tenant_id)
            updated_inv = Enum.reject(invitations, fn i -> i["id"] == member_id end)
            :ets.insert(@table, {{:invitations, tenant_id}, updated_inv})

            {:reply, :ok, state}

          {:error, reason} ->
            {:reply, {:error, reason}, state}
        end
    end
  end

  @impl true
  def handle_call({:update_role, tenant_id, member_id, role}, _from, state) do
    members = list_members(tenant_id)

    case Enum.find_index(members, fn m ->
           m["id"] == member_id || m["user_id"] == member_id
         end) do
      nil ->
        {:reply, {:error, :not_found}, state}

      index ->
        event = %{
          entity_id: "#{@team_entity_prefix}#{tenant_id}",
          event_type: "team.member_role_updated",
          payload: %{
            member_id: member_id,
            role: role,
            updated_at: DateTime.utc_now() |> DateTime.to_iso8601()
          }
        }

        case RustCoreClient.create_event(tenant_id, event) do
          {:ok, _} ->
            updated_member = Map.put(Enum.at(members, index), "role", role)
            updated_members = List.replace_at(members, index, updated_member)
            :ets.insert(@table, {{:members, tenant_id}, updated_members})
            {:reply, {:ok, updated_member}, state}

          {:error, reason} ->
            {:reply, {:error, reason}, state}
        end
    end
  end

  @impl true
  def handle_cast({:load_tenant, tenant_id}, state) do
    entity_id = "#{@team_entity_prefix}#{tenant_id}"

    case RustCoreClient.get_events_by_entity(tenant_id, entity_id) do
      {:ok, %{"events" => events}} ->
        rebuild_from_events(tenant_id, events)

      {:ok, events} when is_list(events) ->
        rebuild_from_events(tenant_id, events)

      {:error, reason} ->
        Logger.warning(
          "[TeamStore] Failed to load team events for #{tenant_id}: #{inspect(reason)}"
        )
    end

    {:noreply, state}
  end

  # -------------------------------------------------------------------
  # Private Helpers
  # -------------------------------------------------------------------

  defp generate_uuid do
    import Bitwise

    <<a::32, b::16, c::16, d::16, e::48>> = :crypto.strong_rand_bytes(16)

    :io_lib.format(
      "~8.16.0b-~4.16.0b-4~3.16.0b-~4.16.0b-~12.16.0b",
      [a, b, c &&& 0x0FFF, (d &&& 0x3FFF) ||| 0x8000, e]
    )
    |> IO.iodata_to_binary()
  end

  defp rebuild_from_events(tenant_id, events) do
    {members, invitations} =
      Enum.reduce(events, {[], []}, fn event, acc ->
        payload = event["payload"] || %{}
        apply_team_event(event["event_type"], payload, acc)
      end)

    :ets.insert(@table, {{:members, tenant_id}, members})
    :ets.insert(@table, {{:invitations, tenant_id}, invitations})
  end

  defp apply_team_event("team.member_invited", payload, {members_acc, inv_acc}) do
    member = %{
      "id" => payload["invitation_id"],
      "user_id" => payload["invitation_id"],
      "email" => payload["email"],
      "name" => payload["email"],
      "role" => payload["role"],
      "joined_at" => payload["invited_at"],
      "status" => "pending"
    }

    invitation = %{
      "id" => payload["invitation_id"],
      "email" => payload["email"],
      "role" => payload["role"],
      "invited_by" => payload["invited_by"],
      "invited_at" => payload["invited_at"],
      "status" => "pending"
    }

    {members_acc ++ [member], inv_acc ++ [invitation]}
  end

  defp apply_team_event("team.member_removed", payload, {members_acc, inv_acc}) do
    member_id = payload["member_id"]

    updated_members =
      Enum.reject(members_acc, fn m ->
        m["id"] == member_id || m["user_id"] == member_id
      end)

    updated_inv = Enum.reject(inv_acc, fn i -> i["id"] == member_id end)
    {updated_members, updated_inv}
  end

  defp apply_team_event("team.member_role_updated", payload, {members_acc, inv_acc}) do
    member_id = payload["member_id"]
    role = payload["role"]

    updated_members =
      Enum.map(members_acc, &update_member_role(&1, member_id, role))

    {updated_members, inv_acc}
  end

  defp apply_team_event(_event_type, _payload, acc), do: acc

  defp update_member_role(member, member_id, role) do
    if member["id"] == member_id || member["user_id"] == member_id do
      Map.put(member, "role", role)
    else
      member
    end
  end
end
