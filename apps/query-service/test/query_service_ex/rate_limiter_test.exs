defmodule QueryServiceEx.RateLimiterTest do
  @moduledoc """
  Tests for the RateLimiter GenServer.

  Verifies the token bucket algorithm correctly limits request rates
  and respects tier-based limits.
  """
  use ExUnit.Case, async: false

  alias QueryServiceEx.RateLimiter

  setup do
    # Clean up any existing entries for isolation
    tenant_id = "test-tenant-#{System.unique_integer([:positive])}"
    on_exit(fn -> RateLimiter.reset(tenant_id) end)
    {:ok, tenant_id: tenant_id}
  end

  describe "check_rate/2" do
    test "allows first request and returns burst - 1 tokens", %{tenant_id: tenant_id} do
      {_rate, burst} = RateLimiter.get_tier_limits(:free)

      assert {:allow, remaining} = RateLimiter.check_rate(tenant_id, :free)
      assert remaining == burst - 1
    end

    test "allows requests when tokens are available", %{tenant_id: tenant_id} do
      # First 5 requests should succeed
      for _ <- 1..5 do
        assert {:allow, _remaining} = RateLimiter.check_rate(tenant_id, :free)
      end
    end

    test "denies requests when bucket is empty", %{tenant_id: tenant_id} do
      {_rate, burst} = RateLimiter.get_tier_limits(:free)

      # Exhaust all tokens
      for _ <- 1..burst do
        RateLimiter.check_rate(tenant_id, :free)
      end

      # Next request should be denied
      assert {:deny, retry_after} = RateLimiter.check_rate(tenant_id, :free)
      assert retry_after > 0
    end

    test "returns nil tenant as allowed", _context do
      assert {:allow, 999} = RateLimiter.check_rate(nil, :free)
    end

    test "refills tokens over time", %{tenant_id: tenant_id} do
      {_rate, burst} = RateLimiter.get_tier_limits(:free)

      # Exhaust all tokens
      for _ <- 1..burst do
        RateLimiter.check_rate(tenant_id, :free)
      end

      # Wait for some refill with retries to handle CI timing variability
      # Rate is 10 tokens/second for free tier, so we need at least 100ms for 1 token
      result =
        Enum.reduce_while(1..10, {:deny, 0}, fn attempt, _acc ->
          :timer.sleep(100 * attempt)

          case RateLimiter.check_rate(tenant_id, :free) do
            {:allow, remaining} -> {:halt, {:allow, remaining}}
            other -> {:cont, other}
          end
        end)

      # Should now have some tokens again
      assert {:allow, _remaining} = result
    end
  end

  describe "get_tier_limits/1" do
    test "returns correct limits for free tier" do
      {rate, burst} = RateLimiter.get_tier_limits(:free)
      assert rate == 10
      assert burst == 20
    end

    test "returns correct limits for pro tier" do
      {rate, burst} = RateLimiter.get_tier_limits(:pro)
      assert rate == 200
      assert burst == 400
    end

    test "returns correct limits for growth tier" do
      {rate, burst} = RateLimiter.get_tier_limits(:growth)
      assert rate == 500
      assert burst == 1000
    end

    test "returns correct limits for enterprise tier" do
      {rate, burst} = RateLimiter.get_tier_limits(:enterprise)
      assert rate == 1000
      assert burst == 2000
    end

    test "returns default limits for unknown tier" do
      {rate, burst} = RateLimiter.get_tier_limits(:unknown)
      assert rate == 100
      assert burst == 200
    end
  end

  describe "get_status/2" do
    test "returns full bucket for new tenant", %{tenant_id: tenant_id} do
      {_rate, burst} = RateLimiter.get_tier_limits(:free)

      status = RateLimiter.get_status(tenant_id, :free)

      assert status.tokens == burst * 1.0
      assert status.burst == burst
      assert status.percentage_remaining == 100.0
    end

    test "returns current token count after requests", %{tenant_id: tenant_id} do
      {_rate, burst} = RateLimiter.get_tier_limits(:free)

      # Make 5 requests
      for _ <- 1..5 do
        RateLimiter.check_rate(tenant_id, :free)
      end

      status = RateLimiter.get_status(tenant_id, :free)

      # Should have burst - 5 tokens (accounting for small refill during test)
      assert status.tokens <= burst - 4
      assert status.tokens >= burst - 6
    end
  end

  describe "reset/1" do
    test "resets the bucket for a tenant", %{tenant_id: tenant_id} do
      {_rate, burst} = RateLimiter.get_tier_limits(:free)

      # Exhaust some tokens
      for _ <- 1..10 do
        RateLimiter.check_rate(tenant_id, :free)
      end

      # Reset
      assert :ok = RateLimiter.reset(tenant_id)

      # Should have full bucket again
      status = RateLimiter.get_status(tenant_id, :free)
      assert status.tokens == burst * 1.0
    end
  end

  describe "tier-based rate limiting" do
    test "higher tiers have more capacity" do
      free = RateLimiter.get_tier_limits(:free)
      pro = RateLimiter.get_tier_limits(:pro)
      growth = RateLimiter.get_tier_limits(:growth)
      enterprise = RateLimiter.get_tier_limits(:enterprise)

      {free_rate, free_burst} = free
      {pro_rate, pro_burst} = pro
      {growth_rate, growth_burst} = growth
      {enterprise_rate, enterprise_burst} = enterprise

      assert pro_rate > free_rate
      assert growth_rate > pro_rate
      assert enterprise_rate > growth_rate

      assert pro_burst > free_burst
      assert growth_burst > pro_burst
      assert enterprise_burst > growth_burst
    end
  end

  describe "concurrent access" do
    test "handles concurrent requests correctly", %{tenant_id: tenant_id} do
      {_rate, burst} = RateLimiter.get_tier_limits(:pro)

      # Launch many concurrent requests
      tasks =
        for _ <- 1..50 do
          Task.async(fn ->
            RateLimiter.check_rate(tenant_id, :pro)
          end)
        end

      results = Task.await_many(tasks)

      # Count allows and denies
      allows = Enum.count(results, fn {status, _} -> status == :allow end)
      denies = Enum.count(results, fn {status, _} -> status == :deny end)

      # Should have allowed up to burst requests
      assert allows <= burst
      # Total should equal 50
      assert allows + denies == 50
    end
  end
end
