defmodule QueryServiceEx.ApiKeyCache do
  @moduledoc """
  ETS-based cache for verified API key lookups.

  Caches the result of `ApiKeys.verify_api_key/1` (which hashes and does a DB lookup)
  to avoid hitting PostgreSQL on every API-key-authenticated request.

  Entries are keyed by the SHA-256 hash of the raw key (not the raw key itself).
  Cache entries expire after a configurable TTL (default: 120 seconds).
  Key revocation is reflected within the TTL window.
  """

  use GenServer

  require Logger

  @table_name :api_key_cache
  @default_ttl 120
  @cleanup_interval 30_000

  ## Client API

  def start_link(opts \\ []) do
    GenServer.start_link(__MODULE__, opts, name: __MODULE__)
  end

  @doc """
  Fetch a verified API key result from cache, or compute and cache it.

  The cache key is the SHA-256 hash of the raw API key (same hash used for DB lookup).
  Only successful verifications are cached — errors are not cached.

  Returns `{:ok, {tenant, api_key}}` or `{:error, reason}`.
  """
  def fetch(raw_key, verify_fun) when is_function(verify_fun, 0) do
    cache_key = hash_key(raw_key)

    case get(cache_key) do
      {:ok, {_tenant, _api_key}} = cached ->
        cached

      :miss ->
        case verify_fun.() do
          {:ok, {tenant, api_key}} ->
            put(cache_key, {tenant, api_key})
            {:ok, {tenant, api_key}}

          error ->
            error
        end
    end
  end

  @doc """
  Invalidate all cached entries (e.g., when a key is revoked).
  """
  def clear do
    :ets.delete_all_objects(@table_name)
    :ok
  rescue
    ArgumentError -> :ok
  end

  ## Internal (used by tests)

  defp get(cache_key) do
    case :ets.lookup(@table_name, cache_key) do
      [{^cache_key, result, expires_at}] ->
        if System.system_time(:second) < expires_at do
          {:ok, result}
        else
          :ets.delete(@table_name, cache_key)
          :miss
        end

      [] ->
        :miss
    end
  rescue
    ArgumentError -> :miss
  end

  defp put(cache_key, result, ttl \\ @default_ttl) do
    expires_at = System.system_time(:second) + ttl
    :ets.insert(@table_name, {cache_key, result, expires_at})
    :ok
  rescue
    ArgumentError -> :ok
  end

  defp hash_key(raw_key) do
    :crypto.hash(:sha256, raw_key) |> Base.encode16(case: :lower)
  end

  ## Server Callbacks

  @impl true
  def init(opts) do
    table_name = Keyword.get(opts, :table_name, @table_name)
    cleanup_interval = Keyword.get(opts, :cleanup_interval, @cleanup_interval)

    :ets.new(table_name, [
      :set,
      :public,
      :named_table,
      read_concurrency: true,
      write_concurrency: true
    ])

    schedule_cleanup(cleanup_interval)

    {:ok, %{table_name: table_name, cleanup_interval: cleanup_interval}}
  end

  @impl true
  def handle_info(:cleanup, state) do
    cleanup_expired()
    schedule_cleanup(state.cleanup_interval)
    {:noreply, state}
  end

  @impl true
  def handle_info(_msg, state) do
    {:noreply, state}
  end

  defp schedule_cleanup(interval) do
    Process.send_after(self(), :cleanup, interval)
  end

  defp cleanup_expired do
    now = System.system_time(:second)

    :ets.select_delete(@table_name, [
      {{:_, :_, :"$1"}, [{:<, :"$1", now}], [true]}
    ])
  rescue
    ArgumentError -> :ok
  end
end
