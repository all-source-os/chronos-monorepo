defmodule QueryServiceExWeb.Plugs.UsageEnforcement do
  @moduledoc """
  Plug that enforces usage quotas for tenant operations.

  Reads quota and usage data from the cached Core tenant metadata
  (via TenantCache) — no PostgreSQL dependency.

  ## Hard Limit Mode (default)

  Returns 402 when `events_used >= events_quota` and overage is disabled.

  ## Soft Limit Mode

  When overage billing is enabled for a tenant, allows the request through
  but adds headers indicating the tenant is in overage. Calls
  `UsageReporter.record/3` to increment the matching meter asynchronously.
  """

  import Plug.Conn

  alias QueryServiceEx.UsageReporter

  require Logger

  @doc """
  Initialize the plug with the usage type to enforce.

  Options:
  - `:type` - The usage type to check: `:events` or `:queries`
  """
  def init(opts) do
    type = Keyword.get(opts, :type, :events)

    unless type in [:events, :queries] do
      raise ArgumentError, "UsageEnforcement :type must be :events or :queries"
    end

    %{type: type}
  end

  def call(conn, %{type: type}) do
    # Community mode: skip quota enforcement entirely
    if QueryServiceEx.Edition.community?() do
      conn
    else
      enforce_quota(conn, type)
    end
  end

  defp enforce_quota(conn, type) do
    tenant = conn.assigns[:current_tenant]

    cond do
      is_nil(tenant) ->
        conn

      demo_tenant?(tenant) ->
        # Demo tenants bypass quota enforcement but still track usage
        increment_usage(tenant, type)
        conn

      not quota_exceeded?(tenant, type) ->
        # Under quota — increment usage async and pass through
        increment_usage(tenant, type)
        conn

      overage_enabled?(tenant) ->
        # Soft limit: allow with overage headers, increment async
        increment_usage(tenant, type)
        allow_with_overage_warning(conn, tenant, type)

      true ->
        # Hard limit: block
        send_quota_exceeded(conn, tenant, type)
    end
  end

  # Quota checking — reads from Core tenant metadata

  defp quota_exceeded?(tenant, type) do
    quota = get_quota(tenant, type)
    used = get_used(tenant, type)

    if quota < 0 do
      # unlimited (-1)
      false
    else
      used >= quota
    end
  end

  defp get_quota(tenant, :events), do: get_quota_field(tenant, "events_quota")
  defp get_quota(tenant, :queries), do: get_quota_field(tenant, "queries_quota")

  defp get_used(tenant, :events), do: get_quota_field(tenant, "events_used")
  defp get_used(tenant, :queries), do: get_quota_field(tenant, "queries_used")

  defp get_quota_field(tenant, field) do
    val =
      get_in_metadata(tenant, ["quotas", field]) ||
        get_in_metadata(tenant, [:quotas, String.to_existing_atom(field)])

    to_integer(val)
  rescue
    ArgumentError -> 0
  end

  defp get_in_metadata(%{"metadata" => metadata}, path) when is_map(metadata) do
    get_in(metadata, path)
  end

  defp get_in_metadata(%{metadata: metadata}, path) when is_map(metadata) do
    get_in(metadata, path)
  end

  defp get_in_metadata(_, _), do: nil

  defp to_integer(nil), do: 0
  defp to_integer(v) when is_integer(v), do: v
  defp to_integer(v) when is_float(v), do: trunc(v)
  defp to_integer(v) when is_binary(v), do: String.to_integer(v)

  # Demo tenant checking — Core returns is_demo at the top level of the tenant response

  defp demo_tenant?(%{"is_demo" => true}), do: true
  defp demo_tenant?(%{is_demo: true}), do: true
  defp demo_tenant?(_), do: false

  # Overage checking

  defp overage_enabled?(tenant) do
    val =
      get_in_metadata(tenant, ["overage", "enabled"]) ||
        get_in_metadata(tenant, [:overage, :enabled])

    val == true
  end

  # Usage increment — async via UsageReporter

  defp increment_usage(tenant, type) do
    tid = tenant_id(tenant)

    # Meter into the counter for this route's type (events vs queries). `type`
    # is already validated to :events | :queries in init/1, so it maps 1:1 to
    # the Core meter — passing it keeps the two dashboard quota bars distinct
    # instead of conflating every increment into events_used.
    if tid do
      UsageReporter.record(tid, 1, type)
    end
  end

  # Soft limit mode: allow through with overage headers

  defp allow_with_overage_warning(conn, tenant, type) do
    quota = get_quota(tenant, type)
    used = get_used(tenant, type)
    overage_units = max(used - quota, 0)

    Logger.info(
      "[UsageEnforcement] Tenant #{tenant_id(tenant)} in overage for #{type}: #{used}/#{quota} (overage billing enabled)"
    )

    conn
    |> put_resp_header("x-usage-overage", "true")
    |> put_resp_header("x-usage-type", to_string(type))
    |> put_resp_header("x-usage-used", to_string(used))
    |> put_resp_header("x-usage-quota", to_string(quota))
    |> put_resp_header("x-overage-units", to_string(overage_units))
    |> assign(:in_overage, true)
    |> assign(:overage_type, type)
  end

  # Hard limit mode: block with 402

  defp send_quota_exceeded(conn, tenant, type) do
    quota = get_quota(tenant, type)
    used = get_used(tenant, type)

    Logger.info(
      "[UsageEnforcement] Tenant #{tenant_id(tenant)} exceeded #{type} quota: #{used}/#{quota}"
    )

    conn
    |> put_status(:payment_required)
    |> Phoenix.Controller.json(%{
      error: %{
        code: "quota_exceeded",
        message: "You have exceeded your #{type} quota for this billing period.",
        usage_type: type,
        used: used,
        quota: quota,
        upgrade_url: "/billing/upgrade",
        enable_overage_url: "/billing/enable-overage"
      }
    })
    |> halt()
  end

  defp tenant_id(%{id: id}), do: id
  defp tenant_id(%{"id" => id}), do: id
  defp tenant_id(_), do: nil
end
