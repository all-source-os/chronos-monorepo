defmodule QueryServiceExWeb.ErrorJSON do
  @moduledoc """
  This module is invoked by Phoenix when rendering errors in JSON format.

  Provides structured error responses with:
  - Error code for programmatic handling
  - Human-readable message
  - Correlation ID for tracing
  - Timestamp for debugging

  ## Supported Status Codes

  - 400 - Bad Request
  - 401 - Unauthorized
  - 403 - Forbidden
  - 404 - Not Found
  - 422 - Unprocessable Entity
  - 500 - Internal Server Error
  - 502 - Bad Gateway
  - 503 - Service Unavailable
  - 504 - Gateway Timeout
  """

  @doc """
  Renders error responses for JSON format requests.
  """
  def render("400.json", assigns) do
    error_response("bad_request", "Bad request", assigns)
  end

  def render("401.json", assigns) do
    error_response("unauthorized", "Authentication required", assigns)
  end

  def render("403.json", assigns) do
    error_response("forbidden", "Access denied", assigns)
  end

  def render("404.json", assigns) do
    error_response("not_found", "Resource not found", assigns)
  end

  def render("422.json", assigns) do
    error_response("unprocessable_entity", "Invalid request data", assigns)
  end

  def render("500.json", assigns) do
    error_response("internal_server_error", "An unexpected error occurred", assigns)
  end

  def render("502.json", assigns) do
    error_response("bad_gateway", "Backend service unavailable", assigns)
  end

  def render("503.json", assigns) do
    error_response("service_unavailable", "Service temporarily unavailable", assigns)
  end

  def render("504.json", assigns) do
    error_response("gateway_timeout", "Request timed out", assigns)
  end

  def render(template, assigns) do
    code = template |> String.replace(".json", "") |> String.downcase()
    message = Phoenix.Controller.status_message_from_template(template)
    error_response(code, message, assigns)
  end

  # Private helpers

  defp error_response(code, message, assigns) do
    correlation_id = get_correlation_id(assigns)

    %{
      error: %{
        code: code,
        message: message,
        correlation_id: correlation_id,
        timestamp: DateTime.utc_now() |> DateTime.to_iso8601()
      }
    }
  end

  defp get_correlation_id(assigns) do
    case assigns do
      %{conn: %{assigns: %{correlation_id: id}}} -> id
      %{correlation_id: id} -> id
      _ -> "unknown"
    end
  end
end
