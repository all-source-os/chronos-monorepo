defmodule McpServerElixir.Infrastructure.CoreClient do
  @moduledoc """
  HTTP client for communicating with the Rust Core API.

  Implements `CoreBackend` behaviour for the remote (HTTP) backend.
  The Tesla client is configured at the module level — no instance state needed.

  ## Authentication (#228)

  A hosted Core / gateway (e.g. `https://api.all-source.xyz`) requires
  `Authorization: Bearer <key>`. Without it every request 401s, which the MCP
  layer surfaces as JSON-RPC `-32603 Internal error`. The raw key *without* the
  `Bearer` scheme 401s just the same, so the scheme is mandatory.

  Note that Tesla evaluates module-level `plug` arguments on every request (they
  are compiled into `__middleware__/0`, not frozen into a module attribute), so
  both the base URL and `auth_headers/0` below pick up whatever
  `config/runtime.exs` resolved at release boot.
  """

  @behaviour McpServerElixir.Infrastructure.CoreBackend

  use Tesla

  @default_base_url "http://localhost:3900"
  @default_timeout 30_000

  # Disable Hackney connection pooling so stale connections don't cause
  # errors after Core restarts. The MCP server makes infrequent calls,
  # so connection reuse provides negligible benefit.
  adapter(Tesla.Adapter.Hackney, pool: false, recv_timeout: @default_timeout)

  plug(
    Tesla.Middleware.BaseUrl,
    Application.get_env(:mcp_server_elixir, :core_url, @default_base_url)
  )

  # Authenticate against a hosted Core. Resolves per request; an empty list is
  # a no-op, so an unconfigured key sends no header rather than a bare "Bearer ".
  plug(Tesla.Middleware.Headers, auth_headers())

  plug(Tesla.Middleware.JSON)
  plug(Tesla.Middleware.Timeout, timeout: @default_timeout)

  plug(Tesla.Middleware.Retry,
    delay: 500,
    max_retries: 3,
    max_delay: 5_000,
    should_retry: fn
      {:ok, %{status: status}} when status in [408, 429, 500, 502, 503, 504] -> true
      {:ok, _} -> false
      {:error, _} -> true
    end
  )

  # ============================================================================
  # Graceful degradation for endpoints the gateway does not route (#229)
  # ============================================================================

  # How many events to read when deriving an aggregate locally. Bounded so a
  # large store cannot turn one stats call into an unbounded transfer.
  @derive_limit 10_000

  # Only 404/405 mean "this deployment does not expose that route", which is the
  # gateway case worth degrading around. 401/403 are credential and permission
  # problems and must surface verbatim — masking them behind an approximate
  # answer is exactly what made #229 hard to diagnose.
  defp route_absent?(status), do: status in [404, 405]

  # Rebuild Core's StoreStats shape (total_events / total_entities /
  # total_event_types / total_ingested) from the tenant's own events, plus the
  # census and time range the MCP stats tools present.
  defp derive_stats_from_events(status) do
    case query_events(%{"limit" => @derive_limit}) do
      {:ok, data} ->
        events = Map.get(data, "events", [])
        reported = Map.get(data, "count", length(events))
        truncated = length(events) >= @derive_limit

        types = events |> Enum.map(&Map.get(&1, "event_type")) |> Enum.reject(&is_nil/1)
        entities = events |> Enum.map(&Map.get(&1, "entity_id")) |> Enum.reject(&is_nil/1)
        timestamps = events |> Enum.map(&Map.get(&1, "timestamp")) |> Enum.reject(&is_nil/1)
        census = Enum.frequencies(types)

        {:ok,
         %{
           "total_events" => max(reported, length(events)),
           "total_entities" => entities |> Enum.uniq() |> length(),
           "total_event_types" => census |> Map.keys() |> length(),
           "total_ingested" => max(reported, length(events)),
           # Keys the MCP stats tools read directly.
           "unique_entities" => entities |> Enum.uniq() |> length(),
           "event_types" => census,
           "oldest_event" => Enum.min(timestamps, fn -> nil end),
           "newest_event" => Enum.max(timestamps, fn -> nil end),
           # Provenance, so a caller can tell this apart from Core's own numbers.
           "approximate" => true,
           "source" => "derived_from_events",
           "truncated" => truncated,
           "note" =>
             "Derived from up to #{@derive_limit} events because GET /api/v1/stats " <>
               "returned HTTP #{status} (the gateway does not expose the global " <>
               "stats endpoint to tenants)." <>
               if(truncated,
                 do: " Counts are a lower bound — the event limit was reached.",
                 else: ""
               )
         }}

      {:error, reason} ->
        {:error,
         "GET /api/v1/stats returned HTTP #{status} and deriving stats from events " <>
           "also failed: #{inspect(reason)}"}
    end
  end

  # Fold an entity's events into current state, matching the shape Core's
  # /entities/{id}/state returns: a shallow last-write-wins merge of payloads in
  # order, wrapped with entity_id / last_updated / event_count / as_of / history.
  #
  # Core seeds the merge from the latest snapshot; this replays raw events
  # instead. Same result unless events behind a snapshot have been archived away,
  # in which case those keys are missing here — flagged via "source".
  defp fold_entity_state(entity_id, as_of, status) do
    params = %{"entity_id" => entity_id, "limit" => @derive_limit}
    params = if as_of, do: Map.put(params, "as_of", as_of), else: params

    case query_events(params) do
      {:ok, data} ->
        # Re-apply the entity filter locally. Folding is only correct over ONE
        # entity's events, so don't rely on the server having honoured the filter
        # — a backend that ignores it would otherwise merge unrelated payloads
        # into this entity's state and report it as fact.
        events =
          data
          |> Map.get("events", [])
          |> Enum.filter(&(Map.get(&1, "entity_id") in [nil, entity_id]))

        case events do
          [] ->
            {:error, "Entity not found: #{entity_id}"}

          events ->
            merged =
              Enum.reduce(events, %{}, fn event, acc ->
                case Map.get(event, "payload") do
                  payload when is_map(payload) -> Map.merge(acc, payload)
                  _ -> acc
                end
              end)

            {:ok,
             %{
               "entity_id" => entity_id,
               "last_updated" => events |> List.last() |> Map.get("timestamp"),
               "event_count" => length(events),
               "as_of" => as_of,
               "current_state" => merged,
               "history" =>
                 Enum.map(events, fn e ->
                   %{
                     "event_id" => Map.get(e, "event_id") || Map.get(e, "id"),
                     "type" => Map.get(e, "event_type"),
                     "timestamp" => Map.get(e, "timestamp"),
                     "payload" => Map.get(e, "payload")
                   }
                 end),
               "source" => "folded_from_events",
               "note" =>
                 "Folded locally from the entity's events because " <>
                   "GET /api/v1/entities/#{entity_id}/state returned HTTP #{status}."
             }}
        end

      {:error, reason} ->
        {:error,
         "GET /api/v1/entities/#{entity_id}/state returned HTTP #{status} and " <>
           "folding the entity's events also failed: #{inspect(reason)}"}
    end
  end

  @doc false
  # The Authorization header sent with every request, or [] when no key is
  # configured. Public (undocumented) so tests can assert it without a socket.
  def auth_headers do
    case Application.get_env(:mcp_server_elixir, :core_api_key) do
      nil -> []
      "" -> []
      "Bearer " <> _ = key -> [{"authorization", key}]
      key -> [{"authorization", "Bearer " <> key}]
    end
  end

  @doc "Query events with flexible filters"
  @impl true
  def query_events(params) when is_map(params) do
    # Clean up params - remove nil values
    query_params =
      params
      |> Enum.reject(fn {_k, v} -> is_nil(v) end)
      |> Enum.into(%{})

    case get("/api/v1/events/query", query: query_params) do
      {:ok, %Tesla.Env{status: 200, body: body}} ->
        {:ok, body}

      {:ok, %Tesla.Env{status: status, body: body}} ->
        {:error, "HTTP #{status}: #{inspect(body)}"}

      {:error, reason} ->
        {:error, reason}
    end
  end

  @doc """
  Reconstruct entity state at a point in time.

  Falls back to folding the entity's events locally when the endpoint is absent
  (see `route_absent?/1` — #229).
  """
  @impl true
  def reconstruct_state(entity_id, as_of \\ nil) do
    path = "/api/v1/entities/#{entity_id}/state"
    query_params = if as_of, do: [as_of: as_of], else: []

    case get(path, query: query_params) do
      {:ok, %Tesla.Env{status: 200, body: body}} ->
        {:ok, body}

      {:ok, %Tesla.Env{status: status, body: body}} ->
        if route_absent?(status) do
          fold_entity_state(entity_id, as_of, status)
        else
          {:error, "HTTP #{status}: #{inspect(body)}"}
        end

      {:error, reason} ->
        {:error, reason}
    end
  end

  @doc "Get current snapshot of an entity"
  @impl true
  def get_snapshot(entity_id) do
    case get("/api/v1/entities/#{entity_id}/snapshot") do
      {:ok, %Tesla.Env{status: 200, body: body}} ->
        {:ok, body}

      {:ok, %Tesla.Env{status: status, body: body}} ->
        {:error, "HTTP #{status}: #{inspect(body)}"}

      {:error, reason} ->
        {:error, reason}
    end
  end

  @doc "Ingest a new event (Core returns 200, not 201)"
  @impl true
  def ingest_event(event_data) do
    case post("/api/v1/events", event_data) do
      {:ok, %Tesla.Env{status: status, body: body}} when status in [200, 201] ->
        {:ok, body}

      {:ok, %Tesla.Env{status: status, body: body}} ->
        {:error, "HTTP #{status}: #{inspect(body)}"}

      {:error, reason} ->
        {:error, reason}
    end
  end

  @doc """
  Get event store statistics.

  `GET /api/v1/stats` is a **global, whole-store** endpoint on Core
  (`store.stats()`), which the hosted gateway deliberately does not route — a
  tenant must not see cross-tenant totals. Against a gateway it therefore 404s,
  which used to fail every stats tool with a bare `-32603` (#229).

  When the route is absent we derive the same shape from the tenant's own event
  stream via `query_events/1` (which the gateway does route, tenant-scoped), and
  label the result as approximate.
  """
  @impl true
  def get_stats do
    case get("/api/v1/stats") do
      {:ok, %Tesla.Env{status: 200, body: body}} ->
        {:ok, body}

      {:ok, %Tesla.Env{status: status, body: body}} ->
        if route_absent?(status) do
          derive_stats_from_events(status)
        else
          {:error, "HTTP #{status}: #{inspect(body)}"}
        end

      {:error, reason} ->
        {:error, reason}
    end
  end

  @doc """
  Perform semantic (vector) search on events.

  Uses embeddings to find semantically similar events based on natural language queries.

  ## Parameters
    - `query` - Natural language search query
    - `limit` - Maximum number of results (default: 100)
    - `threshold` - Minimum similarity threshold 0.0-1.0 (default: 0.7)
  """
  @impl true
  def semantic_search(params) when is_map(params) do
    query_params =
      params
      |> Enum.reject(fn {_k, v} -> is_nil(v) end)
      |> Enum.into(%{})

    case get("/api/v1/search/semantic", query: query_params) do
      {:ok, %Tesla.Env{status: 200, body: body}} ->
        {:ok, body}

      {:ok, %Tesla.Env{status: status, body: body}} ->
        {:error, "HTTP #{status}: #{inspect(body)}"}

      {:error, reason} ->
        {:error, reason}
    end
  end

  @doc """
  Perform hybrid search combining semantic and keyword search.

  Uses both vector similarity (semantic) and BM25 (keyword) scoring with
  Reciprocal Rank Fusion (RRF) for optimal results.

  ## Parameters
    - `semantic_query` - Natural language query for semantic search (optional)
    - `keywords` - Keywords for BM25 search (optional)
    - `filters` - Object with optional filters:
      - `event_type` - Filter by event type
      - `entity_id` - Filter by entity ID
      - `time_from` - Filter events after this ISO timestamp
      - `time_to` - Filter events before this ISO timestamp
    - `limit` - Maximum number of results (default: 100)
  """
  @impl true
  def hybrid_search(params) when is_map(params) do
    # Build the request body, filtering out nil values
    body =
      params
      |> Enum.reject(fn {_k, v} -> is_nil(v) end)
      |> Enum.into(%{})

    case post("/api/v1/search/hybrid", body) do
      {:ok, %Tesla.Env{status: 200, body: response_body}} ->
        {:ok, response_body}

      {:ok, %Tesla.Env{status: status, body: response_body}} ->
        {:error, "HTTP #{status}: #{inspect(response_body)}"}

      {:error, reason} ->
        {:error, reason}
    end
  end

  # ============================================================================
  # Operational Endpoints
  # ============================================================================

  @doc "Trigger manual storage compaction (Core: POST /api/v1/compaction/trigger)"
  @impl true
  def compact_storage(params \\ %{}) do
    case post("/api/v1/compaction/trigger", params) do
      {:ok, %Tesla.Env{status: status, body: body}} when status in [200, 202] ->
        {:ok, body}

      {:ok, %Tesla.Env{status: status, body: body}} ->
        {:error, "HTTP #{status}: #{inspect(body)}"}

      {:error, reason} ->
        {:error, reason}
    end
  end

  @doc "Get storage statistics — derived from Core stats + compaction stats"
  @impl true
  def storage_stats(_params \\ %{}) do
    # Core doesn't have a dedicated storage stats endpoint.
    # Combine event-store stats with compaction stats for a useful picture.
    # get_stats/0 carries the gateway fallback, so this degrades with it (#229).
    compaction =
      case get("/api/v1/compaction/stats") do
        {:ok, %Tesla.Env{status: 200, body: body}} -> body
        _ -> "unavailable"
      end

    case get_stats() do
      {:ok, stats} -> {:ok, %{"event_store" => stats, "compaction" => compaction}}
      {:error, reason} -> {:error, reason}
    end
  end

  @doc "Get partition health and distribution info (Core: GET /api/v1/cluster/partitions)"
  @impl true
  def partition_info(_params \\ %{}) do
    case get("/api/v1/cluster/partitions") do
      {:ok, %Tesla.Env{status: 200, body: body}} ->
        {:ok, body}

      {:ok, %Tesla.Env{status: 503, body: body}} ->
        # Cluster mode not enabled — return informative response
        {:ok, %{"status" => "cluster_not_enabled", "detail" => body}}

      {:ok, %Tesla.Env{status: status, body: body}} ->
        {:error, "HTTP #{status}: #{inspect(body)}"}

      {:error, reason} ->
        {:error, reason}
    end
  end

  @doc "Get WAL/replication status from the health endpoint"
  @impl true
  def wal_status do
    # Core's /health endpoint includes replication status
    case get("/health") do
      {:ok, %Tesla.Env{status: 200, body: body}} ->
        wal_info = %{
          "role" => body["role"],
          "replication" => body["replication"],
          "system_streams" => body["system_streams"]
        }

        {:ok, wal_info}

      {:ok, %Tesla.Env{status: status, body: body}} ->
        {:error, "HTTP #{status}: #{inspect(body)}"}

      {:error, reason} ->
        {:error, reason}
    end
  end

  @doc "Create a backup snapshot — not yet implemented in Core"
  @impl true
  def backup_create(_params \\ %{}) do
    {:error,
     "Backup API is not yet implemented in AllSource Core. Use WAL + Parquet files on disk for manual backups."}
  end

  @doc "Restore from a backup snapshot — not yet implemented in Core"
  @impl true
  def backup_restore(_params) do
    {:error,
     "Backup restore API is not yet implemented in AllSource Core. Restore by replacing WAL/Parquet data directory and restarting."}
  end

  @doc "List available backup snapshots — not yet implemented in Core"
  @impl true
  def backup_list(_params \\ %{}) do
    {:error,
     "Backup API is not yet implemented in AllSource Core. Check the data directory for WAL and Parquet files."}
  end

  @doc "Deep health check — uses Core's /health endpoint which includes system streams and replication"
  @impl true
  def health_deep do
    case get("/health") do
      {:ok, %Tesla.Env{status: 200, body: body}} ->
        {:ok, body}

      {:ok, %Tesla.Env{status: status, body: body}} ->
        {:error, "HTTP #{status}: #{inspect(body)}"}

      {:error, reason} ->
        {:error, reason}
    end
  end

  @doc "Get performance metrics — not yet implemented as a dedicated endpoint"
  @impl true
  def performance_report(_params \\ %{}) do
    # Core exposes Prometheus metrics at /metrics but not a JSON summary.
    # Return stats as a proxy for performance data.
    case get("/api/v1/stats") do
      {:ok, %Tesla.Env{status: 200, body: stats}} ->
        {:ok,
         %{
           "source" => "event_store_stats",
           "note" =>
             "Dedicated performance endpoint not yet available. Showing event store statistics. For detailed metrics, scrape /metrics (Prometheus format).",
           "stats" => stats
         }}

      {:ok, %Tesla.Env{status: status, body: body}} ->
        {:error, "HTTP #{status}: #{inspect(body)}"}

      {:error, reason} ->
        {:error, reason}
    end
  end

  @doc "Query the audit trail (Core: GET /api/v1/audit/events)"
  @impl true
  def audit_log(params \\ %{}) do
    query_params =
      params
      |> Enum.reject(fn {_k, v} -> is_nil(v) end)
      |> Enum.into(%{})

    case get("/api/v1/audit/events", query: query_params) do
      {:ok, %Tesla.Env{status: 200, body: body}} ->
        {:ok, body}

      {:ok, %Tesla.Env{status: status, body: body}} ->
        {:error, "HTTP #{status}: #{inspect(body)}"}

      {:error, reason} ->
        {:error, reason}
    end
  end

  @doc "Get cluster status (Core: GET /api/v1/cluster/status)"
  @impl true
  def get_cluster_status do
    case get("/api/v1/cluster/status") do
      {:ok, %Tesla.Env{status: 200, body: body}} ->
        {:ok, body}

      {:ok, %Tesla.Env{status: 503, body: body}} ->
        # Cluster mode not enabled — fall back to health endpoint
        case get("/health") do
          {:ok, %Tesla.Env{status: 200, body: health}} ->
            {:ok,
             %{
               "cluster_mode" => "disabled",
               "node_status" => health,
               "note" => "Cluster mode is not enabled. Showing single-node health."
             }}

          _ ->
            {:ok, %{"cluster_mode" => "disabled", "detail" => body}}
        end

      {:ok, %Tesla.Env{status: status, body: body}} ->
        {:error, "HTTP #{status}: #{inspect(body)}"}

      {:error, reason} ->
        {:error, reason}
    end
  end

  @impl true
  def durability_status do
    # Remote Core doesn't expose a durability endpoint yet — compose from existing endpoints
    with {:ok, stats} <- get_stats(),
         {:ok, wal} <- wal_status() do
      {:ok,
       %{
         "memory_events" => Map.get(stats, "total_events", 0),
         "wal_enabled" => true,
         "wal_entries" => Map.get(wal, "total_entries", 0),
         "parquet_enabled" => true,
         "durable" => true,
         "warnings" => []
       }}
    end
  end

  # ============================================================================
  # Schema Endpoints
  # ============================================================================

  @doc "Register a new event type schema"
  @impl true
  def register_schema(params) when is_map(params) do
    case post("/api/v1/schemas", params) do
      {:ok, %Tesla.Env{status: status, body: body}} when status in [200, 201] ->
        {:ok, body}

      {:ok, %Tesla.Env{status: status, body: body}} ->
        {:error, "HTTP #{status}: #{inspect(body)}"}

      {:error, reason} ->
        {:error, reason}
    end
  end

  @doc "Validate an event payload against a registered schema"
  @impl true
  def validate_schema(params) when is_map(params) do
    case post("/api/v1/schemas/validate", params) do
      {:ok, %Tesla.Env{status: 200, body: body}} ->
        {:ok, body}

      {:ok, %Tesla.Env{status: status, body: body}} ->
        {:error, "HTTP #{status}: #{inspect(body)}"}

      {:error, reason} ->
        {:error, reason}
    end
  end

  @doc "List all registered schema subjects"
  @impl true
  def list_schemas(params \\ %{}) do
    query_params =
      params
      |> Enum.reject(fn {_k, v} -> is_nil(v) end)
      |> Enum.into(%{})

    case get("/api/v1/schemas", query: query_params) do
      {:ok, %Tesla.Env{status: 200, body: body}} ->
        {:ok, body}

      {:ok, %Tesla.Env{status: status, body: body}} ->
        {:error, "HTTP #{status}: #{inspect(body)}"}

      {:error, reason} ->
        {:error, reason}
    end
  end

  @doc "Get a specific schema by subject and optional version"
  @impl true
  def get_schema(subject, params \\ %{}) do
    query_params =
      params
      |> Enum.reject(fn {_k, v} -> is_nil(v) end)
      |> Enum.into(%{})

    case get("/api/v1/schemas/#{subject}", query: query_params) do
      {:ok, %Tesla.Env{status: 200, body: body}} ->
        {:ok, body}

      {:ok, %Tesla.Env{status: status, body: body}} ->
        {:error, "HTTP #{status}: #{inspect(body)}"}

      {:error, reason} ->
        {:error, reason}
    end
  end

  @doc "List schema versions for a subject"
  @impl true
  def list_schema_versions(subject) do
    case get("/api/v1/schemas/#{subject}/versions") do
      {:ok, %Tesla.Env{status: 200, body: body}} ->
        {:ok, body}

      {:ok, %Tesla.Env{status: status, body: body}} ->
        {:error, "HTTP #{status}: #{inspect(body)}"}

      {:error, reason} ->
        {:error, reason}
    end
  end

  @doc "Set compatibility mode for a schema subject"
  @impl true
  def set_compatibility(subject, params) when is_map(params) do
    case put("/api/v1/schemas/#{subject}/compatibility", params) do
      {:ok, %Tesla.Env{status: 200, body: body}} ->
        {:ok, body}

      {:ok, %Tesla.Env{status: status, body: body}} ->
        {:error, "HTTP #{status}: #{inspect(body)}"}

      {:error, reason} ->
        {:error, reason}
    end
  end

  # ============================================================================
  # Analytics Endpoints
  # ============================================================================

  @doc "Get event frequency analytics"
  @impl true
  def analytics_frequency(params \\ %{}) do
    query_params =
      params
      |> Enum.reject(fn {_k, v} -> is_nil(v) end)
      |> Enum.into(%{})

    case get("/api/v1/analytics/frequency", query: query_params) do
      {:ok, %Tesla.Env{status: 200, body: body}} ->
        {:ok, body}

      {:ok, %Tesla.Env{status: status, body: body}} ->
        {:error, "HTTP #{status}: #{inspect(body)}"}

      {:error, reason} ->
        {:error, reason}
    end
  end

  @doc "Get analytics summary statistics"
  @impl true
  def analytics_summary(params \\ %{}) do
    query_params =
      params
      |> Enum.reject(fn {_k, v} -> is_nil(v) end)
      |> Enum.into(%{})

    case get("/api/v1/analytics/summary", query: query_params) do
      {:ok, %Tesla.Env{status: 200, body: body}} ->
        {:ok, body}

      {:ok, %Tesla.Env{status: status, body: body}} ->
        {:error, "HTTP #{status}: #{inspect(body)}"}

      {:error, reason} ->
        {:error, reason}
    end
  end

  @doc "Get correlation analysis between event types"
  @impl true
  def analytics_correlation(params \\ %{}) do
    query_params =
      params
      |> Enum.reject(fn {_k, v} -> is_nil(v) end)
      |> Enum.into(%{})

    case get("/api/v1/analytics/correlation", query: query_params) do
      {:ok, %Tesla.Env{status: 200, body: body}} ->
        {:ok, body}

      {:ok, %Tesla.Env{status: status, body: body}} ->
        {:error, "HTTP #{status}: #{inspect(body)}"}

      {:error, reason} ->
        {:error, reason}
    end
  end
end
