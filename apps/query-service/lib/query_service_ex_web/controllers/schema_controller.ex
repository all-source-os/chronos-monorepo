defmodule QueryServiceExWeb.SchemaController do
  @moduledoc """
  Controller for event schema management.
  Uses FallbackController for consistent error handling.
  """

  use Phoenix.Controller, formats: [:json]
  use OpenApiSpex.ControllerSpecs

  alias QueryServiceEx.Infrastructure.Adapters.RustCoreClient
  alias QueryServiceExWeb.HAL
  alias QueryServiceExWeb.Schemas.Common
  alias QueryServiceExWeb.Schemas.Schemas

  action_fallback(QueryServiceExWeb.FallbackController)

  tags(["Schemas"])

  operation(:index,
    summary: "List schemas",
    description: "List all registered event schemas.",
    security: [%{"bearer_auth" => []}],
    responses: [
      ok: {"Schemas list", "application/json", Schemas.SchemaListResponse},
      bad_request: {"Bad request", "application/json", Common.SimpleError}
    ]
  )

  @doc """
  List all schemas.
  """
  def index(conn, _params) do
    case RustCoreClient.list_schemas() do
      {:ok, schemas} ->
        response =
          %{data: Enum.map(schemas, &wrap_schema/1), count: length(schemas)}
          |> HAL.wrap(HAL.self("/api/schemas"))

        json(conn, response)

      {:error, reason} ->
        conn
        |> put_status(:bad_request)
        |> json(%{error: to_string(reason)})
    end
  end

  operation(:show,
    summary: "Get schema",
    description: "Get schema for a specific event type, optionally at a specific version.",
    security: [%{"bearer_auth" => []}],
    parameters: [
      event_type: [in: :path, type: :string, description: "Event type", required: true],
      version: [in: :query, type: :integer, description: "Schema version (optional)"]
    ],
    responses: [
      ok: {"Schema details", "application/json", Schemas.SchemaResponse},
      not_found: {"Schema not found", "application/json", Common.SimpleError},
      bad_request: {"Bad request", "application/json", Common.SimpleError}
    ]
  )

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
        json(conn, %{data: wrap_schema(schema)})

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

  operation(:register,
    summary: "Register schema",
    description: "Register a new JSON Schema for validating event payloads.",
    security: [%{"bearer_auth" => []}],
    request_body:
      {"Schema to register", "application/json", Schemas.RegisterSchemaRequest, required: true},
    responses: [
      created: {"Schema registered", "application/json", Schemas.SchemaResponse},
      unprocessable_entity: {"Validation error", "application/json", Common.SimpleError}
    ]
  )

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
        |> json(%{data: wrap_schema(registered)})

      {:error, reason} ->
        conn
        |> put_status(:unprocessable_entity)
        |> json(%{error: to_string(reason)})
    end
  end

  # -- HAL link helpers --

  defp schema_links(schema) do
    event_type = schema["event_type"] || schema[:event_type]
    HAL.self("/api/schemas/#{event_type}")
  end

  defp wrap_schema(schema), do: HAL.wrap(schema, schema_links(schema))
end
