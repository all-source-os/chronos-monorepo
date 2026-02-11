defmodule QueryServiceExWeb.Schemas.Analytics do
  @moduledoc """
  OpenAPI schemas for analytics endpoints.
  """

  require OpenApiSpex

  alias OpenApiSpex.Schema

  # -------------------------------------------------------------------
  # Frequency Response
  # -------------------------------------------------------------------

  defmodule TimeBucket do
    @moduledoc "Time bucket with event counts"
    OpenApiSpex.schema(%{
      title: "TimeBucket",
      description: "Time bucket with event counts",
      type: :object,
      properties: %{
        timestamp: %Schema{type: :string, format: :"date-time", description: "Bucket start time"},
        count: %Schema{type: :integer, description: "Total event count in bucket"},
        event_types: %Schema{
          type: :object,
          additionalProperties: %Schema{type: :integer},
          description: "Event counts by type"
        }
      },
      example: %{
        timestamp: "2026-02-10T00:00:00Z",
        count: 150,
        event_types: %{"user.created" => 50, "user.updated" => 100}
      }
    })
  end

  defmodule TimeRange do
    @moduledoc "Time range"
    OpenApiSpex.schema(%{
      title: "TimeRange",
      description: "Time range for analysis",
      type: :object,
      properties: %{
        from: %Schema{type: :string, format: :"date-time", description: "Start time"},
        to: %Schema{type: :string, format: :"date-time", description: "End time"}
      }
    })
  end

  defmodule FrequencyData do
    @moduledoc "Event frequency data"
    OpenApiSpex.schema(%{
      title: "FrequencyData",
      description: "Event frequency analysis data",
      type: :object,
      properties: %{
        buckets: %Schema{
          type: :array,
          items: TimeBucket,
          description: "Time buckets with event counts"
        },
        total_events: %Schema{type: :integer, description: "Total event count"},
        window: %Schema{type: :string, description: "Time window used"},
        time_range: TimeRange
      }
    })
  end

  defmodule FrequencyResponse do
    @moduledoc "Frequency response"
    OpenApiSpex.schema(%{
      title: "FrequencyResponse",
      description: "Event frequency analysis response",
      type: :object,
      properties: %{
        data: FrequencyData
      },
      required: [:data]
    })
  end

  # -------------------------------------------------------------------
  # Summary Response
  # -------------------------------------------------------------------

  defmodule EventTypeCount do
    @moduledoc "Event type with count"
    OpenApiSpex.schema(%{
      title: "EventTypeCount",
      description: "Event type with count and percentage",
      type: :object,
      properties: %{
        event_type: %Schema{type: :string, description: "Event type name"},
        count: %Schema{type: :integer, description: "Event count"},
        percentage: %Schema{type: :number, format: :float, description: "Percentage of total"}
      }
    })
  end

  defmodule EntityCount do
    @moduledoc "Entity with count"
    OpenApiSpex.schema(%{
      title: "EntityCount",
      description: "Entity with event count and percentage",
      type: :object,
      properties: %{
        entity_id: %Schema{type: :string, description: "Entity ID"},
        count: %Schema{type: :integer, description: "Event count"},
        percentage: %Schema{type: :number, format: :float, description: "Percentage of total"}
      }
    })
  end

  defmodule SummaryData do
    @moduledoc "Statistical summary data"
    OpenApiSpex.schema(%{
      title: "SummaryData",
      description: "Statistical summary data",
      type: :object,
      properties: %{
        total_events: %Schema{type: :integer, description: "Total event count"},
        unique_entities: %Schema{type: :integer, description: "Number of unique entities"},
        unique_event_types: %Schema{type: :integer, description: "Number of unique event types"},
        events_per_day: %Schema{
          type: :number,
          format: :float,
          description: "Average events per day"
        },
        top_event_types: %Schema{
          type: :array,
          items: EventTypeCount,
          description: "Most common event types"
        },
        top_entities: %Schema{
          type: :array,
          items: EntityCount,
          description: "Most active entities"
        },
        first_event: %Schema{
          type: :string,
          format: :"date-time",
          description: "First event timestamp"
        },
        last_event: %Schema{
          type: :string,
          format: :"date-time",
          description: "Last event timestamp"
        },
        time_range: TimeRange
      }
    })
  end

  defmodule SummaryResponse do
    @moduledoc "Summary response"
    OpenApiSpex.schema(%{
      title: "SummaryResponse",
      description: "Statistical summary response",
      type: :object,
      properties: %{
        data: SummaryData
      },
      required: [:data]
    })
  end

  # -------------------------------------------------------------------
  # Correlation Response
  # -------------------------------------------------------------------

  defmodule CorrelationExample do
    @moduledoc "Example of correlated events"
    OpenApiSpex.schema(%{
      title: "CorrelationExample",
      description: "Example of correlated event pair",
      type: :object,
      properties: %{
        entity_id: %Schema{type: :string, description: "Entity ID"},
        event_a_timestamp: %Schema{type: :string, format: :"date-time"},
        event_b_timestamp: %Schema{type: :string, format: :"date-time"},
        time_between_seconds: %Schema{
          type: :integer,
          description: "Time between events in seconds"
        }
      }
    })
  end

  defmodule CorrelationData do
    @moduledoc "Correlation analysis data"
    OpenApiSpex.schema(%{
      title: "CorrelationData",
      description: "Event correlation analysis data",
      type: :object,
      properties: %{
        event_type_a: %Schema{type: :string, description: "First event type"},
        event_type_b: %Schema{type: :string, description: "Second event type"},
        total_a: %Schema{type: :integer, description: "Total events of type A"},
        total_b: %Schema{type: :integer, description: "Total events of type B"},
        correlated_pairs: %Schema{type: :integer, description: "Number of correlated pairs"},
        correlation_percentage: %Schema{
          type: :number,
          format: :float,
          description: "Correlation percentage"
        },
        avg_time_between_seconds: %Schema{
          type: :number,
          format: :float,
          description: "Average time between correlated events"
        },
        examples: %Schema{
          type: :array,
          items: CorrelationExample,
          description: "Sample correlated pairs"
        }
      }
    })
  end

  defmodule CorrelationResponse do
    @moduledoc "Correlation response"
    OpenApiSpex.schema(%{
      title: "CorrelationResponse",
      description: "Event correlation analysis response",
      type: :object,
      properties: %{
        data: CorrelationData
      },
      required: [:data]
    })
  end

  # -------------------------------------------------------------------
  # Percentiles Response
  # -------------------------------------------------------------------

  defmodule PercentilesData do
    @moduledoc "Percentiles data"
    OpenApiSpex.schema(%{
      title: "PercentilesData",
      description: "Percentile calculation results",
      type: :object,
      properties: %{
        event_type: %Schema{type: :string, description: "Event type analyzed"},
        field: %Schema{type: :string, description: "Field analyzed"},
        sample_size: %Schema{type: :integer, description: "Number of data points"},
        percentiles: %Schema{
          type: :object,
          additionalProperties: %Schema{type: :number},
          description: "Percentile values (e.g., {50: 100, 90: 250, 99: 500})"
        }
      },
      example: %{
        event_type: "api.request",
        field: "duration_ms",
        sample_size: 10_000,
        percentiles: %{"50" => 100, "90" => 250, "95" => 400, "99" => 800}
      }
    })
  end

  defmodule PercentilesResponse do
    @moduledoc "Percentiles response"
    OpenApiSpex.schema(%{
      title: "PercentilesResponse",
      description: "Percentile calculation response",
      type: :object,
      properties: %{
        data: PercentilesData
      },
      required: [:data]
    })
  end

  # -------------------------------------------------------------------
  # Stddev Response
  # -------------------------------------------------------------------

  defmodule StddevData do
    @moduledoc "Standard deviation data"
    OpenApiSpex.schema(%{
      title: "StddevData",
      description: "Standard deviation calculation results",
      type: :object,
      properties: %{
        event_type: %Schema{type: :string, description: "Event type analyzed"},
        field: %Schema{type: :string, description: "Field analyzed"},
        sample_size: %Schema{type: :integer, description: "Number of data points"},
        mean: %Schema{type: :number, format: :float, description: "Mean value"},
        variance: %Schema{type: :number, format: :float, description: "Variance"},
        stddev: %Schema{type: :number, format: :float, description: "Standard deviation"},
        min: %Schema{type: :number, description: "Minimum value"},
        max: %Schema{type: :number, description: "Maximum value"}
      },
      example: %{
        event_type: "api.request",
        field: "duration_ms",
        sample_size: 10_000,
        mean: 150.5,
        variance: 2500.0,
        stddev: 50.0,
        min: 10,
        max: 1200
      }
    })
  end

  defmodule StddevResponse do
    @moduledoc "Stddev response"
    OpenApiSpex.schema(%{
      title: "StddevResponse",
      description: "Standard deviation calculation response",
      type: :object,
      properties: %{
        data: StddevData
      },
      required: [:data]
    })
  end

  # -------------------------------------------------------------------
  # Sliding Window Response
  # -------------------------------------------------------------------

  defmodule SlidingWindowData do
    @moduledoc "Sliding window data"
    OpenApiSpex.schema(%{
      title: "SlidingWindowData",
      description: "Sliding window aggregation data",
      type: :object,
      properties: %{
        buckets: %Schema{
          type: :array,
          items: TimeBucket,
          description: "Overlapping window buckets"
        },
        total_events: %Schema{type: :integer, description: "Total event count"},
        window: %Schema{type: :string, description: "Window size"},
        slide: %Schema{type: :string, description: "Slide interval"},
        time_range: TimeRange
      }
    })
  end

  defmodule SlidingWindowResponse do
    @moduledoc "Sliding window response"
    OpenApiSpex.schema(%{
      title: "SlidingWindowResponse",
      description: "Sliding window aggregation response",
      type: :object,
      properties: %{
        data: SlidingWindowData
      },
      required: [:data]
    })
  end

  # -------------------------------------------------------------------
  # Session Window Response
  # -------------------------------------------------------------------

  defmodule SessionSummary do
    @moduledoc "Session summary"
    OpenApiSpex.schema(%{
      title: "SessionSummary",
      description: "Session summary",
      type: :object,
      properties: %{
        session_id: %Schema{type: :integer, description: "Session number"},
        start_time: %Schema{type: :string, format: :"date-time", description: "Session start"},
        end_time: %Schema{type: :string, format: :"date-time", description: "Session end"},
        event_count: %Schema{type: :integer, description: "Events in session"},
        event_types: %Schema{
          type: :object,
          additionalProperties: %Schema{type: :integer},
          description: "Event counts by type"
        },
        duration_seconds: %Schema{type: :integer, description: "Session duration"}
      }
    })
  end

  defmodule SessionWindowData do
    @moduledoc "Session window data"
    OpenApiSpex.schema(%{
      title: "SessionWindowData",
      description: "Session window aggregation data",
      type: :object,
      properties: %{
        entity_id: %Schema{type: :string, description: "Entity ID"},
        sessions: %Schema{
          type: :array,
          items: SessionSummary,
          description: "Session summaries"
        },
        total_sessions: %Schema{type: :integer, description: "Total number of sessions"},
        avg_session_duration: %Schema{
          type: :number,
          format: :float,
          description: "Average session duration"
        },
        avg_events_per_session: %Schema{
          type: :number,
          format: :float,
          description: "Average events per session"
        }
      }
    })
  end

  defmodule SessionWindowResponse do
    @moduledoc "Session window response"
    OpenApiSpex.schema(%{
      title: "SessionWindowResponse",
      description: "Session window aggregation response",
      type: :object,
      properties: %{
        data: SessionWindowData
      },
      required: [:data]
    })
  end

  # -------------------------------------------------------------------
  # Cache Stats Response
  # -------------------------------------------------------------------

  defmodule CacheStatsData do
    @moduledoc "Cache statistics"
    OpenApiSpex.schema(%{
      title: "CacheStatsData",
      description: "Analytics cache statistics",
      type: :object,
      properties: %{
        size: %Schema{type: :integer, description: "Number of cached entries"},
        memory: %Schema{type: :integer, description: "Memory usage in bytes"}
      },
      example: %{size: 150, memory: 524_288}
    })
  end

  defmodule CacheStatsResponse do
    @moduledoc "Cache stats response"
    OpenApiSpex.schema(%{
      title: "CacheStatsResponse",
      description: "Cache statistics response",
      type: :object,
      properties: %{
        data: CacheStatsData
      },
      required: [:data]
    })
  end
end
