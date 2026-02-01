defmodule QueryServiceExWeb.Plugs.RateLimitingTest do
  @moduledoc """
  Tests for the RateLimiting plug.

  Verifies that per-tenant rate limiting is correctly enforced
  and appropriate headers are returned.
  """
  use QueryServiceEx.DataCase

  import Plug.Conn
  import Plug.Test

  alias QueryServiceEx.RateLimiter
  alias QueryServiceEx.Tenants
  alias QueryServiceExWeb.Plugs.RateLimiting

  @moduletag :database

  setup do
    {:ok, tenant} =
      Tenants.create_tenant(%{
        name: "Rate Limit Test",
        slug: "rate-limit-test-#{System.unique_integer([:positive])}"
      })

    on_exit(fn -> RateLimiter.reset(tenant.id) end)

    {:ok, tenant: tenant}
  end

  describe "init/1" do
    test "defaults to enabled" do
      opts = RateLimiting.init([])
      assert opts == %{enabled: true}
    end

    test "can be disabled" do
      opts = RateLimiting.init(enabled: false)
      assert opts == %{enabled: false}
    end
  end

  describe "call/2 when disabled" do
    test "passes through without checking rate limit", %{tenant: tenant} do
      conn =
        conn(:get, "/api/events")
        |> assign(:current_tenant, tenant)

      opts = RateLimiting.init(enabled: false)
      result = RateLimiting.call(conn, opts)

      refute result.halted
      refute Map.has_key?(result.assigns, :rate_limit_remaining)
    end
  end

  describe "call/2 with no tenant context" do
    test "passes through when no tenant is assigned" do
      conn =
        conn(:get, "/api/events")
        |> assign(:current_tenant, nil)

      opts = RateLimiting.init([])
      result = RateLimiting.call(conn, opts)

      refute result.halted
    end
  end

  describe "call/2 with tenant context" do
    test "allows request when under rate limit", %{tenant: tenant} do
      conn =
        conn(:get, "/api/events")
        |> assign(:current_tenant, tenant)

      opts = RateLimiting.init([])
      result = RateLimiting.call(conn, opts)

      refute result.halted
      assert result.assigns[:rate_limit_remaining] != nil
    end

    test "adds rate limit headers on success", %{tenant: tenant} do
      conn =
        conn(:get, "/api/events")
        |> assign(:current_tenant, tenant)

      opts = RateLimiting.init([])
      result = RateLimiting.call(conn, opts)

      assert get_resp_header(result, "x-ratelimit-limit") != []
      assert get_resp_header(result, "x-ratelimit-remaining") != []
      assert get_resp_header(result, "x-ratelimit-reset") != []
    end

    test "blocks request when rate limit exceeded", %{tenant: tenant} do
      {_rate, burst} = RateLimiter.get_tier_limits(:free)

      # Exhaust all tokens directly via RateLimiter
      for _ <- 1..burst do
        RateLimiter.check_rate(tenant.id, :free)
      end

      conn =
        conn(:get, "/api/events")
        |> assign(:current_tenant, tenant)
        |> put_private(:phoenix_format, "json")

      opts = RateLimiting.init([])
      result = RateLimiting.call(conn, opts)

      assert result.halted
      assert result.status == 429

      response = Jason.decode!(result.resp_body)
      assert response["error"]["code"] == "rate_limit_exceeded"
      assert response["error"]["retry_after_seconds"] > 0
    end

    test "includes retry-after header when rate limited", %{tenant: tenant} do
      {_rate, burst} = RateLimiter.get_tier_limits(:free)

      # Exhaust all tokens
      for _ <- 1..burst do
        RateLimiter.check_rate(tenant.id, :free)
      end

      conn =
        conn(:get, "/api/events")
        |> assign(:current_tenant, tenant)
        |> put_private(:phoenix_format, "json")

      opts = RateLimiting.init([])
      result = RateLimiting.call(conn, opts)

      assert get_resp_header(result, "retry-after") != []
      [retry_after] = get_resp_header(result, "retry-after")
      assert String.to_integer(retry_after) > 0
    end

    test "includes upgrade URL in rate limit error", %{tenant: tenant} do
      {_rate, burst} = RateLimiter.get_tier_limits(:free)

      # Exhaust all tokens
      for _ <- 1..burst do
        RateLimiter.check_rate(tenant.id, :free)
      end

      conn =
        conn(:get, "/api/events")
        |> assign(:current_tenant, tenant)
        |> put_private(:phoenix_format, "json")

      opts = RateLimiting.init([])
      result = RateLimiting.call(conn, opts)

      response = Jason.decode!(result.resp_body)
      assert response["error"]["upgrade_url"] == "/billing/upgrade"
    end
  end

  describe "tier-based rate limiting" do
    test "uses correct tier limits for starter", %{tenant: tenant} do
      {:ok, tenant} =
        Tenants.update_subscription(tenant, %{
          subscription_tier: :starter
        })

      conn =
        conn(:get, "/api/events")
        |> assign(:current_tenant, tenant)

      opts = RateLimiting.init([])
      result = RateLimiting.call(conn, opts)

      [limit] = get_resp_header(result, "x-ratelimit-limit")
      assert String.to_integer(limit) == 50
    end

    test "uses correct tier limits for pro", %{tenant: tenant} do
      {:ok, tenant} =
        Tenants.update_subscription(tenant, %{
          subscription_tier: :pro
        })

      conn =
        conn(:get, "/api/events")
        |> assign(:current_tenant, tenant)

      opts = RateLimiting.init([])
      result = RateLimiting.call(conn, opts)

      [limit] = get_resp_header(result, "x-ratelimit-limit")
      assert String.to_integer(limit) == 200
    end

    test "uses correct tier limits for enterprise", %{tenant: tenant} do
      {:ok, tenant} =
        Tenants.update_subscription(tenant, %{
          subscription_tier: :enterprise
        })

      conn =
        conn(:get, "/api/events")
        |> assign(:current_tenant, tenant)

      opts = RateLimiting.init([])
      result = RateLimiting.call(conn, opts)

      [limit] = get_resp_header(result, "x-ratelimit-limit")
      assert String.to_integer(limit) == 1000
    end
  end

  describe "rate limit status tracking" do
    test "sets rate_limit_remaining in assigns", %{tenant: tenant} do
      conn =
        conn(:get, "/api/events")
        |> assign(:current_tenant, tenant)

      opts = RateLimiting.init([])
      result = RateLimiting.call(conn, opts)

      assert is_integer(result.assigns[:rate_limit_remaining])
    end

    test "sets rate_limit_tier in assigns", %{tenant: tenant} do
      conn =
        conn(:get, "/api/events")
        |> assign(:current_tenant, tenant)

      opts = RateLimiting.init([])
      result = RateLimiting.call(conn, opts)

      assert result.assigns[:rate_limit_tier] == :free
    end
  end
end
