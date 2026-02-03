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
end
