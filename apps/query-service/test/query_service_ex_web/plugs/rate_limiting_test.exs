defmodule QueryServiceExWeb.Plugs.RateLimitingTest do
  @moduledoc """
  Regression coverage for the per-tenant rate-limiting plug.

  Once API-key auth was fixed, the first authenticated data request from a tenant
  on the `studio` tier 500'd: the plug did `String.to_existing_atom("studio")`,
  but `:studio` (a post-relaunch tier id) had never been created as an atom, so
  it raised ArgumentError. The plug now resolves tiers safely and the limit table
  carries the canonical 011 tiers.
  """
  use ExUnit.Case, async: false

  import Plug.Conn
  import Phoenix.ConnTest

  alias QueryServiceEx.RateLimiter
  alias QueryServiceExWeb.Plugs.RateLimiting

  defp tenant_with_tier(tier) do
    %{"id" => "tenant-#{tier}", "metadata" => %{"subscription" => %{"tier" => tier}}}
  end

  defp run(tier) do
    build_conn()
    |> assign(:current_tenant, tenant_with_tier(tier))
    |> RateLimiting.call(RateLimiting.init([]))
  end

  describe "subscription tier resolution" do
    test "canonical 'studio' tier does not crash and resolves to :studio (the regression)" do
      conn = run("studio")

      refute conn.halted
      assert conn.status != 500
      assert conn.assigns.rate_limit_tier == :studio
      assert RateLimiter.get_tier_limits(:studio) == {300, 600}
    end

    test "an unknown tier degrades to :free instead of raising ArgumentError" do
      conn = run("moonshot-tier-that-does-not-exist")

      refute conn.halted
      assert conn.assigns.rate_limit_tier == :free
    end

    test "indie and scale resolve to their own buckets (not the generic default)" do
      assert RateLimiter.get_tier_limits(:indie) == {100, 200}
      assert RateLimiter.get_tier_limits(:scale) == {1000, 2000}
    end
  end
end
