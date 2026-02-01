defmodule QueryServiceEx.Repo.Migrations.CreateUsageRecords do
  use Ecto.Migration

  def change do
    create table(:usage_records, primary_key: false) do
      add :id, :binary_id, primary_key: true
      add :tenant_id, references(:tenants, type: :binary_id, on_delete: :delete_all), null: false
      add :usage_type, :string, null: false
      add :count, :integer, null: false
      add :metadata, :map, default: %{}
      add :recorded_at, :utc_datetime, null: false

      timestamps(type: :utc_datetime)
    end

    # Index for querying by tenant and time range
    create index(:usage_records, [:tenant_id, :recorded_at])

    # Index for querying by usage type
    create index(:usage_records, [:tenant_id, :usage_type, :recorded_at])

    # Index for aggregation queries by date (using raw SQL for expression index)
    execute(
      "CREATE INDEX usage_records_tenant_id_usage_type_date_index ON usage_records (tenant_id, usage_type, DATE(recorded_at))",
      "DROP INDEX usage_records_tenant_id_usage_type_date_index"
    )
  end
end
