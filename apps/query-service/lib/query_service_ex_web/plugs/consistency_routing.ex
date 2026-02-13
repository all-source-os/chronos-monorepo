defmodule QueryServiceExWeb.Plugs.ConsistencyRouting do
  @moduledoc """
  Plug that parses the X-Consistency request header to control read routing.

  When `X-Consistency: strong` is set, reads are routed to the leader node
  for read-your-writes consistency. Otherwise, reads use the default
  round-robin across healthy followers (eventual consistency).

  ## Supported values
    - `strong` — route to leader
    - `eventual` (default) — round-robin across followers
  """

  import Plug.Conn

  def init(opts), do: opts

  def call(conn, _opts) do
    case get_req_header(conn, "x-consistency") do
      ["strong"] -> assign(conn, :consistency, :strong)
      _ -> assign(conn, :consistency, :eventual)
    end
  end
end
