defmodule QueryServiceExWeb.OnboardingController do
  @moduledoc """
  Controller for customer onboarding flow.

  Provides endpoints for:
  - Viewing onboarding status and progress
  - Completing onboarding steps
  - Skipping or resetting onboarding
  """

  use Phoenix.Controller, formats: [:json]

  alias QueryServiceEx.Accounts.Guardian
  alias QueryServiceEx.Onboarding
  alias QueryServiceEx.Onboarding.OnboardingProgress

  require Logger

  action_fallback(QueryServiceExWeb.FallbackController)

  @doc """
  Returns the current onboarding status for the authenticated user's tenant.

  GET /api/onboarding
  """
  def show(conn, _params) do
    user = Guardian.Plug.current_resource(conn)

    case Onboarding.get_status(user.tenant_id) do
      {:ok, status} ->
        conn
        |> put_status(:ok)
        |> json(%{data: status})

      {:error, _reason} ->
        conn
        |> put_status(:internal_server_error)
        |> json(%{
          error: %{code: "onboarding_error", message: "Failed to retrieve onboarding status"}
        })
    end
  end

  @doc """
  Returns details about all onboarding steps.

  GET /api/onboarding/steps
  """
  def steps(conn, _params) do
    steps =
      Onboarding.all_step_details()
      |> Enum.map(fn {id, details} ->
        Map.put(details, :id, id)
      end)

    conn
    |> put_status(:ok)
    |> json(%{data: steps})
  end

  @doc """
  Completes a specific onboarding step.

  POST /api/onboarding/steps/:step/complete
  """
  def complete_step(conn, %{"step" => step}) do
    user = Guardian.Plug.current_resource(conn)

    case Onboarding.complete_step(user.tenant_id, step) do
      {:ok, progress} ->
        Logger.info("Onboarding step completed",
          tenant_id: user.tenant_id,
          step: step
        )

        status = build_status_from_progress(progress)

        conn
        |> put_status(:ok)
        |> json(%{data: status})

      {:error, :invalid_step} ->
        conn
        |> put_status(:bad_request)
        |> json(%{
          error: %{
            code: "invalid_step",
            message: "Invalid onboarding step: #{step}",
            valid_steps: OnboardingProgress.step_names()
          }
        })

      {:error, _reason} ->
        conn
        |> put_status(:internal_server_error)
        |> json(%{error: %{code: "onboarding_error", message: "Failed to complete step"}})
    end
  end

  @doc """
  Skips the onboarding process entirely.

  POST /api/onboarding/skip
  """
  def skip(conn, _params) do
    user = Guardian.Plug.current_resource(conn)

    case Onboarding.skip_onboarding(user.tenant_id) do
      {:ok, progress} ->
        Logger.info("Onboarding skipped", tenant_id: user.tenant_id)

        status = build_status_from_progress(progress)

        conn
        |> put_status(:ok)
        |> json(%{data: status})

      {:error, _reason} ->
        conn
        |> put_status(:internal_server_error)
        |> json(%{error: %{code: "onboarding_error", message: "Failed to skip onboarding"}})
    end
  end

  @doc """
  Resets onboarding progress to start over.

  POST /api/onboarding/reset
  """
  def reset(conn, _params) do
    user = Guardian.Plug.current_resource(conn)

    case Onboarding.reset_progress(user.tenant_id) do
      {:ok, progress} ->
        Logger.info("Onboarding reset", tenant_id: user.tenant_id)

        status = build_status_from_progress(progress)

        conn
        |> put_status(:ok)
        |> json(%{data: status})

      {:error, _reason} ->
        conn
        |> put_status(:internal_server_error)
        |> json(%{error: %{code: "onboarding_error", message: "Failed to reset onboarding"}})
    end
  end

  # -------------------------------------------------------------------
  # Private Helpers
  # -------------------------------------------------------------------

  defp build_status_from_progress(progress) do
    alias QueryServiceEx.Onboarding.OnboardingProgress

    steps =
      OnboardingProgress.step_names()
      |> Enum.with_index()
      |> Enum.map(fn {step, index} ->
        details = Onboarding.get_step_details(step)
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

    %{
      tenant_id: progress.tenant_id,
      completed: OnboardingProgress.completed?(progress),
      skipped: OnboardingProgress.skipped?(progress),
      current_step: progress.current_step,
      completion_percentage: OnboardingProgress.completion_percentage(progress),
      completed_at: progress.completed_at,
      skipped_at: progress.skipped_at,
      steps: steps
    }
  end
end
