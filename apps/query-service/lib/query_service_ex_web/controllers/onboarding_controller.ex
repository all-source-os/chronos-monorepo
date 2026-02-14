defmodule QueryServiceExWeb.OnboardingController do
  @moduledoc """
  Controller for customer onboarding flow.

  Onboarding has been moved to the Control Plane (Go service).
  All endpoints return 410 Gone.
  """

  use Phoenix.Controller, formats: [:json]

  require Logger

  @gone_message "Onboarding has moved to the Control Plane. Use the Control Plane API instead."

  def show(conn, _params), do: gone(conn)
  def steps(conn, _params), do: gone(conn)
  def complete_step(conn, _params), do: gone(conn)
  def skip(conn, _params), do: gone(conn)
  def reset(conn, _params), do: gone(conn)

  defp gone(conn) do
    conn
    |> put_status(:gone)
    |> json(%{error: %{code: "gone", message: @gone_message}})
  end
end
