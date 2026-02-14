defmodule QueryServiceEx.TenantCache do
  @moduledoc """
  ETS-based cache for tenant data to avoid hitting PostgreSQL on every request.

  The TenantContext plug uses this cache for fast tenant lookups. The Control Plane
  notifies the Query Service via `POST /internal/tenant-updated` when tenant state
  changes, which invalidates the relevant cache entry.

  ## TTL

  Cache entries expire after a configurable TTL (default: 300 seconds / 5 minutes).
  This is a safety net — in normal operation, cache invalidation is driven by the
  Control Plane webhook rather than TTL expiry.
  """

  use GenServer

  require Logger

  @table_name :tenant_cache
  @default_ttl 300
  @cleanup_interval 60_000

  ## Client API

  def start_link(opts \\ []) do
    GenServer.start_link(__MODULE__, opts, name: __MODULE__)
  end

  @doc """
  Fetch a tenant from cache, or compute and cache it.

  Returns the cached tenant if present and not expired.
  Otherwise calls `fun` to load the tenant, caches the result, and returns it.
  """
  def fetch(tenant_id, fun) when is_function(fun, 0) do
    case get(tenant_id) do
      {:ok, tenant} ->
        tenant

      :miss ->
        tenant = fun.()

        if tenant do
          put(tenant_id, tenant)
        end

        tenant
    end
  end

  @doc """
  Get a tenant from the cache.

  Returns `{:ok, tenant}` on cache hit, `:miss` on miss or expiry.
  """
  def get(tenant_id) do
    case :ets.lookup(@table_name, tenant_id) do
      [{^tenant_id, tenant, expires_at}] ->
        if System.system_time(:second) < expires_at do
          {:ok, tenant}
        else
          :ets.delete(@table_name, tenant_id)
          :miss
        end

      [] ->
        :miss
    end
  rescue
    ArgumentError -> :miss
  end

  @doc """
  Put a tenant in the cache.
  """
  def put(tenant_id, tenant, ttl \\ @default_ttl) do
    expires_at = System.system_time(:second) + ttl
    :ets.insert(@table_name, {tenant_id, tenant, expires_at})
    :ok
  rescue
    ArgumentError -> :ok
  end

  @doc """
  Invalidate (delete) a tenant's cache entry.

  Returns `:ok` regardless of whether the entry existed.
  """
  def invalidate(tenant_id) do
    :ets.delete(@table_name, tenant_id)
    :ok
  rescue
    ArgumentError -> :ok
  end

  @doc """
  Check if a tenant_id is currently cached.
  """
  def cached?(tenant_id) do
    case get(tenant_id) do
      {:ok, _} -> true
      :miss -> false
    end
  end

  @doc """
  Clear all cached tenants.
  """
  def clear do
    :ets.delete_all_objects(@table_name)
    :ok
  rescue
    ArgumentError -> :ok
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
