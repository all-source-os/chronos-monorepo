defmodule QueryServiceExWeb.EventController do
  @moduledoc """
  Controller for event-related endpoints.

  Provides CRUD operations and querying for events.
  """

  use Phoenix.Controller, formats: [:json]

  alias QueryServiceEx.Infrastructure.Adapters.RustCoreClient

  @doc """
  List events with optional filters.

  Query params:
  - entity_id: Filter by entity ID
  - event_type: Filter by event type
  - limit: Maximum number of results (default: 100)
  - offset: Pagination offset (default: 0)
  """
  def index(conn, params) do
    query_params = %{
      entity_id: params["entity_id"],
      event_type: params["event_type"],
      limit: parse_int(params["limit"], 100),
      offset: parse_int(params["offset"], 0)
    }

    case RustCoreClient.query_events(query_params) do
      {:ok, events} ->
        json(conn, %{data: events, count: length(events)})

      {:error, reason} ->
        conn
        |> put_status(:bad_request)
        |> json(%{error: to_string(reason)})
    end
  end

  @doc """
  Get a specific event by ID.
  """
  def show(conn, %{"id" => id}) do
    # Note: RustCoreClient doesn't have get_event_by_id, so we query by entity_id
    # This is a limitation that should be addressed in the backend
    conn
    |> put_status(:not_implemented)
    |> json(%{error: "Direct event ID lookup not yet implemented"})
  end

  @doc """
  Create a new event.

  Body:
  {
    "entity_id": "string",
    "event_type": "string",
    "payload": {}
  }
  """
  def create(conn, params) do
    event = %{
      entity_id: params["entity_id"],
      event_type: params["event_type"],
      payload: params["payload"] || %{}
    }

    case RustCoreClient.create_event(event) do
      {:ok, created_event} ->
        conn
        |> put_status(:created)
        |> json(%{data: created_event})

      {:error, reason} ->
        conn
        |> put_status(:unprocessable_entity)
        |> json(%{error: to_string(reason)})
    end
  end

  @doc """
  Create multiple events in a batch.

  Body:
  {
    "events": [
      {"entity_id": "...", "event_type": "...", "payload": {}},
      ...
    ]
  }
  """
  def create_batch(conn, %{"events" => events}) when is_list(events) do
    case RustCoreClient.create_event_batch(events) do
      {:ok, created_events} ->
        conn
        |> put_status(:created)
        |> json(%{data: created_events, count: length(created_events)})

      {:error, reason} ->
        conn
        |> put_status(:unprocessable_entity)
        |> json(%{error: to_string(reason)})
    end
  end

  def create_batch(conn, _params) do
    conn
    |> put_status(:bad_request)
    |> json(%{error: "Expected 'events' array in request body"})
  end

  @doc """
  Get events for a specific entity.
  """
  def by_entity(conn, %{"entity_id" => entity_id}) do
    case RustCoreClient.get_events_by_entity(entity_id) do
      {:ok, events} ->
        json(conn, %{data: events, count: length(events), entity_id: entity_id})

      {:error, reason} ->
        conn
        |> put_status(:bad_request)
        |> json(%{error: to_string(reason)})
    end
  end

  @doc """
  Get events of a specific type.
  """
  def by_type(conn, %{"event_type" => event_type}) do
    case RustCoreClient.get_events_by_type(event_type) do
      {:ok, events} ->
        json(conn, %{data: events, count: length(events), event_type: event_type})

      {:error, reason} ->
        conn
        |> put_status(:bad_request)
        |> json(%{error: to_string(reason)})
    end
  end

  # Helper to parse integer parameters with defaults
  defp parse_int(nil, default), do: default
  defp parse_int(value, default) when is_binary(value) do
    case Integer.parse(value) do
      {int, _} -> int
      :error -> default
    end
  end
  defp parse_int(value, _default) when is_integer(value), do: value
  defp parse_int(_, default), do: default
end
