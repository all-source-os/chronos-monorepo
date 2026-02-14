defmodule QueryServiceExWeb.BillingController do
  @moduledoc """
  Redirect controller for deprecated billing endpoints.

  All billing operations have moved to the Control Plane.
  These endpoints return 301 Moved Permanently with a Location header
  pointing to the equivalent Control Plane endpoint.
  """

  use Phoenix.Controller, formats: [:json]
  use OpenApiSpex.ControllerSpecs

  alias QueryServiceExWeb.Schemas.Common

  require Logger

  tags(["Billing"])

  operation(:checkout,
    summary: "Create checkout (moved)",
    description:
      "**Moved to Control Plane.** Returns 301 redirect to `POST /api/v1/billing/checkout` on the Control Plane.",
    responses: [
      moved_permanently:
        {"Moved to Control Plane", "application/json", Common.Error}
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
      moved_permanently:
        {"Moved to Control Plane", "application/json", Common.Error}
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
      moved_permanently:
        {"Moved to Control Plane", "application/json", Common.Error}
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
      moved_permanently:
        {"Moved to Control Plane", "application/json", Common.Error}
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
      moved_permanently:
        {"Moved to Control Plane", "application/json", Common.Error}
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
      moved_permanently:
        {"Moved to Control Plane", "application/json", Common.Error}
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
