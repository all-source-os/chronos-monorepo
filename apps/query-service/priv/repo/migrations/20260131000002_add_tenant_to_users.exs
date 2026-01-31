defmodule QueryServiceEx.Repo.Migrations.AddTenantToUsers do
  use Ecto.Migration

  def change do
    alter table(:users) do
      add :tenant_id, references(:tenants, type: :binary_id, on_delete: :delete_all)

      # Support multiple OAuth providers
      add :github_id, :string
      add :github_token, :text
      add :github_refresh_token, :text

      # Generic provider field for future OAuth providers
      add :provider, :string, null: false, default: "google"
    end

    # Make google_id nullable since users might authenticate via GitHub
    execute(
      "ALTER TABLE users ALTER COLUMN google_id DROP NOT NULL",
      "ALTER TABLE users ALTER COLUMN google_id SET NOT NULL"
    )

    create index(:users, [:tenant_id])
    create unique_index(:users, [:github_id], where: "github_id IS NOT NULL")

    # Ensure a user has at least one OAuth provider
    create constraint(:users, :must_have_oauth_provider,
             check: "google_id IS NOT NULL OR github_id IS NOT NULL"
           )
  end
end
