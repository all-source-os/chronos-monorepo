defmodule QueryServiceExWeb.BillingController do
  @moduledoc """
  Billing controller — serves billing status from Core events, redirects mutations to CP.

  - `GET /api/billing/status` — derives current subscription state from billing events in Core
  - Checkout, portal, overage — redirect to Control Plane (needs LemonSqueezy API access)
  """

  use Phoenix.Controller, formats: [:json]
  use OpenApiSpex.ControllerSpecs

  alias QueryServiceEx.Infrastructure.Adapters.RustCoreClient
  alias QueryServiceExWeb.Schemas.Common

  require Logger

  tags(["Billing"])

  # Default quotas per tier. Numbers come from
  # docs/marketing/PRICING_DECISION_2026-04.md and match the Go entity map in
  # apps/control-plane/internal/domain/entities/subscription.go. Enterprise
  # uses -1 to signal unlimited. The legacy "team" string is accepted as an
  # alias for "growth" so old billing events don't break lookups.
  @tier_quotas %{
    "free" => %{events_quota: 100_000, queries_quota: 10_000},
    "pro" => %{events_quota: 1_000_000, queries_quota: 100_000},
    "growth" => %{events_quota: 10_000_000, queries_quota: 1_000_000},
    "team" => %{events_quota: 10_000_000, queries_quota: 1_000_000},
    "enterprise" => %{events_quota: -1, queries_quota: -1}
  }

  @doc """
  Returns current subscription state derived from billing events stored in Core.
  GET /api/billing/status
  """

  def status(conn, _params) do
    tenant_id = conn.assigns[:tenant_id]

    case query_billing_state(tenant_id) do
      {:ok, state} ->
        json(conn, state)

      {:error, reason} ->
        Logger.warning("[BillingController] Failed to query billing state: #{inspect(reason)}")
        # Return default free tier on error
        json(conn, default_billing_state(tenant_id))
    end
  end

  defp query_billing_state(tenant_id) do
    case RustCoreClient.query_events(tenant_id, %{
           event_type: "billing.*",
           limit: 1,
           sort: "desc"
         }) do
      {:ok, [latest | _]} ->
        {:ok, derive_state(tenant_id, latest)}

      {:ok, []} ->
        {:ok, default_billing_state(tenant_id)}

      {:error, reason} ->
        {:error, reason}
    end
  end

  @doc false
  def derive_state(tenant_id, event) do
    payload = event["payload"] || %{}
    tier = payload["tier"] || "free"
    quotas = Map.get(@tier_quotas, tier, @tier_quotas["free"])

    %{
      tenant_id: tenant_id,
      tier: tier,
      status: payload["status"] || "active",
      billing_period: payload["billing_period"] || "monthly",
      payment_provider: payload["payment_provider"] || "lemonsqueezy",
      subscription_id: payload["subscription_id"],
      events_quota: quotas[:events_quota],
      queries_quota: quotas[:queries_quota],
      events_used: 0,
      queries_used: 0,
      last_updated: event["timestamp"]
    }
  end

  @doc false
  def default_billing_state(tenant_id) do
    quotas = @tier_quotas["free"]

    %{
      tenant_id: tenant_id,
      tier: "free",
      status: "active",
      billing_period: nil,
      payment_provider: nil,
      subscription_id: nil,
      events_quota: quotas[:events_quota],
      queries_quota: quotas[:queries_quota],
      events_used: 0,
      queries_used: 0,
      last_updated: nil
    }
  end

  operation(:checkout,
    summary: "Create checkout (moved)",
    description:
      "**Moved to Control Plane.** Returns 301 redirect to `POST /api/v1/billing/checkout` on the Control Plane.",
    responses: [
      moved_permanently: {"Moved to Control Plane", "application/json", Common.Error}
    ]
  )

  def checkout(conn, _params) do
    redirect_to_cp(conn, "/api/v1/billing/checkout")
  end

  operation(:portal,
    summary: "Customer portal (moved)",
    description:
      "**Moved to Control Plane.** Returns 301 redirect to `GET /api/v1/billing/portal` on the Control Plane.",
    responses: [
      moved_permanently: {"Moved to Control Plane", "application/json", Common.Error}
    ]
  )

  def portal(conn, _params) do
    redirect_to_cp(conn, "/api/v1/billing/portal")
  end

  operation(:overage,
    summary: "Overage summary (moved)",
    description:
      "**Moved to Control Plane.** Returns 301 redirect to `GET /api/v1/billing/overage` on the Control Plane.",
    responses: [
      moved_permanently: {"Moved to Control Plane", "application/json", Common.Error}
    ]
  )

  def overage(conn, _params) do
    redirect_to_cp(conn, "/api/v1/billing/overage")
  end

  operation(:enable_overage,
    summary: "Enable overage (moved)",
    description:
      "**Moved to Control Plane.** Returns 301 redirect to `POST /api/v1/billing/overage/enable` on the Control Plane.",
    responses: [
      moved_permanently: {"Moved to Control Plane", "application/json", Common.Error}
    ]
  )

  def enable_overage(conn, _params) do
    redirect_to_cp(conn, "/api/v1/billing/overage/enable")
  end

  operation(:disable_overage,
    summary: "Disable overage (moved)",
    description:
      "**Moved to Control Plane.** Returns 301 redirect to `POST /api/v1/billing/overage/disable` on the Control Plane.",
    responses: [
      moved_permanently: {"Moved to Control Plane", "application/json", Common.Error}
    ]
  )

  def disable_overage(conn, _params) do
    redirect_to_cp(conn, "/api/v1/billing/overage/disable")
  end

  operation(:projected_charges,
    summary: "Projected charges (moved)",
    description:
      "**Moved to Control Plane.** Returns 301 redirect to `GET /api/v1/billing/projected-charges` on the Control Plane.",
    responses: [
      moved_permanently: {"Moved to Control Plane", "application/json", Common.Error}
    ]
  )

  def projected_charges(conn, _params) do
    redirect_to_cp(conn, "/api/v1/billing/projected-charges")
  end

  defp redirect_to_cp(conn, cp_path) do
    base = System.get_env("MGMT_PLANE_URL") || ""
    location = base <> cp_path

    Logger.warning("[BillingController] Redirecting deprecated billing endpoint to Control Plane",
      from: conn.request_path,
      to: location,
      correlation_id: conn.assigns[:correlation_id]
    )

    conn
    |> put_resp_header("location", location)
    |> put_status(301)
    |> json(%{
      error: %{
        code: "moved_permanently",
        message: "Billing endpoints have moved to Control Plane",
        location: location
      }
    })
  end
end
