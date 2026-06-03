defmodule QueryServiceEx.ApiKeyStore do
  @moduledoc """
  Durable store for API key metadata, event-sourced through Core.

  API keys are JWT tokens signed with the shared JWT_SECRET. This module stores
  **metadata** (id, name, tenant_id, scopes, created_at, active) so the dashboard
  can list and revoke keys. The actual secret is the signed JWT and is never
  stored.

  ## Why event-sourced

  This used to be ETS-only, which meant every Query Service restart/redeploy
  wiped all key metadata — the API Keys page showed "No API keys yet" even though
  the keys (and their `api_key.created` audit events) still existed. That violated
  the project rule that stateful features must survive restarts.

  Now each create/revoke appends a durable event to Core:
  - entity_id:   `apikey:{tenant_id}`
  - event_type:  `apikey.created` | `apikey.revoked`

  ETS (`@table`) is a per-tenant **read cache**, not the source of truth. On a
  cache miss it lazily rebuilds a tenant's key list by replaying that tenant's
  `apikey.*` events from Core, so a fresh process serves the correct list on the
  first request after a restart. AuditLog uses the same Core-backed pattern.
  """

  use GenServer

  alias QueryServiceEx.Infrastructure.Adapters.RustCoreClient

  require Logger

  @table :api_key_store
  @entity_prefix "apikey:"

  # -------------------------------------------------------------------
  # Client API
  # -------------------------------------------------------------------

  def start_link(opts \\ []) do
    GenServer.start_link(__MODULE__, opts, name: __MODULE__)
  end

  @doc "List all active API keys for a tenant (durable — rebuilds from Core on cache miss)."
  def list_keys(tenant_id) do
    tenant_id
    |> all_keys()
    |> Enum.filter(&(&1["active"] != false))
  end

  @doc """
  Create an API key record. Writes a durable `apikey.created` event to Core, then
  updates the ETS cache. If the durable write fails the cache is still updated so
  the key works in-process, but a warning is logged — the durable event is what
  survives a restart.
  """
  def create_key(tenant_id, key_metadata) do
    event = %{
      entity_id: "#{@entity_prefix}#{tenant_id}",
      event_type: "apikey.created",
      payload: %{key: key_metadata}
    }

    case RustCoreClient.create_event(tenant_id, event) do
      {:ok, _} ->
        :ok

      {:error, reason} ->
        Logger.warning(
          "[ApiKeyStore] durable create failed for tenant #{tenant_id}: #{inspect(reason)} " <>
            "(key will be lost on restart)"
        )
    end

    keys = all_keys(tenant_id)
    :ets.insert(@table, {{:keys, tenant_id}, [key_metadata | keys]})
    {:ok, key_metadata}
  end

  @doc """
  Revoke an API key by ID. Writes a durable `apikey.revoked` event and marks the
  key inactive in the cache. Returns `{:error, :not_found}` if the tenant has no
  such key.
  """
  def revoke_key(tenant_id, key_id) do
    keys = all_keys(tenant_id)

    case Enum.find(keys, &(&1["id"] == key_id)) do
      nil ->
        {:error, :not_found}

      _key ->
        event = %{
          entity_id: "#{@entity_prefix}#{tenant_id}",
          event_type: "apikey.revoked",
          payload: %{key_id: key_id}
        }

        case RustCoreClient.create_event(tenant_id, event) do
          {:ok, _} ->
            :ok

          {:error, reason} ->
            Logger.warning(
              "[ApiKeyStore] durable revoke failed for tenant #{tenant_id}: #{inspect(reason)}"
            )
        end

        updated =
          Enum.map(keys, fn k ->
            if k["id"] == key_id, do: Map.put(k, "active", false), else: k
          end)

        :ets.insert(@table, {{:keys, tenant_id}, updated})
        :ok
    end
  end

  # -------------------------------------------------------------------
  # GenServer Callbacks
  # -------------------------------------------------------------------

  @impl true
  def init(_opts) do
    table = :ets.new(@table, [:named_table, :public, :set, read_concurrency: true])
    Logger.info("[ApiKeyStore] Started ETS cache #{inspect(table)} (durable backing: Core)")
    {:ok, %{}}
  end

  # -------------------------------------------------------------------
  # Private
  # -------------------------------------------------------------------

  # Return the full key list for a tenant (including revoked), serving from the
  # ETS cache when present and rebuilding from Core on a miss.
  defp all_keys(tenant_id) do
    case :ets.lookup(@table, {:keys, tenant_id}) do
      [{_, keys}] ->
        keys

      [] ->
        keys = rebuild_from_core(tenant_id)
        :ets.insert(@table, {{:keys, tenant_id}, keys})
        keys
    end
  end

  # Replay this tenant's apikey.* events from Core into the current key list.
  # created -> prepend the key metadata; revoked -> flip its `active` to false.
  # Newest-first is fine; we fold oldest->newest so revokes land after creates.
  defp rebuild_from_core(tenant_id) do
    query = %{entity_id: "#{@entity_prefix}#{tenant_id}", limit: 1000}

    events =
      case RustCoreClient.query_events(tenant_id, query) do
        {:ok, list} when is_list(list) -> list
        {:ok, %{"events" => list}} when is_list(list) -> list
        {:error, reason} ->
          Logger.warning(
            "[ApiKeyStore] could not rebuild keys for tenant #{tenant_id}: #{inspect(reason)}"
          )

          []
      end

    by_id =
      events
      |> Enum.sort_by(&(&1["timestamp"] || &1["version"]))
      |> Enum.reduce(%{}, fn event, acc ->
        payload = event["payload"] || %{}

        case event["event_type"] do
          "apikey.created" ->
            case payload["key"] do
              %{"id" => id} = key -> Map.put(acc, id, key)
              _ -> acc
            end

          "apikey.revoked" ->
            id = payload["key_id"]

            case Map.get(acc, id) do
              nil -> acc
              key -> Map.put(acc, id, Map.put(key, "active", false))
            end

          _ ->
            acc
        end
      end)

    # Backfill: keys minted before this store was event-sourced exist only as
    # `audit.api_key.created` entries (entity `audit:{tenant}`). Reconstruct a
    # minimal record for any that aren't already covered by an apikey.* event so
    # they appear in the list and can be revoked. The secret/prefix can't be
    # recovered (it was never stored), which is fine — listing + revoke don't
    # need it.
    legacy =
      tenant_id
      |> legacy_keys_from_audit()
      |> Enum.reject(fn k -> Map.has_key?(by_id, k["id"]) end)
      |> Map.new(fn k -> {k["id"], k} end)

    by_id
    |> Map.merge(legacy)
    |> Map.values()
  end

  # Reconstruct key metadata from the durable audit log (api_key.created /
  # api_key.revoked) for keys that predate native apikey.* events.
  defp legacy_keys_from_audit(tenant_id) do
    query = %{entity_id: "audit:#{tenant_id}", limit: 1000}

    events =
      case RustCoreClient.query_events(tenant_id, query) do
        {:ok, list} when is_list(list) -> list
        {:ok, %{"events" => list}} when is_list(list) -> list
        _ -> []
      end

    events
    |> Enum.sort_by(&(&1["timestamp"] || &1["version"]))
    |> Enum.reduce(%{}, fn event, acc ->
      payload = event["payload"] || %{}
      details = payload["details"] || %{}
      id = details["key_id"]

      case payload["action"] || event["event_type"] do
        action when action in ["api_key.created", "audit.api_key.created"] and is_binary(id) ->
          Map.put(acc, id, %{
            "id" => id,
            "name" => details["name"] || "API Key",
            "description" => nil,
            "key_prefix" => nil,
            "scopes" => details["scopes"] || [],
            "tenant_id" => tenant_id,
            "created_at" => event["timestamp"] || payload["recorded_at"],
            "expires_at" => nil,
            "active" => true,
            "last_used" => nil
          })

        action when action in ["api_key.revoked", "audit.api_key.revoked"] and is_binary(id) ->
          case Map.get(acc, id) do
            nil -> acc
            key -> Map.put(acc, id, Map.put(key, "active", false))
          end

        _ ->
          acc
      end
    end)
    |> Map.values()
  end
end
