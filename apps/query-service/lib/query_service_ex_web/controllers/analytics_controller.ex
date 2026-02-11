defmodule QueryServiceExWeb.AnalyticsController do
  @moduledoc """
  Controller for analytics endpoints.

  Provides time-series analysis, statistical functions, and event correlation.
  Results are cached in ETS with configurable TTL for performance.
  """

  use Phoenix.Controller, formats: [:json]
  use OpenApiSpex.ControllerSpecs

  alias QueryServiceEx.Infrastructure.Adapters.AnalyticsClient
  alias QueryServiceExWeb.Schemas.Analytics
  alias QueryServiceExWeb.Schemas.Common

  action_fallback(QueryServiceExWeb.FallbackController)

  tags(["Analytics"])

  # -------------------------------------------------------------------
  # Event Frequency
  # -------------------------------------------------------------------

  operation(:frequency,
    summary: "Event frequency analysis",
    description: """
    Analyze event frequency over time windows (tumbling windows).
    Returns time-series data with event counts per bucket.
    Results are cached for 5 minutes by default.
    """,
    security: [%{"bearer_auth" => []}],
    parameters: [
      since: [in: :query, type: :string, description: "Start time (ISO8601)", required: true],
      until: [in: :query, type: :string, description: "End time (ISO8601, default: now)"],
      window: [
        in: :query,
        type: :string,
        description: "Time window: minute, hour, day, week, month",
        required: true
      ],
      entity_id: [in: :query, type: :string, description: "Filter by entity ID"],
      event_type: [in: :query, type: :string, description: "Filter by event type"],
      use_cache: [in: :query, type: :boolean, description: "Use cache (default: true)"]
    ],
    responses: [
      ok: {"Frequency data", "application/json", Analytics.FrequencyResponse},
      bad_request: {"Invalid parameters", "application/json", Common.SimpleError},
      unauthorized: {"Unauthorized", "application/json", Common.Error}
    ]
  )

  @doc """
  Analyze event frequency over time windows.
  """
  def frequency(conn, params) do
    tenant_id = get_tenant_id(conn)

    opts = [
      since: params["since"],
      until: params["until"],
      window: params["window"],
      entity_id: params["entity_id"],
      event_type: params["event_type"],
      use_cache: parse_bool(params["use_cache"], true)
    ]

    case AnalyticsClient.event_frequency(tenant_id, opts) do
      {:ok, data} ->
        json(conn, %{data: data})

      {:error, :missing_since} ->
        conn
        |> put_status(:bad_request)
        |> json(%{error: "Missing required parameter: since"})

      {:error, :missing_window} ->
        conn
        |> put_status(:bad_request)
        |> json(%{error: "Missing required parameter: window"})

      {:error, {:invalid_window, window}} ->
        conn
        |> put_status(:bad_request)
        |> json(%{
          error: "Invalid window: #{window}. Valid options: minute, hour, day, week, month"
        })

      {:error, reason} ->
        conn
        |> put_status(:bad_request)
        |> json(%{error: to_string(reason)})
    end
  end

  # -------------------------------------------------------------------
  # Statistical Summary
  # -------------------------------------------------------------------

  operation(:summary,
    summary: "Statistical summary",
    description: """
    Get comprehensive statistical summary of events.
    Includes total counts, unique entities/types, top entities, and time range.
    Results are cached for 10 minutes by default.
    """,
    security: [%{"bearer_auth" => []}],
    parameters: [
      since: [in: :query, type: :string, description: "Start time (ISO8601)"],
      until: [in: :query, type: :string, description: "End time (ISO8601)"],
      entity_id: [in: :query, type: :string, description: "Filter by entity ID"],
      event_type: [in: :query, type: :string, description: "Filter by event type"],
      use_cache: [in: :query, type: :boolean, description: "Use cache (default: true)"]
    ],
    responses: [
      ok: {"Summary data", "application/json", Analytics.SummaryResponse},
      bad_request: {"Invalid parameters", "application/json", Common.SimpleError},
      unauthorized: {"Unauthorized", "application/json", Common.Error}
    ]
  )

  @doc """
  Get comprehensive statistical summary.
  """
  def summary(conn, params) do
    tenant_id = get_tenant_id(conn)

    opts = [
      since: params["since"],
      until: params["until"],
      entity_id: params["entity_id"],
      event_type: params["event_type"],
      use_cache: parse_bool(params["use_cache"], true)
    ]

    case AnalyticsClient.stats_summary(tenant_id, opts) do
      {:ok, data} ->
        json(conn, %{data: data})

      {:error, reason} ->
        conn
        |> put_status(:bad_request)
        |> json(%{error: to_string(reason)})
    end
  end

  # -------------------------------------------------------------------
  # Event Correlation
  # -------------------------------------------------------------------

  operation(:correlation,
    summary: "Event correlation analysis",
    description: """
    Analyze correlation between two event types.
    Finds events of type A followed by events of type B within a time window for the same entity.
    Results are cached for 10 minutes by default.
    """,
    security: [%{"bearer_auth" => []}],
    parameters: [
      event_type_a: [in: :query, type: :string, description: "First event type", required: true],
      event_type_b: [in: :query, type: :string, description: "Second event type", required: true],
      time_window_seconds: [
        in: :query,
        type: :integer,
        description: "Max time between correlated events (default: 3600)"
      ],
      since: [in: :query, type: :string, description: "Start time (ISO8601)"],
      until: [in: :query, type: :string, description: "End time (ISO8601)"],
      use_cache: [in: :query, type: :boolean, description: "Use cache (default: true)"]
    ],
    responses: [
      ok: {"Correlation data", "application/json", Analytics.CorrelationResponse},
      bad_request: {"Invalid parameters", "application/json", Common.SimpleError},
      unauthorized: {"Unauthorized", "application/json", Common.Error}
    ]
  )

  @doc """
  Analyze correlation between two event types.
  """
  def correlation(conn, params) do
    tenant_id = get_tenant_id(conn)

    event_type_a = params["event_type_a"]
    event_type_b = params["event_type_b"]

    if is_nil(event_type_a) or is_nil(event_type_b) do
      conn
      |> put_status(:bad_request)
      |> json(%{error: "Both event_type_a and event_type_b are required"})
    else
      opts = [
        time_window_seconds: parse_int(params["time_window_seconds"], 3600),
        since: params["since"],
        until: params["until"],
        use_cache: parse_bool(params["use_cache"], true)
      ]

      case AnalyticsClient.correlation(tenant_id, event_type_a, event_type_b, opts) do
        {:ok, data} ->
          json(conn, %{data: data})

        {:error, reason} ->
          conn
          |> put_status(:bad_request)
          |> json(%{error: to_string(reason)})
      end
    end
  end

  # -------------------------------------------------------------------
  # Percentiles
  # -------------------------------------------------------------------

  operation(:percentiles,
    summary: "Calculate percentiles",
    description: """
    Calculate percentile values for a numeric field in event payloads.
    Computed locally from queried events.
    """,
    security: [%{"bearer_auth" => []}],
    parameters: [
      event_type: [
        in: :query,
        type: :string,
        description: "Event type to analyze",
        required: true
      ],
      field: [
        in: :query,
        type: :string,
        description: "JSON path to numeric field (e.g., 'duration' or 'metrics.latency')",
        required: true
      ],
      percentiles: [
        in: :query,
        type: :string,
        description: "Comma-separated percentiles (default: 50,90,95,99)"
      ],
      since: [in: :query, type: :string, description: "Start time (ISO8601)"],
      until: [in: :query, type: :string, description: "End time (ISO8601)"]
    ],
    responses: [
      ok: {"Percentile data", "application/json", Analytics.PercentilesResponse},
      bad_request: {"Invalid parameters", "application/json", Common.SimpleError},
      unauthorized: {"Unauthorized", "application/json", Common.Error}
    ]
  )

  @doc """
  Calculate percentile values for a numeric field.
  """
  def percentiles(conn, params) do
    tenant_id = get_tenant_id!(conn)

    event_type = params["event_type"]
    field = params["field"]

    cond do
      is_nil(event_type) ->
        conn
        |> put_status(:bad_request)
        |> json(%{error: "event_type is required"})

      is_nil(field) ->
        conn
        |> put_status(:bad_request)
        |> json(%{error: "field is required"})

      true ->
        percentile_list = parse_percentiles(params["percentiles"])

        opts = [
          event_type: event_type,
          field: field,
          percentiles: percentile_list,
          since: params["since"],
          until: params["until"]
        ]

        case AnalyticsClient.percentiles(tenant_id, opts) do
          {:ok, data} ->
            json(conn, %{data: data})

          {:error, :no_data} ->
            conn
            |> put_status(:not_found)
            |> json(%{error: "No numeric data found for the specified field"})

          {:error, reason} ->
            conn
            |> put_status(:bad_request)
            |> json(%{error: to_string(reason)})
        end
    end
  end

  # -------------------------------------------------------------------
  # Standard Deviation
  # -------------------------------------------------------------------

  operation(:stddev,
    summary: "Calculate standard deviation",
    description: """
    Calculate standard deviation, variance, mean, min, and max for a numeric field.
    Computed locally from queried events.
    """,
    security: [%{"bearer_auth" => []}],
    parameters: [
      event_type: [
        in: :query,
        type: :string,
        description: "Event type to analyze",
        required: true
      ],
      field: [
        in: :query,
        type: :string,
        description: "JSON path to numeric field",
        required: true
      ],
      since: [in: :query, type: :string, description: "Start time (ISO8601)"],
      until: [in: :query, type: :string, description: "End time (ISO8601)"]
    ],
    responses: [
      ok: {"Statistical data", "application/json", Analytics.StddevResponse},
      bad_request: {"Invalid parameters", "application/json", Common.SimpleError},
      unauthorized: {"Unauthorized", "application/json", Common.Error}
    ]
  )

  @doc """
  Calculate standard deviation and related statistics.
  """
  def stddev(conn, params) do
    tenant_id = get_tenant_id!(conn)

    event_type = params["event_type"]
    field = params["field"]

    cond do
      is_nil(event_type) ->
        conn
        |> put_status(:bad_request)
        |> json(%{error: "event_type is required"})

      is_nil(field) ->
        conn
        |> put_status(:bad_request)
        |> json(%{error: "field is required"})

      true ->
        opts = [
          event_type: event_type,
          field: field,
          since: params["since"],
          until: params["until"]
        ]

        case AnalyticsClient.stddev(tenant_id, opts) do
          {:ok, data} ->
            json(conn, %{data: data})

          {:error, :no_data} ->
            conn
            |> put_status(:not_found)
            |> json(%{error: "No numeric data found for the specified field"})

          {:error, reason} ->
            conn
            |> put_status(:bad_request)
            |> json(%{error: to_string(reason)})
        end
    end
  end

  # -------------------------------------------------------------------
  # Sliding Window
  # -------------------------------------------------------------------

  operation(:sliding_window,
    summary: "Sliding window aggregation",
    description: """
    Perform sliding window aggregation for smoothed time-series data.
    Windows overlap, providing continuous analysis.
    """,
    security: [%{"bearer_auth" => []}],
    parameters: [
      since: [in: :query, type: :string, description: "Start time (ISO8601)", required: true],
      until: [in: :query, type: :string, description: "End time (ISO8601)"],
      window: [
        in: :query,
        type: :string,
        description: "Window size: minute, hour, day, week, month",
        required: true
      ],
      slide: [in: :query, type: :string, description: "Slide interval (default: half of window)"],
      entity_id: [in: :query, type: :string, description: "Filter by entity ID"],
      event_type: [in: :query, type: :string, description: "Filter by event type"]
    ],
    responses: [
      ok: {"Sliding window data", "application/json", Analytics.SlidingWindowResponse},
      bad_request: {"Invalid parameters", "application/json", Common.SimpleError},
      unauthorized: {"Unauthorized", "application/json", Common.Error}
    ]
  )

  @doc """
  Perform sliding window aggregation.
  """
  def sliding_window(conn, params) do
    tenant_id = get_tenant_id(conn)

    opts = [
      since: params["since"],
      until: params["until"],
      window: params["window"],
      slide: params["slide"],
      entity_id: params["entity_id"],
      event_type: params["event_type"]
    ]

    case AnalyticsClient.sliding_window(tenant_id, opts) do
      {:ok, data} ->
        json(conn, %{data: data})

      {:error, reason} ->
        conn
        |> put_status(:bad_request)
        |> json(%{error: to_string(reason)})
    end
  end

  # -------------------------------------------------------------------
  # Session Window
  # -------------------------------------------------------------------

  operation(:session_window,
    summary: "Session window aggregation",
    description: """
    Group events into sessions based on activity gaps.
    Sessions end when there's no activity for the specified gap duration.
    """,
    security: [%{"bearer_auth" => []}],
    parameters: [
      entity_id: [
        in: :query,
        type: :string,
        description: "Entity ID (required - sessions are per-entity)",
        required: true
      ],
      gap_seconds: [
        in: :query,
        type: :integer,
        description: "Inactivity gap to start new session (default: 1800 = 30 min)"
      ],
      since: [in: :query, type: :string, description: "Start time (ISO8601)"],
      until: [in: :query, type: :string, description: "End time (ISO8601)"]
    ],
    responses: [
      ok: {"Session data", "application/json", Analytics.SessionWindowResponse},
      bad_request: {"Invalid parameters", "application/json", Common.SimpleError},
      unauthorized: {"Unauthorized", "application/json", Common.Error}
    ]
  )

  @doc """
  Perform session window aggregation.
  """
  def session_window(conn, params) do
    tenant_id = get_tenant_id!(conn)

    entity_id = params["entity_id"]

    if is_nil(entity_id) do
      conn
      |> put_status(:bad_request)
      |> json(%{error: "entity_id is required for session analysis"})
    else
      opts = [
        entity_id: entity_id,
        gap_seconds: parse_int(params["gap_seconds"], 1800),
        since: params["since"],
        until: params["until"]
      ]

      case AnalyticsClient.session_window(tenant_id, opts) do
        {:ok, data} ->
          json(conn, %{data: data})

        {:error, reason} ->
          conn
          |> put_status(:bad_request)
          |> json(%{error: to_string(reason)})
      end
    end
  end

  # -------------------------------------------------------------------
  # Cache Management
  # -------------------------------------------------------------------

  operation(:cache_stats,
    summary: "Get cache statistics",
    description: "Get analytics cache statistics including size and memory usage.",
    security: [%{"bearer_auth" => []}],
    responses: [
      ok: {"Cache stats", "application/json", Analytics.CacheStatsResponse},
      unauthorized: {"Unauthorized", "application/json", Common.Error}
    ]
  )

  @doc """
  Get cache statistics.
  """
  def cache_stats(conn, _params) do
    alias QueryServiceEx.Infrastructure.Adapters.AnalyticsCache
    stats = AnalyticsCache.stats()
    json(conn, %{data: stats})
  end

  operation(:invalidate_cache,
    summary: "Invalidate cache",
    description: "Clear analytics cache for the current tenant.",
    security: [%{"bearer_auth" => []}],
    responses: [
      ok: {"Cache invalidated", "application/json", Common.SimpleSuccess},
      unauthorized: {"Unauthorized", "application/json", Common.Error}
    ]
  )

  @doc """
  Invalidate cache for the current tenant.
  """
  def invalidate_cache(conn, _params) do
    alias QueryServiceEx.Infrastructure.Adapters.AnalyticsCache
    tenant_id = get_tenant_id!(conn)
    AnalyticsCache.invalidate_tenant(tenant_id)
    json(conn, %{success: true, message: "Cache invalidated for tenant"})
  end

  # -------------------------------------------------------------------
  # Private Helpers
  # -------------------------------------------------------------------

  defp get_tenant_id(conn) do
    conn.assigns[:tenant_id]
  end

  defp get_tenant_id!(conn) do
    case conn.assigns[:tenant_id] do
      nil ->
        raise "Tenant context required but not present - security violation"

      tenant_id when is_binary(tenant_id) ->
        tenant_id
    end
  end

  defp parse_int(nil, default), do: default

  defp parse_int(value, default) when is_binary(value) do
    case Integer.parse(value) do
      {int, _} -> int
      :error -> default
    end
  end

  defp parse_int(value, _default) when is_integer(value), do: value
  defp parse_int(_, default), do: default

  defp parse_bool(nil, default), do: default
  defp parse_bool("true", _), do: true
  defp parse_bool("false", _), do: false
  defp parse_bool(true, _), do: true
  defp parse_bool(false, _), do: false
  defp parse_bool(_, default), do: default

  defp parse_percentiles(nil), do: [50, 90, 95, 99]

  defp parse_percentiles(str) when is_binary(str) do
    str
    |> String.split(",")
    |> Enum.map(&String.trim/1)
    |> Enum.map(&parse_int(&1, nil))
    |> Enum.reject(&is_nil/1)
    |> case do
      [] -> [50, 90, 95, 99]
      list -> list
    end
  end
end
