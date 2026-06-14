defmodule QueryServiceEx.RateLimiter do
  @moduledoc """
  Rate limiter using ETS-based token bucket algorithm.

  Provides per-tenant rate limiting to prevent any single tenant from
  overwhelming the system. Uses a token bucket algorithm that allows
  burst traffic while enforcing average rate limits.

  ## Token Bucket Algorithm

  Each tenant has a bucket with a maximum number of tokens. Tokens are
  consumed on each request and refilled at a steady rate. If the bucket
  is empty, requests are rejected until tokens are refilled.

  ## Configuration

  Rate limits are configured per subscription tier in config.exs:

      config :query_service_ex, QueryServiceEx.RateLimiter,
        default_rate: 100,           # requests per second
        default_burst: 200,          # max tokens (burst capacity)
        cleanup_interval: 60_000,    # cleanup old entries every 60s
        tier_limits: %{
          free: %{rate: 10, burst: 20},
          starter: %{rate: 50, burst: 100},
          pro: %{rate: 200, burst: 400},
          enterprise: %{rate: 1000, burst: 2000}
        }

  ## Usage

  The RateLimiter is typically used via the RateLimiting plug, but can
  also be called directly:

      case RateLimiter.check_rate(tenant_id, tier) do
        {:allow, remaining} -> proceed_with_request()
        {:deny, retry_after} -> reject_with_429(retry_after)
      end
  """

  use GenServer

  require Logger

  @table_name :rate_limiter_buckets
  @default_rate 100
  @default_burst 200
  @cleanup_interval 60_000

  # -------------------------------------------------------------------
  # Client API
  # -------------------------------------------------------------------

  @doc """
  Starts the RateLimiter GenServer.
  """
  def start_link(opts \\ []) do
    GenServer.start_link(__MODULE__, opts, name: __MODULE__)
  end

  @doc """
  Checks if a request should be allowed based on the tenant's rate limit.

  Returns:
  - `{:allow, remaining}` - Request allowed, with remaining tokens
  - `{:deny, retry_after_ms}` - Request denied, with retry delay in milliseconds
  """
  @spec check_rate(String.t() | nil, atom()) ::
          {:allow, non_neg_integer()} | {:deny, non_neg_integer()}
  def check_rate(tenant_id, tier \\ :free)

  def check_rate(nil, _tier), do: {:allow, 999}

  def check_rate(tenant_id, tier) do
    now = System.monotonic_time(:millisecond)
    {rate, burst} = get_tier_limits(tier)

    case :ets.lookup(@table_name, tenant_id) do
      [] ->
        # First request - initialize bucket with burst - 1 tokens
        :ets.insert(@table_name, {tenant_id, burst - 1, now})
        {:allow, burst - 1}

      [{^tenant_id, tokens, last_update}] ->
        # Calculate token refill based on elapsed time
        elapsed_ms = now - last_update
        refill = elapsed_ms * rate / 1000
        new_tokens = min(burst, tokens + refill)

        if new_tokens >= 1 do
          # Consume one token
          updated_tokens = new_tokens - 1
          :ets.insert(@table_name, {tenant_id, updated_tokens, now})
          {:allow, trunc(updated_tokens)}
        else
          # No tokens available - calculate retry time
          tokens_needed = 1 - new_tokens
          retry_after_ms = trunc(tokens_needed / rate * 1000) + 1
          {:deny, retry_after_ms}
        end
    end
  end

  @doc """
  Gets the current rate limit status for a tenant without consuming tokens.
  """
  @spec get_status(String.t(), atom()) :: %{
          tokens: float(),
          rate: non_neg_integer(),
          burst: non_neg_integer(),
          percentage_remaining: float()
        }
  def get_status(tenant_id, tier \\ :free) do
    now = System.monotonic_time(:millisecond)
    {rate, burst} = get_tier_limits(tier)

    tokens =
      case :ets.lookup(@table_name, tenant_id) do
        [] ->
          burst * 1.0

        [{^tenant_id, stored_tokens, last_update}] ->
          elapsed_ms = now - last_update
          refill = elapsed_ms * rate / 1000
          min(burst * 1.0, stored_tokens + refill)
      end

    %{
      tokens: tokens,
      rate: rate,
      burst: burst,
      percentage_remaining: tokens / burst * 100
    }
  end

  @doc """
  Resets the rate limit bucket for a tenant.
  """
  @spec reset(String.t()) :: :ok
  def reset(tenant_id) do
    :ets.delete(@table_name, tenant_id)
    :ok
  end

  @doc """
  Gets the rate limits for a subscription tier.
  """
  @spec get_tier_limits(atom()) :: {non_neg_integer(), non_neg_integer()}
  def get_tier_limits(tier) do
    tier_limits = get_config(:tier_limits, default_tier_limits())
    default_rate = get_config(:default_rate, @default_rate)
    default_burst = get_config(:default_burst, @default_burst)

    case Map.get(tier_limits, tier) do
      %{rate: rate, burst: burst} -> {rate, burst}
      _ -> {default_rate, default_burst}
    end
  end

  # -------------------------------------------------------------------
  # Server Callbacks
  # -------------------------------------------------------------------

  @impl true
  def init(_opts) do
    # Create ETS table for storing rate limit buckets
    :ets.new(@table_name, [
      :set,
      :public,
      :named_table,
      read_concurrency: true,
      write_concurrency: true
    ])

    # Schedule periodic cleanup of stale entries
    cleanup_interval = get_config(:cleanup_interval, @cleanup_interval)
    schedule_cleanup(cleanup_interval)

    Logger.info("[RateLimiter] Started with ETS table #{@table_name}")

    {:ok, %{cleanup_interval: cleanup_interval}}
  end

  @impl true
  def handle_info(:cleanup, state) do
    cleanup_stale_entries()
    schedule_cleanup(state.cleanup_interval)
    {:noreply, state}
  end

  # -------------------------------------------------------------------
  # Private Functions
  # -------------------------------------------------------------------

  defp schedule_cleanup(interval) do
    Process.send_after(self(), :cleanup, interval)
  end

  defp cleanup_stale_entries do
    now = System.monotonic_time(:millisecond)
    # Remove entries older than 5 minutes (no activity = bucket would be full anyway)
    stale_threshold = now - 300_000

    # Use match spec to find and delete stale entries
    match_spec = [{{:"$1", :_, :"$2"}, [{:<, :"$2", stale_threshold}], [:"$1"]}]

    stale_keys = :ets.select(@table_name, match_spec)

    Enum.each(stale_keys, fn key ->
      :ets.delete(@table_name, key)
    end)

    if stale_keys != [] do
      Logger.debug("[RateLimiter] Cleaned up #{length(stale_keys)} stale entries")
    end
  end

  defp get_config(key, default) do
    Application.get_env(:query_service_ex, __MODULE__, [])
    |> Keyword.get(key, default)
  end

  defp default_tier_limits do
    %{
      # Canonical 011 tiers (control-plane entities/subscription.go):
      # free → indie → studio → scale → enterprise.
      free: %{rate: 10, burst: 20},
      indie: %{rate: 100, burst: 200},
      studio: %{rate: 300, burst: 600},
      scale: %{rate: 1000, burst: 2000},
      enterprise: %{rate: 1000, burst: 2000},
      # Retired tier ids, normalized to canonical ids upstream. Kept here so a
      # not-yet-migrated tenant lands in a sensible bucket (and so these remain
      # existing atoms). Limits mirror their canonical successors.
      pro: %{rate: 300, burst: 600},
      growth: %{rate: 300, burst: 600},
      starter: %{rate: 100, burst: 200},
      developer: %{rate: 100, burst: 200},
      team: %{rate: 300, burst: 600}
    }
  end
end
