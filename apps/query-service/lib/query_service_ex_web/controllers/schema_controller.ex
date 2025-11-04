defmodule QueryServiceExWeb.SchemaController do
  @moduledoc """
  Controller for event schema management.
  """

  use Phoenix.Controller, formats: [:json]

  alias QueryServiceEx.Infrastructure.Adapters.RustCoreClient

  @doc """
  List all schemas.
  """
  def index(conn, _params) do
    case RustCoreClient.list_schemas() do
      {:ok, schemas} ->
        json(conn, %{data: schemas, count: length(schemas)})

      {:error, reason} ->
        conn
        |> put_status(:bad_request)
        |> json(%{error: to_string(reason)})
    end
  end

  @doc """
  Get schema for a specific event type.

  Query params:
  - version: Optional version number
  """
  def show(conn, %{"event_type" => event_type} = params) do
    result =
      if version = params["version"] do
        RustCoreClient.get_schema(event_type, String.to_integer(version))
      else
        RustCoreClient.get_schema(event_type)
      end

    case result do
      {:ok, schema} ->
        json(conn, %{data: schema})

      {:error, :not_found} ->
        conn
        |> put_status(:not_found)
        |> json(%{error: "Schema not found"})

      {:error, reason} ->
        conn
        |> put_status(:bad_request)
        |> json(%{error: to_string(reason)})
    end
  end

  @doc """
  Register a new schema.

  Body:
  {
    "event_type": "string",
    "version": 1,
    "schema": {
      "type": "object",
      "properties": {...}
    }
  }
  """
  def register(conn, params) do
    schema = %{
      event_type: params["event_type"],
      version: params["version"] || 1,
      schema: params["schema"]
    }

    case RustCoreClient.register_schema(schema) do
      {:ok, registered} ->
        conn
        |> put_status(:created)
        |> json(%{data: registered})

      {:error, reason} ->
        conn
        |> put_status(:unprocessable_entity)
        |> json(%{error: to_string(reason)})
    end
  end
end
