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

  @doc "Ingest a new event"
  def ingest_event(_client, event_data) do
    case post("/api/v1/events", event_data) do
      {:ok, %Tesla.Env{status: 201, body: body}} ->
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

  @doc "Trigger manual storage compaction"
  def compact_storage(_client, params \\ %{}) do
    case post("/api/v1/ops/compact", params) do
      {:ok, %Tesla.Env{status: status, body: body}} when status in [200, 202] ->
        {:ok, body}

      {:ok, %Tesla.Env{status: status, body: body}} ->
        {:error, "HTTP #{status}: #{inspect(body)}"}

      {:error, reason} ->
        {:error, reason}
    end
  end

  @doc "Get storage statistics and disk usage analytics"
  def storage_stats(_client, params \\ %{}) do
    query_params =
      params
      |> Enum.reject(fn {_k, v} -> is_nil(v) end)
      |> Enum.into(%{})

    case get("/api/v1/ops/storage/stats", query: query_params) do
      {:ok, %Tesla.Env{status: 200, body: body}} ->
        {:ok, body}

      {:ok, %Tesla.Env{status: status, body: body}} ->
        {:error, "HTTP #{status}: #{inspect(body)}"}

      {:error, reason} ->
        {:error, reason}
    end
  end

  @doc "Get partition health and distribution info"
  def partition_info(_client, params \\ %{}) do
    query_params =
      params
      |> Enum.reject(fn {_k, v} -> is_nil(v) end)
      |> Enum.into(%{})

    case get("/api/v1/ops/partitions", query: query_params) do
      {:ok, %Tesla.Env{status: 200, body: body}} ->
        {:ok, body}

      {:ok, %Tesla.Env{status: status, body: body}} ->
        {:error, "HTTP #{status}: #{inspect(body)}"}

      {:error, reason} ->
        {:error, reason}
    end
  end

  @doc "Get WAL statistics and lag information"
  def wal_status(_client) do
    case get("/api/v1/ops/wal/status") do
      {:ok, %Tesla.Env{status: 200, body: body}} ->
        {:ok, body}

      {:ok, %Tesla.Env{status: status, body: body}} ->
        {:error, "HTTP #{status}: #{inspect(body)}"}

      {:error, reason} ->
        {:error, reason}
    end
  end

  @doc "Create a backup snapshot"
  def backup_create(_client, params \\ %{}) do
    case post("/api/v1/ops/backups", params) do
      {:ok, %Tesla.Env{status: status, body: body}} when status in [200, 201, 202] ->
        {:ok, body}

      {:ok, %Tesla.Env{status: status, body: body}} ->
        {:error, "HTTP #{status}: #{inspect(body)}"}

      {:error, reason} ->
        {:error, reason}
    end
  end

  @doc "Restore from a backup snapshot"
  def backup_restore(_client, params) do
    case post("/api/v1/ops/backups/restore", params) do
      {:ok, %Tesla.Env{status: status, body: body}} when status in [200, 202] ->
        {:ok, body}

      {:ok, %Tesla.Env{status: status, body: body}} ->
        {:error, "HTTP #{status}: #{inspect(body)}"}

      {:error, reason} ->
        {:error, reason}
    end
  end

  @doc "List available backup snapshots"
  def backup_list(_client, params \\ %{}) do
    query_params =
      params
      |> Enum.reject(fn {_k, v} -> is_nil(v) end)
      |> Enum.into(%{})

    case get("/api/v1/ops/backups", query: query_params) do
      {:ok, %Tesla.Env{status: 200, body: body}} ->
        {:ok, body}

      {:ok, %Tesla.Env{status: status, body: body}} ->
        {:error, "HTTP #{status}: #{inspect(body)}"}

      {:error, reason} ->
        {:error, reason}
    end
  end

  @doc "Deep health check across all components"
  def health_deep(_client) do
    case get("/api/v1/ops/health/deep") do
      {:ok, %Tesla.Env{status: 200, body: body}} ->
        {:ok, body}

      {:ok, %Tesla.Env{status: status, body: body}} ->
        {:error, "HTTP #{status}: #{inspect(body)}"}

      {:error, reason} ->
        {:error, reason}
    end
  end

  @doc "Get performance metrics summary"
  def performance_report(_client, params \\ %{}) do
    query_params =
      params
      |> Enum.reject(fn {_k, v} -> is_nil(v) end)
      |> Enum.into(%{})

    case get("/api/v1/ops/performance", query: query_params) do
      {:ok, %Tesla.Env{status: 200, body: body}} ->
        {:ok, body}

      {:ok, %Tesla.Env{status: status, body: body}} ->
        {:error, "HTTP #{status}: #{inspect(body)}"}

      {:error, reason} ->
        {:error, reason}
    end
  end

  @doc "Query the audit trail"
  def audit_log(_client, params \\ %{}) do
    query_params =
      params
      |> Enum.reject(fn {_k, v} -> is_nil(v) end)
      |> Enum.into(%{})

    case get("/api/v1/ops/audit", query: query_params) do
      {:ok, %Tesla.Env{status: 200, body: body}} ->
        {:ok, body}

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
