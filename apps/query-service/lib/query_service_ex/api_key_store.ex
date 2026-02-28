defmodule QueryServiceEx.ApiKeyStore do
  @moduledoc """
  ETS-backed store for API key metadata.

  API keys are JWT tokens signed with the shared JWT_SECRET.
  This module stores metadata (id, name, tenant_id, created_at)
  for listing and revocation. The actual key is a signed JWT
  that can be validated by any service sharing the secret.
  """

  use GenServer

  require Logger

  @table :api_key_store

  # -------------------------------------------------------------------
  # Client API
  # -------------------------------------------------------------------

  def start_link(opts \\ []) do
    GenServer.start_link(__MODULE__, opts, name: __MODULE__)
  end

  @doc "List all API keys for a tenant"
  def list_keys(tenant_id) do
    case :ets.lookup(@table, {:keys, tenant_id}) do
      [{_, keys}] -> Enum.filter(keys, &(&1["active"] != false))
      [] -> []
    end
  end

  @doc "Create an API key record"
  def create_key(tenant_id, key_metadata) do
    keys = list_all_keys(tenant_id)
    updated = [key_metadata | keys]
    :ets.insert(@table, {{:keys, tenant_id}, updated})
    {:ok, key_metadata}
  end

  @doc "Revoke an API key by ID"
  def revoke_key(tenant_id, key_id) do
    keys = list_all_keys(tenant_id)

    case Enum.find(keys, &(&1["id"] == key_id)) do
      nil ->
        {:error, :not_found}

      _key ->
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
    Logger.info("[ApiKeyStore] Started ETS table #{inspect(table)}")
    {:ok, %{}}
  end

  # -------------------------------------------------------------------
  # Private
  # -------------------------------------------------------------------

  defp list_all_keys(tenant_id) do
    case :ets.lookup(@table, {:keys, tenant_id}) do
      [{_, keys}] -> keys
      [] -> []
    end
  end
end
