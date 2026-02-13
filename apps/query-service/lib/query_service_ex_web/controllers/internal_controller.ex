defmodule QueryServiceExWeb.InternalController do
  @moduledoc """
  Internal endpoints called by infrastructure components (e.g., allsource-sentinel).

  These endpoints are NOT authenticated — they are intended for service-to-service
  communication on the internal network only. Do not expose them publicly.
  """

  use Phoenix.Controller, formats: [:json]

  require Logger

  @doc """
  Update the leader URL at runtime.

  Called by the sentinel process during failover to notify the Query Service
  that a new Core leader has been promoted. Updates the `:core_write_url`
  application config so subsequent writes are routed to the new leader
  without requiring a restart.

  ## Request body

      {"leader_url": "http://new-leader:3900"}
  """
  def update_leader(conn, %{"leader_url" => leader_url}) when is_binary(leader_url) do
    previous_url = Application.get_env(:query_service_ex, :core_write_url)

    Application.put_env(:query_service_ex, :core_write_url, leader_url)

    Logger.warning(
      "[InternalController] Leader changed: #{inspect(previous_url)} -> #{inspect(leader_url)}"
    )

    conn
    |> put_status(:ok)
    |> json(%{status: "updated", leader_url: leader_url})
  end

  def update_leader(conn, _params) do
    conn
    |> put_status(:bad_request)
    |> json(%{error: "missing or invalid leader_url"})
  end
end
