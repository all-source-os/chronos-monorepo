defmodule QueryServiceEx.Infrastructure.Adapters.RustCoreClient do
  @moduledoc """
  HTTP client adapter for communicating with the Rust Event Store Core.

  Implements the QueryExecutor protocol and provides methods for:
  - Event ingestion
  - Event querying
  - Schema management
  - Snapshot operations

  Uses Tesla for HTTP client with connection pooling via Hackney.

  ## Tenant Isolation

  All data operations require a `tenant_id` parameter to ensure strict
  tenant isolation. Events are stored with tenant_id and all queries
  are automatically filtered by tenant_id.
  """

  use Tesla

  alias QueryServiceEx.Domain.Entities.Query

  @default_base_url "http://localhost:3900"
  @default_timeout 30_000

  plug(
    Tesla.Middleware.BaseUrl,
    Application.get_env(:query_service_ex, :rust_core_url, @default_base_url)
  )

  plug(Tesla.Middleware.JSON)
  plug(Tesla.Middleware.Timeout, timeout: @default_timeout)

  plug(Tesla.Middleware.Retry,
    delay: 100,
    max_retries: 3,
    max_delay: 2_000,
    should_retry: fn
      {:ok, %{status: status}} when status in [408, 429, 500, 502, 503, 504] -> true
      {:ok, _} -> false
      {:error, _} -> true
    end
  )

  ## Event Management

  @doc """
  Create a single event with tenant isolation.

  ## Parameters
    * `tenant_id` - The tenant ID (required for isolation)
    * `event` - Map with event data

  ## Returns
    * `{:ok, event}` - Created event
    * `{:error, reason}` - Error details

  ## Examples

      iex> create_event("tenant-uuid", %{
      ...>   entity_id: "user-123",
      ...>   event_type: "user.created",
      ...>   payload: %{email: "user@example.com"}
      ...> })
      {:ok, %{id: "evt-123", ...}}
  """
  def create_event(tenant_id, event) when is_binary(tenant_id) and is_map(event) do
    event_with_tenant = Map.put(event, :tenant_id, tenant_id)

    case post("/api/events", event_with_tenant) do
      {:ok, %Tesla.Env{status: 201, body: body}} ->
        {:ok, body}

      {:ok, %Tesla.Env{status: status, body: body}} ->
        {:error, "HTTP #{status}: #{inspect(body)}"}

      {:error, reason} ->
        {:error, reason}
    end
  end

  # Deprecated: Use create_event/2 with tenant_id for proper isolation
  @doc false
  def create_event(event) when is_map(event) do
    case post("/api/events", event) do
      {:ok, %Tesla.Env{status: 201, body: body}} ->
        {:ok, body}

      {:ok, %Tesla.Env{status: status, body: body}} ->
        {:error, "HTTP #{status}: #{inspect(body)}"}

      {:error, reason} ->
        {:error, reason}
    end
  end

  @doc """
  Create multiple events in a batch with tenant isolation.

  ## Parameters
    * `tenant_id` - The tenant ID (required for isolation)
    * `events` - List of event maps

  ## Returns
    * `{:ok, events}` - List of created events
    * `{:error, reason}` - Error details
  """
  def create_event_batch(tenant_id, events) when is_binary(tenant_id) and is_list(events) do
    events_with_tenant = Enum.map(events, &Map.put(&1, :tenant_id, tenant_id))

    case post("/api/events/batch", %{events: events_with_tenant}) do
      {:ok, %Tesla.Env{status: 201, body: body}} ->
        {:ok, body}

      {:ok, %Tesla.Env{status: status, body: body}} ->
        {:error, "HTTP #{status}: #{inspect(body)}"}

      {:error, reason} ->
        {:error, reason}
    end
  end

  # Deprecated: Use create_event_batch/2 with tenant_id for proper isolation
  @doc false
  def create_event_batch(events) when is_list(events) do
    case post("/api/events/batch", %{events: events}) do
      {:ok, %Tesla.Env{status: 201, body: body}} ->
        {:ok, body}

      {:ok, %Tesla.Env{status: status, body: body}} ->
        {:error, "HTTP #{status}: #{inspect(body)}"}

      {:error, reason} ->
        {:error, reason}
    end
  end

  @doc """
  Query events from the event store with tenant isolation.

  ## Parameters
    * `tenant_id` - The tenant ID (required for isolation)
    * `query` - Query struct or map with query parameters

  ## Returns
    * `{:ok, events}` - List of matching events
    * `{:error, reason}` - Error details

  ## Examples

      iex> query_events("tenant-uuid", %{entity_id: "user-123", limit: 10})
      {:ok, [%{id: "evt-1", ...}, ...]}
  """
  def query_events(tenant_id, %Query{} = query) when is_binary(tenant_id) do
    params = compile_query(query)
    query_events(tenant_id, params)
  end

  def query_events(tenant_id, params) when is_binary(tenant_id) and is_map(params) do
    params_with_tenant = Map.put(params, :tenant_id, tenant_id)

    case get("/api/events", query: params_with_tenant) do
      {:ok, %Tesla.Env{status: 200, body: body}} ->
        {:ok, body}

      {:ok, %Tesla.Env{status: status, body: body}} ->
        {:error, "HTTP #{status}: #{inspect(body)}"}

      {:error, reason} ->
        {:error, reason}
    end
  end

  # Deprecated: Use query_events/2 with tenant_id for proper isolation
  @doc false
  def query_events(%Query{} = query) do
    params = compile_query(query)
    query_events(params)
  end

  @doc false
  def query_events(params) when is_map(params) do
    case get("/api/events", query: params) do
      {:ok, %Tesla.Env{status: 200, body: body}} ->
        {:ok, body}

      {:ok, %Tesla.Env{status: status, body: body}} ->
        {:error, "HTTP #{status}: #{inspect(body)}"}

      {:error, reason} ->
        {:error, reason}
    end
  end

  @doc "Get events by entity ID with tenant isolation"
  def get_events_by_entity(tenant_id, entity_id) when is_binary(tenant_id) do
    query_events(tenant_id, %{entity_id: entity_id})
  end

  # Deprecated: Use get_events_by_entity/2 with tenant_id
  @doc false
  def get_events_by_entity(entity_id) do
    query_events(%{entity_id: entity_id})
  end

  @doc "Get events by event type with tenant isolation"
  def get_events_by_type(tenant_id, event_type) when is_binary(tenant_id) do
    query_events(tenant_id, %{event_type: event_type})
  end

  # Deprecated: Use get_events_by_type/2 with tenant_id
  @doc false
  def get_events_by_type(event_type) do
    query_events(%{event_type: event_type})
  end

  ## Projections

  @doc "List all projections"
  def list_projections do
    case get("/api/projections") do
      {:ok, %Tesla.Env{status: 200, body: body}} ->
        {:ok, body}

      {:ok, %Tesla.Env{status: status, body: body}} ->
        {:error, "HTTP #{status}: #{inspect(body)}"}

      {:error, reason} ->
        {:error, reason}
    end
  end

  @doc "Get a specific projection by ID"
  def get_projection(id) do
    case get("/api/projections/#{id}") do
      {:ok, %Tesla.Env{status: 200, body: body}} ->
        {:ok, body}

      {:ok, %Tesla.Env{status: 404}} ->
        {:error, :not_found}

      {:ok, %Tesla.Env{status: status, body: body}} ->
        {:error, "HTTP #{status}: #{inspect(body)}"}

      {:error, reason} ->
        {:error, reason}
    end
  end

  @doc "Create a new projection"
  def create_projection(projection) when is_map(projection) do
    case post("/api/projections", projection) do
      {:ok, %Tesla.Env{status: 201, body: body}} ->
        {:ok, body}

      {:ok, %Tesla.Env{status: status, body: body}} ->
        {:error, "HTTP #{status}: #{inspect(body)}"}

      {:error, reason} ->
        {:error, reason}
    end
  end

  ## Schemas

  @doc "List all schemas"
  def list_schemas do
    case get("/api/schemas") do
      {:ok, %Tesla.Env{status: 200, body: body}} ->
        {:ok, body}

      {:ok, %Tesla.Env{status: status, body: body}} ->
        {:error, "HTTP #{status}: #{inspect(body)}"}

      {:error, reason} ->
        {:error, reason}
    end
  end

  @doc "Get schema for event type"
  def get_schema(event_type, version \\ nil) do
    path =
      if version do
        "/api/schemas/#{event_type}?version=#{version}"
      else
        "/api/schemas/#{event_type}"
      end

    case get(path) do
      {:ok, %Tesla.Env{status: 200, body: body}} ->
        {:ok, body}

      {:ok, %Tesla.Env{status: 404}} ->
        {:error, :not_found}

      {:ok, %Tesla.Env{status: status, body: body}} ->
        {:error, "HTTP #{status}: #{inspect(body)}"}

      {:error, reason} ->
        {:error, reason}
    end
  end

  @doc "Register a new schema"
  def register_schema(schema) when is_map(schema) do
    case post("/api/schemas", schema) do
      {:ok, %Tesla.Env{status: 201, body: body}} ->
        {:ok, body}

      {:ok, %Tesla.Env{status: status, body: body}} ->
        {:error, "HTTP #{status}: #{inspect(body)}"}

      {:error, reason} ->
        {:error, reason}
    end
  end

  ## Snapshots

  @doc "List snapshots"
  def list_snapshots(entity_id \\ nil) do
    path =
      if entity_id do
        "/api/snapshots?entity_id=#{entity_id}"
      else
        "/api/snapshots"
      end

    case get(path) do
      {:ok, %Tesla.Env{status: 200, body: body}} ->
        {:ok, body}

      {:ok, %Tesla.Env{status: status, body: body}} ->
        {:error, "HTTP #{status}: #{inspect(body)}"}

      {:error, reason} ->
        {:error, reason}
    end
  end

  @doc "Create a snapshot"
  def create_snapshot(entity_id, snapshot_type) do
    case post("/api/snapshots", %{entity_id: entity_id, snapshot_type: snapshot_type}) do
      {:ok, %Tesla.Env{status: 201, body: body}} ->
        {:ok, body}

      {:ok, %Tesla.Env{status: status, body: body}} ->
        {:error, "HTTP #{status}: #{inspect(body)}"}

      {:error, reason} ->
        {:error, reason}
    end
  end

  ## Projection State (v0.7 Core Integration)

  @doc """
  Get projection state for an entity from Core's DashMap.

  ## Parameters
    * `projection_name` - Name of the projection (e.g., "entity_snapshots")
    * `entity_id` - Entity identifier

  ## Returns
    * `{:ok, state}` - Projection state as map
    * `{:error, :not_found}` - No state found
    * `{:error, reason}` - Error details
  """
  def get_projection_state(projection_name, entity_id) do
    case get("/api/v1/projections/#{projection_name}/#{entity_id}/state") do
      {:ok, %Tesla.Env{status: 200, body: %{"found" => true, "state" => state}}} ->
        {:ok, state}

      {:ok, %Tesla.Env{status: 200, body: %{"found" => false}}} ->
        {:error, :not_found}

      {:ok, %Tesla.Env{status: 404}} ->
        {:error, :not_found}

      {:ok, %Tesla.Env{status: status, body: body}} ->
        {:error, "HTTP #{status}: #{inspect(body)}"}

      {:error, reason} ->
        {:error, reason}
    end
  end

  @doc """
  Save projection state for an entity to Core's DashMap.

  ## Parameters
    * `projection_name` - Name of the projection
    * `entity_id` - Entity identifier
    * `state` - State to save (map)

  ## Returns
    * `:ok` - State saved successfully
    * `{:error, reason}` - Error details
  """
  def save_projection_state(projection_name, entity_id, state) when is_map(state) do
    case put("/api/v1/projections/#{projection_name}/#{entity_id}/state", %{state: state}) do
      {:ok, %Tesla.Env{status: 200, body: %{"saved" => true}}} ->
        :ok

      {:ok, %Tesla.Env{status: status, body: body}} ->
        {:error, "HTTP #{status}: #{inspect(body)}"}

      {:error, reason} ->
        {:error, reason}
    end
  end

  @doc """
  Bulk get projection states for multiple entities.

  ## Parameters
    * `projection_name` - Name of the projection
    * `entity_ids` - List of entity identifiers

  ## Returns
    * `{:ok, states}` - List of %{entity_id, state, found}
    * `{:error, reason}` - Error details
  """
  def bulk_get_projection_states(projection_name, entity_ids) when is_list(entity_ids) do
    case post("/api/v1/projections/#{projection_name}/bulk", %{entity_ids: entity_ids}) do
      {:ok, %Tesla.Env{status: 200, body: %{"states" => states}}} ->
        {:ok, states}

      {:ok, %Tesla.Env{status: status, body: body}} ->
        {:error, "HTTP #{status}: #{inspect(body)}"}

      {:error, reason} ->
        {:error, reason}
    end
  end

  @doc """
  Bulk save projection states for multiple entities.

  ## Parameters
    * `states` - List of %{projection_name, entity_id, state}

  ## Returns
    * `:ok` - All states saved
    * `{:error, reason}` - Error details
  """
  def bulk_save_projection_states(states) when is_list(states) do
    # Save states sequentially (could be parallelized with Task.async_stream)
    results =
      Enum.map(states, fn %{projection_name: name, entity_id: id, state: state} ->
        save_projection_state(name, id, state)
      end)

    if Enum.all?(results, &(&1 == :ok)) do
      :ok
    else
      {:error, {:partial_failure, results}}
    end
  end

  ## Metrics & Health

  @doc "Get system metrics"
  def get_metrics do
    case get("/api/metrics") do
      {:ok, %Tesla.Env{status: 200, body: body}} ->
        {:ok, body}

      {:ok, %Tesla.Env{status: status, body: body}} ->
        {:error, "HTTP #{status}: #{inspect(body)}"}

      {:error, reason} ->
        {:error, reason}
    end
  end

  @doc "Check health status"
  def health_check do
    case get("/health") do
      {:ok, %Tesla.Env{status: 200, body: body}} ->
        {:ok, body}

      {:ok, %Tesla.Env{status: status}} ->
        {:error, "HTTP #{status}"}

      {:error, reason} ->
        {:error, reason}
    end
  end

  ## Private Helpers

  defp compile_query(%Query{} = query) do
    %{}
    |> maybe_add_param(:entity_id, query.where, &extract_entity_id/1)
    |> maybe_add_param(:event_type, query.where, &extract_event_type/1)
    |> maybe_add_param(:limit, query.limit)
    |> maybe_add_param(:offset, query.offset)
    |> Map.reject(fn {_k, v} -> is_nil(v) end)
  end

  defp maybe_add_param(params, _key, nil), do: params
  defp maybe_add_param(params, _key, value) when is_function(value), do: params
  defp maybe_add_param(params, key, value), do: Map.put(params, key, value)

  defp maybe_add_param(params, key, source, extractor) when is_function(extractor) do
    case extractor.(source) do
      nil -> params
      value -> Map.put(params, key, value)
    end
  end

  defp extract_entity_id(%{field: :entity_id, operator: :eq, value: value}), do: value
  defp extract_entity_id(_), do: nil

  defp extract_event_type(%{field: :event_type, operator: :eq, value: value}), do: value
  defp extract_event_type(_), do: nil
end
