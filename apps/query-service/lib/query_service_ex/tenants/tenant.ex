defmodule QueryServiceEx.Tenants.Tenant do
  @moduledoc """
  Schema for multi-tenant workspaces.

  Each tenant represents a customer account with their own:
  - Users (team members)
  - Subscription and billing
  - Usage quotas and metering
  - Isolated event streams
  """

  use Ecto.Schema
  import Ecto.Changeset

  @primary_key {:id, :binary_id, autogenerate: true}
  @foreign_key_type :binary_id

  @subscription_statuses ~w(trialing active past_due cancelled expired)a
  @subscription_tiers ~w(free starter pro enterprise)a

  # Tier quotas (events per month, queries per month)
  @tier_quotas %{
    free: %{events: 10_000, queries: 1_000},
    starter: %{events: 100_000, queries: 10_000},
    pro: %{events: 1_000_000, queries: 100_000},
    enterprise: %{events: :unlimited, queries: :unlimited}
  }

  schema "tenants" do
    field(:name, :string)
    field(:slug, :string)

    # LemonSqueezy integration
    field(:lemon_squeezy_customer_id, :string)
    field(:lemon_squeezy_subscription_id, :string)

    # Subscription
    field(:subscription_status, Ecto.Enum, values: @subscription_statuses, default: :trialing)
    field(:subscription_tier, Ecto.Enum, values: @subscription_tiers, default: :free)
    field(:trial_ends_at, :utc_datetime)
    field(:subscription_ends_at, :utc_datetime)

    # Usage quotas
    field(:events_quota, :integer, default: 10_000)
    field(:queries_quota, :integer, default: 1_000)

    # Current usage
    field(:events_used, :integer, default: 0)
    field(:queries_used, :integer, default: 0)
    field(:usage_reset_at, :utc_datetime)

    # Settings
    field(:settings, :map, default: %{})

    has_many(:users, QueryServiceEx.Accounts.User)

    timestamps(type: :utc_datetime)
  end

  @doc """
  Changeset for creating a new tenant.
  """
  def changeset(tenant, attrs) do
    tenant
    |> cast(attrs, [:name, :slug, :settings])
    |> validate_required([:name, :slug])
    |> validate_length(:name, min: 2, max: 100)
    |> validate_format(:slug, ~r/^[a-z0-9][a-z0-9-]*[a-z0-9]$/,
      message: "must be lowercase alphanumeric with hyphens, 2+ characters"
    )
    |> unique_constraint(:slug)
    |> set_trial_period()
  end

  @doc """
  Changeset for updating subscription status from LemonSqueezy webhooks.
  """
  def subscription_changeset(tenant, attrs) do
    tenant
    |> cast(attrs, [
      :lemon_squeezy_customer_id,
      :lemon_squeezy_subscription_id,
      :subscription_status,
      :subscription_tier,
      :trial_ends_at,
      :subscription_ends_at
    ])
    |> update_quotas_for_tier()
  end

  @doc """
  Changeset for updating usage counters.
  """
  def usage_changeset(tenant, attrs) do
    tenant
    |> cast(attrs, [:events_used, :queries_used, :usage_reset_at])
  end

  @doc """
  Returns the tier quotas map.
  """
  def tier_quotas, do: @tier_quotas

  @doc """
  Returns quota limits for a given tier.
  """
  def quota_for_tier(tier) when tier in @subscription_tiers do
    Map.get(@tier_quotas, tier)
  end

  @doc """
  Checks if the tenant has exceeded their events quota.
  """
  def events_quota_exceeded?(%__MODULE__{} = tenant) do
    case tenant.events_quota do
      :unlimited -> false
      quota -> tenant.events_used >= quota
    end
  end

  @doc """
  Checks if the tenant has exceeded their queries quota.
  """
  def queries_quota_exceeded?(%__MODULE__{} = tenant) do
    case tenant.queries_quota do
      :unlimited -> false
      quota -> tenant.queries_used >= quota
    end
  end

  @doc """
  Checks if the tenant has an active subscription (including trial).
  """
  def subscription_active?(%__MODULE__{} = tenant) do
    tenant.subscription_status in [:trialing, :active]
  end

  @doc """
  Checks if the tenant is in trial period.
  """
  def in_trial?(%__MODULE__{} = tenant) do
    tenant.subscription_status == :trialing &&
      tenant.trial_ends_at != nil &&
      DateTime.compare(tenant.trial_ends_at, DateTime.utc_now()) == :gt
  end

  # Private helpers

  defp set_trial_period(changeset) do
    if get_change(changeset, :trial_ends_at) == nil do
      trial_end = DateTime.utc_now() |> DateTime.add(14, :day) |> DateTime.truncate(:second)
      put_change(changeset, :trial_ends_at, trial_end)
    else
      changeset
    end
  end

  defp update_quotas_for_tier(changeset) do
    case get_change(changeset, :subscription_tier) do
      nil ->
        changeset

      tier ->
        quotas = quota_for_tier(tier)

        changeset
        |> put_change(:events_quota, quotas.events)
        |> put_change(:queries_quota, quotas.queries)
    end
  end
end
