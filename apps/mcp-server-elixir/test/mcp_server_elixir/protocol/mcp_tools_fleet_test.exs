defmodule McpServerElixir.Protocol.McpToolsFleetTest do
  @moduledoc """
  P3 acceptance tests for the Admin fleet-health + guarded-recovery MCP tools.

  These prove the gating + dry-run pass-through behaviour, which is independently
  testable WITHOUT a live Control Plane:

  - the read tools are control_plane_enabled-gated;
  - the mutating recovery_* tools are system_admin-gated AND hidden from
    tools/list by default;
  - calling a recovery_* tool with system_admin false returns the
    "system-admin not enabled" isError (short-circuits before any network call);
  - the reprovision param builder passes dry_run + does not mutate (no
    confirmation) when confirm_tenant_id is omitted — and forwards confirmation
    when supplied.

  No tier scoring or guard logic lives in the MCP server; that all lives in the
  Control Plane. These tools are thin pass-throughs.
  """

  use ExUnit.Case, async: true

  alias McpServerElixir.Protocol.McpTools

  @moduletag :mcp_tools_fleet

  # tenant_overview (Phase 8) is a READ composition → control_plane-gated, same
  # tier as the health reads. tenant_notice (Phase 8) is a comms mutation →
  # system_admin-gated, same tier as the Destructive recovery tools.
  @read_tools ["fleet_health_summary", "tenant_health_assessment", "tenant_overview"]
  @recovery_tools [
    "recovery_resync",
    "recovery_reconcile_subscription",
    "recovery_resolve_dunning",
    "recovery_rotate_keys",
    "recovery_reprovision",
    "recovery_restore",
    "recovery_batch",
    "recovery_diagnose_edition",
    "tenant_notice"
  ]

  # ==========================================================================
  # tools/list gating (P3 acceptance: read shown, recovery_* omitted by default)
  # ==========================================================================

  describe "list_tools/1 gating" do
    test "control plane disabled hides every fleet tool" do
      names =
        McpTools.list_tools(%{control_plane_enabled: false})
        |> Enum.map(& &1.name)

      for tool <- @read_tools ++ @recovery_tools do
        refute tool in names, "#{tool} should be hidden when control plane is disabled"
      end
    end

    test "control plane enabled but system_admin unset shows read tools, OMITS every recovery_*" do
      names =
        McpTools.list_tools(%{control_plane_enabled: true})
        |> Enum.map(& &1.name)

      # read tools present
      for tool <- @read_tools do
        assert tool in names, "#{tool} should be listed when control plane is enabled"
      end

      # every mutating recovery tool hidden by default
      for tool <- @recovery_tools do
        refute tool in names, "#{tool} must be hidden unless ALLSOURCE_SYSTEM_ADMIN is set"
      end
    end

    test "system_admin true also exposes every recovery_* tool" do
      names =
        McpTools.list_tools(%{control_plane_enabled: true, system_admin: true})
        |> Enum.map(& &1.name)

      for tool <- @read_tools ++ @recovery_tools do
        assert tool in names, "#{tool} should be listed when system-admin mode is on"
      end
    end

    test "system_admin true WITHOUT control plane still hides recovery_* (read gate is the floor)" do
      names =
        McpTools.list_tools(%{control_plane_enabled: false, system_admin: true})
        |> Enum.map(& &1.name)

      for tool <- @read_tools ++ @recovery_tools do
        refute tool in names, "#{tool} requires control_plane_enabled before anything is shown"
      end
    end
  end

  # ==========================================================================
  # Tool definitions — shape + loud ADMIN/SAFETY blocks + dry-run note
  # ==========================================================================

  describe "read tool definitions" do
    setup do
      tools = McpTools.list_tools(%{control_plane_enabled: true})
      {:ok, tools: tools}
    end

    test "fleet_health_summary is read-only, ADMIN ONLY, no destructive params", %{tools: tools} do
      tool = Enum.find(tools, &(&1.name == "fleet_health_summary"))
      assert tool.description =~ "ADMIN ONLY"
      assert tool.inputSchema.required == []
      refute Map.has_key?(tool.inputSchema.properties, "dry_run")
      refute Map.has_key?(tool.inputSchema.properties, "confirm_tenant_id")
    end

    test "tenant_health_assessment requires tenant_id and is read-only", %{tools: tools} do
      tool = Enum.find(tools, &(&1.name == "tenant_health_assessment"))
      assert tool.description =~ "ADMIN ONLY"
      assert "tenant_id" in tool.inputSchema.required
      refute Map.has_key?(tool.inputSchema.properties, "dry_run")
    end
  end

  describe "recovery tool definitions" do
    setup do
      tools = McpTools.list_tools(%{control_plane_enabled: true, system_admin: true})
      {:ok, tools: tools}
    end

    test "every recovery_* tool carries ADMIN ONLY + a dry-run-by-default note", %{tools: tools} do
      for name <- @recovery_tools do
        tool = Enum.find(tools, &(&1.name == name))
        assert tool, "#{name} should be defined"
        assert tool.description =~ "ADMIN ONLY", "#{name} missing ADMIN ONLY block"

        # diagnose_edition is read-only; the rest mutate and must carry SAFETY + dry-run note
        unless name == "recovery_diagnose_edition" do
          assert tool.description =~ "SAFETY WARNING", "#{name} missing SAFETY WARNING block"

          assert tool.description =~ "dry-run" or tool.description =~ "Dry-run",
                 "#{name} missing dry-run note"
        end
      end
    end

    test "recovery_reprovision is DESTRUCTIVE: dry_run + typed confirm_tenant_id", %{tools: tools} do
      tool = Enum.find(tools, &(&1.name == "recovery_reprovision"))
      props = tool.inputSchema.properties

      assert Map.has_key?(props, "dry_run")
      assert props["dry_run"].type == "boolean"
      assert Map.has_key?(props, "confirm_tenant_id")
      assert props["confirm_tenant_id"].description =~ "must exactly equal tenant_id"
      assert "tenant_id" in tool.inputSchema.required
      assert "reason" in tool.inputSchema.required
    end

    test "recovery_batch enforces the Guarded-only action enum + max_tenants cap note", %{
      tools: tools
    } do
      tool = Enum.find(tools, &(&1.name == "recovery_batch"))
      props = tool.inputSchema.properties

      assert props["action"].enum == ["resync", "reconcile_subscription", "resolve_dunning"]
      assert Map.has_key?(props, "max_tenants")
      assert props["max_tenants"].description =~ "100"
      assert Map.has_key?(props, "confirm_token")
      assert "filter" in tool.inputSchema.required
      assert "action" in tool.inputSchema.required
    end
  end

  # ==========================================================================
  # Phase 8 tool definitions — tenant_overview (read) + tenant_notice (mutating)
  # ==========================================================================

  describe "tenant_overview definition (Phase 8 read)" do
    setup do
      tools = McpTools.list_tools(%{control_plane_enabled: true})
      {:ok, tools: tools}
    end

    test "is ADMIN ONLY, requires tenant_id, and carries NO mutating params", %{tools: tools} do
      tool = Enum.find(tools, &(&1.name == "tenant_overview"))
      assert tool, "tenant_overview should be listed when control plane is enabled"
      assert tool.description =~ "ADMIN ONLY"
      assert "tenant_id" in tool.inputSchema.required
      # pure read composition — no dry_run / confirm_token / confirm_tenant_id
      refute Map.has_key?(tool.inputSchema.properties, "dry_run")
      refute Map.has_key?(tool.inputSchema.properties, "confirm_token")
      refute Map.has_key?(tool.inputSchema.properties, "confirm_tenant_id")
    end

    test "tenant_overview is HIDDEN when control plane is disabled" do
      names =
        McpTools.list_tools(%{control_plane_enabled: false})
        |> Enum.map(& &1.name)

      refute "tenant_overview" in names
    end
  end

  describe "tenant_notice definition (Phase 8 mutating)" do
    setup do
      tools = McpTools.list_tools(%{control_plane_enabled: true, system_admin: true})
      {:ok, tools: tools}
    end

    test "is ADMIN ONLY, carries the dry-run + confirm_token guard params + audience/severity", %{
      tools: tools
    } do
      tool = Enum.find(tools, &(&1.name == "tenant_notice"))
      assert tool, "tenant_notice should be listed in system-admin mode"
      assert tool.description =~ "ADMIN ONLY"
      assert tool.description =~ "SAFETY WARNING"
      assert tool.description =~ "dry-run" or tool.description =~ "DRY-RUN"

      props = tool.inputSchema.properties
      assert props["dry_run"].type == "boolean"
      assert Map.has_key?(props, "confirm_token")
      assert props["severity"].enum == ["info", "warning", "critical"]

      # audience must allow single tenant_id AND the cohort selectors
      audience_props = props["audience"].properties
      assert Map.has_key?(audience_props, "tenant_id")
      assert Map.has_key?(audience_props, "tier")
      assert Map.has_key?(audience_props, "plan")
      assert Map.has_key?(audience_props, "health_tier")

      assert "audience" in tool.inputSchema.required
      assert "title" in tool.inputSchema.required
      assert "body" in tool.inputSchema.required
      assert "severity" in tool.inputSchema.required
    end

    test "tenant_notice is OMITTED when system_admin is unset (control plane on)" do
      names =
        McpTools.list_tools(%{control_plane_enabled: true})
        |> Enum.map(& &1.name)

      refute "tenant_notice" in names,
             "tenant_notice must be hidden unless ALLSOURCE_SYSTEM_ADMIN is set"
    end
  end

  # ==========================================================================
  # Phase 8 pass-through: tenant_notice forwards dry_run + confirm_token to the CP
  # (no live CP needed — the param builder is the bound contract)
  # ==========================================================================

  describe "tenant_notice_params/1 (dry-run + confirm_token pass-through)" do
    test "a COHORT send defaults dry_run: true (no implicit blast) and forwards the audience" do
      params =
        McpTools.tenant_notice_params(%{
          "audience" => %{"health_tier" => "critical"},
          "title" => "Action needed",
          "body" => "Your sync stalled",
          "severity" => "warning"
        })

      assert params["dry_run"] == true
      assert params["audience"] == %{"health_tier" => "critical"}
      # no confirmation supplied on the preview → the CP guard cannot fan out
      refute Map.has_key?(params, "confirm_token")
    end

    test "a SINGLE-tenant send defaults dry_run: false (no blast radius → applies)" do
      params =
        McpTools.tenant_notice_params(%{
          "audience" => %{"tenant_id" => "t-123"},
          "title" => "Hi",
          "body" => "note",
          "severity" => "info"
        })

      assert params["dry_run"] == false
      assert params["audience"] == %{"tenant_id" => "t-123"}
    end

    test "an explicit dry_run + confirm_token are forwarded to the CP unchanged (cohort apply)" do
      params =
        McpTools.tenant_notice_params(%{
          "audience" => %{"tier" => "indie"},
          "title" => "Upgrade",
          "body" => "...",
          "severity" => "info",
          "dry_run" => false,
          "confirm_token" => "ct-abc123"
        })

      assert params["dry_run"] == false
      assert params["confirm_token"] == "ct-abc123"
      assert params["audience"] == %{"tier" => "indie"}
    end

    test "cohort_audience?/1 distinguishes a single tenant from a cohort selector" do
      refute McpTools.cohort_audience?(%{"tenant_id" => "t-1"})
      assert McpTools.cohort_audience?(%{"tier" => "indie"})
      assert McpTools.cohort_audience?(%{"plan" => "studio"})
      assert McpTools.cohort_audience?(%{"health_tier" => "at_risk"})
      # tenant_id present wins even if a cohort key is also set
      refute McpTools.cohort_audience?(%{"tenant_id" => "t-1", "tier" => "indie"})
      refute McpTools.cohort_audience?(nil)
    end
  end

  # ==========================================================================
  # call_tool/3 gating (these short-circuit before any network call)
  # ==========================================================================

  describe "call_tool/3 system-admin gate" do
    test "every recovery_* tool returns the system-admin-not-enabled isError when system_admin is false" do
      state = %{control_plane_enabled: true, system_admin: false}

      for tool <- @recovery_tools do
        # Minimal valid args per tool so we exercise the gate, not arg validation.
        args = recovery_args(tool)
        {:ok, result} = McpTools.call_tool(tool, args, state)

        assert Map.get(result, :isError) == true, "#{tool} should be gated"
        [%{type: "text", text: text}] = result.content
        assert text =~ "system-admin not enabled", "#{tool} should report the system-admin gate"
        assert text =~ "ALLSOURCE_SYSTEM_ADMIN"
      end
    end

    test "read tools are NOT system-admin-gated (control-plane gate only)" do
      # control_plane disabled → read tools return the control-plane isError,
      # NOT the system-admin one.
      state = %{control_plane_enabled: false, system_admin: false}

      {:ok, result} = McpTools.call_tool("fleet_health_summary", %{}, state)
      assert Map.get(result, :isError) == true
      [%{type: "text", text: text}] = result.content
      assert text =~ "control plane not configured"
      refute text =~ "system-admin not enabled"
    end
  end

  # ==========================================================================
  # Dry-run pass-through (no live CP needed): the reprovision param builder
  # ==========================================================================

  describe "reprovision_recovery_params/1 (dry-run pass-through)" do
    test "omitting confirm_tenant_id passes dry_run: true and supplies NO confirmation (no mutate)" do
      params = McpTools.reprovision_recovery_params(%{"tenant_id" => "t-123", "reason" => "broken"})

      # dry_run defaults ON → the tool would NOT mutate
      assert params["dry_run"] == true
      # no confirmation supplied → the CP guard cannot execute a mutation
      refute Map.has_key?(params, "confirm_tenant_id")
      assert params["reason"] == "broken"
    end

    test "default dry_run is true even when only tenant_id is given" do
      assert McpTools.recovery_dry_run(%{}) == true
      assert McpTools.recovery_dry_run(%{"dry_run" => false}) == false
    end

    test "supplying dry_run:false + confirm_tenant_id forwards both to the CP unchanged" do
      params =
        McpTools.reprovision_recovery_params(%{
          "tenant_id" => "t-123",
          "dry_run" => false,
          "confirm_tenant_id" => "t-123",
          "reason" => "broken onboarding"
        })

      assert params["dry_run"] == false
      assert params["confirm_tenant_id"] == "t-123"
      assert params["reason"] == "broken onboarding"
    end
  end

  # Minimal valid args to drive each recovery tool past Map.fetch! in its handler
  # (only reached when the gate is open; here we only need the gate path, but
  # call_tool short-circuits before the handler so any args work — we still build
  # realistic ones).
  defp recovery_args("recovery_batch"),
    do: %{"filter" => %{"tier" => "free"}, "action" => "resync"}

  defp recovery_args("recovery_diagnose_edition"), do: %{}

  defp recovery_args("tenant_notice"),
    do: %{
      "audience" => %{"tenant_id" => "t-123"},
      "title" => "Heads up",
      "body" => "test",
      "severity" => "info"
    }

  defp recovery_args(_tool), do: %{"tenant_id" => "t-123", "reason" => "test"}
end
