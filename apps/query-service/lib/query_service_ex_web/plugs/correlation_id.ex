defmodule QueryServiceExWeb.Plugs.CorrelationId do
  @moduledoc """
  Plug for handling correlation IDs for distributed tracing.

  Extracts or generates a correlation ID from the request headers and adds it to:
  - The conn assigns for downstream use
  - The Logger metadata for structured logging
  - The response headers for tracing across services

  The correlation ID follows the format: `x-correlation-id` header.
  If not present, a new UUID is generated.
  """

  import Plug.Conn
  require Logger

  @behaviour Plug

  @correlation_id_header "x-correlation-id"

  @impl true
  def init(opts), do: opts

  @impl true
  def call(conn, _opts) do
    correlation_id = get_or_generate_correlation_id(conn)

    Logger.metadata(correlation_id: correlation_id)

    conn
    |> assign(:correlation_id, correlation_id)
    |> put_resp_header(@correlation_id_header, correlation_id)
  end

  defp get_or_generate_correlation_id(conn) do
    case get_req_header(conn, @correlation_id_header) do
      [correlation_id | _] -> correlation_id
      [] -> generate_correlation_id()
    end
  end

  defp generate_correlation_id do
    # Generate a UUID-like correlation ID
    :crypto.strong_rand_bytes(16)
    |> Base.encode16(case: :lower)
    |> format_as_uuid()
  end

  defp format_as_uuid(
         <<a::binary-size(8), b::binary-size(4), c::binary-size(4), d::binary-size(4),
           e::binary-size(12)>>
       ) do
    "#{a}-#{b}-#{c}-#{d}-#{e}"
  end
end
