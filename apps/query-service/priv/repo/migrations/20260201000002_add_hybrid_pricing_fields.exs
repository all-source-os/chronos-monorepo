defmodule QueryServiceEx.Repo.Migrations.AddHybridPricingFields do
  use Ecto.Migration

  def change do
    alter table(:tenants) do
      # Overage billing settings
      add :overage_enabled, :boolean, null: false, default: false
      add :overage_rate_events, :integer, default: 100
      add :overage_rate_queries, :integer, default: 1000

      # LemonSqueezy subscription item IDs for metered billing
      add :lemon_squeezy_events_item_id, :string
      add :lemon_squeezy_queries_item_id, :string

      # Track overage separately from base usage
      add :events_overage, :integer, null: false, default: 0
      add :queries_overage, :integer, null: false, default: 0

      # Last time overage was reported to billing provider
      add :overage_reported_at, :utc_datetime
    end

    # Index for finding tenants with overage enabled
    create index(:tenants, [:overage_enabled])
  end
end
