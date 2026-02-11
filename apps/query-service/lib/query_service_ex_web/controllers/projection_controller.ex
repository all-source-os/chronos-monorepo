defmodule QueryServiceExWeb.ProjectionController do
  @moduledoc """
  Controller for managing projections.
  Uses FallbackController for consistent error handling.
  """

  use Phoenix.Controller, formats: [:json]
  use OpenApiSpex.ControllerSpecs

  alias QueryServiceEx.Infrastructure.Adapters.RustCoreClient
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
        json(conn, %{data: projections, count: length(projections)})

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
        json(conn, %{data: projection})

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
        |> json(%{data: created})

      {:error, reason} ->
        conn
        |> put_status(:unprocessable_entity)
        |> json(%{error: to_string(reason)})
    end
  end

  operation(:delete,
    summary: "Delete projection",
    description: "Delete a projection by name. (Not yet implemented)",
    security: [%{"bearer_auth" => []}],
    parameters: [
      name: [in: :path, type: :string, description: "Projection name", required: true]
    ],
    responses: [
      ok: {"Projection deleted", "application/json", Common.SimpleError},
      not_found: {"Projection not found", "application/json", Common.SimpleError},
      not_implemented: {"Not implemented", "application/json", Common.SimpleError}
    ]
  )

  @doc """
  Delete a projection.
  """
  def delete(conn, %{"name" => _name}) do
    # Note: RustCoreClient doesn't have delete_projection yet
    conn
    |> put_status(:not_implemented)
    |> json(%{error: "Projection deletion not yet implemented"})
  end

  operation(:get_state,
    summary: "Get projection state",
    description: "Get the current state of a projection. (Not yet implemented)",
    security: [%{"bearer_auth" => []}],
    parameters: [
      name: [in: :path, type: :string, description: "Projection name", required: true]
    ],
    responses: [
      ok: {"Projection state", "application/json", Projections.ProjectionStateResponse},
      not_found: {"Projection not found", "application/json", Common.SimpleError},
      not_implemented: {"Not implemented", "application/json", Common.SimpleError}
    ]
  )

  @doc """
  Get projection state.
  """
  def get_state(conn, %{"name" => _name}) do
    # Note: This would query the projection state from storage
    conn
    |> put_status(:not_implemented)
    |> json(%{error: "Projection state query not yet implemented"})
  end

  operation(:reset,
    summary: "Reset projection",
    description: "Reset a projection to its initial state. (Not yet implemented)",
    security: [%{"bearer_auth" => []}],
    parameters: [
      name: [in: :path, type: :string, description: "Projection name", required: true]
    ],
    responses: [
      ok: {"Projection reset", "application/json", Projections.ProjectionResponse},
      not_found: {"Projection not found", "application/json", Common.SimpleError},
      not_implemented: {"Not implemented", "application/json", Common.SimpleError}
    ]
  )

  @doc """
  Reset a projection to initial state.
  """
  def reset(conn, %{"name" => _name}) do
    # Note: This would trigger projection reset
    conn
    |> put_status(:not_implemented)
    |> json(%{error: "Projection reset not yet implemented"})
  end
end
