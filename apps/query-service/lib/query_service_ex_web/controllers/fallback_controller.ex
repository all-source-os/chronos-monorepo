defmodule QueryServiceExWeb.FallbackController do
  @moduledoc """
  Fallback controller for handling errors from controller actions.

  Used as action_fallback in controllers to provide consistent error responses
  when actions return error tuples instead of conn structs.

  ## Handled Error Types

  - `{:error, :not_found}` - Returns 404 Not Found
  - `{:error, :circuit_open}` - Returns 503 Service Unavailable (circuit breaker)
  - `{:error, :timeout}` - Returns 504 Gateway Timeout
  - `{:error, :validation_failed, details}` - Returns 422 with validation details
  - `{:error, :unauthorized}` - Returns 401 Unauthorized
  - `{:error, :forbidden}` - Returns 403 Forbidden
  - `{:error, :bad_request, message}` - Returns 400 with custom message
  - `{:error, %Tesla.Error{}}` - Returns 502 Bad Gateway
  - `{:error, message}` (binary) - Returns appropriate status based on message
  - `{:error, reason}` (atom) - Returns status based on error type
  - `{:error, reason}` (other) - Returns 500 Internal Server Error

  ## Usage in Controllers

      defmodule MyController do
        use Phoenix.Controller
        action_fallback QueryServiceExWeb.FallbackController

        def show(conn, %{"id" => id}) do
          with {:ok, item} <- MyService.get(id) do
            json(conn, item)
          end
        end
      end

  """

  use Phoenix.Controller, formats: [:json]

  require Logger

  @doc """
  Handle error tuples from controller actions.
  """
  def call(conn, {:error, :not_found}) do
    correlation_id = conn.assigns[:correlation_id] || "unknown"

    conn
    |> put_status(:not_found)
    |> json(error_response("not_found", "Resource not found", correlation_id))
  end

  def call(conn, {:error, :circuit_open}) do
    correlation_id = conn.assigns[:correlation_id] || "unknown"

    Logger.warning("[FallbackController] Circuit breaker open", correlation_id: correlation_id)

    conn
    |> put_status(:service_unavailable)
    |> json(
      error_response(
        "service_unavailable",
        "Service temporarily unavailable, please retry later",
        correlation_id
      )
    )
  end

  def call(conn, {:error, :timeout}) do
    correlation_id = conn.assigns[:correlation_id] || "unknown"

    Logger.warning("[FallbackController] Request timeout", correlation_id: correlation_id)

    conn
    |> put_status(:gateway_timeout)
    |> json(error_response("timeout", "Request timed out", correlation_id))
  end

  def call(conn, {:error, :validation_failed, details}) do
    correlation_id = conn.assigns[:correlation_id] || "unknown"

    conn
    |> put_status(:unprocessable_entity)
    |> json(error_response("validation_failed", "Validation failed", correlation_id, details))
  end

  def call(conn, {:error, :unauthorized}) do
    correlation_id = conn.assigns[:correlation_id] || "unknown"

    conn
    |> put_status(:unauthorized)
    |> json(error_response("unauthorized", "Authentication required", correlation_id))
  end

  def call(conn, {:error, :forbidden}) do
    correlation_id = conn.assigns[:correlation_id] || "unknown"

    conn
    |> put_status(:forbidden)
    |> json(error_response("forbidden", "Access denied", correlation_id))
  end

  def call(conn, {:error, :bad_request, message}) when is_binary(message) do
    correlation_id = conn.assigns[:correlation_id] || "unknown"

    conn
    |> put_status(:bad_request)
    |> json(error_response("bad_request", message, correlation_id))
  end

  def call(conn, {:error, %Tesla.Error{reason: reason}}) do
    correlation_id = conn.assigns[:correlation_id] || "unknown"

    Logger.warning("[FallbackController] Tesla error: #{inspect(reason)}",
      correlation_id: correlation_id
    )

    conn
    |> put_status(:bad_gateway)
    |> json(error_response("backend_error", "Backend service unavailable", correlation_id))
  end

  def call(conn, {:error, message}) when is_binary(message) do
    correlation_id = conn.assigns[:correlation_id] || "unknown"

    # Check if it's an HTTP error from backend
    case parse_http_error(message) do
      {:ok, status, _body} when status >= 500 ->
        conn
        |> put_status(:bad_gateway)
        |> json(error_response("backend_error", "Backend service error", correlation_id))

      {:ok, status, _body} when status >= 400 ->
        conn
        |> put_status(status)
        |> json(error_response("backend_error", message, correlation_id))

      :error ->
        conn
        |> put_status(:internal_server_error)
        |> json(error_response("internal_error", message, correlation_id))
    end
  end

  def call(conn, {:error, reason}) when is_atom(reason) do
    correlation_id = conn.assigns[:correlation_id] || "unknown"

    Logger.warning("[FallbackController] Error: #{reason}",
      correlation_id: correlation_id,
      error: reason
    )

    {status, code, message} = atom_error_to_response(reason)

    conn
    |> put_status(status)
    |> json(error_response(code, message, correlation_id))
  end

  def call(conn, {:error, reason}) do
    correlation_id = conn.assigns[:correlation_id] || "unknown"

    Logger.error("[FallbackController] Unhandled error format: #{inspect(reason)}",
      correlation_id: correlation_id
    )

    conn
    |> put_status(:internal_server_error)
    |> json(error_response("internal_error", "An unexpected error occurred", correlation_id))
  end

  # Private helpers

  defp error_response(code, message, correlation_id, details \\ nil) do
    base = %{
      error: %{
        code: code,
        message: message,
        correlation_id: correlation_id,
        timestamp: DateTime.utc_now() |> DateTime.to_iso8601()
      }
    }

    if details do
      put_in(base, [:error, :details], details)
    else
      base
    end
  end

  defp parse_http_error(message) do
    case Regex.run(~r/^HTTP (\d+):/, message) do
      [_, status_str] ->
        status = String.to_integer(status_str)
        {:ok, status, message}

      nil ->
        :error
    end
  end

  defp atom_error_to_response(:econnrefused),
    do: {:service_unavailable, "connection_refused", "Backend service unavailable"}

  defp atom_error_to_response(:timeout), do: {:gateway_timeout, "timeout", "Request timed out"}

  defp atom_error_to_response(:closed),
    do: {:service_unavailable, "connection_closed", "Connection closed"}

  defp atom_error_to_response(:nxdomain),
    do: {:service_unavailable, "dns_error", "Cannot resolve backend service"}

  defp atom_error_to_response(reason),
    do: {:internal_server_error, "#{reason}", "An error occurred: #{reason}"}
end
