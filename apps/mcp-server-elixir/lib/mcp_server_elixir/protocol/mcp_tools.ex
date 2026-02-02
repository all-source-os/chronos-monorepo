defmodule McpServerElixir.Protocol.McpTools do
  @moduledoc """
  MCP Tools implementation - defines all available tools and their handlers.

  This module provides the tools that the MCP server exposes to AI assistants,
  enabling natural language interaction with the AllSource event store.
  """

  require Logger

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
      tool_get_cluster_status()
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
    "ingest_event" => :handle_ingest_event
  }

  @stateless_tool_handlers %{
    "get_stats" => :handle_get_stats,
    "get_cluster_status" => :handle_get_cluster_status
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

      true ->
        {:error, "Unknown tool: #{tool_name}"}
    end
  end

  # ============================================================================
  # Tool Definitions
  # ============================================================================

  defp tool_query_events do
    %{
      name: "query_events",
      description:
        "Query events with flexible filters. Use natural language timeframes like \"since yesterday\" and the LLM will convert them to ISO timestamps. Returns TOON format by default (~50% fewer tokens than JSON).",
      inputSchema: %{
        type: "object",
        properties: %{
          "entity_id" => %{type: "string", description: "Filter by entity ID (e.g., \"user-123\")"},
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
      description:
        "Reconstruct the complete state of an entity at any point in time by replaying its event stream. Perfect for answering \"What did this entity look like on date X?\"",
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
      description:
        "Get the current snapshot of an entity (much faster than reconstruction). Use this when you need the latest state without time-travel.",
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
      description:
        "Analyze what changed for an entity between two points in time. Returns a detailed diff showing added, modified, and removed fields.",
      inputSchema: %{
        type: "object",
        properties: %{
          "entity_id" => %{type: "string", description: "The entity to analyze"},
          "from_time" => %{type: "string", description: "Start timestamp (ISO format)"},
          "to_time" => %{type: "string", description: "End timestamp (ISO format, defaults to now)"}
        },
        required: ["entity_id", "from_time"]
      }
    }
  end

  defp tool_find_patterns do
    %{
      name: "find_patterns",
      description:
        "Detect patterns in event streams: frequency analysis, event sequences, or anomalies. Perfect for answering \"What unusual patterns exist?\"",
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
      description:
        "Compare multiple entities to find similarities and differences in their event histories.",
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
      description:
        "Get a chronological timeline of all events for an entity, formatted for easy reading and understanding.",
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
      description:
        "Get a comprehensive explanation of an entity: current state, event history, key changes, and timeline summary.",
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
      description: "Ingest a new event into the AllSource event store.",
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
      description: "Get comprehensive statistics about the AllSource event store.",
      inputSchema: %{
        type: "object",
        properties: %{}
      }
    }
  end

  defp tool_get_cluster_status do
    %{
      name: "get_cluster_status",
      description: "Get current cluster health and status information.",
      inputSchema: %{
        type: "object",
        properties: %{}
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
