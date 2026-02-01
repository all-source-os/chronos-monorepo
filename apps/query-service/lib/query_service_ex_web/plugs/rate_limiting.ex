defmodule QueryServiceExWeb.Plugs.RateLimiting do
  @moduledoc """
  Plug that enforces per-tenant rate limiting.

  This plug checks if the tenant has exceeded their rate limit before
  allowing the request to proceed. Uses a token bucket algorithm to
  allow burst traffic while enforcing average rate limits.

  ## Usage in Router

      pipeline :rate_limited do
        plug QueryServiceExWeb.Plugs.RateLimiting
      end

  ## Response Headers

  On successful requests, adds:
  - `x-ratelimit-limit` - Maximum requests per second
  - `x-ratelimit-remaining` - Remaining tokens in the bucket
  - `x-ratelimit-reset` - Time when bucket will be full (Unix timestamp)

  On rate-limited requests (429), adds:
  - `retry-after` - Seconds until request can be retried
  """

  import Plug.Conn

  alias QueryServiceEx.RateLimiter
  alias QueryServiceEx.Tenants.Tenant

  require Logger

  @doc """
  Initialize the plug.

  Options:
  - `:enabled` - Whether rate limiting is enabled (default: true)
  """
  def init(opts) do
    %{
      enabled: Keyword.get(opts, :enabled, true)
    }
  end

  def call(conn, %{enabled: false}), do: conn

  def call(conn, _opts) do
    tenant = conn.assigns[:current_tenant]

    if is_nil(tenant) do
      # No tenant context yet - allow request to proceed
      # Rate limiting will be enforced after authentication if needed
      conn
    else
      check_and_enforce_rate_limit(conn, tenant)
    end
  end

  defp check_and_enforce_rate_limit(conn, tenant) do
    tier = get_subscription_tier(tenant)

    case RateLimiter.check_rate(tenant.id, tier) do
      {:allow, remaining} ->
        add_rate_limit_headers(conn, tenant.id, tier, remaining)

      {:deny, retry_after_ms} ->
        send_rate_limited_response(conn, retry_after_ms, tier)
    end
  end

  defp get_subscription_tier(%Tenant{subscription_tier: tier}) when is_atom(tier), do: tier
  defp get_subscription_tier(_), do: :free

  defp add_rate_limit_headers(conn, _tenant_id, tier, remaining) do
    {rate, burst} = RateLimiter.get_tier_limits(tier)

    # Calculate reset time (when bucket would be full if no more requests)
    tokens_to_refill = burst - remaining
    seconds_to_full = if rate > 0, do: trunc(tokens_to_refill / rate), else: 0
    reset_time = System.system_time(:second) + seconds_to_full

    conn
    |> put_resp_header("x-ratelimit-limit", Integer.to_string(rate))
    |> put_resp_header("x-ratelimit-remaining", Integer.to_string(remaining))
    |> put_resp_header("x-ratelimit-reset", Integer.to_string(reset_time))
    |> assign(:rate_limit_remaining, remaining)
    |> assign(:rate_limit_tier, tier)
  end

  defp send_rate_limited_response(conn, retry_after_ms, tier) do
    {rate, _burst} = RateLimiter.get_tier_limits(tier)
    retry_after_seconds = max(1, div(retry_after_ms, 1000))

    Logger.info(
      "[RateLimiting] Rate limit exceeded for tenant, tier=#{tier}, retry_after=#{retry_after_seconds}s"
    )

    # Emit telemetry for monitoring
    :telemetry.execute(
      [:query_service_ex, :rate_limiter, :rejected],
      %{count: 1},
      %{tier: tier}
    )

    conn
    |> put_resp_header("retry-after", Integer.to_string(retry_after_seconds))
    |> put_resp_header("x-ratelimit-limit", Integer.to_string(rate))
    |> put_resp_header("x-ratelimit-remaining", "0")
    |> put_status(:too_many_requests)
    |> Phoenix.Controller.json(%{
      error: %{
        code: "rate_limit_exceeded",
        message: "Rate limit exceeded. Please slow down your requests.",
        retry_after_seconds: retry_after_seconds,
        tier: tier,
        limit: rate,
        upgrade_url: "/billing/upgrade"
      }
    })
    |> halt()
  end
end
