defmodule QueryServiceEx.Tenants do
  @moduledoc """
  The Tenants context handles multi-tenant workspace management.

  Provides functions for:
  - Creating and managing tenants
  - Subscription status and billing integration
  - Usage metering and quota enforcement
  """

  import Ecto.Query
  alias QueryServiceEx.Repo
  alias QueryServiceEx.Tenants.Tenant

  require Logger

  # -------------------------------------------------------------------
  # Tenant CRUD
  # -------------------------------------------------------------------

  @doc """
  Creates a new tenant with the given attributes.
  """
  def create_tenant(attrs) do
    %Tenant{}
    |> Tenant.changeset(attrs)
    |> Repo.insert()
  end

  @doc """
  Gets a tenant by ID.

  Returns `nil` if the tenant does not exist.
  """
  def get_tenant(id) do
    Repo.get(Tenant, id)
  end

  @doc """
  Gets a tenant by ID.

  Raises `Ecto.NoResultsError` if the tenant does not exist.
  """
  def get_tenant!(id) do
    Repo.get!(Tenant, id)
  end

  @doc """
  Gets a tenant by slug.

  Returns `nil` if the tenant does not exist.
  """
  def get_tenant_by_slug(slug) do
    Repo.get_by(Tenant, slug: slug)
  end

  @doc """
  Gets a tenant by LemonSqueezy customer ID.
  """
  def get_tenant_by_customer_id(customer_id) do
    Repo.get_by(Tenant, lemon_squeezy_customer_id: customer_id)
  end

  @doc """
  Gets a tenant by LemonSqueezy subscription ID.
  """
  def get_tenant_by_subscription_id(subscription_id) do
    Repo.get_by(Tenant, lemon_squeezy_subscription_id: subscription_id)
  end

  @doc """
  Updates a tenant.
  """
  def update_tenant(%Tenant{} = tenant, attrs) do
    tenant
    |> Tenant.changeset(attrs)
    |> Repo.update()
  end

  @doc """
  Lists all tenants.
  """
  def list_tenants do
    Repo.all(Tenant)
  end

  @doc """
  Deletes a tenant and all associated data.
  """
  def delete_tenant(%Tenant{} = tenant) do
    Repo.delete(tenant)
  end

  # -------------------------------------------------------------------
  # Subscription Management
  # -------------------------------------------------------------------

  @doc """
  Updates tenant subscription from LemonSqueezy webhook data.
  """
  def update_subscription(%Tenant{} = tenant, attrs) do
    tenant
    |> Tenant.subscription_changeset(attrs)
    |> Repo.update()
  end

  @doc """
  Checks if a tenant has an active subscription (including trial).
  """
  def subscription_active?(%Tenant{} = tenant) do
    Tenant.subscription_active?(tenant)
  end

  @doc """
  Gets all tenants with expiring trials (within N days).
  """
  def get_expiring_trials(days \\ 3) do
    cutoff = DateTime.utc_now() |> DateTime.add(days, :day)

    from(t in Tenant,
      where: t.subscription_status == :trialing,
      where: t.trial_ends_at <= ^cutoff,
      where: t.trial_ends_at > ^DateTime.utc_now()
    )
    |> Repo.all()
  end

  @doc """
  Expires all trials past their end date.
  """
  def expire_trials do
    now = DateTime.utc_now()

    from(t in Tenant,
      where: t.subscription_status == :trialing,
      where: t.trial_ends_at <= ^now
    )
    |> Repo.update_all(set: [subscription_status: :expired, updated_at: now])
  end

  # -------------------------------------------------------------------
  # Usage Metering
  # -------------------------------------------------------------------

  @doc """
  Increments the event usage counter for a tenant.

  Returns `{:ok, tenant}` on success, or `{:error, :quota_exceeded}` if over quota.
  """
  def increment_events_usage(tenant_id, count \\ 1) do
    tenant = get_tenant!(tenant_id)

    if Tenant.events_quota_exceeded?(tenant) do
      {:error, :quota_exceeded}
    else
      new_count = tenant.events_used + count

      tenant
      |> Tenant.usage_changeset(%{events_used: new_count})
      |> Repo.update()
    end
  end

  @doc """
  Increments the query usage counter for a tenant.

  Returns `{:ok, tenant}` on success, or `{:error, :quota_exceeded}` if over quota.
  """
  def increment_queries_usage(tenant_id, count \\ 1) do
    tenant = get_tenant!(tenant_id)

    if Tenant.queries_quota_exceeded?(tenant) do
      {:error, :quota_exceeded}
    else
      new_count = tenant.queries_used + count

      tenant
      |> Tenant.usage_changeset(%{queries_used: new_count})
      |> Repo.update()
    end
  end

  @doc """
  Gets current usage statistics for a tenant.
  """
  def get_usage_stats(tenant_id) do
    tenant = get_tenant!(tenant_id)

    %{
      events: %{
        used: tenant.events_used,
        quota: tenant.events_quota,
        remaining: max(0, tenant.events_quota - tenant.events_used),
        percentage: calculate_percentage(tenant.events_used, tenant.events_quota)
      },
      queries: %{
        used: tenant.queries_used,
        quota: tenant.queries_quota,
        remaining: max(0, tenant.queries_quota - tenant.queries_used),
        percentage: calculate_percentage(tenant.queries_used, tenant.queries_quota)
      },
      reset_at: tenant.usage_reset_at
    }
  end

  @doc """
  Resets usage counters for a tenant (called at billing period start).
  """
  def reset_usage(tenant_id) do
    tenant = get_tenant!(tenant_id)

    tenant
    |> Tenant.usage_changeset(%{
      events_used: 0,
      queries_used: 0,
      usage_reset_at: DateTime.utc_now()
    })
    |> Repo.update()
  end

  @doc """
  Resets usage for all tenants (for batch processing).
  """
  def reset_all_usage do
    now = DateTime.utc_now()

    from(t in Tenant)
    |> Repo.update_all(set: [events_used: 0, queries_used: 0, usage_reset_at: now])
  end

  # -------------------------------------------------------------------
  # Slug Generation
  # -------------------------------------------------------------------

  @doc """
  Generates a unique slug from a name.

  If the base slug already exists, appends a random suffix.
  """
  def generate_slug(name) do
    base_slug =
      name
      |> String.downcase()
      |> String.replace(~r/[^a-z0-9]+/, "-")
      |> String.replace(~r/^-|-$/, "")
      |> String.slice(0, 50)

    # Ensure minimum length
    base_slug = if String.length(base_slug) < 2, do: "tenant-#{base_slug}", else: base_slug

    ensure_unique_slug(base_slug)
  end

  defp ensure_unique_slug(slug, attempt \\ 0) do
    candidate = if attempt == 0, do: slug, else: "#{slug}-#{random_suffix()}"

    if get_tenant_by_slug(candidate) == nil do
      candidate
    else
      ensure_unique_slug(slug, attempt + 1)
    end
  end

  defp random_suffix do
    :crypto.strong_rand_bytes(3) |> Base.url_encode64(padding: false) |> String.downcase()
  end

  defp calculate_percentage(_used, :unlimited), do: 0.0

  defp calculate_percentage(used, quota) when quota > 0 do
    Float.round(used / quota * 100, 1)
  end

  defp calculate_percentage(_used, _quota), do: 0.0
end
