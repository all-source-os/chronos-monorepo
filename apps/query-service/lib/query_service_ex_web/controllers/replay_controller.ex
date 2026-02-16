defmodule QueryServiceExWeb.ReplayController do
  @moduledoc """
  Controller for event replay operations.

  Proxies replay requests to Core's replay API, allowing tenants to
  replay events from a specific point in time for projection rebuilds
  or event re-processing.
  """

  use Phoenix.Controller, formats: [:json]

  alias QueryServiceEx.Infrastructure.Adapters.RustCoreClient

  action_fallback(QueryServiceExWeb.FallbackController)

  require Logger

  @doc """
  List all replays.

  GET /api/replay
  """
  def index(conn, _params) do
    case RustCoreClient.list_replays() do
      {:ok, %{replays: replays, total: total}} ->
        json(conn, %{data: replays, total: total})

      {:ok, body} ->
        json(conn, %{data: body})

      {:error, reason} ->
        conn
        |> put_status(:bad_request)
        |> json(%{error: to_string(reason)})
    end
  end

  @doc """
  Start a new event replay.

  POST /api/replay

  Body:
    - from_timestamp: Start time (ISO 8601, optional)
    - to_timestamp: End time (ISO 8601, optional)
    - event_type: Filter by event type (optional)
    - entity_id: Filter by entity ID (optional)
    - projection_name: Target projection to rebuild (optional)
    - config: Replay config overrides (optional)
      - batch_size: Events per batch (default 1000)
      - parallel: Enable parallel processing (default false)
  """
  def create(conn, params) do
    replay_params =
      %{}
      |> maybe_put(:from_timestamp, params["from_timestamp"])
      |> maybe_put(:to_timestamp, params["to_timestamp"])
      |> maybe_put(:event_type, params["event_type"])
      |> maybe_put(:entity_id, params["entity_id"])
      |> maybe_put(:projection_name, params["projection_name"])
      |> maybe_put(:config, params["config"])

    case RustCoreClient.start_replay(replay_params) do
      {:ok, replay} ->
        conn
        |> put_status(:ok)
        |> json(%{data: replay})

      {:error, reason} ->
        conn
        |> put_status(:bad_request)
        |> json(%{error: to_string(reason)})
    end
  end

  @doc """
  Get replay progress.

  GET /api/replay/:id
  """
  def show(conn, %{"id" => replay_id}) do
    case RustCoreClient.get_replay(replay_id) do
      {:ok, replay} ->
        json(conn, %{data: replay})

      {:error, _reason} ->
        conn
        |> put_status(:not_found)
        |> json(%{error: "Replay not found"})
    end
  end

  @doc """
  Cancel a running replay.

  POST /api/replay/:id/cancel
  """
  def cancel(conn, %{"id" => replay_id}) do
    case RustCoreClient.cancel_replay(replay_id) do
      {:ok, body} ->
        json(conn, %{data: body})

      {:error, _reason} ->
        conn
        |> put_status(:not_found)
        |> json(%{error: "Replay not found"})
    end
  end

  @doc """
  Delete a completed or failed replay.

  DELETE /api/replay/:id
  """
  def delete(conn, %{"id" => replay_id}) do
    case RustCoreClient.delete_replay(replay_id) do
      {:ok, _body} ->
        conn
        |> put_status(:ok)
        |> json(%{deleted: true})

      {:error, _reason} ->
        conn
        |> put_status(:not_found)
        |> json(%{error: "Replay not found"})
    end
  end

  # -------------------------------------------------------------------
  # Helpers
  # -------------------------------------------------------------------

  defp maybe_put(map, _key, nil), do: map
  defp maybe_put(map, key, value), do: Map.put(map, key, value)
end
