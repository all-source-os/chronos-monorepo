defmodule QueryServiceEx.Onboarding.OnboardingProgress do
  @moduledoc """
  Schema for tracking user onboarding progress.

  Tracks which onboarding steps a tenant has completed and their overall
  onboarding status.
  """

  use Ecto.Schema
  import Ecto.Changeset

  @primary_key {:id, :binary_id, autogenerate: true}
  @foreign_key_type :binary_id

  @onboarding_steps ~w(
    workspace_setup
    first_event
    first_query
    first_projection
    invite_team
  )a

  schema "onboarding_progress" do
    field(:completed_steps, {:array, :string}, default: [])
    field(:current_step, :string, default: "workspace_setup")
    field(:completed_at, :utc_datetime)
    field(:skipped_at, :utc_datetime)
    field(:metadata, :map, default: %{})

    belongs_to(:tenant, QueryServiceEx.Tenants.Tenant)

    timestamps(type: :utc_datetime)
  end

  @doc """
  Returns the list of all onboarding steps in order.
  """
  def steps, do: @onboarding_steps

  @doc """
  Returns the ordered list of step names as strings.
  """
  def step_names, do: Enum.map(@onboarding_steps, &Atom.to_string/1)

  @doc """
  Changeset for creating onboarding progress.
  """
  def changeset(progress, attrs) do
    progress
    |> cast(attrs, [
      :tenant_id,
      :completed_steps,
      :current_step,
      :completed_at,
      :skipped_at,
      :metadata
    ])
    |> validate_required([:tenant_id])
    |> validate_inclusion(:current_step, step_names())
    |> validate_completed_steps()
    |> unique_constraint(:tenant_id)
  end

  @doc """
  Changeset for completing a step.
  """
  def complete_step_changeset(progress, step) do
    step_str = to_string(step)
    new_completed = Enum.uniq(progress.completed_steps ++ [step_str])
    next_step = determine_next_step(new_completed)

    attrs =
      if all_steps_completed?(new_completed) do
        %{
          completed_steps: new_completed,
          current_step: next_step,
          completed_at: DateTime.utc_now() |> DateTime.truncate(:second)
        }
      else
        %{completed_steps: new_completed, current_step: next_step}
      end

    progress
    |> cast(attrs, [:completed_steps, :current_step, :completed_at])
  end

  @doc """
  Changeset for skipping onboarding.
  """
  def skip_changeset(progress) do
    progress
    |> cast(
      %{skipped_at: DateTime.utc_now() |> DateTime.truncate(:second)},
      [:skipped_at]
    )
  end

  @doc """
  Checks if onboarding is complete.
  """
  def completed?(%__MODULE__{completed_at: completed_at}), do: completed_at != nil

  @doc """
  Checks if onboarding was skipped.
  """
  def skipped?(%__MODULE__{skipped_at: skipped_at}), do: skipped_at != nil

  @doc """
  Checks if a specific step is completed.
  """
  def step_completed?(%__MODULE__{completed_steps: completed}, step) do
    to_string(step) in completed
  end

  @doc """
  Returns the completion percentage (0-100).
  """
  def completion_percentage(%__MODULE__{completed_steps: completed}) do
    total = length(@onboarding_steps)
    completed_count = length(completed)
    round(completed_count / total * 100)
  end

  # Private helpers

  defp validate_completed_steps(changeset) do
    case get_field(changeset, :completed_steps) do
      nil ->
        changeset

      steps ->
        invalid = Enum.reject(steps, &(&1 in step_names()))

        if Enum.empty?(invalid) do
          changeset
        else
          add_error(changeset, :completed_steps, "contains invalid steps: #{inspect(invalid)}")
        end
    end
  end

  defp determine_next_step(completed_steps) do
    step_names()
    |> Enum.find(fn step -> step not in completed_steps end)
    |> case do
      nil -> List.last(step_names())
      step -> step
    end
  end

  defp all_steps_completed?(completed_steps) do
    Enum.all?(step_names(), &(&1 in completed_steps))
  end
end
