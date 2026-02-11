defmodule QueryServiceEx.Infrastructure.Adapters.AnalyticsCache do
  @moduledoc """
  ETS-based cache for analytics results with configurable TTL.

  Provides:
  - Fast in-memory caching for expensive analytics queries
  - Automatic expiration based on TTL
  - Thread-safe concurrent access
  - Cache invalidation by key pattern

  ## Configuration

      config :query_service_ex, QueryServiceEx.Infrastructure.Adapters.AnalyticsCache,
        table_name: :analytics_cache,
        default_ttl: 300,  # 5 minutes
        cleanup_interval: 60_000  # 1 minute
  """

  use GenServer

  @table_name :analytics_cache
  @default_ttl 300
  @cleanup_interval 60_000

  ## Client API

  @doc """
  Start the cache server.
  """
  def start_link(opts \\ []) do
    GenServer.start_link(__MODULE__, opts, name: __MODULE__)
  end

  @doc """
  Fetch a value from cache or compute it.

  If the key exists and hasn't expired, returns the cached value.
  Otherwise, calls the function to compute the value, caches it, and returns it.

  ## Parameters
    * `key` - Cache key (any term)
    * `ttl` - Time-to-live in seconds
    * `fun` - Function to compute the value if not cached

  ## Returns
    The cached or computed value.

  ## Examples

      AnalyticsCache.fetch(:my_key, 300, fn -> expensive_computation() end)
  """
  def fetch(key, ttl \\ @default_ttl, fun) when is_function(fun, 0) do
    case get(key) do
      {:ok, value} ->
        value

      :miss ->
        value = fun.()
        put(key, value, ttl)
        value
    end
  end

  @doc """
  Get a value from the cache.

  ## Returns
    * `{:ok, value}` - Cache hit
    * `:miss` - Cache miss or expired
  """
  def get(key) do
    case :ets.lookup(@table_name, key) do
      [{^key, value, expires_at}] ->
        if System.system_time(:second) < expires_at do
          {:ok, value}
        else
          # Entry expired, delete it
          :ets.delete(@table_name, key)
          :miss
        end

      [] ->
        :miss
    end
  rescue
    ArgumentError -> :miss
  end

  @doc """
  Put a value in the cache with TTL.

  ## Parameters
    * `key` - Cache key
    * `value` - Value to cache
    * `ttl` - Time-to-live in seconds (default: 300)
  """
  def put(key, value, ttl \\ @default_ttl) do
    expires_at = System.system_time(:second) + ttl
    :ets.insert(@table_name, {key, value, expires_at})
    :ok
  rescue
    ArgumentError -> :ok
  end

  @doc """
  Delete a key from the cache.
  """
  def delete(key) do
    :ets.delete(@table_name, key)
    :ok
  rescue
    ArgumentError -> :ok
  end

  @doc """
  Delete all keys matching a pattern.

  Uses `:ets.match_delete/2` for efficient pattern-based deletion.

  ## Parameters
    * `pattern` - Match pattern (use `:_` for wildcards)

  ## Examples

      # Delete all keys starting with {:frequency, tenant_id}
      AnalyticsCache.delete_pattern({:frequency, "tenant-123", :_})
  """
  def delete_pattern(pattern) do
    :ets.match_delete(@table_name, {pattern, :_, :_})
    :ok
  rescue
    ArgumentError -> :ok
  end

  @doc """
  Clear all entries from the cache.
  """
  def clear do
    :ets.delete_all_objects(@table_name)
    :ok
  rescue
    ArgumentError -> :ok
  end

  @doc """
  Get cache statistics.

  ## Returns
    * `%{size: integer, memory: integer}` - Cache stats
  """
  def stats do
    %{
      size: :ets.info(@table_name, :size),
      memory: :ets.info(@table_name, :memory) * :erlang.system_info(:wordsize)
    }
  rescue
    ArgumentError -> %{size: 0, memory: 0}
  end

  @doc """
  Invalidate cache entries for a specific tenant.

  Useful when tenant data changes and cached analytics become stale.
  """
  def invalidate_tenant(tenant_id) do
    # Get all keys and filter by tenant_id
    # This is less efficient but more flexible than match_delete
    :ets.foldl(
      fn {key, _value, _expires}, acc ->
        case key do
          {_, ^tenant_id, _} -> [:ets.delete(@table_name, key) | acc]
          _ -> acc
        end
      end,
      [],
      @table_name
    )

    :ok
  rescue
    ArgumentError -> :ok
  end

  ## Server Callbacks

  @impl true
  def init(opts) do
    table_name = Keyword.get(opts, :table_name, @table_name)
    cleanup_interval = Keyword.get(opts, :cleanup_interval, @cleanup_interval)

    # Create ETS table
    :ets.new(table_name, [
      :set,
      :public,
      :named_table,
      read_concurrency: true,
      write_concurrency: true
    ])

    # Schedule periodic cleanup
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

  ## Private Functions

  defp schedule_cleanup(interval) do
    Process.send_after(self(), :cleanup, interval)
  end

  defp cleanup_expired do
    now = System.system_time(:second)

    # Use select_delete for efficient expired entry removal
    :ets.select_delete(@table_name, [
      {{:_, :_, :"$1"}, [{:<, :"$1", now}], [true]}
    ])
  rescue
    ArgumentError -> :ok
  end
end
