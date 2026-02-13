defmodule QueryServiceEx.DevMode do
  @moduledoc """
  Development and standalone mode utilities.

  When `AUTH_DISABLED=true` or `AUTH_DISABLED=1` is set, authentication
  and authorization checks are bypassed, allowing local development or
  standalone deployment without requiring OAuth configuration.

  Standalone mode (no PostgreSQL) is automatically detected when the
  Repo process is not running. All DB-dependent code paths gracefully
  degrade — usage metering is skipped, tenant endpoints return the
  in-memory dev tenant, and health checks skip the database.
  """

  require Logger

  @dev_user_id "00000000-0000-0000-0000-000000000001"
  @dev_tenant_id "00000000-0000-0000-0000-000000000002"

  @doc """
  Check if authentication is disabled via environment variable.

  Returns `true` if `AUTH_DISABLED` is set to "true" or "1".
  """
  def auth_disabled? do
    case System.get_env("AUTH_DISABLED") do
      val when val in ["true", "1"] -> true
      _ -> false
    end
  end

  @doc """
  Check if the Ecto Repo process is running.

  Returns `true` if `QueryServiceEx.Repo` is alive, meaning PostgreSQL
  is configured and available. Returns `false` in standalone mode
  (no DATABASE_URL configured).
  """
  def repo_available? do
    Process.whereis(QueryServiceEx.Repo) != nil
  end

  @doc """
  Log a warning on startup if dev mode or standalone mode is enabled.
  Should be called during application startup.
  """
  def maybe_log_warning do
    if auth_disabled?() do
      Logger.warning("AUTH_DISABLED is enabled - authentication is BYPASSED")

      if repo_available?() do
        Logger.warning("Running in dev mode with PostgreSQL")
      else
        Logger.warning(
          "Running in standalone mode (no PostgreSQL) - usage metering and tenant persistence disabled"
        )
      end
    end
  end

  @doc """
  Returns a dev user struct for use when auth is disabled.

  This creates an in-memory user struct (not persisted to database)
  that can be used for local development.
  """
  def dev_user do
    %QueryServiceEx.Accounts.User{
      id: @dev_user_id,
      email: "dev@localhost",
      name: "Dev User",
      provider: "dev",
      tenant_id: @dev_tenant_id,
      tenant: dev_tenant(),
      inserted_at: DateTime.utc_now(),
      updated_at: DateTime.utc_now()
    }
  end

  @doc """
  Returns a dev tenant struct for use when auth is disabled.

  This creates an in-memory tenant struct with enterprise-level
  quotas for unrestricted local development.
  """
  def dev_tenant do
    now = DateTime.utc_now()

    %QueryServiceEx.Tenants.Tenant{
      id: @dev_tenant_id,
      name: "Dev Tenant",
      slug: "dev-tenant",
      subscription_status: :active,
      subscription_tier: :enterprise,
      trial_ends_at: DateTime.add(now, 365, :day),
      subscription_ends_at: DateTime.add(now, 365, :day),
      events_quota: -1,
      queries_quota: -1,
      events_used: 0,
      queries_used: 0,
      overage_enabled: false,
      settings: %{},
      inserted_at: now,
      updated_at: now
    }
  end

  @doc """
  Returns the dev user ID constant.
  """
  def dev_user_id, do: @dev_user_id

  @doc """
  Returns the dev tenant ID constant.
  """
  def dev_tenant_id, do: @dev_tenant_id
end
