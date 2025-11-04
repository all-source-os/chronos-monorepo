defmodule QueryServiceExWeb.HealthController do
  @moduledoc """
  Health check endpoint for monitoring and load balancers.
  """

  use Phoenix.Controller, formats: [:json]

  alias QueryServiceEx.Infrastructure.Adapters.RustCoreClient

  @doc """
  Health check endpoint.

  Returns service status and backend connectivity.
  """
  def show(conn, _params) do
    # Check backend connectivity
    backend_status =
      case RustCoreClient.health_check() do
        {:ok, _health} -> "healthy"
        {:error, _} -> "degraded"
      end

    response = %{
      service: "query_service_ex",
      status: "healthy",
      backend: backend_status,
      timestamp: DateTime.utc_now() |> DateTime.to_iso8601(),
      version: Application.spec(:query_service_ex, :vsn) |> to_string()
    }

    json(conn, response)
  end
end
