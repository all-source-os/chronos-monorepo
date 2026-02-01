defmodule QueryServiceEx.Repo.Migrations.CreateApiKeys do
  use Ecto.Migration

  def change do
    create table(:api_keys, primary_key: false) do
      add :id, :binary_id, primary_key: true
      add :tenant_id, references(:tenants, type: :binary_id, on_delete: :delete_all), null: false
      add :created_by_user_id, references(:users, type: :binary_id, on_delete: :nilify_all)

      # Key identification
      add :name, :string, null: false
      add :description, :text

      # Key storage (only hash stored, prefix for display)
      add :key_hash, :string, null: false
      add :key_prefix, :string, null: false

      # Permissions
      add :scopes, {:array, :string}, null: false, default: []

      # Lifecycle
      add :last_used_at, :utc_datetime
      add :expires_at, :utc_datetime
      add :revoked_at, :utc_datetime

      timestamps(type: :utc_datetime)
    end

    # Unique constraint: key names must be unique per tenant (for active keys)
    create unique_index(:api_keys, [:tenant_id, :name],
      where: "revoked_at IS NULL",
      name: :api_keys_tenant_name_active_unique
    )

    # Fast lookup by hash for authentication
    create unique_index(:api_keys, [:key_hash])

    # List active keys for a tenant
    create index(:api_keys, [:tenant_id, :revoked_at])

    # Find expiring keys
    create index(:api_keys, [:expires_at], where: "revoked_at IS NULL")
  end
end
