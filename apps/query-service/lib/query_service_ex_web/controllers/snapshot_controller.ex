defmodule QueryServiceExWeb.SnapshotController do
  @moduledoc """
  Controller for snapshot-related endpoints.
  Proxies snapshot operations to the Rust Core service.
  """

  use Phoenix.Controller, formats: [:json]

  alias QueryServiceEx.Infrastructure.Adapters.RustCoreClient

  action_fallback(QueryServiceExWeb.FallbackController)

  @doc """
  List all snapshots, optionally filtered by entity_id.
  """
  def index(conn, params) do
    entity_id = params["entity_id"]

    case RustCoreClient.list_snapshots(entity_id) do
      {:ok, %{"snapshots" => snapshots, "total" => total}} ->
        json(conn, %{data: snapshots, count: length(snapshots), total: total})

      {:ok, body} when is_list(body) ->
        json(conn, %{data: body, count: length(body), total: length(body)})

      {:ok, body} when is_map(body) ->
        json(conn, body)

      {:error, reason} ->
        conn
        |> put_status(:bad_request)
        |> json(%{error: to_string(reason)})
    end
  end
end
