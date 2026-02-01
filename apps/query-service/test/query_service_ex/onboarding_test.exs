defmodule QueryServiceEx.OnboardingTest do
  @moduledoc """
  Tests for the Onboarding context.
  """
  use QueryServiceEx.DataCase

  alias QueryServiceEx.Onboarding
  alias QueryServiceEx.Onboarding.OnboardingProgress
  alias QueryServiceEx.Tenants

  @moduletag :database

  setup do
    {:ok, tenant} =
      Tenants.create_tenant(%{
        name: "Test Workspace",
        slug: "test-workspace-#{:rand.uniform(10000)}"
      })

    {:ok, tenant: tenant}
  end

  describe "get_or_create_progress/1" do
    test "creates progress for new tenant", %{tenant: tenant} do
      assert Onboarding.get_progress(tenant.id) == nil

      {:ok, progress} = Onboarding.get_or_create_progress(tenant.id)

      assert progress.tenant_id == tenant.id
      assert progress.current_step == "workspace_setup"
      assert progress.completed_steps == []
    end

    test "returns existing progress", %{tenant: tenant} do
      {:ok, original} = Onboarding.create_progress(tenant.id)
      {:ok, fetched} = Onboarding.get_or_create_progress(tenant.id)

      assert original.id == fetched.id
    end
  end

  describe "complete_step/2" do
    test "completes a valid step", %{tenant: tenant} do
      {:ok, _} = Onboarding.create_progress(tenant.id)
      {:ok, progress} = Onboarding.complete_step(tenant.id, :workspace_setup)

      assert "workspace_setup" in progress.completed_steps
      assert progress.current_step == "first_event"
    end

    test "completing all steps marks onboarding complete", %{tenant: tenant} do
      {:ok, _} = Onboarding.create_progress(tenant.id)

      for step <- OnboardingProgress.step_names() do
        {:ok, _} = Onboarding.complete_step(tenant.id, step)
      end

      progress = Onboarding.get_progress(tenant.id)
      assert OnboardingProgress.completed?(progress)
      assert progress.completed_at != nil
    end

    test "completing same step twice is idempotent", %{tenant: tenant} do
      {:ok, _} = Onboarding.create_progress(tenant.id)
      {:ok, progress1} = Onboarding.complete_step(tenant.id, :workspace_setup)
      {:ok, progress2} = Onboarding.complete_step(tenant.id, :workspace_setup)

      assert progress1.completed_steps == progress2.completed_steps
    end

    test "returns error for invalid step", %{tenant: tenant} do
      {:ok, _} = Onboarding.create_progress(tenant.id)
      result = Onboarding.complete_step(tenant.id, :invalid_step)

      assert result == {:error, :invalid_step}
    end

    test "creates progress if it doesn't exist", %{tenant: tenant} do
      {:ok, progress} = Onboarding.complete_step(tenant.id, :first_event)

      assert "first_event" in progress.completed_steps
    end
  end

  describe "skip_onboarding/1" do
    test "marks onboarding as skipped", %{tenant: tenant} do
      {:ok, _} = Onboarding.create_progress(tenant.id)
      {:ok, progress} = Onboarding.skip_onboarding(tenant.id)

      assert OnboardingProgress.skipped?(progress)
      assert progress.skipped_at != nil
    end
  end

  describe "reset_progress/1" do
    test "resets completed progress", %{tenant: tenant} do
      {:ok, _} = Onboarding.create_progress(tenant.id)
      {:ok, _} = Onboarding.complete_step(tenant.id, :workspace_setup)
      {:ok, _} = Onboarding.complete_step(tenant.id, :first_event)

      {:ok, progress} = Onboarding.reset_progress(tenant.id)

      assert progress.completed_steps == []
      assert progress.current_step == "workspace_setup"
      assert progress.completed_at == nil
    end

    test "resets skipped progress", %{tenant: tenant} do
      {:ok, _} = Onboarding.create_progress(tenant.id)
      {:ok, _} = Onboarding.skip_onboarding(tenant.id)

      {:ok, progress} = Onboarding.reset_progress(tenant.id)

      assert progress.skipped_at == nil
    end

    test "creates progress if doesn't exist", %{tenant: tenant} do
      {:ok, progress} = Onboarding.reset_progress(tenant.id)

      assert progress.tenant_id == tenant.id
      assert progress.current_step == "workspace_setup"
    end
  end

  describe "get_status/1" do
    test "returns full status for new tenant", %{tenant: tenant} do
      {:ok, status} = Onboarding.get_status(tenant.id)

      assert status.tenant_id == tenant.id
      assert status.completed == false
      assert status.skipped == false
      assert status.current_step == "workspace_setup"
      assert status.completion_percentage == 0
      assert length(status.steps) == 5
      assert status.next_action != nil
    end

    test "returns status with completed steps", %{tenant: tenant} do
      {:ok, _} = Onboarding.complete_step(tenant.id, :workspace_setup)
      {:ok, _} = Onboarding.complete_step(tenant.id, :first_event)

      {:ok, status} = Onboarding.get_status(tenant.id)

      assert status.completion_percentage == 40
      completed_step_ids = status.steps |> Enum.filter(& &1.completed) |> Enum.map(& &1.id)
      assert "workspace_setup" in completed_step_ids
      assert "first_event" in completed_step_ids
    end

    test "returns nil next_action when complete", %{tenant: tenant} do
      for step <- OnboardingProgress.step_names() do
        {:ok, _} = Onboarding.complete_step(tenant.id, step)
      end

      {:ok, status} = Onboarding.get_status(tenant.id)

      assert status.completed == true
      assert status.next_action == nil
    end
  end

  describe "auto-completion triggers" do
    test "on_workspace_updated/1 completes workspace_setup", %{tenant: tenant} do
      {:ok, progress} = Onboarding.on_workspace_updated(tenant.id)

      assert "workspace_setup" in progress.completed_steps
    end

    test "on_first_event/1 completes first_event", %{tenant: tenant} do
      {:ok, progress} = Onboarding.on_first_event(tenant.id)

      assert "first_event" in progress.completed_steps
    end

    test "on_first_query/1 completes first_query", %{tenant: tenant} do
      {:ok, progress} = Onboarding.on_first_query(tenant.id)

      assert "first_query" in progress.completed_steps
    end

    test "on_first_projection/1 completes first_projection", %{tenant: tenant} do
      {:ok, progress} = Onboarding.on_first_projection(tenant.id)

      assert "first_projection" in progress.completed_steps
    end

    test "on_team_invited/1 completes invite_team", %{tenant: tenant} do
      {:ok, progress} = Onboarding.on_team_invited(tenant.id)

      assert "invite_team" in progress.completed_steps
    end
  end
end
