defmodule QueryServiceExWeb.ApiKeyController do
  @moduledoc """
  Controller for customer self-service API key management.

  Allows customers to create, list, update, rotate, and revoke
  their API keys without needing support intervention.
  """

  use Phoenix.Controller, formats: [:json]
  use OpenApiSpex.ControllerSpecs

  alias QueryServiceEx.Accounts.Guardian
  alias QueryServiceEx.ApiKeys
  alias QueryServiceEx.ApiKeys.ApiKey
  alias QueryServiceExWeb.Schemas.ApiKeys, as: ApiKeySchemas
  alias QueryServiceExWeb.Schemas.Common

  require Logger

  action_fallback(QueryServiceExWeb.FallbackController)

  tags(["API Keys"])

  operation(:index,
    summary: "List API keys",
    description: "List all active API keys for the current tenant.",
    security: [%{"bearer_auth" => []}],
    responses: [
      ok: {"API keys list", "application/json", ApiKeySchemas.ApiKeyListResponse}
    ]
  )

  @doc """
  Lists all active API keys for the current tenant.

  GET /api/api-keys
  """
  def index(conn, _params) do
    user = Guardian.Plug.current_resource(conn)
    api_keys = ApiKeys.list_api_keys(user.tenant_id)

    conn
    |> put_status(:ok)
    |> json(%{data: Enum.map(api_keys, &serialize_api_key/1)})
  end

  operation(:create,
    summary: "Create API key",
    description: "Create a new API key. The raw key is returned only once - save it immediately.",
    security: [%{"bearer_auth" => []}],
    request_body:
      {"API key to create", "application/json", ApiKeySchemas.CreateApiKeyRequest, required: true},
    responses: [
      created: {"API key created", "application/json", ApiKeySchemas.ApiKeyCreatedResponse},
      unprocessable_entity: {"Validation error", "application/json", Common.Error}
    ]
  )

  @doc """
  Creates a new API key.

  POST /api/api-keys

  The raw key is returned only once in the response. It cannot be
  retrieved again - if lost, the key must be rotated.
  """
  def create(conn, params) do
    user = Guardian.Plug.current_resource(conn)

    attrs = %{
      name: params["name"],
      description: params["description"],
      scopes: params["scopes"],
      expires_at: parse_expires_at(params["expires_at"])
    }

    case ApiKeys.create_api_key(user.tenant_id, user.id, attrs) do
      {:ok, {raw_key, api_key}} ->
        conn
        |> put_status(:created)
        |> json(%{
          data: serialize_api_key(api_key) |> Map.put(:key, raw_key),
          warning: "Save this key immediately. You won't be able to see it again."
        })

      {:error, changeset} ->
        conn
        |> put_status(:unprocessable_entity)
        |> json(%{error: format_changeset_errors(changeset)})
    end
  end

  operation(:show,
    summary: "Get API key",
    description: "Get a single API key by ID.",
    security: [%{"bearer_auth" => []}],
    parameters: [
      id: [
        in: :path,
        schema: %OpenApiSpex.Schema{type: :string, format: :uuid},
        description: "API key ID",
        required: true
      ]
    ],
    responses: [
      ok: {"API key details", "application/json", ApiKeySchemas.ApiKeyResponse},
      not_found: {"API key not found", "application/json", Common.Error}
    ]
  )

  @doc """
  Gets a single API key by ID.

  GET /api/api-keys/:id
  """
  def show(conn, %{"id" => id}) do
    user = Guardian.Plug.current_resource(conn)

    case ApiKeys.get_api_key(id, user.tenant_id) do
      nil ->
        conn
        |> put_status(:not_found)
        |> json(%{error: %{code: "not_found", message: "API key not found"}})

      api_key ->
        conn
        |> put_status(:ok)
        |> json(%{data: serialize_api_key(api_key)})
    end
  end

  operation(:update,
    summary: "Update API key",
    description:
      "Update an API key's metadata. Only name, description, and scopes can be updated.",
    security: [%{"bearer_auth" => []}],
    parameters: [
      id: [
        in: :path,
        schema: %OpenApiSpex.Schema{type: :string, format: :uuid},
        description: "API key ID",
        required: true
      ]
    ],
    request_body:
      {"API key updates", "application/json", ApiKeySchemas.UpdateApiKeyRequest, required: true},
    responses: [
      ok: {"API key updated", "application/json", ApiKeySchemas.ApiKeyResponse},
      not_found: {"API key not found", "application/json", Common.Error},
      unprocessable_entity: {"Validation error", "application/json", Common.Error}
    ]
  )

  @doc """
  Updates an API key's metadata.

  PUT /api/api-keys/:id

  Only name, description, and scopes can be updated.
  """
  def update(conn, %{"id" => id} = params) do
    user = Guardian.Plug.current_resource(conn)

    case ApiKeys.get_api_key(id, user.tenant_id) do
      nil ->
        conn
        |> put_status(:not_found)
        |> json(%{error: %{code: "not_found", message: "API key not found"}})

      api_key ->
        attrs = Map.take(params, ["name", "description", "scopes"])

        case ApiKeys.update_api_key(api_key, attrs) do
          {:ok, updated_api_key} ->
            conn
            |> put_status(:ok)
            |> json(%{data: serialize_api_key(updated_api_key)})

          {:error, changeset} ->
            conn
            |> put_status(:unprocessable_entity)
            |> json(%{error: format_changeset_errors(changeset)})
        end
    end
  end

  operation(:rotate,
    summary: "Rotate API key",
    description: "Rotate an API key - revokes the old one and creates a new one.",
    security: [%{"bearer_auth" => []}],
    parameters: [
      id: [
        in: :path,
        schema: %OpenApiSpex.Schema{type: :string, format: :uuid},
        description: "API key ID",
        required: true
      ]
    ],
    responses: [
      ok: {"API key rotated", "application/json", ApiKeySchemas.ApiKeyCreatedResponse},
      not_found: {"API key not found", "application/json", Common.Error},
      unprocessable_entity: {"Rotation failed", "application/json", Common.Error}
    ]
  )

  @doc """
  Rotates an API key - revokes the old one and creates a new one.

  POST /api/api-keys/:id/rotate

  Returns the new raw key, which should be saved immediately.
  """
  def rotate(conn, %{"id" => id}) do
    user = Guardian.Plug.current_resource(conn)

    case ApiKeys.get_api_key(id, user.tenant_id) do
      nil ->
        conn
        |> put_status(:not_found)
        |> json(%{error: %{code: "not_found", message: "API key not found"}})

      api_key ->
        case ApiKeys.rotate_api_key(api_key, user.id) do
          {:ok, {raw_key, new_api_key}} ->
            conn
            |> put_status(:ok)
            |> json(%{
              data: serialize_api_key(new_api_key) |> Map.put(:key, raw_key),
              warning: "Save this new key immediately. The old key has been revoked."
            })

          {:error, changeset} when is_struct(changeset, Ecto.Changeset) ->
            conn
            |> put_status(:unprocessable_entity)
            |> json(%{error: format_changeset_errors(changeset)})

          {:error, reason} ->
            conn
            |> put_status(:internal_server_error)
            |> json(%{
              error: %{code: "rotation_failed", message: "Failed to rotate key: #{reason}"}
            })
        end
    end
  end

  operation(:delete,
    summary: "Revoke API key",
    description: "Revoke an API key, making it permanently unusable.",
    security: [%{"bearer_auth" => []}],
    parameters: [
      id: [
        in: :path,
        schema: %OpenApiSpex.Schema{type: :string, format: :uuid},
        description: "API key ID",
        required: true
      ]
    ],
    responses: [
      ok: {"API key revoked", "application/json", ApiKeySchemas.RevokeResponse},
      not_found: {"API key not found", "application/json", Common.Error},
      internal_server_error: {"Revocation failed", "application/json", Common.Error}
    ]
  )

  @doc """
  Revokes an API key.

  DELETE /api/api-keys/:id
  """
  def delete(conn, %{"id" => id}) do
    user = Guardian.Plug.current_resource(conn)

    case ApiKeys.revoke_api_key(id, user.tenant_id) do
      {:ok, _api_key} ->
        conn
        |> put_status(:ok)
        |> json(%{data: %{message: "API key revoked successfully"}})

      {:error, :not_found} ->
        conn
        |> put_status(:not_found)
        |> json(%{error: %{code: "not_found", message: "API key not found"}})

      {:error, _reason} ->
        conn
        |> put_status(:internal_server_error)
        |> json(%{error: %{code: "revoke_failed", message: "Failed to revoke API key"}})
    end
  end

  operation(:scopes,
    summary: "List available scopes",
    description: "Returns all available permission scopes for API keys.",
    security: [%{"bearer_auth" => []}],
    responses: [
      ok: {"Available scopes", "application/json", ApiKeySchemas.ScopesResponse}
    ]
  )

  @doc """
  Returns available scopes for API keys.

  GET /api/api-keys/scopes
  """
  def scopes(conn, _params) do
    scopes =
      ApiKey.available_scopes()
      |> Enum.map(fn scope ->
        %{
          name: scope,
          description: scope_description(scope)
        }
      end)

    conn
    |> put_status(:ok)
    |> json(%{data: scopes})
  end

  # -------------------------------------------------------------------
  # Private Helpers
  # -------------------------------------------------------------------

  defp serialize_api_key(%ApiKey{} = api_key) do
    %{
      id: api_key.id,
      name: api_key.name,
      description: api_key.description,
      key_prefix: api_key.key_prefix,
      scopes: api_key.scopes,
      last_used_at: api_key.last_used_at,
      expires_at: api_key.expires_at,
      created_at: api_key.inserted_at,
      updated_at: api_key.updated_at
    }
  end

  defp parse_expires_at(nil), do: nil
  defp parse_expires_at(""), do: nil

  defp parse_expires_at(expires_at) when is_binary(expires_at) do
    case DateTime.from_iso8601(expires_at) do
      {:ok, dt, _offset} -> dt
      _ -> nil
    end
  end

  defp parse_expires_at(_), do: nil

  defp format_changeset_errors(changeset) do
    errors =
      Ecto.Changeset.traverse_errors(changeset, fn {msg, opts} ->
        Enum.reduce(opts, msg, fn {key, value}, acc ->
          String.replace(acc, "%{#{key}}", to_string(value))
        end)
      end)

    %{
      code: "validation_error",
      message: "Validation failed",
      details: errors
    }
  end

  defp scope_description(scope) do
    descriptions = %{
      "events:read" => "Read event data",
      "events:write" => "Write/ingest events",
      "queries:execute" => "Execute queries",
      "projections:read" => "Read projection state",
      "projections:write" => "Create and manage projections",
      "schemas:read" => "Read event schemas",
      "schemas:write" => "Register event schemas"
    }

    Map.get(descriptions, scope, scope)
  end
end
