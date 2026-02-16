defmodule QueryServiceExWeb.Plugs.UsageEnforcementTest do
  use ExUnit.Case, async: true

  import Plug.Conn
  import Plug.Test

  alias QueryServiceExWeb.Plugs.UsageEnforcement

  # Core-format tenant builders

  defp core_tenant(id, opts) do
    events_quota = Keyword.get(opts, :events_quota, 10_000)
    queries_quota = Keyword.get(opts, :queries_quota, 5_000)
    events_used = Keyword.get(opts, :events_used, 0)
    queries_used = Keyword.get(opts, :queries_used, 0)
    overage_enabled = Keyword.get(opts, :overage_enabled, false)
    event_rate = Keyword.get(opts, :event_rate, nil)

    overage =
      %{"enabled" => overage_enabled}
      |> then(fn m -> if event_rate, do: Map.put(m, "event_rate", event_rate), else: m end)

    %{
      "id" => id,
      "name" => "Test Tenant",
      "status" => "active",
      "metadata" => %{
        "subscription" => %{"status" => "active", "tier" => "free"},
        "quotas" => %{
          "events_quota" => events_quota,
          "queries_quota" => queries_quota,
          "events_used" => events_used,
          "queries_used" => queries_used
        },
        "overage" => overage
      }
    }
  end

  describe "init/1" do
    test "accepts :events type" do
      assert UsageEnforcement.init(type: :events) == %{type: :events}
    end

    test "accepts :queries type" do
      assert UsageEnforcement.init(type: :queries) == %{type: :queries}
    end

    test "defaults to :events type" do
      assert UsageEnforcement.init([]) == %{type: :events}
    end

    test "raises on invalid type" do
      assert_raise ArgumentError, ~r/must be :events or :queries/, fn ->
        UsageEnforcement.init(type: :invalid)
      end
    end
  end

  describe "call/2 with no tenant context" do
    test "passes through when no tenant is assigned" do
      conn =
        conn(:get, "/api/events")
        |> assign(:current_tenant, nil)

      opts = UsageEnforcement.init(type: :events)
      result = UsageEnforcement.call(conn, opts)

      refute result.halted
    end
  end

  describe "call/2 with events quota - hard limit mode" do
    test "allows request when under quota" do
      tenant = core_tenant("t-under", events_used: 5_000, events_quota: 10_000)

      conn =
        conn(:post, "/api/events")
        |> assign(:current_tenant, tenant)

      opts = UsageEnforcement.init(type: :events)
      result = UsageEnforcement.call(conn, opts)

      refute result.halted
    end

    test "blocks request when quota exceeded" do
      tenant = core_tenant("t-over", events_used: 10_000, events_quota: 10_000)

      conn =
        conn(:post, "/api/events")
        |> assign(:current_tenant, tenant)
        |> put_private(:phoenix_format, "json")

      opts = UsageEnforcement.init(type: :events)
      result = UsageEnforcement.call(conn, opts)

      assert result.halted
      assert result.status == 402

      response = Jason.decode!(result.resp_body)
      assert response["error"]["code"] == "quota_exceeded"
      assert response["error"]["usage_type"] == "events"
      assert response["error"]["used"] == 10_000
      assert response["error"]["quota"] == 10_000
    end

    test "blocks request when over quota (not just at quota)" do
      tenant = core_tenant("t-way-over", events_used: 15_000, events_quota: 10_000)

      conn =
        conn(:post, "/api/events")
        |> assign(:current_tenant, tenant)
        |> put_private(:phoenix_format, "json")

      opts = UsageEnforcement.init(type: :events)
      result = UsageEnforcement.call(conn, opts)

      assert result.halted
      assert result.status == 402
    end

    test "includes upgrade URL in error response" do
      tenant = core_tenant("t-upgrade", events_used: 10_000, events_quota: 10_000)

      conn =
        conn(:post, "/api/events")
        |> assign(:current_tenant, tenant)
        |> put_private(:phoenix_format, "json")

      opts = UsageEnforcement.init(type: :events)
      result = UsageEnforcement.call(conn, opts)

      response = Jason.decode!(result.resp_body)
      assert response["error"]["upgrade_url"] == "/billing/upgrade"
      assert response["error"]["enable_overage_url"] == "/billing/enable-overage"
    end
  end

  describe "call/2 with queries quota - hard limit mode" do
    test "allows request when under quota" do
      tenant = core_tenant("t-q-under", queries_used: 500, queries_quota: 5_000)

      conn =
        conn(:post, "/api/query")
        |> assign(:current_tenant, tenant)

      opts = UsageEnforcement.init(type: :queries)
      result = UsageEnforcement.call(conn, opts)

      refute result.halted
    end

    test "blocks request when quota exceeded" do
      tenant = core_tenant("t-q-over", queries_used: 5_000, queries_quota: 5_000)

      conn =
        conn(:post, "/api/query")
        |> assign(:current_tenant, tenant)
        |> put_private(:phoenix_format, "json")

      opts = UsageEnforcement.init(type: :queries)
      result = UsageEnforcement.call(conn, opts)

      assert result.halted
      assert result.status == 402

      response = Jason.decode!(result.resp_body)
      assert response["error"]["code"] == "quota_exceeded"
      assert response["error"]["usage_type"] == "queries"
    end
  end

  describe "call/2 with overage billing enabled - soft limit mode" do
    test "allows request with overage headers when quota exceeded" do
      tenant =
        core_tenant("t-overage",
          events_used: 12_000,
          events_quota: 10_000,
          overage_enabled: true
        )

      conn =
        conn(:post, "/api/events")
        |> assign(:current_tenant, tenant)

      opts = UsageEnforcement.init(type: :events)
      result = UsageEnforcement.call(conn, opts)

      refute result.halted

      assert get_resp_header(result, "x-usage-overage") == ["true"]
      assert get_resp_header(result, "x-usage-type") == ["events"]
      assert get_resp_header(result, "x-usage-used") == ["12000"]
      assert get_resp_header(result, "x-usage-quota") == ["10000"]
      assert get_resp_header(result, "x-overage-units") == ["2000"]

      assert result.assigns[:in_overage] == true
      assert result.assigns[:overage_type] == :events
    end

    test "does not add overage headers when under quota even if overage enabled" do
      tenant =
        core_tenant("t-overage-under",
          events_used: 5_000,
          events_quota: 10_000,
          overage_enabled: true
        )

      conn =
        conn(:post, "/api/events")
        |> assign(:current_tenant, tenant)

      opts = UsageEnforcement.init(type: :events)
      result = UsageEnforcement.call(conn, opts)

      refute result.halted
      assert get_resp_header(result, "x-usage-overage") == []
    end
  end

  describe "unlimited quota (enterprise tier)" do
    test "allows requests without quota check" do
      tenant =
        core_tenant("t-enterprise", events_quota: -1, queries_quota: -1, events_used: 10_000_000)

      conn =
        conn(:post, "/api/events")
        |> assign(:current_tenant, tenant)

      opts = UsageEnforcement.init(type: :events)
      result = UsageEnforcement.call(conn, opts)

      refute result.halted
    end
  end

  describe "tenant with no metadata" do
    test "treats missing quotas as 0 (allows through since 0 used >= 0 quota triggers block)" do
      # Tenant with no metadata at all — quotas default to 0, used defaults to 0
      # 0 >= 0 is true, so quota_exceeded? returns true, and with no overage → 402
      tenant = %{"id" => "t-no-meta", "name" => "Empty", "status" => "active"}

      conn =
        conn(:post, "/api/events")
        |> assign(:current_tenant, tenant)
        |> put_private(:phoenix_format, "json")

      opts = UsageEnforcement.init(type: :events)
      result = UsageEnforcement.call(conn, opts)

      assert result.halted
      assert result.status == 402
    end

    test "tenant with metadata but no quotas key" do
      tenant = %{
        "id" => "t-no-quotas",
        "name" => "No Quotas",
        "status" => "active",
        "metadata" => %{"subscription" => %{"status" => "active"}}
      }

      conn =
        conn(:post, "/api/events")
        |> assign(:current_tenant, tenant)
        |> put_private(:phoenix_format, "json")

      opts = UsageEnforcement.init(type: :events)
      result = UsageEnforcement.call(conn, opts)

      # No quota info means 0/0 → blocked
      assert result.halted
      assert result.status == 402
    end
  end
end
