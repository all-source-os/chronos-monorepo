defmodule QueryServiceExWeb.Endpoint do
  @moduledoc """
  Phoenix endpoint for the Query Service API.

  Provides RESTful HTTP endpoints for querying events, managing projections,
  and executing queries through the DSL.
  """

  use Phoenix.Endpoint, otp_app: :query_service_ex

  # Serve at "/" the static files from "priv/static" directory.
  #
  # You should set gzip to true if you are running phx.digest
  # when deploying your static files in production.
  plug(Plug.Static,
    at: "/",
    from: :query_service_ex,
    gzip: false,
    only: QueryServiceExWeb.static_paths()
  )

  # Code reloading can be explicitly enabled under the
  # :code_reloader configuration of your endpoint.
  if code_reloading? do
    plug(Phoenix.CodeReloader)
  end

  plug(Plug.RequestId)
  plug(QueryServiceExWeb.Plugs.CorrelationId)
  plug(Plug.Telemetry, event_prefix: [:phoenix, :endpoint])
  plug(QueryServiceExWeb.Plugs.RequestLogger)

  plug(Plug.Parsers,
    parsers: [:json],
    pass: ["*/*"],
    json_decoder: Jason
  )

  plug(Plug.MethodOverride)
  plug(Plug.Head)
  plug(QueryServiceExWeb.Router)
end
