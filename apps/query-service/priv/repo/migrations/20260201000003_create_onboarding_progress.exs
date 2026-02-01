defmodule QueryServiceEx.Repo.Migrations.CreateOnboardingProgress do
  use Ecto.Migration

  def change do
    create table(:onboarding_progress, primary_key: false) do
      add :id, :binary_id, primary_key: true
      add :tenant_id, references(:tenants, type: :binary_id, on_delete: :delete_all), null: false
      add :completed_steps, {:array, :string}, null: false, default: []
      add :current_step, :string, null: false, default: "workspace_setup"
      add :completed_at, :utc_datetime
      add :skipped_at, :utc_datetime
      add :metadata, :map, null: false, default: %{}

      timestamps(type: :utc_datetime)
    end

    create unique_index(:onboarding_progress, [:tenant_id])
    create index(:onboarding_progress, [:current_step])
    create index(:onboarding_progress, [:completed_at])
  end
end
