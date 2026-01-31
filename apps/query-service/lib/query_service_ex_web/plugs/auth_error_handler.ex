defmodule QueryServiceExWeb.Plugs.AuthErrorHandler do
  @moduledoc """
  Error handler for Guardian authentication failures.

  Returns consistent JSON error responses for authentication errors.
  """

  @behaviour Guardian.Plug.ErrorHandler

  import Plug.Conn
  import Phoenix.Controller, only: [json: 2]

  @impl Guardian.Plug.ErrorHandler
  def auth_error(conn, {type, _reason}, _opts) do
    correlation_id = conn.assigns[:correlation_id] || "unknown"

    {status, code, message} = error_details(type)

    conn
    |> put_status(status)
    |> json(%{
      error: %{
        code: code,
        message: message,
        correlation_id: correlation_id,
        timestamp: DateTime.utc_now() |> DateTime.to_iso8601()
      }
    })
  end

  defp error_details(:unauthenticated) do
    {:unauthorized, "unauthenticated", "Authentication required"}
  end

  defp error_details(:invalid_token) do
    {:unauthorized, "invalid_token", "Invalid or expired token"}
  end

  defp error_details(:no_resource_found) do
    {:unauthorized, "user_not_found", "User not found"}
  end

  defp error_details(_type) do
    {:unauthorized, "unauthorized", "Authentication failed"}
  end
end
