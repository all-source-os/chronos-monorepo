defmodule QueryServiceEx.AuditLog do
  @moduledoc """
  Records audit log events for customer-facing tenant actions.

  Audit entries are stored as events in Core's event store with:
  - entity_id: "audit:{tenant_id}"
  - event_type: "audit.{action}"

  Logged actions:
  - api_key.created / api_key.revoked
  - member.invited / member.removed
  - plan.changed
  - webhook.created / webhook.deleted
  """

  alias QueryServiceEx.Infrastructure.Adapters.RustCoreClient

  require Logger

  @audit_entity_prefix "audit:"

  @valid_actions ~w(
    api_key.created api_key.revoked
    member.invited member.removed
    plan.changed
    webhook.created webhook.deleted
  )

  @doc """
  Record an audit log entry.

  ## Parameters
    * `tenant_id` - The tenant this action belongs to
    * `action` - One of the valid audit actions (e.g. "api_key.created")
    * `actor_email` - Email of the user who performed the action
    * `details` - Map with action-specific details
  """
  def record(tenant_id, action, actor_email, details \\ %{})
      when is_binary(tenant_id) and action in @valid_actions do
    event = %{
      entity_id: "#{@audit_entity_prefix}#{tenant_id}",
      event_type: "audit.#{action}",
      payload: %{
        actor: actor_email,
        action: action,
        details: details,
        recorded_at: DateTime.utc_now() |> DateTime.to_iso8601()
      }
    }

    case RustCoreClient.create_event(tenant_id, event) do
      {:ok, _} ->
        :ok

      {:error, reason} ->
        Logger.warning(
          "[AuditLog] Failed to record #{action} for tenant #{tenant_id}: " <>
            inspect(reason)
        )

        {:error, reason}
    end
  end

  @doc """
  Query audit log entries for a tenant.

  ## Options
    * `:action` - Filter by action type (e.g. "api_key.created")
    * `:limit` - Max entries to return (default 50)
    * `:offset` - Offset for pagination (default 0)
  """
  def list(tenant_id, opts \\ []) when is_binary(tenant_id) do
    entity_id = "#{@audit_entity_prefix}#{tenant_id}"
    limit = Keyword.get(opts, :limit, 50)
    offset = Keyword.get(opts, :offset, 0)

    query =
      case Keyword.get(opts, :action) do
        nil ->
          %{entity_id: entity_id, limit: limit, offset: offset}

        action ->
          %{
            entity_id: entity_id,
            event_type: "audit.#{action}",
            limit: limit,
            offset: offset
          }
      end

    case RustCoreClient.query_events(tenant_id, query) do
      {:ok, events} when is_list(events) ->
        entries = Enum.map(events, &format_entry/1)
        {:ok, entries}

      {:ok, %{"events" => events}} when is_list(events) ->
        entries = Enum.map(events, &format_entry/1)
        {:ok, entries}

      {:error, reason} ->
        {:error, reason}
    end
  end

  @doc "Returns the list of valid audit action types."
  def valid_actions, do: @valid_actions

  defp format_entry(event) do
    payload = event["payload"] || %{}

    %{
      "id" => event["id"],
      "timestamp" => event["timestamp"] || payload["recorded_at"],
      "actor" => payload["actor"],
      "action" => payload["action"],
      "details" => payload["details"] || %{}
    }
  end
end
