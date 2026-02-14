defmodule QueryServiceEx.Infrastructure.Adapters.RustCoreClient do
  @moduledoc """
  HTTP client adapter for communicating with the Rust Event Store Core.

  Implements the QueryExecutor protocol and provides methods for:
  - Event ingestion
  - Event querying
  - Schema management
  - Snapshot operations

  Uses Tesla for HTTP client with connection pooling via Hackney.

  ## Authentication

  The client authenticates to Core using the `CORE_API_KEY` environment variable.
  This key is sent as an Authorization header with all requests.

  ## Tenant Isolation

  All data operations require a `tenant_id` parameter to ensure strict
  tenant isolation. Events are stored with tenant_id and all queries
  are automatically filtered by tenant_id.
  """

  alias QueryServiceEx.Domain.Entities.Query
  alias QueryServiceEx.Infrastructure.Adapters.CoreHealthChecker

  @default_base_url "http://localhost:3900"
  @default_timeout 30_000
  @read_counter_name :core_read_round_robin

  @doc false
  def client do
    write_client()
  end

  @doc """
  Returns a Tesla client targeting the write (leader) URL.

  Uses `core_write_url` config, falling back to `core_url`.
  All POST/PUT/DELETE calls should use this client.
  """
  def write_client do
    base_url =
      Application.get_env(:query_service_ex, :core_write_url) ||
        Application.get_env(:query_service_ex, :core_url, @default_base_url)

    build_client(base_url)
  end

  @doc """
  Returns a Tesla client respecting the given consistency mode.

  - `:strong` — routes to the leader (write URL) for read-your-writes
  - `:eventual` (default) — round-robin across healthy followers
  """
  def client_for_consistency(:strong), do: write_client()
  def client_for_consistency(_), do: read_client()

  @doc """
  Returns a Tesla client targeting one of the healthy read URLs via round-robin.

  Checks CoreHealthChecker ETS state to skip unhealthy or lagging nodes.
  If all read nodes are unhealthy, falls back to the write (leader) URL.
  Uses `core_read_urls` config (list), falling back to `core_url`.
  All GET calls should use this client.
  """
  def read_client do
    all_urls =
      case Application.get_env(:query_service_ex, :core_read_urls) do
        urls when is_list(urls) and urls != [] -> urls
        _ -> [Application.get_env(:query_service_ex, :core_url, @default_base_url)]
      end

    healthy_urls = CoreHealthChecker.healthy_read_urls()

    # Use healthy subset if any are available, otherwise fall back to leader
    urls =
      case Enum.filter(all_urls, &(&1 in healthy_urls)) do
        [] ->
          # All read nodes unhealthy — fall back to write (leader) URL
          write_url =
            Application.get_env(:query_service_ex, :core_write_url) ||
              Application.get_env(:query_service_ex, :core_url, @default_base_url)

          [write_url]

        available ->
          available
      end

    base_url = pick_read_url(urls)
    build_client(base_url)
  end

  defp build_client(base_url) do
    middleware = [
      {Tesla.Middleware.BaseUrl, base_url},
      Tesla.Middleware.JSON,
      {Tesla.Middleware.Timeout, timeout: @default_timeout},
      {Tesla.Middleware.Retry,
       delay: 100,
       max_retries: 3,
       max_delay: 2_000,
       should_retry: fn
         {:ok, %{status: status}} when status in [408, 429, 500, 502, 503, 504] -> true
         {:ok, _} -> false
         {:error, _} -> true
       end}
    ]

    # Add Authorization header if CORE_API_KEY is configured
    middleware =
      case Application.get_env(:query_service_ex, :core_api_key) do
        nil -> middleware
        "" -> middleware
        api_key -> [{Tesla.Middleware.Headers, [{"authorization", api_key}]} | middleware]
      end

    Tesla.client(middleware)
  end

  defp pick_read_url([single_url]), do: single_url

  defp pick_read_url(urls) do
    count = length(urls)
    ref = ensure_counter()
    index = :atomics.add_get(ref, 1, 1)
    Enum.at(urls, rem(abs(index), count))
  end

  defp ensure_counter do
    case :persistent_term.get(@read_counter_name, nil) do
      nil ->
        ref = :atomics.new(1, signed: true)
        :persistent_term.put(@read_counter_name, ref)
        ref

      ref ->
        ref
    end
  end

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

    case Tesla.post(write_client(), "/api/v1/events", event_with_tenant) do
      {:ok, %Tesla.Env{status: status, body: body}} when status in [200, 201] ->
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
    case Tesla.post(write_client(), "/api/v1/events", event) do
      {:ok, %Tesla.Env{status: status, body: body}} when status in [200, 201] ->
        {:ok, body}

      {:ok, %Tesla.Env{status: status, body: body}} ->
        {:error, "HTTP #{status}: #{inspect(body)}"}

      {:error, reason} ->
        {:error, reason}
    end
  end

  @doc """
  Create multiple events in a batch with tenant isolation.

  Uses the Core batch endpoint for efficient bulk ingestion.

  ## Parameters
    * `tenant_id` - The tenant ID (required for isolation)
    * `events` - List of event maps

  ## Returns
    * `{:ok, response}` - Batch response with `total`, `ingested`, and `events` list
    * `{:error, reason}` - Error details
  """
  def create_event_batch(tenant_id, events) when is_binary(tenant_id) and is_list(events) do
    events_with_tenant = Enum.map(events, &Map.put(&1, :tenant_id, tenant_id))

    case Tesla.post(write_client(), "/api/v1/events/batch", %{events: events_with_tenant}) do
      {:ok, %Tesla.Env{status: 200, body: body}} ->
        {:ok, body}

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
    case Tesla.post(write_client(), "/api/v1/events/batch", %{events: events}) do
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
  def query_events(tenant_id, query, opts \\ [])

  def query_events(tenant_id, %Query{} = query, opts) when is_binary(tenant_id) do
    params = compile_query(query)
    query_events(tenant_id, params, opts)
  end

  def query_events(tenant_id, params, opts) when is_binary(tenant_id) and is_map(params) do
    params_with_tenant = Map.put(params, :tenant_id, tenant_id)
    client = client_for_consistency(Keyword.get(opts, :consistency))

    case Tesla.get(client, "/api/v1/events/query", query: params_with_tenant) do
      {:ok, %Tesla.Env{status: 200, body: %{"events" => events}}} when is_list(events) ->
        {:ok, events}

      {:ok, %Tesla.Env{status: 200, body: body}} when is_list(body) ->
        {:ok, body}

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
    case Tesla.get(read_client(), "/api/v1/events/query", query: params) do
      {:ok, %Tesla.Env{status: 200, body: %{"events" => events}}} when is_list(events) ->
        {:ok, events}

      {:ok, %Tesla.Env{status: 200, body: body}} when is_list(body) ->
        {:ok, body}

      {:ok, %Tesla.Env{status: 200, body: body}} ->
        {:ok, body}

      {:ok, %Tesla.Env{status: status, body: body}} ->
        {:error, "HTTP #{status}: #{inspect(body)}"}

      {:error, reason} ->
        {:error, reason}
    end
  end

  @doc "Get events by entity ID with tenant isolation"
  def get_events_by_entity(tenant_id, entity_id, opts \\ []) when is_binary(tenant_id) do
    query_events(tenant_id, %{entity_id: entity_id}, opts)
  end

  # Deprecated: Use get_events_by_entity/2 with tenant_id
  @doc false
  def get_events_by_entity(entity_id) do
    query_events(%{entity_id: entity_id})
  end

  @doc "Get events by event type with tenant isolation"
  def get_events_by_type(tenant_id, event_type, opts \\ []) when is_binary(tenant_id) do
    query_events(tenant_id, %{event_type: event_type}, opts)
  end

  # Deprecated: Use get_events_by_type/2 with tenant_id
  @doc false
  def get_events_by_type(event_type) do
    query_events(%{event_type: event_type})
  end

  ## Streams (Entity Discovery)

  @doc """
  List all streams (entity_ids) in the event store.

  Streams represent unique entity_ids that have events. This endpoint
  is useful for discovering available entities in the system.

  ## Parameters
    * `opts` - Keyword list of options:
      * `:limit` - Maximum number of streams to return
      * `:offset` - Number of streams to skip (for pagination)

  ## Returns
    * `{:ok, %{streams: [...], total: N}}` - List of streams with metadata
    * `{:error, reason}` - Error details

  ## Examples

      iex> list_streams(limit: 10)
      {:ok, %{streams: [%{stream_id: "user-123", event_count: 42, ...}], total: 150}}
  """
  def list_streams(opts \\ []) do
    {consistency, opts} = Keyword.pop(opts, :consistency)

    query_params =
      opts
      |> Keyword.take([:limit, :offset])
      |> Enum.reject(fn {_k, v} -> is_nil(v) end)
      |> Map.new()

    client = client_for_consistency(consistency)

    case Tesla.get(client, "/api/v1/streams", query: query_params) do
      {:ok, %Tesla.Env{status: 200, body: body}} ->
        {:ok, body}

      {:ok, %Tesla.Env{status: status, body: body}} ->
        {:error, "HTTP #{status}: #{inspect(body)}"}

      {:error, reason} ->
        {:error, reason}
    end
  end

  ## Event Types (Type Discovery)

  @doc """
  List all unique event types in the event store.

  This endpoint is useful for discovering what types of events
  exist in the system and their usage statistics.

  ## Parameters
    * `opts` - Keyword list of options:
      * `:limit` - Maximum number of event types to return
      * `:offset` - Number of event types to skip (for pagination)

  ## Returns
    * `{:ok, %{event_types: [...], total: N}}` - List of event types with metadata
    * `{:error, reason}` - Error details

  ## Examples

      iex> list_event_types(limit: 10)
      {:ok, %{event_types: [%{event_type: "user.created", event_count: 1500, ...}], total: 25}}
  """
  def list_event_types(opts \\ []) do
    {consistency, opts} = Keyword.pop(opts, :consistency)

    query_params =
      opts
      |> Keyword.take([:limit, :offset])
      |> Enum.reject(fn {_k, v} -> is_nil(v) end)
      |> Map.new()

    client = client_for_consistency(consistency)

    case Tesla.get(client, "/api/v1/event-types", query: query_params) do
      {:ok, %Tesla.Env{status: 200, body: body}} ->
        {:ok, body}

      {:ok, %Tesla.Env{status: status, body: body}} ->
        {:error, "HTTP #{status}: #{inspect(body)}"}

      {:error, reason} ->
        {:error, reason}
    end
  end

  ## Projections

  @doc "List all projections"
  def list_projections do
    case Tesla.get(read_client(), "/api/v1/projections") do
      {:ok, %Tesla.Env{status: 200, body: %{"projections" => projections}}}
      when is_list(projections) ->
        {:ok, projections}

      {:ok, %Tesla.Env{status: 200, body: body}} when is_list(body) ->
        {:ok, body}

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
    case Tesla.get(read_client(), "/api/v1/projections/#{id}") do
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
    case Tesla.post(write_client(), "/api/v1/projections", projection) do
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
    case Tesla.get(read_client(), "/api/v1/schemas") do
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
        "/api/v1/schemas/#{event_type}?version=#{version}"
      else
        "/api/v1/schemas/#{event_type}"
      end

    case Tesla.get(read_client(), path) do
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
    case Tesla.post(write_client(), "/api/v1/schemas", schema) do
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
        "/api/v1/snapshots?entity_id=#{entity_id}"
      else
        "/api/v1/snapshots"
      end

    case Tesla.get(read_client(), path) do
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
    case Tesla.post(write_client(), "/api/v1/snapshots", %{
           entity_id: entity_id,
           snapshot_type: snapshot_type
         }) do
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
    case Tesla.get(read_client(), "/api/v1/projections/#{projection_name}/#{entity_id}/state") do
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
    case Tesla.put(write_client(), "/api/v1/projections/#{projection_name}/#{entity_id}/state", %{
           state: state
         }) do
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
    case Tesla.post(read_client(), "/api/v1/projections/#{projection_name}/bulk", %{
           entity_ids: entity_ids
         }) do
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

  ## API Key Verification

  @doc """
  Verify an API key by calling Core's `/api/v1/auth/me` with the key as Authorization header.

  Core validates the key (hash lookup, expiry, revocation) and returns the authenticated
  user/claims. We extract the tenant_id from the response and fetch the tenant from Core.

  Returns `{:ok, {tenant_map, api_key_info}}` or `{:error, reason}`.
  """
  def verify_api_key(raw_key) when is_binary(raw_key) do
    # Build a one-off client with the API key as Authorization header
    base_url =
      Application.get_env(:query_service_ex, :core_write_url) ||
        Application.get_env(:query_service_ex, :core_url, @default_base_url)

    client =
      Tesla.client([
        {Tesla.Middleware.BaseUrl, base_url},
        Tesla.Middleware.JSON,
        {Tesla.Middleware.Timeout, timeout: @default_timeout},
        {Tesla.Middleware.Headers, [{"authorization", raw_key}]}
      ])

    case Tesla.get(client, "/api/v1/auth/me") do
      {:ok, %Tesla.Env{status: 200, body: body}} ->
        tenant_id = body["tenant_id"]

        if tenant_id do
          case get_tenant(tenant_id) do
            {:ok, tenant} ->
              api_key_info = %{
                "id" => body["id"],
                "name" => body["name"] || body["username"],
                "tenant_id" => tenant_id,
                "role" => body["role"],
                "scopes" => body["scopes"] || []
              }

              {:ok, {tenant, api_key_info}}

            {:error, :not_found} ->
              {:error, :invalid_key}

            {:error, _reason} ->
              {:error, :invalid_key}
          end
        else
          {:error, :invalid_key}
        end

      {:ok, %Tesla.Env{status: 401}} ->
        {:error, :invalid_key}

      {:ok, %Tesla.Env{status: 403}} ->
        {:error, :key_revoked}

      {:ok, %Tesla.Env{status: _status}} ->
        {:error, :invalid_key}

      {:error, _reason} ->
        {:error, :invalid_key}
    end
  end

  ## Tenants

  @doc """
  Get a tenant from Core by ID.

  Returns the tenant as a map with `"id"`, `"name"`, `"status"`, and `"metadata"` keys.

  ## Returns
    * `{:ok, tenant}` - Tenant map from Core
    * `{:error, :not_found}` - Tenant does not exist
    * `{:error, reason}` - Error details
  """
  def get_tenant(tenant_id) when is_binary(tenant_id) do
    case Tesla.get(read_client(), "/api/v1/tenants/#{tenant_id}") do
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

  ## Metrics & Health

  @doc "Get system metrics"
  def get_metrics do
    case Tesla.get(read_client(), "/metrics") do
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
    case Tesla.get(read_client(), "/health") do
      {:ok, %Tesla.Env{status: 200, body: body}} ->
        {:ok, body}

      {:ok, %Tesla.Env{status: status}} ->
        {:error, "HTTP #{status}"}

      {:error, reason} ->
        {:error, reason}
    end
  end

  @doc """
  Aggregated cluster health check.

  Queries Core's health endpoint and optionally the Control Plane's
  cluster status endpoint to provide a unified cluster health view.
  Returns a map with core health, control plane status, and read URL count.
  """
  def cluster_health do
    core_task = Task.async(fn -> health_check() end)
    cp_task = Task.async(fn -> control_plane_cluster_status() end)

    core_result = Task.await(core_task, 5_000)
    cp_result = Task.await(cp_task, 5_000)

    read_urls =
      case Application.get_env(:query_service_ex, :core_read_urls) do
        urls when is_list(urls) -> length(urls)
        _ -> 1
      end

    core_health =
      case core_result do
        {:ok, body} -> %{status: "healthy", details: body}
        {:error, reason} -> %{status: "unhealthy", error: inspect(reason)}
      end

    control_plane =
      case cp_result do
        {:ok, body} -> %{status: "healthy", details: body}
        {:error, _reason} -> %{status: "unavailable"}
      end

    {:ok,
     %{
       core: core_health,
       control_plane: control_plane,
       read_replicas: read_urls,
       timestamp: DateTime.utc_now() |> DateTime.to_iso8601()
     }}
  rescue
    error -> {:error, inspect(error)}
  catch
    :exit, reason -> {:error, inspect(reason)}
  end

  defp control_plane_cluster_status do
    cp_url =
      Application.get_env(:query_service_ex, :control_plane_url, "http://localhost:3901")

    cp_client =
      Tesla.client([
        {Tesla.Middleware.BaseUrl, cp_url},
        Tesla.Middleware.JSON,
        {Tesla.Middleware.Timeout, timeout: 5_000}
      ])

    case Tesla.get(cp_client, "/api/v1/cluster/health") do
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
