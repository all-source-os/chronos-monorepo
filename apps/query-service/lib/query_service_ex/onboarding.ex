defmodule QueryServiceEx.Onboarding do
  @moduledoc """
  The Onboarding context manages the customer onboarding flow.

  Provides functions for:
  - Tracking onboarding progress for tenants
  - Completing individual onboarding steps
  - Retrieving onboarding status and guidance
  """

  alias QueryServiceEx.Repo
  alias QueryServiceEx.Onboarding.OnboardingProgress

  require Logger

  @step_details %{
    "workspace_setup" => %{
      title: "Set up your workspace",
      description: "Customize your workspace name and settings",
      action: "Update your workspace settings to personalize Chronos for your team.",
      docs_url: "/docs/getting-started/workspace"
    },
    "first_event" => %{
      title: "Send your first event",
      description: "Ingest an event into your event store",
      action: "Use the Events API to send your first event to Chronos.",
      docs_url: "/docs/getting-started/events"
    },
    "first_query" => %{
      title: "Run your first query",
      description: "Query your event data",
      action: "Execute a query to retrieve and analyze your events.",
      docs_url: "/docs/getting-started/queries"
    },
    "first_projection" => %{
      title: "Create a projection",
      description: "Build a materialized view of your events",
      action: "Create a projection to automatically aggregate your event data.",
      docs_url: "/docs/getting-started/projections"
    },
    "invite_team" => %{
      title: "Invite your team",
      description: "Add team members to your workspace",
      action: "Invite colleagues to collaborate on your event store.",
      docs_url: "/docs/getting-started/team"
    }
  }

  # -------------------------------------------------------------------
  # Progress Retrieval
  # -------------------------------------------------------------------

  @doc """
  Gets or creates onboarding progress for a tenant.

  If no progress record exists, creates one with default values.
  """
  def get_or_create_progress(tenant_id) do
    case get_progress(tenant_id) do
      nil -> create_progress(tenant_id)
      progress -> {:ok, progress}
    end
  end

  @doc """
  Gets onboarding progress for a tenant.

  Returns `nil` if no progress exists.
  """
  def get_progress(tenant_id) do
    Repo.get_by(OnboardingProgress, tenant_id: tenant_id)
  end

  @doc """
  Creates initial onboarding progress for a tenant.
  """
  def create_progress(tenant_id) do
    %OnboardingProgress{}
    |> OnboardingProgress.changeset(%{tenant_id: tenant_id})
    |> Repo.insert()
  end

  # -------------------------------------------------------------------
  # Step Management
  # -------------------------------------------------------------------

  @doc """
  Marks an onboarding step as completed.

  Returns `{:ok, progress}` on success.
  """
  def complete_step(tenant_id, step) do
    step_str = to_string(step)

    unless step_str in OnboardingProgress.step_names() do
      {:error, :invalid_step}
    else
      with {:ok, progress} <- get_or_create_progress(tenant_id) do
        if OnboardingProgress.step_completed?(progress, step) do
          {:ok, progress}
        else
          progress
          |> OnboardingProgress.complete_step_changeset(step)
          |> Repo.update()
          |> tap_completed_event(step_str)
        end
      end
    end
  end

  @doc """
  Marks the entire onboarding as skipped.

  Users can skip onboarding if they're already familiar with the product.
  """
  def skip_onboarding(tenant_id) do
    with {:ok, progress} <- get_or_create_progress(tenant_id) do
      progress
      |> OnboardingProgress.skip_changeset()
      |> Repo.update()
      |> tap_skip_event()
    end
  end

  @doc """
  Resets onboarding progress for a tenant.

  Useful for testing or if a user wants to restart onboarding.
  """
  def reset_progress(tenant_id) do
    case get_progress(tenant_id) do
      nil ->
        create_progress(tenant_id)

      progress ->
        progress
        |> OnboardingProgress.changeset(%{
          completed_steps: [],
          current_step: "workspace_setup",
          completed_at: nil,
          skipped_at: nil
        })
        |> Repo.update()
    end
  end

  # -------------------------------------------------------------------
  # Status and Guidance
  # -------------------------------------------------------------------

  @doc """
  Returns the full onboarding status for a tenant.

  Includes progress, next steps, and guidance information.
  """
  def get_status(tenant_id) do
    case get_or_create_progress(tenant_id) do
      {:ok, progress} ->
        {:ok, build_status(progress)}

      {:error, _} = error ->
        error
    end
  end

  @doc """
  Returns details about a specific step.
  """
  def get_step_details(step) do
    step_str = to_string(step)
    Map.get(@step_details, step_str)
  end

  @doc """
  Returns all step details.
  """
  def all_step_details, do: @step_details

  # -------------------------------------------------------------------
  # Auto-completion Triggers
  # -------------------------------------------------------------------

  @doc """
  Called when a tenant updates their workspace settings.
  Auto-completes the workspace_setup step.
  """
  def on_workspace_updated(tenant_id) do
    complete_step(tenant_id, :workspace_setup)
  end

  @doc """
  Called when a tenant creates their first event.
  Auto-completes the first_event step.
  """
  def on_first_event(tenant_id) do
    complete_step(tenant_id, :first_event)
  end

  @doc """
  Called when a tenant runs their first query.
  Auto-completes the first_query step.
  """
  def on_first_query(tenant_id) do
    complete_step(tenant_id, :first_query)
  end

  @doc """
  Called when a tenant creates their first projection.
  Auto-completes the first_projection step.
  """
  def on_first_projection(tenant_id) do
    complete_step(tenant_id, :first_projection)
  end

  @doc """
  Called when a tenant invites their first team member.
  Auto-completes the invite_team step.
  """
  def on_team_invited(tenant_id) do
    complete_step(tenant_id, :invite_team)
  end

  # -------------------------------------------------------------------
  # Private Helpers
  # -------------------------------------------------------------------

  defp build_status(progress) do
    steps = build_steps_list(progress)

    %{
      tenant_id: progress.tenant_id,
      completed: OnboardingProgress.completed?(progress),
      skipped: OnboardingProgress.skipped?(progress),
      current_step: progress.current_step,
      completion_percentage: OnboardingProgress.completion_percentage(progress),
      completed_at: progress.completed_at,
      skipped_at: progress.skipped_at,
      steps: steps,
      next_action: get_next_action(progress)
    }
  end

  defp build_steps_list(progress) do
    OnboardingProgress.step_names()
    |> Enum.with_index()
    |> Enum.map(fn {step, index} ->
      details = Map.get(@step_details, step)
      completed = OnboardingProgress.step_completed?(progress, step)

      %{
        id: step,
        order: index + 1,
        title: details.title,
        description: details.description,
        completed: completed,
        current: step == progress.current_step && !completed,
        docs_url: details.docs_url
      }
    end)
  end

  defp get_next_action(progress) do
    if OnboardingProgress.completed?(progress) || OnboardingProgress.skipped?(progress) do
      nil
    else
      step = progress.current_step
      details = Map.get(@step_details, step)

      %{
        step: step,
        title: details.title,
        action: details.action,
        docs_url: details.docs_url
      }
    end
  end

  defp tap_completed_event({:ok, progress} = result, step) do
    :telemetry.execute(
      [:query_service_ex, :onboarding, :step_completed],
      %{count: 1},
      %{tenant_id: progress.tenant_id, step: step}
    )

    if OnboardingProgress.completed?(progress) do
      :telemetry.execute(
        [:query_service_ex, :onboarding, :completed],
        %{count: 1},
        %{tenant_id: progress.tenant_id}
      )
    end

    result
  end

  defp tap_completed_event(error, _step), do: error

  defp tap_skip_event({:ok, progress} = result) do
    :telemetry.execute(
      [:query_service_ex, :onboarding, :skipped],
      %{count: 1},
      %{tenant_id: progress.tenant_id}
    )

    result
  end

  defp tap_skip_event(error), do: error
end
