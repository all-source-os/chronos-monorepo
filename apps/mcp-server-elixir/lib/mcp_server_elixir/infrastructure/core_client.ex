defmodule McpServerElixir.Infrastructure.CoreClient do
  @moduledoc """
  HTTP client for communicating with the Rust Core API.

  Provides methods for querying events, reconstructing state, and managing
  the event store.
  """

  use Tesla

  @default_base_url "http://localhost:3900"
  @default_timeout 30_000

  plug(
    Tesla.Middleware.BaseUrl,
    Application.get_env(:mcp_server_elixir, :core_url, @default_base_url)
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

  def new do
    %{client: __MODULE__}
  end

  @doc "Query events with flexible filters"
  def query_events(_client, params) when is_map(params) do
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

  @doc "Reconstruct entity state at a point in time"
  def reconstruct_state(_client, entity_id, as_of \\ nil) do
    path = "/api/v1/entities/#{entity_id}/state"
    query_params = if as_of, do: [as_of: as_of], else: []

    case get(path, query: query_params) do
      {:ok, %Tesla.Env{status: 200, body: body}} ->
        {:ok, body}

      {:ok, %Tesla.Env{status: status, body: body}} ->
        {:error, "HTTP #{status}: #{inspect(body)}"}

      {:error, reason} ->
        {:error, reason}
    end
  end

  @doc "Get current snapshot of an entity"
  def get_snapshot(_client, entity_id) do
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
  def ingest_event(_client, event_data) do
    case post("/api/v1/events", event_data) do
      {:ok, %Tesla.Env{status: status, body: body}} when status in [200, 201] ->
        {:ok, body}

      {:ok, %Tesla.Env{status: status, body: body}} ->
        {:error, "HTTP #{status}: #{inspect(body)}"}

      {:error, reason} ->
        {:error, reason}
    end
  end

  @doc "Get event store statistics"
  def get_stats(_client) do
    case get("/api/v1/stats") do
      {:ok, %Tesla.Env{status: 200, body: body}} ->
        {:ok, body}

      {:ok, %Tesla.Env{status: status, body: body}} ->
        {:error, "HTTP #{status}: #{inspect(body)}"}

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
  def semantic_search(_client, params) when is_map(params) do
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
  def hybrid_search(_client, params) when is_map(params) do
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
  def compact_storage(_client, params \\ %{}) do
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
  def storage_stats(_client, _params \\ %{}) do
    # Core doesn't have a dedicated storage stats endpoint.
    # Combine /api/v1/stats and /api/v1/compaction/stats for a useful picture.
    stats_result = get("/api/v1/stats")
    compaction_result = get("/api/v1/compaction/stats")

    case {stats_result, compaction_result} do
      {{:ok, %Tesla.Env{status: 200, body: stats}},
       {:ok, %Tesla.Env{status: 200, body: compaction}}} ->
        {:ok, %{"event_store" => stats, "compaction" => compaction}}

      {{:ok, %Tesla.Env{status: 200, body: stats}}, _} ->
        {:ok, %{"event_store" => stats, "compaction" => "unavailable"}}

      {{:error, reason}, _} ->
        {:error, reason}

      {{:ok, %Tesla.Env{status: status, body: body}}, _} ->
        {:error, "HTTP #{status}: #{inspect(body)}"}
    end
  end

  @doc "Get partition health and distribution info (Core: GET /api/v1/cluster/partitions)"
  def partition_info(_client, _params \\ %{}) do
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
  def wal_status(_client) do
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
  def backup_create(_client, _params \\ %{}) do
    {:error,
     "Backup API is not yet implemented in AllSource Core. Use WAL + Parquet files on disk for manual backups."}
  end

  @doc "Restore from a backup snapshot — not yet implemented in Core"
  def backup_restore(_client, _params) do
    {:error,
     "Backup restore API is not yet implemented in AllSource Core. Restore by replacing WAL/Parquet data directory and restarting."}
  end

  @doc "List available backup snapshots — not yet implemented in Core"
  def backup_list(_client, _params \\ %{}) do
    {:error,
     "Backup API is not yet implemented in AllSource Core. Check the data directory for WAL and Parquet files."}
  end

  @doc "Deep health check — uses Core's /health endpoint which includes system streams and replication"
  def health_deep(_client) do
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
  def performance_report(_client, _params \\ %{}) do
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
  def audit_log(_client, params \\ %{}) do
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
  def get_cluster_status(_client) do
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

  # ============================================================================
  # Schema Endpoints
  # ============================================================================

  @doc "Register a new event type schema"
  def register_schema(_client, params) when is_map(params) do
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
  def validate_schema(_client, params) when is_map(params) do
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
  def list_schemas(_client, params \\ %{}) do
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
  def get_schema(_client, subject, params \\ %{}) do
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
  def list_schema_versions(_client, subject) do
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
  def set_compatibility(_client, subject, params) when is_map(params) do
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
  def analytics_frequency(_client, params \\ %{}) do
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
  def analytics_summary(_client, params \\ %{}) do
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
  def analytics_correlation(_client, params \\ %{}) do
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
