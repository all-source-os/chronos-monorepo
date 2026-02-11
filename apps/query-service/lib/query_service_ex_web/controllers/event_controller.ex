defmodule QueryServiceExWeb.EventController do
  @moduledoc """
  Controller for event-related endpoints.

  Provides CRUD operations and querying for events.
  Uses FallbackController for consistent error handling.
  Tracks usage metering for billable operations.
  """

  use Phoenix.Controller, formats: [:json]
  use OpenApiSpex.ControllerSpecs

  alias QueryServiceEx.Infrastructure.Adapters.RustCoreClient
  alias QueryServiceEx.UsageMeter
  alias QueryServiceExWeb.Schemas.Common
  alias QueryServiceExWeb.Schemas.Events

  action_fallback(QueryServiceExWeb.FallbackController)

  tags(["Events"])

  operation(:index,
    summary: "List events",
    description:
      "List events with optional filters. Results are scoped to the authenticated tenant.",
    security: [%{"bearer_auth" => []}],
    parameters: [
      entity_id: [in: :query, type: :string, description: "Filter by entity ID"],
      event_type: [in: :query, type: :string, description: "Filter by event type"],
      limit: [in: :query, type: :integer, description: "Maximum number of results (default: 100)"],
      offset: [in: :query, type: :integer, description: "Pagination offset (default: 0)"]
    ],
    responses: [
      ok: {"Events list", "application/json", Events.EventListResponse},
      bad_request: {"Bad request", "application/json", Common.SimpleError},
      unauthorized: {"Unauthorized", "application/json", Common.Error}
    ]
  )

  @doc """
  List events with optional filters.

  Query params:
  - entity_id: Filter by entity ID
  - event_type: Filter by event type
  - limit: Maximum number of results (default: 100)
  - offset: Pagination offset (default: 0)

  All queries are automatically filtered by the authenticated tenant.
  """
  def index(conn, params) do
    tenant_id = get_tenant_id!(conn)

    query_params = %{
      entity_id: params["entity_id"],
      event_type: params["event_type"],
      limit: parse_int(params["limit"], 100),
      offset: parse_int(params["offset"], 0)
    }

    case RustCoreClient.query_events(tenant_id, query_params) do
      {:ok, events} ->
        json(conn, %{data: events, count: length(events)})

      {:error, reason} ->
        conn
        |> put_status(:bad_request)
        |> json(%{error: to_string(reason)})
    end
  end

  operation(:show,
    summary: "Get event by ID",
    description: "Retrieve a specific event by its ID. (Not yet implemented)",
    security: [%{"bearer_auth" => []}],
    parameters: [
      id: [in: :path, type: :string, description: "Event ID", required: true]
    ],
    responses: [
      ok: {"Event details", "application/json", Events.EventResponse},
      not_found: {"Event not found", "application/json", Common.SimpleError},
      not_implemented: {"Not implemented", "application/json", Common.SimpleError}
    ]
  )

  @doc """
  Get a specific event by ID.
  """
  def show(conn, %{"id" => _id}) do
    # Note: RustCoreClient doesn't have get_event_by_id, so we query by entity_id
    # This is a limitation that should be addressed in the backend
    conn
    |> put_status(:not_implemented)
    |> json(%{error: "Direct event ID lookup not yet implemented"})
  end

  operation(:create,
    summary: "Create event",
    description:
      "Create a new event. Events are automatically associated with the authenticated tenant.",
    security: [%{"bearer_auth" => []}],
    request_body:
      {"Event to create", "application/json", Events.CreateEventRequest, required: true},
    responses: [
      created: {"Event created", "application/json", Events.EventResponse},
      unprocessable_entity: {"Validation error", "application/json", Common.SimpleError},
      unauthorized: {"Unauthorized", "application/json", Common.Error}
    ]
  )

  @doc """
  Create a new event.

  Body:
  {
    "entity_id": "string",
    "event_type": "string",
    "payload": {}
  }

  Events are automatically associated with the authenticated tenant.
  """
  def create(conn, params) do
    tenant_id = get_tenant_id!(conn)

    event = %{
      entity_id: params["entity_id"],
      event_type: params["event_type"],
      payload: params["payload"] || %{}
    }

    case RustCoreClient.create_event(tenant_id, event) do
      {:ok, created_event} ->
        # Record usage after successful event creation
        record_event_usage(conn, 1, %{
          entity_id: event.entity_id,
          event_type: event.event_type
        })

        conn
        |> put_status(:created)
        |> json(%{data: created_event})

      {:error, reason} ->
        conn
        |> put_status(:unprocessable_entity)
        |> json(%{error: to_string(reason)})
    end
  end

  operation(:create_batch,
    summary: "Create events batch",
    description:
      "Create multiple events in a single request. All events are associated with the authenticated tenant.",
    security: [%{"bearer_auth" => []}],
    request_body:
      {"Events to create", "application/json", Events.CreateBatchRequest, required: true},
    responses: [
      created: {"Events created", "application/json", Events.BatchCreateResponse},
      bad_request: {"Invalid request", "application/json", Common.SimpleError},
      unprocessable_entity: {"Validation error", "application/json", Common.SimpleError}
    ]
  )

  @doc """
  Create multiple events in a batch.

  Body:
  {
    "events": [
      {"entity_id": "...", "event_type": "...", "payload": {}},
      ...
    ]
  }

  All events are automatically associated with the authenticated tenant.
  """
  def create_batch(conn, %{"events" => events}) when is_list(events) do
    tenant_id = get_tenant_id!(conn)

    case RustCoreClient.create_event_batch(tenant_id, events) do
      {:ok, created_events} ->
        # Record usage for all events in the batch
        event_count = length(created_events)

        record_event_usage(conn, event_count, %{
          batch: true,
          event_count: event_count
        })

        conn
        |> put_status(:created)
        |> json(%{data: created_events, count: event_count})

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

  operation(:by_entity,
    summary: "Get events by entity",
    description: "Retrieve all events for a specific entity ID.",
    security: [%{"bearer_auth" => []}],
    parameters: [
      entity_id: [in: :path, type: :string, description: "Entity ID", required: true]
    ],
    responses: [
      ok: {"Entity events", "application/json", Events.EntityEventsResponse},
      bad_request: {"Bad request", "application/json", Common.SimpleError}
    ]
  )

  @doc """
  Get events for a specific entity.

  Results are filtered to the authenticated tenant's events only.
  """
  def by_entity(conn, %{"entity_id" => entity_id}) do
    tenant_id = get_tenant_id!(conn)

    case RustCoreClient.get_events_by_entity(tenant_id, entity_id) do
      {:ok, events} ->
        json(conn, %{data: events, count: length(events), entity_id: entity_id})

      {:error, reason} ->
        conn
        |> put_status(:bad_request)
        |> json(%{error: to_string(reason)})
    end
  end

  operation(:by_type,
    summary: "Get events by type",
    description: "Retrieve all events of a specific event type.",
    security: [%{"bearer_auth" => []}],
    parameters: [
      event_type: [in: :path, type: :string, description: "Event type", required: true]
    ],
    responses: [
      ok: {"Events by type", "application/json", Events.TypeEventsResponse},
      bad_request: {"Bad request", "application/json", Common.SimpleError}
    ]
  )

  @doc """
  Get events of a specific type.

  Results are filtered to the authenticated tenant's events only.
  """
  def by_type(conn, %{"event_type" => event_type}) do
    tenant_id = get_tenant_id!(conn)

    case RustCoreClient.get_events_by_type(tenant_id, event_type) do
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

  # Gets tenant_id from connection, raises if not present (security check)
  defp get_tenant_id!(conn) do
    case conn.assigns[:tenant_id] do
      nil ->
        raise "Tenant context required but not present - security violation"

      tenant_id when is_binary(tenant_id) ->
        tenant_id
    end
  end

  # Records event usage for the current tenant
  defp record_event_usage(conn, count, metadata) do
    case conn.assigns[:current_tenant] do
      nil ->
        # No tenant context - skip metering (should not happen in normal flow)
        :ok

      tenant ->
        # Record usage asynchronously to not block the response
        # In test mode (sandbox), run synchronously to avoid connection issues
        if Application.get_env(:query_service_ex, :sql_sandbox, false) do
          UsageMeter.record_events(tenant.id, count: count, metadata: metadata)
        else
          Task.start(fn ->
            UsageMeter.record_events(tenant.id, count: count, metadata: metadata)
          end)
        end
    end
  end
end
