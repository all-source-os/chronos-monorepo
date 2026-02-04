defmodule McpServerElixir.Protocol.McpTools do
  @moduledoc """
  MCP Tools implementation - defines all available tools and their handlers.

  This module provides the tools that the MCP server exposes to AI assistants,
  enabling natural language interaction with the AllSource event store.
  """

  require Logger

  alias McpServerElixir.Context.ConversationContext
  alias McpServerElixir.Infrastructure.{ControlPlaneClient, CoreClient}
  alias McpServerElixir.Protocol.ToonEncoder

  @doc """
  List all available MCP tools.
  """
  def list_tools do
    [
      tool_query_events(),
      tool_reconstruct_state(),
      tool_get_snapshot(),
      tool_analyze_changes(),
      tool_find_patterns(),
      tool_compare_entities(),
      tool_event_timeline(),
      tool_explain_entity(),
      tool_ingest_event(),
      tool_get_stats(),
      tool_get_cluster_status(),
      tool_semantic_search_events(),
      tool_hybrid_search(),
      tool_get_query_advice(),
      # Quick exploration tools
      tool_sample_events(),
      tool_quick_stats(),
      # Conversation context tools
      tool_start_session(),
      tool_refine_query(),
      tool_get_session_context()
    ]
  end

  @tool_handlers %{
    "query_events" => :handle_query_events,
    "reconstruct_state" => :handle_reconstruct_state,
    "get_snapshot" => :handle_get_snapshot,
    "analyze_changes" => :handle_analyze_changes,
    "find_patterns" => :handle_find_patterns,
    "compare_entities" => :handle_compare_entities,
    "event_timeline" => :handle_event_timeline,
    "explain_entity" => :handle_explain_entity,
    "ingest_event" => :handle_ingest_event,
    "semantic_search_events" => :handle_semantic_search_events,
    "hybrid_search" => :handle_hybrid_search
  }

  @stateless_tool_handlers %{
    "get_stats" => :handle_get_stats,
    "get_cluster_status" => :handle_get_cluster_status
  }

  # Advisory tools don't need state but do need args
  @advisory_tool_handlers %{
    "get_query_advice" => :handle_get_query_advice
  }

  # Quick exploration tools - optimized for sub-second response
  @exploration_tool_handlers %{
    "sample_events" => :handle_sample_events,
    "quick_stats" => :handle_quick_stats
  }

  # Context tools work with ConversationContext GenServer
  @context_tool_handlers %{
    "start_session" => :handle_start_session,
    "refine_query" => :handle_refine_query,
    "get_session_context" => :handle_get_session_context
  }

  @doc """
  Call a tool by name with arguments.

  Supports optional `format` parameter:
  - `"toon"` - Force TOON format (~50% fewer tokens)
  - `"json"` - Force JSON format
  - Omitted - Auto-detect (default: TOON for tabular data)
  """
  def call_tool(tool_name, args, state) do
    format = Map.get(args, "format", nil)
    args_without_format = Map.delete(args, "format")

    dispatch_tool(tool_name, args_without_format, state, format)
  end

  defp dispatch_tool(tool_name, args, state, format) do
    cond do
      handler = Map.get(@tool_handlers, tool_name) ->
        apply(__MODULE__, handler, [args, state, format])

      handler = Map.get(@stateless_tool_handlers, tool_name) ->
        apply(__MODULE__, handler, [state, format])

      handler = Map.get(@advisory_tool_handlers, tool_name) ->
        apply(__MODULE__, handler, [args, format])

      handler = Map.get(@exploration_tool_handlers, tool_name) ->
        apply(__MODULE__, handler, [args, state, format])

      handler = Map.get(@context_tool_handlers, tool_name) ->
        apply(__MODULE__, handler, [args, state, format])

      true ->
        {:error, "Unknown tool: #{tool_name}"}
    end
  end

  # ============================================================================
  # Tool Definitions
  # ============================================================================
  #
  # DECISION TREE: Choosing the Right Tool
  # ======================================
  #
  # 1. "What is the current state of entity X?"
  #    → get_snapshot (fastest) OR reconstruct_state (if you need time-travel)
  #
  # 2. "What happened to entity X?" / "Show me the history"
  #    → event_timeline (chronological view) OR query_events (raw events)
  #
  # 3. "What did entity X look like on date Y?"
  #    → reconstruct_state with as_of parameter
  #
  # 4. "What changed between date A and date B?"
  #    → analyze_changes
  #
  # 5. "Find events related to [concept/topic]"
  #    → semantic_search_events (natural language) OR query_events (exact filters)
  #
  # 6. "Complex search with filters AND semantic understanding"
  #    → hybrid_search (combines semantic + keyword + metadata filters)
  #
  # 7. "Tell me everything about entity X"
  #    → explain_entity (comprehensive overview)
  #
  # 8. "Compare entities A, B, C"
  #    → compare_entities
  #
  # 9. "What patterns exist in the data?"
  #    → find_patterns
  #
  # 10. "Store a new event"
  #     → ingest_event
  #
  # 11. "System health / statistics"
  #     → get_stats (data stats) OR get_cluster_status (infrastructure health)
  #
  # PERFORMANCE CONSIDERATIONS:
  # - get_snapshot is 10-100x faster than reconstruct_state for current state
  # - Use limit parameter to cap results in query_events
  # - semantic_search_events has higher latency due to embedding generation
  # - hybrid_search is most expensive but most powerful for complex queries
  #
  # ============================================================================

  defp tool_query_events do
    %{
      name: "query_events",
      description: """
      Query events with flexible filters. Returns TOON format by default (~50% fewer tokens).

      **When to use this tool:**
      - Retrieving raw events for a specific entity or event type
      - Filtering events by time range (since/until)
      - Getting events as they existed at a point in time (as_of for time-travel)
      - When you know the exact entity_id or event_type you're looking for

      **Common patterns:**
      - Get all events for an entity: `entity_id: "user-123"`
      - Get specific event types: `event_type: "user.created"`
      - Time-bounded queries: `since: "2024-01-01T00:00:00Z", until: "2024-01-31T23:59:59Z"`
      - Time-travel: `as_of: "2024-01-15T00:00:00Z"` to see events as they were on that date

      **Performance tips:**
      - Always use `limit` for exploratory queries (start with 10-50)
      - Combine `entity_id` with `event_type` for fastest queries
      - Use `since/until` to narrow time ranges on large datasets
      - Prefer get_snapshot over query_events when you only need current state

      **Decision guide:**
      - Need raw events? → query_events
      - Need current state? → get_snapshot (faster)
      - Need state at point in time? → reconstruct_state
      - Don't know exact filters? → semantic_search_events or hybrid_search
      """,
      inputSchema: %{
        type: "object",
        properties: %{
          "entity_id" => %{
            type: "string",
            description: "Filter by entity ID (e.g., \"user-123\")"
          },
          "event_type" => %{
            type: "string",
            description: "Filter by event type (e.g., \"user.created\")"
          },
          "as_of" => %{
            type: "string",
            description: "Time-travel: get events as of this ISO timestamp"
          },
          "since" => %{type: "string", description: "Get events since this ISO timestamp"},
          "until" => %{type: "string", description: "Get events until this ISO timestamp"},
          "limit" => %{type: "number", description: "Limit number of results (default: all)"},
          "format" => %{
            type: "string",
            enum: ["toon", "json"],
            description: "Response format: 'toon' (default, ~50% fewer tokens) or 'json'"
          }
        }
      }
    }
  end

  defp tool_reconstruct_state do
    %{
      name: "reconstruct_state",
      description: """
      Reconstruct the complete state of an entity at any point in time by replaying \
      its event stream. Essential for time-travel queries and audit scenarios.

      **When to use this tool:**
      - Answering "What did this entity look like on [date]?"
      - Debugging: understanding state at the time of an incident
      - Compliance/audit: proving what data looked like at a specific moment
      - Comparing historical vs current state (use with analyze_changes)

      **Common patterns:**
      - Current state: `entity_id: "user-123"` (no as_of = current)
      - Historical state: `entity_id: "user-123", as_of: "2024-01-15T14:30:00Z"`
      - Before an incident: `as_of: "[incident_time - 1 hour]"`

      **Performance tips:**
      - Use get_snapshot instead if you only need current state (10-100x faster)
      - Reconstruction time scales with number of events for the entity
      - For very active entities (1000s of events), consider narrowing time range first
      - Cache results if you need to query the same point multiple times

      **Decision guide:**
      - Need current state? → get_snapshot (much faster)
      - Need state at specific time? → reconstruct_state
      - Need diff between two times? → analyze_changes (uses reconstruct internally)
      - Need full history? → event_timeline
      """,
      inputSchema: %{
        type: "object",
        properties: %{
          "entity_id" => %{type: "string", description: "The entity ID to reconstruct state for"},
          "as_of" => %{
            type: "string",
            description:
              "Reconstruct state as of this ISO timestamp (optional, defaults to current)"
          }
        },
        required: ["entity_id"]
      }
    }
  end

  defp tool_get_snapshot do
    %{
      name: "get_snapshot",
      description: """
      Get the current snapshot of an entity. This is the fastest way to retrieve \
      current state (10-100x faster than reconstruct_state).

      **When to use this tool:**
      - You need the current state of an entity (most common use case)
      - Quick lookups during conversations or workflows
      - Checking if an entity exists and its current values
      - Real-time dashboards or status checks

      **Common patterns:**
      - Simple lookup: `entity_id: "user-123"`
      - Check entity exists: Call get_snapshot, handle not-found gracefully
      - Quick reference during analysis: Get snapshot before deeper investigation

      **Performance tips:**
      - ALWAYS prefer this over reconstruct_state for current state
      - Snapshots are pre-computed and indexed for fast retrieval
      - Negligible cost compared to other operations - use liberally
      - Good for iterative exploration: snapshot → decide → drill deeper

      **Decision guide:**
      - Need current state? → get_snapshot (this tool!)
      - Need historical state? → reconstruct_state
      - Need full context about entity? → explain_entity (includes snapshot)
      - Need to compare states over time? → analyze_changes
      """,
      inputSchema: %{
        type: "object",
        properties: %{
          "entity_id" => %{type: "string", description: "The entity ID to get snapshot for"}
        },
        required: ["entity_id"]
      }
    }
  end

  defp tool_analyze_changes do
    %{
      name: "analyze_changes",
      description: """
      Analyze what changed for an entity between two points in time. Returns a \
      detailed diff showing added, modified, and removed fields.

      **When to use this tool:**
      - Answering "What changed between [date A] and [date B]?"
      - Investigating what happened during an incident window
      - Auditing changes made in a specific time period
      - Understanding the evolution of an entity's state

      **Common patterns:**
      - Changes today: `from_time: "[start of today]"` (to_time defaults to now)
      - Incident window: `from_time: "[incident_start]", to_time: "[incident_end]"`
      - Before/after deployment: `from_time: "[pre-deploy]", to_time: "[post-deploy]"`
      - Monthly audit: `from_time: "[month start]", to_time: "[month end]"`

      **Performance tips:**
      - Internally calls reconstruct_state twice (at from_time and to_time)
      - Cost scales with event volume in the entity's history
      - For very active entities, narrow the time window if possible
      - Consider event_timeline first to identify interesting time ranges

      **Decision guide:**
      - Need to know what changed? → analyze_changes (this tool!)
      - Need to see the events that caused changes? → event_timeline
      - Need state at one point? → reconstruct_state
      - Need to compare multiple entities? → compare_entities
      """,
      inputSchema: %{
        type: "object",
        properties: %{
          "entity_id" => %{type: "string", description: "The entity to analyze"},
          "from_time" => %{type: "string", description: "Start timestamp (ISO format)"},
          "to_time" => %{
            type: "string",
            description: "End timestamp (ISO format, defaults to now)"
          }
        },
        required: ["entity_id", "from_time"]
      }
    }
  end

  defp tool_find_patterns do
    %{
      name: "find_patterns",
      description: """
      Detect patterns in event streams: frequency analysis, event sequences, or anomalies. \
      Essential for understanding behavior trends and detecting unusual activity.

      **When to use this tool:**
      - Answering "What patterns exist in the data?"
      - Detecting anomalies or unusual behavior
      - Understanding event frequency distributions
      - Discovering common event sequences (workflow analysis)

      **Common patterns:**
      - Frequency analysis: `pattern_type: "frequency"` → shows event type distribution
      - Sequence discovery: `pattern_type: "sequence"` → shows common event orderings
      - Anomaly detection: `pattern_type: "anomaly"` → flags unusual events
      - Scoped analysis: Add `entity_id` or `event_type` to focus on specific data

      **Pattern types explained:**
      - `frequency`: Counts events by type, sorted by occurrence. Good for understanding workload.
      - `sequence`: Finds common event progressions (A→B→C). Good for workflow analysis.
      - `anomaly`: Identifies outliers based on timing, frequency, or content deviations.

      **Performance tips:**
      - Use `since` to limit analysis window (patterns are computed on fetched events)
      - Start with frequency analysis to understand the data shape
      - Combine with `entity_id` or `event_type` to reduce data volume
      - Anomaly detection works best with sufficient historical data

      **Decision guide:**
      - Want event counts/distribution? → find_patterns with frequency
      - Want to understand workflows? → find_patterns with sequence
      - Looking for unusual activity? → find_patterns with anomaly
      - Need specific events? → query_events or semantic_search_events
      """,
      inputSchema: %{
        type: "object",
        properties: %{
          "entity_id" => %{
            type: "string",
            description: "Analyze patterns for specific entity (optional)"
          },
          "event_type" => %{
            type: "string",
            description: "Analyze patterns for specific event type (optional)"
          },
          "since" => %{
            type: "string",
            description: "Analyze patterns since this timestamp (optional)"
          },
          "pattern_type" => %{
            type: "string",
            enum: ["frequency", "sequence", "anomaly"],
            description:
              "Type of pattern to detect (frequency=event counts, sequence=event order, anomaly=unusual events)"
          }
        }
      }
    }
  end

  defp tool_compare_entities do
    %{
      name: "compare_entities",
      description: """
      Compare multiple entities to find similarities and differences in their \
      event histories. Useful for understanding behavioral differences between entities.

      **When to use this tool:**
      - Comparing user behaviors (e.g., "How do these two users differ?")
      - Analyzing cohort differences (e.g., "Premium vs free users")
      - Finding outliers by comparing against normal entities
      - Understanding why similar entities have different outcomes

      **Common patterns:**
      - Simple comparison: `entity_ids: ["user-123", "user-456"]`
      - Time-bounded: Add `timeframe: "2024-01-01T00:00:00Z"` to compare recent activity
      - Cohort analysis: Compare multiple entities (up to reasonable limits)
      - A/B analysis: Compare entities from different groups

      **What the comparison shows:**
      - Event count per entity
      - Event types used by each entity
      - Unique vs shared event types across entities
      - Activity timeline overlap

      **Performance tips:**
      - Queries all events for each entity - use timeframe to limit scope
      - Compare 2-5 entities at a time for meaningful results
      - Start with timeframe filter on active entities
      - For large-scale comparisons, use find_patterns with cohort filtering

      **Decision guide:**
      - Comparing specific entities? → compare_entities (this tool!)
      - Analyzing one entity deeply? → explain_entity
      - Looking at aggregate patterns? → find_patterns
      - Need state differences over time? → analyze_changes (single entity)
      """,
      inputSchema: %{
        type: "object",
        properties: %{
          "entity_ids" => %{
            type: "array",
            items: %{type: "string"},
            description: "Array of entity IDs to compare"
          },
          "timeframe" => %{
            type: "string",
            description: "Compare within this timeframe (ISO timestamp)"
          }
        },
        required: ["entity_ids"]
      }
    }
  end

  defp tool_event_timeline do
    %{
      name: "event_timeline",
      description: """
      Get a chronological timeline of all events for an entity, formatted for easy \
      reading. Shows the complete history in a human-friendly narrative format.

      **When to use this tool:**
      - Answering "What happened to this entity?" or "Show me the history"
      - Understanding the sequence of events leading to current state
      - Debugging: tracing through events chronologically
      - Creating audit trails or activity logs for review

      **Common patterns:**
      - Full history: `entity_id: "user-123"` (no time filters)
      - Recent activity: `entity_id: "user-123", since: "[recent date]"`
      - Specific window: `since: "[start]", until: "[end]"` for incident analysis
      - Last N events: Use since with recent timestamp

      **Output format:**
      - Numbered steps showing chronological progression
      - Each step includes: timestamp, event type, and payload preview
      - Easy to read narrative format for understanding flow

      **Performance tips:**
      - Use since/until to limit time range on active entities
      - For entities with many events, start with recent window
      - Consider query_events with limit if you only need latest few events
      - Timeline is better for understanding flow; query_events for raw data

      **Decision guide:**
      - Want to understand what happened? → event_timeline (this tool!)
      - Need raw event data for processing? → query_events
      - Need current state only? → get_snapshot
      - Need complete entity overview? → explain_entity (includes timeline)
      """,
      inputSchema: %{
        type: "object",
        properties: %{
          "entity_id" => %{type: "string", description: "Entity to get timeline for"},
          "since" => %{type: "string", description: "Timeline start time (optional)"},
          "until" => %{type: "string", description: "Timeline end time (optional)"}
        },
        required: ["entity_id"]
      }
    }
  end

  defp tool_explain_entity do
    %{
      name: "explain_entity",
      description: """
      Get a comprehensive explanation of an entity: current state, event history, \
      key changes, and timeline summary. The "tell me everything" tool.

      **When to use this tool:**
      - Answering "Tell me about entity X" or "What is this entity?"
      - Starting an investigation with no prior context about the entity
      - Getting a complete picture before diving into specifics
      - Onboarding: understanding an entity you're unfamiliar with

      **What this tool returns:**
      - Current state (snapshot)
      - Total event count and event type breakdown
      - Creation timestamp and last update time
      - Complete lifecycle timeline showing all state transitions

      **Common patterns:**
      - First contact: `entity_id: "user-123"` to understand a new entity
      - Investigation start: Get explain_entity, then drill into specifics
      - Context gathering: Use before answering detailed questions

      **Performance tips:**
      - This is a comprehensive tool - it fetches state + all events
      - More expensive than single-purpose tools like get_snapshot
      - Use for initial exploration, then switch to targeted tools
      - For very active entities, consider event_timeline with since filter

      **Decision guide:**
      - Need everything about an entity? → explain_entity (this tool!)
      - Only need current state? → get_snapshot (faster)
      - Only need history? → event_timeline
      - Only need specific events? → query_events
      - Comparing entities? → compare_entities
      """,
      inputSchema: %{
        type: "object",
        properties: %{
          "entity_id" => %{type: "string", description: "Entity ID to explain"}
        },
        required: ["entity_id"]
      }
    }
  end

  defp tool_ingest_event do
    %{
      name: "ingest_event",
      description: """
      Ingest a new event into the AllSource event store. This is the only write \
      operation - all other tools are read-only.

      **When to use this tool:**
      - Recording new facts/events as they occur
      - Creating audit trail entries
      - Storing state changes from external systems
      - Manual event injection for testing or data correction

      **Common patterns:**
      - User action: `event_type: "user.updated", entity_id: "user-123", payload: {...}`
      - System event: `event_type: "system.health_check", entity_id: "node-1", payload: {...}`
      - Audit entry: Include `metadata: {source: "manual", reason: "..."}` for context

      **Event type naming conventions:**
      - Use dot notation: `domain.action` (e.g., "user.created", "order.shipped")
      - Past tense for completed actions: "created", "updated", "deleted"
      - Be consistent across your domain model

      **Payload best practices:**
      - Include all relevant data - events are immutable once stored
      - Use consistent field names across related event types
      - Consider what queries you'll need later

      **Performance tips:**
      - Events are indexed automatically for fast retrieval
      - Vector embeddings are generated asynchronously for semantic search
      - Snapshots are updated incrementally after ingestion
      - High-volume ingestion is optimized but be mindful of burst rates

      **Important notes:**
      - Events are IMMUTABLE once stored - plan your data model carefully
      - This is the only tool that modifies data - all others are read-only
      - Include metadata for audit trails and debugging context
      """,
      inputSchema: %{
        type: "object",
        properties: %{
          "event_type" => %{type: "string", description: "Type of event (e.g., \"user.created\")"},
          "entity_id" => %{type: "string", description: "ID of the entity this event relates to"},
          "payload" => %{type: "object", description: "Event payload as JSON object"},
          "metadata" => %{type: "object", description: "Optional metadata"}
        },
        required: ["event_type", "entity_id", "payload"]
      }
    }
  end

  defp tool_get_stats do
    %{
      name: "get_stats",
      description: """
      Get comprehensive statistics about the AllSource event store. Provides \
      data-level metrics about stored events, entities, and usage patterns.

      **When to use this tool:**
      - Understanding the size and scope of the event store
      - Capacity planning and monitoring growth
      - Answering "How much data is in the system?"
      - Getting an overview before diving into specific queries

      **What this tool returns:**
      - Total event count and storage size
      - Entity count and distribution
      - Event type breakdown and frequencies
      - Timestamp ranges (oldest/newest events)
      - Index statistics and performance metrics

      **Common patterns:**
      - System overview: Call get_stats with no parameters
      - Capacity check: Review storage size and growth trends
      - Data exploration: See event types before querying

      **Performance tips:**
      - This is a lightweight operation using pre-computed statistics
      - Safe to call frequently for monitoring dashboards
      - Use this before expensive queries to understand data volume

      **Decision guide:**
      - Need data statistics? → get_stats (this tool!)
      - Need infrastructure health? → get_cluster_status
      - Need to query actual events? → query_events or search tools
      - Need patterns in data? → find_patterns
      """,
      inputSchema: %{
        type: "object",
        properties: %{}
      }
    }
  end

  defp tool_get_cluster_status do
    %{
      name: "get_cluster_status",
      description: """
      Get current cluster health and status information. Provides infrastructure-level \
      metrics about the distributed AllSource deployment.

      **When to use this tool:**
      - Checking system health before operations
      - Troubleshooting slow queries or timeouts
      - Monitoring node availability and replication status
      - Answering "Is the system healthy?" or "Why is it slow?"

      **What this tool returns:**
      - Node status (healthy/unhealthy/degraded)
      - Replication lag and sync status
      - Resource utilization (CPU, memory, disk)
      - Connection pool status
      - Recent errors or warnings

      **Common patterns:**
      - Health check: Call before important operations
      - Incident response: First step in troubleshooting
      - Monitoring: Periodic checks for dashboards

      **Performance tips:**
      - Lightweight operation - safe to call frequently
      - Use before expensive queries to ensure system is healthy
      - Check cluster status when queries are unexpectedly slow

      **Decision guide:**
      - Need infrastructure health? → get_cluster_status (this tool!)
      - Need data statistics? → get_stats
      - Troubleshooting query performance? → Check cluster_status first, then get_stats
      - System seems slow? → get_cluster_status to identify bottlenecks
      """,
      inputSchema: %{
        type: "object",
        properties: %{}
      }
    }
  end

  defp tool_semantic_search_events do
    %{
      name: "semantic_search_events",
      description: """
      Search events using natural language semantic similarity. Uses vector embeddings \
      to find events that are conceptually related to your query, even if they don't \
      contain the exact words.

      **When to use this tool:**
      - Finding events related to a concept (e.g., "user authentication failures")
      - Discovering similar events across different naming conventions
      - Exploring events when you don't know exact field names or values
      - Answering questions like "What events relate to payment processing?"

      **Tips for effective queries:**
      - Use natural language descriptions of what you're looking for
      - Be specific about the domain or context (e.g., "user onboarding" vs just "user")
      - Combine with query_events for exact matches when you know the event type

      Returns events ranked by semantic similarity score (0.0-1.0).
      """,
      inputSchema: %{
        type: "object",
        properties: %{
          "query" => %{
            type: "string",
            description:
              "Natural language search query describing what you're looking for (e.g., \"user login attempts\" or \"failed payment transactions\")"
          },
          "limit" => %{
            type: "number",
            description: "Maximum number of results to return (default: 100, max: 1000)"
          },
          "threshold" => %{
            type: "number",
            description:
              "Minimum similarity threshold from 0.0 to 1.0 (default: 0.7). Lower values return more results but with less relevance. Use 0.5 for broad exploration, 0.8+ for precise matches."
          },
          "format" => %{
            type: "string",
            enum: ["toon", "json"],
            description: "Response format: 'toon' (default, ~50% fewer tokens) or 'json'"
          }
        },
        required: ["query"]
      }
    }
  end

  defp tool_hybrid_search do
    %{
      name: "hybrid_search",
      description: """
      Perform advanced search combining semantic understanding with keyword matching \
      and metadata filters. This is the most powerful search tool, using Reciprocal \
      Rank Fusion (RRF) to blend results from multiple search strategies.

      **When to use this tool:**
      - Complex queries requiring both concept matching AND specific keywords
      - Filtering by event type, entity, or time range while searching content
      - Finding events when you know some specifics but want semantic expansion
      - Questions like "Show me user-related events from last week about authentication"

      **Search strategies:**
      - Provide `semantic_query` for concept/meaning-based search
      - Provide `keywords` for exact term matching (BM25)
      - Provide both for hybrid search (recommended for complex queries)
      - Use `filters` to narrow results by event_type, entity_id, or time range

      **Tips:**
      - Combine semantic_query with filters for best results
      - Use keywords when you know exact terms that must appear
      - Time ranges use ISO 8601 format (e.g., "2024-01-15T00:00:00Z")
      - Results are ranked by combined RRF score from all active search strategies

      Returns events with combined relevance scores and individual strategy scores.
      """,
      inputSchema: %{
        type: "object",
        properties: %{
          "semantic_query" => %{
            type: "string",
            description:
              "Natural language query for semantic (meaning-based) search. Finds conceptually similar events even without exact keyword matches."
          },
          "keywords" => %{
            type: "string",
            description:
              "Keywords for BM25 text search. Supports boolean operators (AND, OR), phrases (\"exact phrase\"), and field-specific search (field:value)."
          },
          "filters" => %{
            type: "object",
            description: "Optional filters to narrow search results",
            properties: %{
              "event_type" => %{
                type: "string",
                description: "Filter by exact event type (e.g., \"user.created\", \"order.placed\")"
              },
              "entity_id" => %{
                type: "string",
                description: "Filter by exact entity ID (e.g., \"user-123\", \"order-456\")"
              },
              "time_from" => %{
                type: "string",
                description:
                  "Include events on or after this ISO timestamp (e.g., \"2024-01-15T00:00:00Z\")"
              },
              "time_to" => %{
                type: "string",
                description:
                  "Include events on or before this ISO timestamp (e.g., \"2024-01-31T23:59:59Z\")"
              }
            }
          },
          "limit" => %{
            type: "number",
            description: "Maximum number of results to return (default: 100, max: 1000)"
          },
          "format" => %{
            type: "string",
            enum: ["toon", "json"],
            description: "Response format: 'toon' (default, ~50% fewer tokens) or 'json'"
          }
        }
      }
    }
  end

  defp tool_get_query_advice do
    %{
      name: "get_query_advice",
      description: """
      Get expert recommendations for querying AllSource based on your specific use case. \
      Returns recommended tool combinations, query patterns, performance tips, and \
      common pitfalls to avoid.

      **When to use this tool:**
      - Starting a new investigation and unsure which tools to use
      - Optimizing query performance for a specific use case
      - Learning best practices for common scenarios
      - Before building complex query workflows

      **Common patterns:**
      - Audit investigation: `use_case: "audit_trail"`
      - User behavior analysis: `use_case: "user_analytics"`
      - Debugging issues: `use_case: "debugging"`
      - Regulatory compliance: `use_case: "compliance"`
      - System optimization: `use_case: "performance_analysis"`
      - Add `context` for domain-specific advice: `context: "e-commerce checkout flow"`

      **What this tool returns:**
      - Recommended tools and their optimal order
      - Specific query patterns with example parameters
      - Performance optimization strategies
      - Common mistakes to avoid for your use case

      **Performance tips:**
      - This is a stateless advisory tool with no backend calls
      - Use before complex investigations to plan your approach
      - Re-query with different use cases to compare strategies

      **Decision guide:**
      - Not sure where to start? → get_query_advice (this tool!)
      - Know what you need? → Use the recommended tools directly
      - Need system overview first? → get_stats or get_cluster_status
      """,
      inputSchema: %{
        type: "object",
        properties: %{
          "use_case" => %{
            type: "string",
            enum: [
              "audit_trail",
              "user_analytics",
              "debugging",
              "compliance",
              "performance_analysis"
            ],
            description:
              "The use case you're trying to solve: audit_trail (who did what when), user_analytics (behavior patterns), debugging (incident investigation), compliance (regulatory evidence), performance_analysis (system optimization)"
          },
          "context" => %{
            type: "string",
            description:
              "Optional additional context about your specific scenario (e.g., 'e-commerce checkout', 'user authentication', 'payment processing')"
          }
        },
        required: ["use_case"]
      }
    }
  end

  # ============================================================================
  # Quick Exploration Tool Definitions
  # ============================================================================

  defp tool_sample_events do
    %{
      name: "sample_events",
      description: """
      Get a representative sample of events for rapid data exploration without \
      scanning all events. Optimized for sub-second response times.

      **When to use this tool:**
      - Starting exploration of an unfamiliar dataset
      - Getting a quick feel for event structure and content
      - Validating data quality on a sample before deeper analysis
      - Understanding event distribution across types, entities, or time

      **Stratification options:**
      - `event_type`: Sample evenly across different event types
      - `entity_id`: Sample evenly across different entities
      - `time`: Sample evenly across the time range (temporal stratification)

      **Common patterns:**
      - Quick overview: `sample_size: 100` (default stratification)
      - By event type: `sample_size: 50, stratified_by: "event_type"`
      - By entity: `sample_size: 100, stratified_by: "entity_id"`
      - Temporal sample: `sample_size: 200, stratified_by: "time"`

      **Performance tips:**
      - This is a FAST tool - use it before expensive full scans
      - Default sample_size of 1000 balances coverage and speed
      - Stratification adds minimal overhead but improves representativeness
      - Use with entity_id/event_type filters to sample within a subset

      **Decision guide:**
      - Want quick data overview? → sample_events (this tool!)
      - Need all events? → query_events (slower but complete)
      - Need statistics only? → quick_stats (even faster)
      - Need specific search? → semantic_search_events or hybrid_search
      """,
      inputSchema: %{
        type: "object",
        properties: %{
          "sample_size" => %{
            type: "number",
            description: "Number of events to sample (default: 1000, max: 10000)"
          },
          "stratified_by" => %{
            type: "string",
            enum: ["event_type", "entity_id", "time"],
            description:
              "Stratification strategy: event_type (sample across types), entity_id (sample across entities), time (temporal sampling)"
          },
          "entity_id" => %{
            type: "string",
            description: "Optional: limit sampling to this entity"
          },
          "event_type" => %{
            type: "string",
            description: "Optional: limit sampling to this event type"
          },
          "since" => %{
            type: "string",
            description: "Optional: sample from events since this timestamp"
          },
          "until" => %{
            type: "string",
            description: "Optional: sample from events until this timestamp"
          },
          "format" => %{
            type: "string",
            enum: ["toon", "json"],
            description: "Response format: 'toon' (default, ~50% fewer tokens) or 'json'"
          }
        }
      }
    }
  end

  defp tool_quick_stats do
    %{
      name: "quick_stats",
      description: """
      Get fast approximate statistics about the event store. Trades precision \
      for speed - ideal for rapid exploration and orientation.

      **When to use this tool:**
      - Quick orientation before deeper analysis
      - Checking data volume before running expensive queries
      - Getting approximate counts without scanning all data
      - Rapid health checks on data distribution

      **Available metrics:**
      - `event_count`: Total number of events (approximate)
      - `unique_entities`: Count of distinct entity IDs
      - `event_types`: List of event types with approximate counts
      - `time_range`: Oldest and newest event timestamps

      **Common patterns:**
      - Full overview: `metric: "all"` or omit parameter
      - Just count: `metric: "event_count"`
      - Data shape: `metric: "event_types"`
      - Time bounds: `metric: "time_range"`

      **Performance tips:**
      - This is the FASTEST stats tool - uses pre-computed approximations
      - Use before get_stats for quick orientation
      - Counts may be approximate (within ~5% accuracy)
      - Ideal for answering "roughly how much data?" questions

      **Accuracy note:**
      - Counts are approximate using probabilistic data structures
      - Use get_stats for exact counts (slower)
      - Approximations are refreshed periodically, not real-time

      **Decision guide:**
      - Need quick approximation? → quick_stats (this tool!)
      - Need exact statistics? → get_stats (slower but precise)
      - Need actual events? → sample_events or query_events
      - Need system health? → get_cluster_status
      """,
      inputSchema: %{
        type: "object",
        properties: %{
          "metric" => %{
            type: "string",
            enum: ["event_count", "unique_entities", "event_types", "time_range", "all"],
            description:
              "Which metric to retrieve: event_count, unique_entities, event_types, time_range, or all (default)"
          },
          "entity_id" => %{
            type: "string",
            description: "Optional: get stats for specific entity only"
          },
          "event_type" => %{
            type: "string",
            description: "Optional: get stats for specific event type only"
          }
        }
      }
    }
  end

  # ============================================================================
  # Conversation Context Tool Definitions
  # ============================================================================

  defp tool_start_session do
    %{
      name: "start_session",
      description: """
      Start a new conversation session for multi-turn query building. Optionally \
      initialize with context from an initial query.

      **When to use this tool:**
      - Beginning a new investigation or analysis workflow
      - When you want to iteratively refine queries across multiple turns
      - Before a series of related queries that build on each other

      **Common patterns:**
      - Start fresh: `session_id: "investigation-123"`
      - Start with entity: `session_id: "user-analysis", entity_id: "user-456"`
      - Start with time range: `session_id: "audit-jan", since: "2024-01-01", until: "2024-01-31"`

      **How multi-turn context works:**
      1. Start session with initial filters
      2. Use refine_query to add filters or change scope
      3. Each refinement builds on previous context
      4. Use get_session_context to see accumulated state

      **Example workflow:**
      ```
      Turn 1: start_session(session_id: "s1", entity_type: "user", since: "yesterday")
      Turn 2: refine_query(session_id: "s1", filters: {tier: "premium"})
      Turn 3: refine_query(session_id: "s1", event_type: "purchase.completed")
      ```

      **Performance tips:**
      - Sessions auto-expire after 30 minutes of inactivity
      - Use descriptive session_ids for easier tracking
      - Clear old sessions if you're starting fresh on a topic

      **Decision guide:**
      - Starting fresh investigation? → start_session (this tool!)
      - Already have a session? → refine_query
      - Need to see current context? → get_session_context
      """,
      inputSchema: %{
        type: "object",
        properties: %{
          "session_id" => %{
            type: "string",
            description:
              "Unique identifier for this conversation session (e.g., 'user-investigation-123')"
          },
          "entity_id" => %{
            type: "string",
            description: "Optional initial entity ID to focus on"
          },
          "entity_type" => %{
            type: "string",
            description: "Optional entity type (e.g., 'user', 'order', 'payment')"
          },
          "event_type" => %{
            type: "string",
            description: "Optional initial event type filter"
          },
          "since" => %{
            type: "string",
            description: "Optional start time filter (ISO timestamp)"
          },
          "until" => %{
            type: "string",
            description: "Optional end time filter (ISO timestamp)"
          },
          "semantic_query" => %{
            type: "string",
            description: "Optional initial semantic search context"
          }
        },
        required: ["session_id"]
      }
    }
  end

  defp tool_refine_query do
    %{
      name: "refine_query",
      description: """
      Refine the query context for an existing session. New filters and parameters \
      merge with existing context, enabling iterative query building.

      **When to use this tool:**
      - Adding filters to narrow down results: "Filter to premium tier"
      - Changing time range: "Look at last week instead"
      - Adding entity focus: "Focus on user-123"
      - Building comparison queries: "Compare with last month"

      **Common patterns:**
      - Add filter: `refine_query(session_id: "s1", filters: {status: "active"})`
      - Narrow time: `refine_query(session_id: "s1", since: "2024-01-15")`
      - Add entity: `refine_query(session_id: "s1", entity_id: "user-789")`
      - Add event type: `refine_query(session_id: "s1", event_type: "user.updated")`

      **Merge behavior:**
      - `entity_id` / `entity_ids`: Accumulates (adds to existing list)
      - `event_type` / `event_types`: Accumulates
      - `filters`: Merges (new keys added, existing keys updated)
      - `since` / `until` / `as_of`: Replaces (newest wins)
      - `semantic_query` / `keywords`: Replaces

      **To replace instead of accumulate:**
      Use `replace_entity_ids: true` to clear existing entities

      **Performance tips:**
      - Each refinement updates last_accessed, extending session life
      - Query history is preserved (last 10 refinements)
      - Use get_session_context to verify current state before querying

      **Decision guide:**
      - Narrowing down results? → refine_query (this tool!)
      - Starting fresh? → start_session
      - Ready to execute? → Use query_events, semantic_search, etc. with session context
      """,
      inputSchema: %{
        type: "object",
        properties: %{
          "session_id" => %{
            type: "string",
            description: "The session ID to refine"
          },
          "entity_id" => %{
            type: "string",
            description: "Add an entity ID to focus on (accumulates with existing)"
          },
          "entity_ids" => %{
            type: "array",
            items: %{type: "string"},
            description: "Add multiple entity IDs (accumulates with existing)"
          },
          "event_type" => %{
            type: "string",
            description: "Add an event type filter (accumulates with existing)"
          },
          "event_types" => %{
            type: "array",
            items: %{type: "string"},
            description: "Add multiple event types (accumulates with existing)"
          },
          "filters" => %{
            type: "object",
            description: "Additional metadata filters to apply (merges with existing)"
          },
          "since" => %{
            type: "string",
            description: "Update start time filter (replaces existing)"
          },
          "until" => %{
            type: "string",
            description: "Update end time filter (replaces existing)"
          },
          "as_of" => %{
            type: "string",
            description: "Update as_of time for time-travel queries (replaces existing)"
          },
          "semantic_query" => %{
            type: "string",
            description: "Update semantic search query (replaces existing)"
          },
          "keywords" => %{
            type: "string",
            description: "Update keyword search (replaces existing)"
          },
          "replace_entity_ids" => %{
            type: "boolean",
            description: "If true, replace existing entity_ids instead of accumulating"
          }
        },
        required: ["session_id"]
      }
    }
  end

  defp tool_get_session_context do
    %{
      name: "get_session_context",
      description: """
      Get the current accumulated context for a conversation session. Shows all \
      filters, entities, and query parameters that have been built up.

      **When to use this tool:**
      - Before executing a query, to verify the accumulated context
      - To understand what filters are currently active
      - To debug unexpected query results
      - To show the user what context you're working with

      **What this tool returns:**
      - Current query parameters (ready to pass to other tools)
      - Session metadata (created, last accessed, query count)
      - Query history (last 10 refinements)
      - Last result summary (if available)

      **Common patterns:**
      - Check context: `get_session_context(session_id: "s1")`
      - Verify before query: Check context, then call query_events with returned params

      **Performance tips:**
      - Lightweight operation, safe to call frequently
      - Use to verify context before expensive queries
      - Helps catch accumulated filters you might have forgotten

      **Decision guide:**
      - Need to see current state? → get_session_context (this tool!)
      - Need to modify context? → refine_query
      - Ready to execute with context? → Use returned params with query tools
      """,
      inputSchema: %{
        type: "object",
        properties: %{
          "session_id" => %{
            type: "string",
            description: "The session ID to retrieve context for"
          },
          "include_history" => %{
            type: "boolean",
            description: "Include query refinement history (default: true)"
          }
        },
        required: ["session_id"]
      }
    }
  end

  # ============================================================================
  # Tool Handlers
  # ============================================================================

  @doc false
  def handle_query_events(args, state, format) do
    params = Map.take(args, ["entity_id", "event_type", "as_of", "since", "until", "limit"])

    case CoreClient.query_events(state.core_client, params) do
      {:ok, data} ->
        count = Map.get(data, "count", 0)
        summary = "📊 Found #{count} events\n"
        formatted_data = ToonEncoder.format_response(data, format)
        text = summary <> formatted_data

        {:ok,
         %{
           content: [
             %{
               type: "text",
               text: text
             }
           ]
         }}

      {:error, reason} ->
        {:error, "Failed to query events: #{inspect(reason)}"}
    end
  end

  @doc false
  def handle_reconstruct_state(args, state, format) do
    entity_id = Map.fetch!(args, "entity_id")
    as_of = Map.get(args, "as_of")

    case CoreClient.reconstruct_state(state.core_client, entity_id, as_of) do
      {:ok, state_data} ->
        event_count = Map.get(state_data, "event_count", 0)
        last_updated = Map.get(state_data, "last_updated", "unknown")
        as_of_str = as_of || "current"

        summary = """
        🔄 Reconstructed state for "#{entity_id}"
        📅 As of: #{as_of_str}
        📊 Events processed: #{event_count}
        ⏰ Last updated: #{last_updated}

        """

        formatted_data = ToonEncoder.format_response(state_data, format)
        text = summary <> formatted_data

        {:ok,
         %{
           content: [
             %{
               type: "text",
               text: text
             }
           ]
         }}

      {:error, reason} ->
        {:error, "Failed to reconstruct state: #{inspect(reason)}"}
    end
  end

  @doc false
  def handle_get_snapshot(args, state, format) do
    entity_id = Map.fetch!(args, "entity_id")

    case CoreClient.get_snapshot(state.core_client, entity_id) do
      {:ok, snapshot} ->
        summary = "⚡ Fast snapshot for \"#{entity_id}\"\n\n"
        formatted_data = ToonEncoder.format_response(snapshot, format)
        text = summary <> formatted_data

        {:ok,
         %{
           content: [
             %{
               type: "text",
               text: text
             }
           ]
         }}

      {:error, reason} ->
        {:error, "Failed to get snapshot: #{inspect(reason)}"}
    end
  end

  @doc false
  def handle_analyze_changes(args, state, format) do
    entity_id = Map.fetch!(args, "entity_id")
    from_time = Map.fetch!(args, "from_time")
    to_time = Map.get(args, "to_time")

    # Get state at from_time
    case CoreClient.reconstruct_state(state.core_client, entity_id, from_time) do
      {:ok, before_state} ->
        # Get state at to_time (or current)
        case CoreClient.reconstruct_state(state.core_client, entity_id, to_time) do
          {:ok, after_state} ->
            before_state_map = Map.get(before_state, "current_state", %{})
            after_state_map = Map.get(after_state, "current_state", %{})

            changes = calculate_diff(before_state_map, after_state_map)

            summary = """
            🔍 Change Analysis for "#{entity_id}"
            📅 From: #{from_time}
            📅 To: #{to_time || "now"}
            ➕ Added fields: #{length(changes.added)}
            ✏️  Modified fields: #{length(changes.modified)}
            ➖ Removed fields: #{length(changes.removed)}

            """

            formatted_data = ToonEncoder.format_response(changes, format)
            text = summary <> formatted_data

            {:ok,
             %{
               content: [
                 %{
                   type: "text",
                   text: text
                 }
               ]
             }}

          {:error, reason} ->
            {:error, "Failed to get after state: #{inspect(reason)}"}
        end

      {:error, reason} ->
        {:error, "Failed to get before state: #{inspect(reason)}"}
    end
  end

  @doc false
  def handle_find_patterns(args, state, format) do
    params = Map.take(args, ["entity_id", "event_type", "since"])

    case CoreClient.query_events(state.core_client, params) do
      {:ok, data} ->
        events = Map.get(data, "events", [])
        pattern_type = Map.get(args, "pattern_type")

        analysis = analyze_patterns(events, pattern_type)

        summary = """
        🔎 Pattern Analysis
        📊 Events analyzed: #{length(events)}
        🎯 Pattern type: #{pattern_type || "all"}

        """

        formatted_data = ToonEncoder.format_response(analysis, format)
        text = summary <> formatted_data

        {:ok,
         %{
           content: [
             %{
               type: "text",
               text: text
             }
           ]
         }}

      {:error, reason} ->
        {:error, "Failed to find patterns: #{inspect(reason)}"}
    end
  end

  @doc false
  def handle_compare_entities(args, state, format) do
    entity_ids = Map.fetch!(args, "entity_ids")
    timeframe = Map.get(args, "timeframe")

    comparisons =
      Enum.map(entity_ids, fn id ->
        params = if timeframe, do: Map.put(%{}, "since", timeframe), else: %{}
        params = Map.put(params, "entity_id", id)

        case CoreClient.query_events(state.core_client, params) do
          {:ok, data} ->
            events = Map.get(data, "events", [])
            event_types = events |> Enum.map(&Map.get(&1, "event_type")) |> Enum.uniq()

            %{
              entity_id: id,
              event_count: Map.get(data, "count", 0),
              event_types: event_types
            }

          {:error, _reason} ->
            %{
              entity_id: id,
              event_count: 0,
              event_types: []
            }
        end
      end)

    summary = """
    🔬 Entity Comparison
    📊 Entities compared: #{length(entity_ids)}
    ⏰ Timeframe: #{timeframe || "all time"}

    """

    formatted_data = ToonEncoder.format_response(comparisons, format)
    text = summary <> formatted_data

    {:ok,
     %{
       content: [
         %{
           type: "text",
           text: text
         }
       ]
     }}
  end

  @doc false
  def handle_event_timeline(args, state, format) do
    entity_id = Map.fetch!(args, "entity_id")
    params = Map.take(args, ["since", "until"])
    params = Map.put(params, "entity_id", entity_id)

    case CoreClient.query_events(state.core_client, params) do
      {:ok, data} ->
        events = Map.get(data, "events", [])

        timeline =
          events
          |> Enum.with_index(1)
          |> Enum.map(fn {event, index} ->
            payload_preview =
              event
              |> Map.get("payload", %{})
              |> Jason.encode!()
              |> String.slice(0, 100)

            %{
              step: index,
              timestamp: Map.get(event, "timestamp"),
              event_type: Map.get(event, "event_type"),
              summary: "#{Map.get(event, "event_type")} - #{payload_preview}..."
            }
          end)

        summary = """
        📅 Timeline for "#{entity_id}"
        📊 Events: #{length(events)}
        ⏰ Period: #{Map.get(args, "since", "start")} to #{Map.get(args, "until", "now")}

        """

        formatted_data = ToonEncoder.format_response(timeline, format)
        text = summary <> formatted_data

        {:ok,
         %{
           content: [
             %{
               type: "text",
               text: text
             }
           ]
         }}

      {:error, reason} ->
        {:error, "Failed to get timeline: #{inspect(reason)}"}
    end
  end

  @doc false
  def handle_explain_entity(args, state, format) do
    entity_id = Map.fetch!(args, "entity_id")

    # Get current state
    case CoreClient.reconstruct_state(state.core_client, entity_id, nil) do
      {:ok, state_data} ->
        # Get all events
        case CoreClient.query_events(state.core_client, %{"entity_id" => entity_id}) do
          {:ok, events_data} ->
            events = Map.get(events_data, "events", [])
            event_types = events |> Enum.map(&Map.get(&1, "event_type")) |> Enum.uniq()

            explanation = %{
              entity_id: entity_id,
              current_state: Map.get(state_data, "current_state", %{}),
              total_events: length(events),
              event_types: event_types,
              created_at: List.first(events) |> Map.get("timestamp"),
              last_updated: Map.get(state_data, "last_updated"),
              lifecycle:
                Enum.map(events, fn e ->
                  %{
                    when: Map.get(e, "timestamp"),
                    what: Map.get(e, "event_type")
                  }
                end)
            }

            summary = """
            📋 Entity Explanation: "#{entity_id}"

            🔹 Total Events: #{length(events)}
            🔹 Event Types: #{length(event_types)}
            🔹 Created: #{List.first(events) |> Map.get("timestamp", "unknown")}
            🔹 Last Updated: #{Map.get(state_data, "last_updated", "unknown")}

            """

            formatted_data = ToonEncoder.format_response(explanation, format)
            text = summary <> formatted_data

            {:ok,
             %{
               content: [
                 %{
                   type: "text",
                   text: text
                 }
               ]
             }}

          {:error, reason} ->
            {:error, "Failed to get events: #{inspect(reason)}"}
        end

      {:error, reason} ->
        {:error, "Failed to get state: #{inspect(reason)}"}
    end
  end

  @doc false
  def handle_ingest_event(args, state, format) do
    event_data = %{
      "event_type" => Map.fetch!(args, "event_type"),
      "entity_id" => Map.fetch!(args, "entity_id"),
      "payload" => Map.fetch!(args, "payload"),
      "metadata" => Map.get(args, "metadata")
    }

    case CoreClient.ingest_event(state.core_client, event_data) do
      {:ok, result} ->
        event_id = Map.get(result, "event_id", "unknown")
        timestamp = Map.get(result, "timestamp", "unknown")

        summary = """
        ✅ Event ingested successfully
        🆔 Event ID: #{event_id}
        ⏰ Timestamp: #{timestamp}

        """

        formatted_data = ToonEncoder.format_response(result, format)
        text = summary <> formatted_data

        {:ok,
         %{
           content: [
             %{
               type: "text",
               text: text
             }
           ]
         }}

      {:error, reason} ->
        {:error, "Failed to ingest event: #{inspect(reason)}"}
    end
  end

  @doc false
  def handle_get_stats(state, format) do
    case CoreClient.get_stats(state.core_client) do
      {:ok, stats} ->
        summary = "📊 AllSource Statistics\n\n"
        formatted_data = ToonEncoder.format_response(stats, format)
        text = summary <> formatted_data

        {:ok,
         %{
           content: [
             %{
               type: "text",
               text: text
             }
           ]
         }}

      {:error, reason} ->
        {:error, "Failed to get stats: #{inspect(reason)}"}
    end
  end

  @doc false
  def handle_get_cluster_status(state, format) do
    case ControlPlaneClient.get_cluster_status(state.control_client) do
      {:ok, status} ->
        summary = "🎯 Cluster Status\n\n"
        formatted_data = ToonEncoder.format_response(status, format)
        text = summary <> formatted_data

        {:ok,
         %{
           content: [
             %{
               type: "text",
               text: text
             }
           ]
         }}

      {:error, reason} ->
        {:error, "Failed to get cluster status: #{inspect(reason)}"}
    end
  end

  @doc false
  def handle_semantic_search_events(args, state, format) do
    query = Map.fetch!(args, "query")
    limit = Map.get(args, "limit", 100)
    threshold = Map.get(args, "threshold", 0.7)

    params = %{
      "query" => query,
      "limit" => limit,
      "threshold" => threshold
    }

    case CoreClient.semantic_search(state.core_client, params) do
      {:ok, data} ->
        results = Map.get(data, "results", [])
        count = length(results)

        summary = """
        🔍 Semantic Search Results
        📝 Query: "#{query}"
        📊 Found: #{count} matching events
        🎯 Threshold: #{threshold}

        """

        formatted_data = ToonEncoder.format_response(data, format)
        text = summary <> formatted_data

        {:ok,
         %{
           content: [
             %{
               type: "text",
               text: text
             }
           ]
         }}

      {:error, reason} ->
        {:error, "Failed to perform semantic search: #{inspect(reason)}"}
    end
  end

  @doc false
  def handle_hybrid_search(args, state, format) do
    semantic_query = Map.get(args, "semantic_query")
    keywords = Map.get(args, "keywords")
    filters = Map.get(args, "filters", %{})
    limit = Map.get(args, "limit", 100)

    # Build the search params
    params = %{
      "limit" => limit
    }

    params =
      if semantic_query, do: Map.put(params, "semantic_query", semantic_query), else: params

    params = if keywords, do: Map.put(params, "keywords", keywords), else: params

    # Add filters if present
    params =
      if map_size(filters) > 0 do
        Map.put(params, "filters", filters)
      else
        params
      end

    case CoreClient.hybrid_search(state.core_client, params) do
      {:ok, data} ->
        results = Map.get(data, "results", [])
        count = length(results)

        # Build a descriptive summary of what was searched
        search_desc =
          [
            if(semantic_query, do: "Semantic: \"#{semantic_query}\"", else: nil),
            if(keywords, do: "Keywords: \"#{keywords}\"", else: nil)
          ]
          |> Enum.reject(&is_nil/1)
          |> Enum.join(" + ")

        search_desc = if search_desc == "", do: "No query provided", else: search_desc

        # Build filter description
        filter_desc =
          [
            if(Map.get(filters, "event_type"),
              do: "event_type=#{Map.get(filters, "event_type")}",
              else: nil
            ),
            if(Map.get(filters, "entity_id"),
              do: "entity_id=#{Map.get(filters, "entity_id")}",
              else: nil
            ),
            if(Map.get(filters, "time_from"),
              do: "from=#{Map.get(filters, "time_from")}",
              else: nil
            ),
            if(Map.get(filters, "time_to"), do: "to=#{Map.get(filters, "time_to")}", else: nil)
          ]
          |> Enum.reject(&is_nil/1)
          |> Enum.join(", ")

        filter_line = if filter_desc != "", do: "🔧 Filters: #{filter_desc}\n", else: ""

        summary = """
        🔀 Hybrid Search Results
        📝 #{search_desc}
        #{filter_line}📊 Found: #{count} matching events

        """

        formatted_data = ToonEncoder.format_response(data, format)
        text = summary <> formatted_data

        {:ok,
         %{
           content: [
             %{
               type: "text",
               text: text
             }
           ]
         }}

      {:error, reason} ->
        {:error, "Failed to perform hybrid search: #{inspect(reason)}"}
    end
  end

  @doc false
  def handle_get_query_advice(args, _format) do
    use_case = Map.fetch!(args, "use_case")
    context = Map.get(args, "context")

    advice = get_advice_for_use_case(use_case, context)

    # Format advice as structured text (more readable than TOON for this use case)
    text = format_advice_as_text(use_case, context, advice)

    {:ok,
     %{
       content: [
         %{
           type: "text",
           text: text
         }
       ]
     }}
  end

  # ============================================================================
  # Quick Exploration Handlers
  # ============================================================================

  @doc false
  def handle_sample_events(args, state, format) do
    sample_size = min(Map.get(args, "sample_size", 1000), 10000)
    stratified_by = Map.get(args, "stratified_by")

    # Build base query params
    base_params = Map.take(args, ["entity_id", "event_type", "since", "until"])

    # For sampling, we fetch more than needed and then subsample
    # This allows us to do stratification on the client side
    fetch_limit = min(sample_size * 3, 30000)
    params = Map.put(base_params, "limit", fetch_limit)

    case CoreClient.query_events(state.core_client, params) do
      {:ok, data} ->
        events = Map.get(data, "events", [])

        # Apply stratified sampling
        sampled_events = apply_stratified_sampling(events, sample_size, stratified_by)

        # Build statistics about the sample
        sample_stats = calculate_sample_stats(sampled_events, stratified_by)

        summary = """
        🎲 Sample Events (#{length(sampled_events)} of ~#{length(events)} available)
        📊 Sample size requested: #{sample_size}
        🎯 Stratification: #{stratified_by || "random"}

        #{format_sample_stats(sample_stats)}
        """

        sampled_data = %{
          "events" => sampled_events,
          "sample_size" => length(sampled_events),
          "total_available" => length(events),
          "stratified_by" => stratified_by,
          "sample_stats" => sample_stats
        }

        formatted_data = ToonEncoder.format_response(sampled_data, format)
        text = summary <> formatted_data

        {:ok,
         %{
           content: [
             %{
               type: "text",
               text: text
             }
           ]
         }}

      {:error, reason} ->
        {:error, "Failed to sample events: #{inspect(reason)}"}
    end
  end

  @doc false
  def handle_quick_stats(args, state, format) do
    metric = Map.get(args, "metric", "all")
    entity_id = Map.get(args, "entity_id")
    event_type = Map.get(args, "event_type")

    # Build query params for filtered stats
    params =
      %{}
      |> maybe_put("entity_id", entity_id)
      |> maybe_put("event_type", event_type)
      |> Map.put("limit", 10000)

    # Get events for approximation (or use pre-computed stats if available)
    result =
      if entity_id || event_type do
        # Filtered query - need to compute from events
        CoreClient.query_events(state.core_client, params)
      else
        # Unfiltered - can use get_stats for pre-computed values
        CoreClient.get_stats(state.core_client)
      end

    case result do
      {:ok, data} ->
        stats = compute_quick_stats(data, metric, entity_id, event_type)

        summary = """
        ⚡ Quick Stats#{if entity_id, do: " for entity: #{entity_id}", else: ""}#{if event_type, do: " for type: #{event_type}", else: ""}
        📊 Metric: #{metric}
        ⚠️  Note: These are approximate values for speed

        """

        formatted_data = ToonEncoder.format_response(stats, format)
        text = summary <> formatted_data

        {:ok,
         %{
           content: [
             %{
               type: "text",
               text: text
             }
           ]
         }}

      {:error, reason} ->
        {:error, "Failed to get quick stats: #{inspect(reason)}"}
    end
  end

  # ============================================================================
  # Quick Exploration Helpers
  # ============================================================================

  defp apply_stratified_sampling(events, sample_size, nil) do
    # Random sampling without stratification
    events
    |> Enum.shuffle()
    |> Enum.take(sample_size)
  end

  defp apply_stratified_sampling(events, sample_size, "event_type") do
    # Group by event type and sample proportionally
    grouped = Enum.group_by(events, &Map.get(&1, "event_type", "unknown"))
    total = length(events)

    if total == 0 do
      []
    else
      # Calculate samples per group proportionally
      grouped
      |> Enum.flat_map(fn {_type, group_events} ->
        proportion = length(group_events) / total
        group_sample_size = max(1, round(sample_size * proportion))

        group_events
        |> Enum.shuffle()
        |> Enum.take(group_sample_size)
      end)
      |> Enum.shuffle()
      |> Enum.take(sample_size)
    end
  end

  defp apply_stratified_sampling(events, sample_size, "entity_id") do
    # Group by entity and sample proportionally
    grouped = Enum.group_by(events, &Map.get(&1, "entity_id", "unknown"))
    total = length(events)

    if total == 0 do
      []
    else
      grouped
      |> Enum.flat_map(fn {_entity, group_events} ->
        proportion = length(group_events) / total
        group_sample_size = max(1, round(sample_size * proportion))

        group_events
        |> Enum.shuffle()
        |> Enum.take(group_sample_size)
      end)
      |> Enum.shuffle()
      |> Enum.take(sample_size)
    end
  end

  defp apply_stratified_sampling(events, sample_size, "time") do
    # Temporal stratification - sample evenly across time range
    sorted = Enum.sort_by(events, &Map.get(&1, "timestamp", ""))
    total = length(sorted)

    if total == 0 do
      []
    else
      # Calculate stride to sample evenly across the timeline
      stride = max(1, div(total, sample_size))

      sorted
      |> Enum.with_index()
      |> Enum.filter(fn {_event, idx} -> rem(idx, stride) == 0 end)
      |> Enum.map(fn {event, _idx} -> event end)
      |> Enum.take(sample_size)
    end
  end

  defp calculate_sample_stats(events, stratified_by) do
    event_types =
      events
      |> Enum.map(&Map.get(&1, "event_type", "unknown"))
      |> Enum.frequencies()

    entity_ids =
      events
      |> Enum.map(&Map.get(&1, "entity_id", "unknown"))
      |> Enum.uniq()
      |> length()

    timestamps = Enum.map(events, &Map.get(&1, "timestamp", ""))
    min_time = Enum.min(timestamps, fn -> nil end)
    max_time = Enum.max(timestamps, fn -> nil end)

    %{
      event_type_distribution: event_types,
      unique_entities: entity_ids,
      time_range: %{earliest: min_time, latest: max_time},
      stratification: stratified_by || "random"
    }
  end

  defp format_sample_stats(stats) do
    type_count = map_size(stats.event_type_distribution)

    top_types =
      stats.event_type_distribution
      |> Enum.sort_by(fn {_k, v} -> -v end)
      |> Enum.take(5)
      |> Enum.map(fn {type, count} -> "  • #{type}: #{count}" end)
      |> Enum.join("\n")

    """
    📈 Sample Distribution:
    • Event types: #{type_count}
    • Unique entities: #{stats.unique_entities}
    • Time range: #{stats.time_range.earliest || "N/A"} to #{stats.time_range.latest || "N/A"}

    🔝 Top event types in sample:
    #{top_types}
    """
  end

  defp compute_quick_stats(data, metric, entity_id, event_type) do
    # Check if we have pre-computed stats or raw events
    events = Map.get(data, "events", [])
    has_events = length(events) > 0

    base_stats =
      if has_events do
        # Compute from events
        compute_stats_from_events(events)
      else
        # Use pre-computed stats
        %{
          event_count: Map.get(data, "total_events", Map.get(data, "event_count", 0)),
          unique_entities: Map.get(data, "unique_entities", 0),
          event_types: Map.get(data, "event_types", %{}),
          time_range: %{
            earliest: Map.get(data, "oldest_event"),
            latest: Map.get(data, "newest_event")
          }
        }
      end

    # Add approximation note
    base_stats = Map.put(base_stats, :approximate, true)

    # Filter to requested metric if not "all"
    case metric do
      "all" ->
        base_stats

      "event_count" ->
        %{event_count: base_stats.event_count, approximate: true}

      "unique_entities" ->
        %{unique_entities: base_stats.unique_entities, approximate: true}

      "event_types" ->
        %{event_types: base_stats.event_types, approximate: true}

      "time_range" ->
        %{time_range: base_stats.time_range, approximate: true}

      _ ->
        base_stats
    end
    |> maybe_add_filter_context(entity_id, event_type)
  end

  defp compute_stats_from_events(events) do
    event_types =
      events
      |> Enum.map(&Map.get(&1, "event_type", "unknown"))
      |> Enum.frequencies()

    unique_entities =
      events
      |> Enum.map(&Map.get(&1, "entity_id", "unknown"))
      |> Enum.uniq()
      |> length()

    timestamps = Enum.map(events, &Map.get(&1, "timestamp", ""))

    %{
      event_count: length(events),
      unique_entities: unique_entities,
      event_types: event_types,
      time_range: %{
        earliest: Enum.min(timestamps, fn -> nil end),
        latest: Enum.max(timestamps, fn -> nil end)
      }
    }
  end

  defp maybe_add_filter_context(stats, nil, nil), do: stats

  defp maybe_add_filter_context(stats, entity_id, event_type) do
    context = %{}
    context = if entity_id, do: Map.put(context, :filtered_by_entity, entity_id), else: context
    context = if event_type, do: Map.put(context, :filtered_by_type, event_type), else: context
    Map.put(stats, :filter_context, context)
  end

  defp maybe_put(map, _key, nil), do: map
  defp maybe_put(map, key, value), do: Map.put(map, key, value)

  # ============================================================================
  # Conversation Context Handlers
  # ============================================================================

  @doc false
  def handle_start_session(args, _state, _format) do
    session_id = Map.fetch!(args, "session_id")

    # Build initial context from provided args
    initial_context =
      args
      |> Map.drop(["session_id"])
      |> Enum.reduce(%{}, fn
        {"entity_id", v}, acc when is_binary(v) -> Map.put(acc, :entity_id, v)
        {"entity_type", v}, acc when is_binary(v) -> Map.put(acc, :entity_type, v)
        {"event_type", v}, acc when is_binary(v) -> Map.put(acc, :event_type, v)
        {"since", v}, acc when is_binary(v) -> Map.put(acc, :since, v)
        {"until", v}, acc when is_binary(v) -> Map.put(acc, :until, v)
        {"semantic_query", v}, acc when is_binary(v) -> Map.put(acc, :semantic_query, v)
        _, acc -> acc
      end)

    # Create or get the session
    case ConversationContext.get_or_create_session(ConversationContext, session_id) do
      {:ok, session} ->
        # Apply initial context if provided
        {:ok, updated_session} =
          if map_size(initial_context) > 0 do
            ConversationContext.update_session(ConversationContext, session_id, initial_context)
          else
            {:ok, session}
          end

        # Build response
        query_params = session_to_display_params(updated_session)

        text = """
        🎯 Session Started: "#{session_id}"
        📅 Created: #{format_datetime(updated_session.created_at)}

        #{if map_size(initial_context) > 0, do: "📋 Initial Context:\n#{format_context_summary(initial_context)}", else: "📋 No initial context (session is empty)"}

        💡 Next steps:
        - Use refine_query to add filters: `refine_query(session_id: "#{session_id}", filters: {...})`
        - Use get_session_context to see current state
        - Use query parameters with query_events, semantic_search, etc.

        📊 Current Query Parameters:
        #{format_query_params(query_params)}
        """

        {:ok,
         %{
           content: [
             %{
               type: "text",
               text: text
             }
           ]
         }}

      {:error, reason} ->
        {:error, "Failed to create session: #{inspect(reason)}"}
    end
  end

  @doc false
  def handle_refine_query(args, _state, _format) do
    session_id = Map.fetch!(args, "session_id")

    # Build refinement context from provided args
    refinement =
      args
      |> Map.drop(["session_id"])
      |> Enum.reduce(%{}, fn
        {"entity_id", v}, acc when is_binary(v) -> Map.put(acc, :entity_id, v)
        {"entity_ids", v}, acc when is_list(v) -> Map.put(acc, :entity_ids, v)
        {"entity_type", v}, acc when is_binary(v) -> Map.put(acc, :entity_type, v)
        {"event_type", v}, acc when is_binary(v) -> Map.put(acc, :event_type, v)
        {"event_types", v}, acc when is_list(v) -> Map.put(acc, :event_types, v)
        {"filters", v}, acc when is_map(v) -> Map.put(acc, :filters, v)
        {"since", v}, acc when is_binary(v) -> Map.put(acc, :since, v)
        {"until", v}, acc when is_binary(v) -> Map.put(acc, :until, v)
        {"as_of", v}, acc when is_binary(v) -> Map.put(acc, :as_of, v)
        {"semantic_query", v}, acc when is_binary(v) -> Map.put(acc, :semantic_query, v)
        {"keywords", v}, acc when is_binary(v) -> Map.put(acc, :keywords, v)
        {"replace_entity_ids", v}, acc when is_boolean(v) -> Map.put(acc, :replace_entity_ids, v)
        _, acc -> acc
      end)

    if map_size(refinement) == 0 do
      {:error, "No refinement parameters provided. Specify at least one filter to add."}
    else
      case ConversationContext.build_query(ConversationContext, session_id, refinement) do
        {:ok, query_params} ->
          # Get updated session for display
          {:ok, session} = ConversationContext.get_session(ConversationContext, session_id)

          text = """
          ✏️  Query Refined for session "#{session_id}"
          ⏰ Last accessed: #{format_datetime(session.last_accessed)}
          📊 Total refinements: #{length(session.query_history)}

          📋 Applied Refinement:
          #{format_context_summary(refinement)}

          📊 Updated Query Parameters:
          #{format_query_params(query_params)}

          💡 Use these parameters with:
          - query_events(#{format_inline_params(query_params)})
          - semantic_search_events(#{format_inline_params(query_params)})
          - hybrid_search(#{format_inline_params(query_params)})
          """

          {:ok,
           %{
             content: [
               %{
                 type: "text",
                 text: text
               }
             ]
           }}

        {:error, reason} ->
          {:error, "Failed to refine query: #{inspect(reason)}"}
      end
    end
  end

  @doc false
  def handle_get_session_context(args, _state, _format) do
    session_id = Map.fetch!(args, "session_id")
    include_history = Map.get(args, "include_history", true)

    case ConversationContext.get_session(ConversationContext, session_id) do
      {:ok, session} ->
        query_params = session_to_display_params(session)

        history_section =
          if include_history and length(session.query_history) > 0 do
            history_lines =
              session.query_history
              |> Enum.with_index(1)
              |> Enum.map(fn {entry, idx} ->
                refinement_summary =
                  entry.refinement
                  |> Enum.map(fn {k, v} -> "#{k}: #{inspect(v)}" end)
                  |> Enum.join(", ")

                "   #{idx}. #{format_datetime(entry.timestamp)} - #{refinement_summary}"
              end)
              |> Enum.join("\n")

            """

            📜 Query History (most recent first):
            #{history_lines}
            """
          else
            ""
          end

        result_section =
          if session.last_result_summary do
            """

            📊 Last Result Summary:
            #{inspect(session.last_result_summary, pretty: true)}
            """
          else
            ""
          end

        text = """
        🔍 Session Context: "#{session_id}"

        📅 Created: #{format_datetime(session.created_at)}
        ⏰ Last accessed: #{format_datetime(session.last_accessed)}
        📊 Query refinements: #{length(session.query_history)}

        📋 Current Context:
        #{format_session_context(session)}

        📊 Query Parameters (ready to use):
        #{format_query_params(query_params)}
        #{history_section}#{result_section}
        💡 Copy these parameters to query_events, semantic_search_events, or hybrid_search
        """

        {:ok,
         %{
           content: [
             %{
               type: "text",
               text: text
             }
           ]
         }}

      {:error, :not_found} ->
        {:error, "Session '#{session_id}' not found. Use start_session to create one."}
    end
  end

  # ============================================================================
  # Context Formatting Helpers
  # ============================================================================

  defp session_to_display_params(session) do
    params = %{}

    params =
      case session.entity_ids do
        [single_id] -> Map.put(params, "entity_id", single_id)
        ids when length(ids) > 1 -> Map.put(params, "entity_ids", ids)
        _ -> params
      end

    params =
      case session.event_types do
        [single_type] -> Map.put(params, "event_type", single_type)
        types when length(types) > 1 -> Map.put(params, "event_types", types)
        _ -> params
      end

    params = if session.time_since, do: Map.put(params, "since", session.time_since), else: params
    params = if session.time_until, do: Map.put(params, "until", session.time_until), else: params
    params = if session.time_as_of, do: Map.put(params, "as_of", session.time_as_of), else: params

    params =
      if session.semantic_query,
        do: Map.put(params, "semantic_query", session.semantic_query),
        else: params

    params = if session.keywords, do: Map.put(params, "keywords", session.keywords), else: params

    params =
      if map_size(session.filters) > 0 do
        Map.put(params, "filters", session.filters)
      else
        params
      end

    params
  end

  defp format_datetime(nil), do: "N/A"

  defp format_datetime(%DateTime{} = dt) do
    Calendar.strftime(dt, "%Y-%m-%d %H:%M:%S UTC")
  end

  defp format_context_summary(context) when is_map(context) do
    context
    |> Enum.map(fn {k, v} -> "   • #{k}: #{inspect(v)}" end)
    |> Enum.join("\n")
  end

  defp format_query_params(params) when map_size(params) == 0 do
    "   (empty - no filters applied)"
  end

  defp format_query_params(params) do
    params
    |> Enum.map(fn {k, v} -> "   #{k}: #{Jason.encode!(v)}" end)
    |> Enum.join("\n")
  end

  defp format_inline_params(params) when map_size(params) == 0 do
    ""
  end

  defp format_inline_params(params) do
    params
    |> Enum.take(3)
    |> Enum.map(fn {k, v} -> "#{k}: #{Jason.encode!(v)}" end)
    |> Enum.join(", ")
    |> then(fn s ->
      if map_size(params) > 3, do: s <> ", ...", else: s
    end)
  end

  defp format_session_context(session) do
    lines = []

    lines =
      if length(session.entity_ids) > 0 do
        lines ++ ["   • Entity IDs: #{inspect(session.entity_ids)}"]
      else
        lines
      end

    lines =
      if session.entity_type do
        lines ++ ["   • Entity Type: #{session.entity_type}"]
      else
        lines
      end

    lines =
      if length(session.event_types) > 0 do
        lines ++ ["   • Event Types: #{inspect(session.event_types)}"]
      else
        lines
      end

    lines =
      if session.time_since do
        lines ++ ["   • Since: #{session.time_since}"]
      else
        lines
      end

    lines =
      if session.time_until do
        lines ++ ["   • Until: #{session.time_until}"]
      else
        lines
      end

    lines =
      if session.time_as_of do
        lines ++ ["   • As Of: #{session.time_as_of}"]
      else
        lines
      end

    lines =
      if session.semantic_query do
        lines ++ ["   • Semantic Query: \"#{session.semantic_query}\""]
      else
        lines
      end

    lines =
      if session.keywords do
        lines ++ ["   • Keywords: \"#{session.keywords}\""]
      else
        lines
      end

    lines =
      if map_size(session.filters) > 0 do
        lines ++ ["   • Filters: #{inspect(session.filters)}"]
      else
        lines
      end

    if length(lines) == 0 do
      "   (no context - session is empty)"
    else
      Enum.join(lines, "\n")
    end
  end

  defp format_advice_as_text(use_case, context, advice) do
    header = """
    💡 Query Advice for: #{format_use_case(use_case)}
    #{if context, do: "📋 Context: #{context}\n", else: ""}
    """

    tools_section = format_recommended_tools(advice.recommended_tools)
    patterns_section = format_query_patterns(advice.query_patterns)
    tips_section = format_performance_tips(advice.performance_tips)
    pitfalls_section = format_common_pitfalls(advice.common_pitfalls)

    header <> tools_section <> patterns_section <> tips_section <> pitfalls_section
  end

  defp format_recommended_tools(tools) do
    tool_lines =
      tools
      |> Enum.sort_by(& &1.priority)
      |> Enum.map(fn t -> "   #{t.priority}. #{t.tool} - #{t.purpose}" end)
      |> Enum.join("\n")

    """
    📦 RECOMMENDED TOOLS (in order):
    #{tool_lines}

    """
  end

  defp format_query_patterns(patterns) do
    pattern_lines =
      patterns
      |> Enum.map(fn p -> "   • #{p.pattern}\n     → #{p.approach}" end)
      |> Enum.join("\n")

    """
    🔍 QUERY PATTERNS:
    #{pattern_lines}

    """
  end

  defp format_performance_tips(tips) do
    tip_lines =
      tips
      |> Enum.map(fn t -> "   • #{t}" end)
      |> Enum.join("\n")

    """
    ⚡ PERFORMANCE TIPS:
    #{tip_lines}

    """
  end

  defp format_common_pitfalls(pitfalls) do
    pitfall_lines =
      pitfalls
      |> Enum.map(fn p -> "   ⚠️  #{p}" end)
      |> Enum.join("\n")

    """
    🚫 COMMON PITFALLS TO AVOID:
    #{pitfall_lines}
    """
  end

  defp format_use_case(use_case) do
    case use_case do
      "audit_trail" -> "Audit Trail Investigation"
      "user_analytics" -> "User Analytics"
      "debugging" -> "Debugging & Incident Response"
      "compliance" -> "Compliance & Regulatory"
      "performance_analysis" -> "Performance Analysis"
      _ -> use_case
    end
  end

  defp get_advice_for_use_case(use_case, context) do
    base_advice = advice_database()[use_case]

    # Add context-specific tips if context is provided
    context_tips =
      if context do
        generate_context_tips(use_case, context)
      else
        []
      end

    Map.update(base_advice, :performance_tips, [], fn tips ->
      tips ++ context_tips
    end)
  end

  defp generate_context_tips(_use_case, context) do
    context_lower = String.downcase(context)

    tips = []

    tips =
      if String.contains?(context_lower, ["payment", "checkout", "transaction"]) do
        tips ++
          [
            "For payment flows, always include correlation IDs in your queries",
            "Consider using hybrid_search with 'payment' or 'transaction' semantic queries"
          ]
      else
        tips
      end

    tips =
      if String.contains?(context_lower, ["auth", "login", "session"]) do
        tips ++
          [
            "Authentication events often span multiple entities - use semantic_search_events",
            "Consider querying by IP address or session ID patterns in metadata"
          ]
      else
        tips
      end

    tips =
      if String.contains?(context_lower, ["user", "customer", "account"]) do
        tips ++
          [
            "Use explain_entity first to understand user's complete journey",
            "Compare entities to identify behavior patterns across user segments"
          ]
      else
        tips
      end

    tips
  end

  defp advice_database do
    %{
      "audit_trail" => %{
        recommended_tools: [
          %{
            tool: "event_timeline",
            purpose: "Get chronological view of all actions",
            priority: 1
          },
          %{
            tool: "query_events",
            purpose: "Filter by specific entity, time range, or event type",
            priority: 2
          },
          %{
            tool: "reconstruct_state",
            purpose: "See exact state at any point in time",
            priority: 3
          },
          %{
            tool: "analyze_changes",
            purpose: "Compare state before/after an incident",
            priority: 4
          }
        ],
        query_patterns: [
          %{
            pattern: "Who modified entity X?",
            approach:
              "event_timeline(entity_id: 'X') → look for modification events with actor metadata"
          },
          %{
            pattern: "What was the state before incident?",
            approach: "reconstruct_state(entity_id: 'X', as_of: '[incident_time - 1h]')"
          },
          %{
            pattern: "All changes in time window",
            approach:
              "query_events(since: '[start]', until: '[end]') → then analyze_changes for each entity"
          },
          %{
            pattern: "Track specific action type",
            approach: "query_events(event_type: 'user.updated', since: '[date]')"
          }
        ],
        performance_tips: [
          "Always use time bounds (since/until) to limit result sets",
          "Use event_timeline for human-readable audit logs",
          "Use query_events with limit for initial exploration, then narrow down",
          "Cache reconstruct_state results if you need multiple point-in-time comparisons"
        ],
        common_pitfalls: [
          "Forgetting to include time bounds on high-volume entities",
          "Using reconstruct_state when event_timeline would be more appropriate",
          "Not checking metadata for actor/source information",
          "Querying without limit on entities with thousands of events"
        ]
      },
      "user_analytics" => %{
        recommended_tools: [
          %{
            tool: "find_patterns",
            purpose: "Discover behavior patterns and sequences",
            priority: 1
          },
          %{
            tool: "compare_entities",
            purpose: "Compare behavior across user cohorts",
            priority: 2
          },
          %{
            tool: "semantic_search_events",
            purpose: "Find events by behavior description",
            priority: 3
          },
          %{
            tool: "explain_entity",
            purpose: "Deep dive into individual user journey",
            priority: 4
          }
        ],
        query_patterns: [
          %{
            pattern: "Common user workflows",
            approach: "find_patterns(pattern_type: 'sequence') → analyze event progressions"
          },
          %{
            pattern: "Compare user segments",
            approach:
              "compare_entities(entity_ids: ['user-A', 'user-B', ...]) → find behavioral differences"
          },
          %{
            pattern: "Find engagement patterns",
            approach:
              "semantic_search_events(query: 'user engagement interaction') → discover related events"
          },
          %{
            pattern: "User lifecycle analysis",
            approach: "explain_entity(entity_id: 'user-X') → full journey overview"
          }
        ],
        performance_tips: [
          "Start with find_patterns(pattern_type: 'frequency') to understand event distribution",
          "Use timeframes in compare_entities to focus on recent behavior",
          "Limit semantic searches to avoid large result sets during exploration",
          "Use get_stats first to understand overall data volume"
        ],
        common_pitfalls: [
          "Analyzing too many users at once - start with representative samples",
          "Not accounting for seasonality in behavior patterns",
          "Missing the 'sequence' pattern type which shows workflow progressions",
          "Forgetting that compare_entities returns event type overlap, not detailed comparison"
        ]
      },
      "debugging" => %{
        recommended_tools: [
          %{
            tool: "event_timeline",
            purpose: "Trace exact sequence of events leading to issue",
            priority: 1
          },
          %{
            tool: "reconstruct_state",
            purpose: "See state at moment of failure",
            priority: 2
          },
          %{
            tool: "semantic_search_events",
            purpose: "Find related errors or anomalies",
            priority: 3
          },
          %{
            tool: "find_patterns",
            purpose: "Detect anomalies or unusual sequences",
            priority: 4
          },
          %{
            tool: "get_cluster_status",
            purpose: "Check system health during incident window",
            priority: 5
          }
        ],
        query_patterns: [
          %{
            pattern: "Trace error back to root cause",
            approach:
              "event_timeline(entity_id: 'X', until: '[error_time]') → follow events backward"
          },
          %{
            pattern: "Find related failures",
            approach: "semantic_search_events(query: 'error failure exception [domain]')"
          },
          %{
            pattern: "State at time of failure",
            approach: "reconstruct_state(entity_id: 'X', as_of: '[error_time - 1s]')"
          },
          %{
            pattern: "Detect anomalous patterns",
            approach: "find_patterns(pattern_type: 'anomaly', since: '[incident_window_start]')"
          }
        ],
        performance_tips: [
          "Always bound queries to incident time window",
          "Use get_cluster_status first to rule out infrastructure issues",
          "Start with broad semantic search, then narrow with specific filters",
          "Use hybrid_search to combine error keywords with semantic context"
        ],
        common_pitfalls: [
          "Not checking cluster health first - infrastructure issues mask application bugs",
          "Starting too narrow - use semantic search for exploration before filtering",
          "Ignoring metadata which often contains error context and stack traces",
          "Forgetting to check events just BEFORE the error timestamp"
        ]
      },
      "compliance" => %{
        recommended_tools: [
          %{
            tool: "query_events",
            purpose: "Retrieve complete event records with full audit trail",
            priority: 1
          },
          %{
            tool: "reconstruct_state",
            purpose: "Prove state at specific regulatory checkpoint",
            priority: 2
          },
          %{
            tool: "analyze_changes",
            purpose: "Document all changes in compliance period",
            priority: 3
          },
          %{
            tool: "event_timeline",
            purpose: "Generate human-readable audit reports",
            priority: 4
          }
        ],
        query_patterns: [
          %{
            pattern: "Data retention compliance",
            approach: "query_events(entity_id: 'X') → verify all events exist for retention period"
          },
          %{
            pattern: "Prove state at audit date",
            approach:
              "reconstruct_state(entity_id: 'X', as_of: '[audit_date]') → immutable proof of state"
          },
          %{
            pattern: "Document all modifications",
            approach:
              "analyze_changes(entity_id: 'X', from_time: '[period_start]', to_time: '[period_end]')"
          },
          %{
            pattern: "Generate audit report",
            approach:
              "event_timeline(entity_id: 'X', since: '[start]', until: '[end]') → chronological report"
          }
        ],
        performance_tips: [
          "Use JSON format for compliance reports (format: 'json') - more suitable for archival",
          "Query complete time ranges without limits for audit completeness",
          "Include metadata in queries - it often contains required actor/source information",
          "Use get_stats to verify event counts match expected totals"
        ],
        common_pitfalls: [
          "Using limits on compliance queries - you need complete records",
          "Not preserving original timestamps - always use stored event times",
          "Forgetting to document the query parameters used for reproducibility",
          "Missing metadata fields that contain required audit information (actor, source, IP)"
        ]
      },
      "performance_analysis" => %{
        recommended_tools: [
          %{
            tool: "get_stats",
            purpose: "Understand data volume and growth patterns",
            priority: 1
          },
          %{
            tool: "get_cluster_status",
            purpose: "Monitor system health and resource usage",
            priority: 2
          },
          %{
            tool: "find_patterns",
            purpose: "Identify high-frequency events or hotspots",
            priority: 3
          },
          %{
            tool: "query_events",
            purpose: "Measure query performance on specific patterns",
            priority: 4
          }
        ],
        query_patterns: [
          %{
            pattern: "Identify write hotspots",
            approach: "find_patterns(pattern_type: 'frequency') → find high-volume event types"
          },
          %{
            pattern: "Monitor system health",
            approach: "get_cluster_status() → check node health, replication lag, resource usage"
          },
          %{
            pattern: "Data growth analysis",
            approach: "get_stats() → review event counts, storage size, index statistics"
          },
          %{
            pattern: "Query performance baseline",
            approach: "query_events with increasing limits → measure response time scaling"
          }
        ],
        performance_tips: [
          "Run get_stats before and after major ingestion to track growth",
          "Monitor get_cluster_status during peak usage periods",
          "Use find_patterns(pattern_type: 'frequency') to identify optimization targets",
          "Test query patterns with limits before running unbounded queries"
        ],
        common_pitfalls: [
          "Not establishing baselines - always compare against previous measurements",
          "Ignoring cluster_status warnings about replication lag or resource pressure",
          "Focusing only on query performance without considering ingestion rates",
          "Not using get_snapshot for current state (much faster than reconstruct_state)"
        ]
      }
    }
  end

  # ============================================================================
  # Helper Functions
  # ============================================================================

  defp calculate_diff(before_map, after_map) do
    added =
      after_map
      |> Map.keys()
      |> Enum.filter(fn key -> not Map.has_key?(before_map, key) end)

    removed =
      before_map
      |> Map.keys()
      |> Enum.filter(fn key -> not Map.has_key?(after_map, key) end)

    modified =
      before_map
      |> Map.keys()
      |> Enum.filter(fn key ->
        Map.has_key?(after_map, key) and Map.get(before_map, key) != Map.get(after_map, key)
      end)
      |> Enum.map(fn key ->
        %{
          field: key,
          before_value: Map.get(before_map, key),
          after_value: Map.get(after_map, key)
        }
      end)

    %{
      added: added,
      modified: modified,
      removed: removed
    }
  end

  defp analyze_patterns(events, pattern_type) do
    analysis = %{}

    analysis =
      if pattern_type == "frequency" or is_nil(pattern_type) do
        frequency_map =
          events
          |> Enum.reduce(%{}, fn event, acc ->
            event_type = Map.get(event, "event_type", "unknown")
            Map.update(acc, event_type, 1, &(&1 + 1))
          end)

        frequency =
          frequency_map
          |> Enum.map(fn {type, count} -> %{event_type: type, count: count} end)
          |> Enum.sort_by(& &1.count, :desc)

        Map.put(analysis, :frequency, frequency)
      else
        analysis
      end

    analysis =
      if pattern_type == "sequence" or is_nil(pattern_type) do
        sequences =
          events
          |> Enum.take(10)
          |> Enum.chunk_every(2, 1, :discard)
          |> Enum.map(fn [e1, e2] ->
            "#{Map.get(e1, "event_type")} → #{Map.get(e2, "event_type")}"
          end)

        Map.put(analysis, :common_sequences, sequences)
      else
        analysis
      end

    analysis
  end
end
