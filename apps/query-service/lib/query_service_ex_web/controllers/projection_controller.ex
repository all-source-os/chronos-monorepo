defmodule QueryServiceExWeb.ProjectionController do
  @moduledoc """
  Controller for managing projections.
  """

  use Phoenix.Controller, formats: [:json]

  alias QueryServiceEx.Infrastructure.Adapters.RustCoreClient

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

  @doc """
  Delete a projection.
  """
  def delete(conn, %{"name" => _name}) do
    # Note: RustCoreClient doesn't have delete_projection yet
    conn
    |> put_status(:not_implemented)
    |> json(%{error: "Projection deletion not yet implemented"})
  end

  @doc """
  Get projection state.
  """
  def get_state(conn, %{"name" => _name}) do
    # Note: This would query the projection state from storage
    conn
    |> put_status(:not_implemented)
    |> json(%{error: "Projection state query not yet implemented"})
  end

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
