defmodule QueryServiceEx.DevMode do
  @moduledoc """
  Development and standalone mode utilities.

  When `AUTH_DISABLED=true` or `AUTH_DISABLED=1` is set, authentication
  and authorization checks are bypassed, allowing local development or
  standalone deployment without requiring external auth configuration.
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
      val when val in ["true", "1"] ->
        if production?() do
          Logger.error(
            "AUTH_DISABLED is set but ignored in production. " <>
              "Unset AUTH_DISABLED or set MIX_ENV to dev/test."
          )

          false
        else
          true
        end

      _ ->
        false
    end
  end

  @doc """
  Returns true when running in production mode.

  Checks the `RELEASE_MODE` env var (set in Docker/release builds) and
  the compile-time Mix environment.
  """
  def production? do
    System.get_env("RELEASE_MODE") == "true" ||
      Application.get_env(:query_service_ex, :env) == :prod
  end

  @doc """
  Log a warning on startup if dev mode is enabled.
  Should be called during application startup.
  """
  def maybe_log_warning do
    if auth_disabled?() do
      Logger.warning("AUTH_DISABLED is enabled - authentication is BYPASSED")
    end
  end

  @doc """
  Returns a dev user map for use when auth is disabled.
  """
  def dev_user do
    %{
      id: @dev_user_id,
      email: "dev@localhost",
      name: "Dev User",
      provider: "dev",
      tenant_id: @dev_tenant_id,
      inserted_at: DateTime.utc_now(),
      updated_at: DateTime.utc_now()
    }
  end

  @doc """
  Returns a dev tenant map for use when auth is disabled.

  Uses Core-format map with enterprise-level quotas for unrestricted local development.
  """
  def dev_tenant do
    now = DateTime.utc_now()

    %{
      "id" => @dev_tenant_id,
      "name" => "Dev Tenant",
      "slug" => "dev-tenant",
      "status" => "active",
      "metadata" => %{
        "subscription" => %{
          "status" => "active",
          "tier" => "enterprise"
        },
        "quotas" => %{
          "events_quota" => -1,
          "queries_quota" => -1,
          "events_used" => 0,
          "queries_used" => 0
        }
      },
      "inserted_at" => DateTime.to_iso8601(now),
      "updated_at" => DateTime.to_iso8601(now)
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
