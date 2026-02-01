defmodule QueryServiceExWeb.Plugs.RawBodyReader do
  @moduledoc """
  Custom body reader that caches the raw request body.

  This is required for webhook signature verification where the HMAC
  signature is computed over the raw, unparsed request body.

  ## Usage

  Configure in Plug.Parsers:

      plug Plug.Parsers,
        parsers: [:json],
        body_reader: {QueryServiceExWeb.Plugs.RawBodyReader, :read_body, []},
        ...

  Then access via:

      raw_body = conn.assigns[:raw_body]
  """

  @doc """
  Reads the request body and caches it in conn.assigns[:raw_body].

  This function is called by Plug.Parsers when configured as the body_reader.
  """
  def read_body(conn, opts) do
    case Plug.Conn.read_body(conn, opts) do
      {:ok, body, conn} ->
        conn = update_in(conn.assigns[:raw_body], &((&1 || "") <> body))
        {:ok, body, conn}

      {:more, partial, conn} ->
        conn = update_in(conn.assigns[:raw_body], &((&1 || "") <> partial))
        {:more, partial, conn}

      {:error, reason} ->
        {:error, reason}
    end
  end
end
