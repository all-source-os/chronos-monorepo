defmodule QueryServiceExWeb.Plugs.InternalApiKey do
  @moduledoc """
  Plug that authenticates internal service-to-service requests using a shared API key.

  Expects the key in the `x-internal-api-key` header. Compares against the
  `INTERNAL_API_KEY` environment variable using constant-time comparison.

  Returns 401 if the key is missing, invalid, or the server has no key configured.
  """

  import Plug.Conn

  require Logger

  def init(opts), do: opts

  def call(conn, _opts) do
    expected_key = System.get_env("INTERNAL_API_KEY")
    provided_key = get_req_header(conn, "x-internal-api-key") |> List.first()

    cond do
      is_nil(expected_key) or expected_key == "" ->
        Logger.warning("[InternalApiKey] INTERNAL_API_KEY not configured, rejecting request")
        reject(conn, "internal API key not configured")

      is_nil(provided_key) ->
        reject(conn, "missing x-internal-api-key header")

      Plug.Crypto.secure_compare(expected_key, provided_key) ->
        conn

      true ->
        reject(conn, "invalid API key")
    end
  end

  defp reject(conn, reason) do
    Logger.warning("[InternalApiKey] Rejected: #{reason}")

    conn
    |> put_status(:unauthorized)
    |> Phoenix.Controller.json(%{error: "unauthorized"})
    |> halt()
  end
end
