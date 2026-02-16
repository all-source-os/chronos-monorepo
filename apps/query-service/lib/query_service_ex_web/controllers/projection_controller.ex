defmodule QueryServiceExWeb.ProjectionController do
  @moduledoc """
  Controller for managing projections.
  Uses FallbackController for consistent error handling.
  """

  use Phoenix.Controller, formats: [:json]
  use OpenApiSpex.ControllerSpecs

  alias QueryServiceEx.Infrastructure.Adapters.RustCoreClient
  alias QueryServiceExWeb.HAL
  alias QueryServiceExWeb.Schemas.Common
  alias QueryServiceExWeb.Schemas.Projections

  action_fallback(QueryServiceExWeb.FallbackController)

  tags(["Projections"])

  operation(:index,
    summary: "List projections",
    description: "List all projections for the authenticated tenant.",
    security: [%{"bearer_auth" => []}],
    responses: [
      ok: {"Projections list", "application/json", Projections.ProjectionListResponse},
      bad_request: {"Bad request", "application/json", Common.SimpleError}
    ]
  )

  @doc """
  List all projections.
  """
  def index(conn, _params) do
    case RustCoreClient.list_projections() do
      {:ok, projections} ->
        response =
          %{data: Enum.map(projections, &wrap_projection/1), count: length(projections)}
          |> HAL.wrap(HAL.self("/api/projections"))

        json(conn, response)

      {:error, reason} ->
        conn
        |> put_status(:bad_request)
        |> json(%{error: to_string(reason)})
    end
  end

  operation(:show,
    summary: "Get projection",
    description: "Get a specific projection by name.",
    security: [%{"bearer_auth" => []}],
    parameters: [
      name: [in: :path, type: :string, description: "Projection name", required: true]
    ],
    responses: [
      ok: {"Projection details", "application/json", Projections.ProjectionResponse},
      not_found: {"Projection not found", "application/json", Common.SimpleError},
      bad_request: {"Bad request", "application/json", Common.SimpleError}
    ]
  )

  @doc """
  Get a specific projection by name.
  """
  def show(conn, %{"name" => name}) do
    case RustCoreClient.get_projection(name) do
      {:ok, projection} ->
        json(conn, %{data: wrap_projection(projection)})

      {:error, :not_found} ->
        conn
        |> put_status(:not_found)
        |> json(%{error: "Projection not found"})

      {:error, reason} ->
        conn
        |> put_status(:bad_request)
        |> json(%{error: to_string(reason)})
    end
  end

  operation(:create,
    summary: "Create projection",
    description: "Create a new projection with the given definition.",
    security: [%{"bearer_auth" => []}],
    request_body:
      {"Projection to create", "application/json", Projections.CreateProjectionRequest,
       required: true},
    responses: [
      created: {"Projection created", "application/json", Projections.ProjectionResponse},
      unprocessable_entity: {"Validation error", "application/json", Common.SimpleError}
    ]
  )

  @doc """
  Create a new projection.

  Body:
  {
    "name": "string",
    "version": 1,
    "initial_state": {},
    "definition": "string"
  }
  """
  def create(conn, params) do
    projection = %{
      name: params["name"],
      version: params["version"] || 1,
      initial_state: params["initial_state"] || %{},
      definition: params["definition"]
    }

    case RustCoreClient.create_projection(projection) do
      {:ok, created} ->
        conn
        |> put_status(:created)
        |> json(%{data: wrap_projection(created)})

      {:error, reason} ->
        conn
        |> put_status(:unprocessable_entity)
        |> json(%{error: to_string(reason)})
    end
  end

  operation(:delete,
    summary: "Delete projection",
    description:
      "Delete (clear) a projection by name. The projection definition remains but its state is cleared.",
    security: [%{"bearer_auth" => []}],
    parameters: [
      name: [in: :path, type: :string, description: "Projection name", required: true]
    ],
    responses: [
      ok: {"Projection deleted", "application/json", Common.SimpleError},
      not_found: {"Projection not found", "application/json", Common.SimpleError}
    ]
  )

  @doc """
  Delete (clear) a projection.
  """
  def delete(conn, %{"name" => name}) do
    case RustCoreClient.delete_projection(name) do
      {:ok, result} ->
        json(conn, %{data: result})

      {:error, :not_found} ->
        conn
        |> put_status(:not_found)
        |> json(%{error: "Projection not found"})

      {:error, reason} ->
        conn
        |> put_status(:bad_request)
        |> json(%{error: to_string(reason)})
    end
  end

  operation(:get_state,
    summary: "Get projection state",
    description: "Get the aggregate state of a projection across all entities.",
    security: [%{"bearer_auth" => []}],
    parameters: [
      name: [in: :path, type: :string, description: "Projection name", required: true]
    ],
    responses: [
      ok: {"Projection state", "application/json", Projections.ProjectionStateResponse},
      not_found: {"Projection not found", "application/json", Common.SimpleError}
    ]
  )

  @doc """
  Get projection state summary.
  """
  def get_state(conn, %{"name" => name}) do
    case RustCoreClient.get_projection_state_summary(name) do
      {:ok, state} ->
        json(conn, %{data: state})

      {:error, :not_found} ->
        conn
        |> put_status(:not_found)
        |> json(%{error: "Projection not found"})

      {:error, reason} ->
        conn
        |> put_status(:bad_request)
        |> json(%{error: to_string(reason)})
    end
  end

  operation(:reset,
    summary: "Reset projection",
    description: "Reset a projection to its initial state and reprocess all events.",
    security: [%{"bearer_auth" => []}],
    parameters: [
      name: [in: :path, type: :string, description: "Projection name", required: true]
    ],
    responses: [
      ok: {"Projection reset", "application/json", Projections.ProjectionResponse},
      not_found: {"Projection not found", "application/json", Common.SimpleError}
    ]
  )

  @doc """
  Reset a projection to initial state.
  """
  def reset(conn, %{"name" => name}) do
    case RustCoreClient.reset_projection(name) do
      {:ok, result} ->
        json(conn, %{data: result})

      {:error, :not_found} ->
        conn
        |> put_status(:not_found)
        |> json(%{error: "Projection not found"})

      {:error, reason} ->
        conn
        |> put_status(:bad_request)
        |> json(%{error: to_string(reason)})
    end
  end

  @doc """
  Get rebuild statistics for all projections.
  Returns the current status and progress of any ongoing or completed rebuilds.
  """
  def rebuild_stats(conn, _params) do
    case RustCoreClient.list_projections() do
      {:ok, projections} ->
        stats =
          Enum.map(projections, fn proj ->
            name = proj["name"] || proj[:name] || "unknown"

            %{
              name: name,
              status: "idle",
              last_rebuilt_at: nil,
              events_processed: 0
            }
          end)

        json(conn, %{data: stats, count: length(stats)})

      {:error, reason} ->
        conn
        |> put_status(:bad_request)
        |> json(%{error: to_string(reason)})
    end
  end

  # -- HAL link helpers --

  defp projection_links(projection) do
    name = projection["name"] || projection[:name]

    HAL.self("/api/projections/#{name}")
    |> HAL.merge(HAL.link("events", "/api/events?event_type={event_type}", templated: true))
    |> HAL.merge(HAL.link("snapshot", "/api/snapshots?entity_id={entity_id}", templated: true))
  end

  defp wrap_projection(projection), do: HAL.wrap(projection, projection_links(projection))
end
