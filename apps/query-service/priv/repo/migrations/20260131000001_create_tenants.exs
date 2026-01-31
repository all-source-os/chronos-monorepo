defmodule QueryServiceEx.Repo.Migrations.CreateTenants do
  use Ecto.Migration

  def change do
    # Create enum for subscription status
    execute(
      "CREATE TYPE subscription_status AS ENUM ('trialing', 'active', 'past_due', 'cancelled', 'expired')",
      "DROP TYPE subscription_status"
    )

    # Create enum for subscription tier
    execute(
      "CREATE TYPE subscription_tier AS ENUM ('free', 'starter', 'pro', 'enterprise')",
      "DROP TYPE subscription_tier"
    )

    create table(:tenants, primary_key: false) do
      add :id, :binary_id, primary_key: true
      add :name, :string, null: false
      add :slug, :string, null: false

      # LemonSqueezy integration
      add :lemon_squeezy_customer_id, :string
      add :lemon_squeezy_subscription_id, :string

      # Subscription details
      add :subscription_status, :subscription_status, null: false, default: "trialing"
      add :subscription_tier, :subscription_tier, null: false, default: "free"
      add :trial_ends_at, :utc_datetime
      add :subscription_ends_at, :utc_datetime

      # Usage quotas (per billing period)
      add :events_quota, :integer, null: false, default: 10_000
      add :queries_quota, :integer, null: false, default: 1_000

      # Current usage (reset each billing period)
      add :events_used, :integer, null: false, default: 0
      add :queries_used, :integer, null: false, default: 0
      add :usage_reset_at, :utc_datetime

      # Settings
      add :settings, :map, null: false, default: %{}

      timestamps(type: :utc_datetime)
    end

    create unique_index(:tenants, [:slug])
    create index(:tenants, [:lemon_squeezy_customer_id])
    create index(:tenants, [:lemon_squeezy_subscription_id])
    create index(:tenants, [:subscription_status])
  end
end
