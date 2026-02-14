defmodule QueryServiceExWeb.Plugs.ErrorHandler do
  @moduledoc """
  Plug for catching unhandled exceptions and returning structured error responses.

  Provides consistent error formatting for production environments including:
  - Correlation ID for tracing
  - Structured error details
  - Proper HTTP status codes
  - Telemetry emission for monitoring

  ## Error Response Format

      {
        "error": {
          "code": "internal_server_error",
          "message": "An unexpected error occurred",
          "correlation_id": "abc123-def456-...",
          "timestamp": "2024-01-01T12:00:00Z"
        }
      }

  """

  @behaviour Plug

  require Logger

  @impl true
  def init(opts), do: opts

  @impl true
  def call(conn, _opts) do
    conn
  rescue
    exception ->
      handle_exception(conn, exception, __STACKTRACE__)
  catch
    kind, reason ->
      handle_catch(conn, kind, reason, __STACKTRACE__)
  end

  @doc """
  Handle an exception and return a structured error response.
  Can be used by action_fallback in controllers.
  """
  def handle_exception(conn, exception, stacktrace \\ []) do
    correlation_id = conn.assigns[:correlation_id] || "unknown"

    {status, code, message} = exception_to_error(exception)

    log_error(exception, stacktrace, correlation_id, status)
    emit_telemetry(exception, correlation_id, status)

    send_error_response(conn, status, code, message, correlation_id)
  end

  defp handle_catch(conn, kind, reason, stacktrace) do
    correlation_id = conn.assigns[:correlation_id] || "unknown"

    Logger.error(
      "[ErrorHandler] Caught #{kind}: #{inspect(reason)}",
      correlation_id: correlation_id,
      kind: kind,
      reason: inspect(reason),
      stacktrace: Exception.format_stacktrace(stacktrace)
    )

    :telemetry.execute(
      [:query_service_ex, :http, :error],
      %{},
      %{
        correlation_id: correlation_id,
        error_type: "#{kind}",
        status: 500
      }
    )

    send_error_response(
      conn,
      500,
      "internal_server_error",
      "An unexpected error occurred",
      correlation_id
    )
  end

  defp exception_to_error(%Plug.Parsers.ParseError{}) do
    {400, "parse_error", "Invalid request body"}
  end

  defp exception_to_error(%Plug.Parsers.UnsupportedMediaTypeError{}) do
    {415, "unsupported_media_type", "Content-Type not supported"}
  end

  defp exception_to_error(%Phoenix.Router.NoRouteError{}) do
    {404, "not_found", "Route not found"}
  end

  defp exception_to_error(%ArgumentError{message: message}) do
    {400, "argument_error", message}
  end

  defp exception_to_error(%FunctionClauseError{}) do
    {400, "bad_request", "Invalid request parameters"}
  end

  defp exception_to_error(%Jason.DecodeError{}) do
    {400, "json_decode_error", "Invalid JSON in request body"}
  end

  defp exception_to_error(_exception) do
    {500, "internal_server_error", "An unexpected error occurred"}
  end

  defp log_error(exception, stacktrace, correlation_id, status) do
    level = if status >= 500, do: :error, else: :warning

    Logger.log(
      level,
      "[ErrorHandler] #{exception.__struct__}: #{Exception.message(exception)}",
      correlation_id: correlation_id,
      exception_type: exception.__struct__,
      status: status,
      stacktrace: Exception.format_stacktrace(stacktrace)
    )
  end

  defp emit_telemetry(exception, correlation_id, status) do
    :telemetry.execute(
      [:query_service_ex, :http, :error],
      %{},
      %{
        correlation_id: correlation_id,
        error_type: "#{exception.__struct__}",
        status: status
      }
    )
  end

  defp send_error_response(conn, status, code, message, correlation_id) do
    body =
      Jason.encode!(%{
        error: %{
          code: code,
          message: message,
          correlation_id: correlation_id,
          timestamp: DateTime.utc_now() |> DateTime.to_iso8601()
        }
      })

    conn
    |> Plug.Conn.put_resp_content_type("application/json")
    |> Plug.Conn.send_resp(status, body)
    |> Plug.Conn.halt()
  end
end
