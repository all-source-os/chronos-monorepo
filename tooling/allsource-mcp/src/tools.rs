//! MCP tool definitions and execution.

use allsource_core::embedded::{EmbeddedCore, Query};
use anyhow::Result;
use serde_json::Value;

use crate::protocol::{ToolDef, tool_error, tool_result};

/// Return all available tool definitions.
pub fn tool_definitions() -> Vec<ToolDef> {
    vec![
        ToolDef {
            name: "query_events".to_string(),
            description: "Query events from the AllSource event store. Filter by entity_id, event_type (prefix match), time range, and limit.".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "entity_id": { "type": "string", "description": "Filter by entity ID (exact match)" },
                    "event_type": { "type": "string", "description": "Filter by event type prefix (e.g. 'workflow_run' matches 'workflow_run.started')" },
                    "limit": { "type": "integer", "description": "Max events to return (default 50)", "default": 50 },
                    "since": { "type": "string", "description": "Only events after this ISO 8601 timestamp" },
                    "until": { "type": "string", "description": "Only events before this ISO 8601 timestamp" }
                }
            }),
        },
        ToolDef {
            name: "sample_events".to_string(),
            description: "Return a sample of recent events across all entities. Useful for discovering what data exists.".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "count": { "type": "integer", "description": "Number of events to sample (default 20)", "default": 20 }
                }
            }),
        },
        ToolDef {
            name: "quick_stats".to_string(),
            description: "Get a quick summary of the event store: total events, entity count, event type distribution, date range, and durability status.".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {}
            }),
        },
        ToolDef {
            name: "get_snapshot".to_string(),
            description: "Get the latest projection/snapshot state for an entity. Shows the current computed state derived from all events.".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "entity_id": { "type": "string", "description": "The entity ID to get snapshot for" }
                },
                "required": ["entity_id"]
            }),
        },
        ToolDef {
            name: "event_timeline".to_string(),
            description: "Show a chronological timeline of all events for an entity, with timestamps, types, and payload summaries.".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "entity_id": { "type": "string", "description": "The entity ID to trace" },
                    "limit": { "type": "integer", "description": "Max events (default 100)", "default": 100 }
                },
                "required": ["entity_id"]
            }),
        },
        ToolDef {
            name: "explain_entity".to_string(),
            description: "Generate a human-readable summary of an entity's lifecycle by analyzing its event history.".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "entity_id": { "type": "string", "description": "The entity ID to explain" }
                },
                "required": ["entity_id"]
            }),
        },
        ToolDef {
            name: "reconstruct_state".to_string(),
            description: "Fold all events for an entity to reconstruct its current state. Shows the full event-sourced state reconstruction.".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "entity_id": { "type": "string", "description": "The entity ID to reconstruct" }
                },
                "required": ["entity_id"]
            }),
        },
        ToolDef {
            name: "analyze_changes".to_string(),
            description: "Analyze what changed for an entity between two points in time or between two event versions.".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "entity_id": { "type": "string", "description": "The entity ID to analyze" },
                    "since": { "type": "string", "description": "Start of analysis window (ISO 8601)" },
                    "until": { "type": "string", "description": "End of analysis window (ISO 8601)" }
                },
                "required": ["entity_id"]
            }),
        },
    ]
}

/// Execute a tool call and return the MCP result.
pub async fn execute_tool(core: &EmbeddedCore, name: &str, args: &Value) -> Value {
    match execute_tool_inner(core, name, args).await {
        Ok(result) => tool_result(result),
        Err(e) => tool_error(&format!("Tool '{name}' failed: {e}")),
    }
}

async fn execute_tool_inner(core: &EmbeddedCore, name: &str, args: &Value) -> Result<Value> {
    match name {
        "query_events" => exec_query_events(core, args).await,
        "sample_events" => exec_sample_events(core, args).await,
        "quick_stats" => exec_quick_stats(core).await,
        "get_snapshot" => exec_get_snapshot(core, args).await,
        "event_timeline" => exec_event_timeline(core, args).await,
        "explain_entity" => exec_explain_entity(core, args).await,
        "reconstruct_state" => exec_reconstruct_state(core, args).await,
        "analyze_changes" => exec_analyze_changes(core, args).await,
        _ => anyhow::bail!("Unknown tool: {name}"),
    }
}

async fn exec_query_events(core: &EmbeddedCore, args: &Value) -> Result<Value> {
    let mut query = Query::new();

    if let Some(entity_id) = args.get("entity_id").and_then(|v| v.as_str()) {
        query = query.entity_id(entity_id);
    }
    if let Some(event_type) = args.get("event_type").and_then(|v| v.as_str()) {
        query = query.event_type_prefix(event_type);
    }
    if let Some(since) = args.get("since").and_then(|v| v.as_str()) {
        if let Ok(t) = since.parse() {
            query = query.since(t);
        }
    }
    if let Some(until) = args.get("until").and_then(|v| v.as_str()) {
        if let Ok(t) = until.parse() {
            query = query.until(t);
        }
    }
    let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(50) as usize;
    query = query.limit(limit);

    let events = core.query(query).await?;
    let result: Vec<Value> = events
        .iter()
        .map(|e| {
            serde_json::json!({
                "id": e.id.to_string(),
                "entity_id": e.entity_id,
                "event_type": e.event_type,
                "timestamp": e.timestamp.to_rfc3339(),
                "version": e.version,
                "payload": e.payload,
                "metadata": e.metadata,
            })
        })
        .collect();

    Ok(serde_json::json!({
        "events": result,
        "count": result.len(),
    }))
}

async fn exec_sample_events(core: &EmbeddedCore, args: &Value) -> Result<Value> {
    let count = args.get("count").and_then(|v| v.as_u64()).unwrap_or(20) as usize;

    let events = core.query(Query::new().limit(count)).await?;
    let result: Vec<Value> = events
        .iter()
        .map(|e| {
            serde_json::json!({
                "entity_id": e.entity_id,
                "event_type": e.event_type,
                "timestamp": e.timestamp.to_rfc3339(),
                "payload_keys": e.payload.as_object().map(|o| o.keys().collect::<Vec<_>>()),
            })
        })
        .collect();

    Ok(serde_json::json!({
        "sample": result,
        "count": result.len(),
    }))
}

async fn exec_quick_stats(core: &EmbeddedCore) -> Result<Value> {
    let stats = core.stats();
    let durability = core.durability_status();

    // Get event type distribution by sampling
    let all_events = core.query(Query::new().limit(10000)).await?;
    let mut type_counts: std::collections::HashMap<String, usize> =
        std::collections::HashMap::new();
    let mut entity_ids: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut min_ts = None;
    let mut max_ts = None;

    for e in &all_events {
        *type_counts.entry(e.event_type.clone()).or_default() += 1;
        entity_ids.insert(e.entity_id.clone());
        let ts = e.timestamp;
        min_ts = Some(min_ts.map_or(ts, |m: chrono::DateTime<chrono::Utc>| m.min(ts)));
        max_ts = Some(max_ts.map_or(ts, |m: chrono::DateTime<chrono::Utc>| m.max(ts)));
    }

    let mut top_types: Vec<(String, usize)> = type_counts.into_iter().collect();
    top_types.sort_by(|a, b| b.1.cmp(&a.1));
    top_types.truncate(20);

    Ok(serde_json::json!({
        "total_events": stats.total_events,
        "unique_entities": entity_ids.len(),
        "date_range": {
            "earliest": min_ts.map(|t| t.to_rfc3339()),
            "latest": max_ts.map(|t| t.to_rfc3339()),
        },
        "top_event_types": top_types.iter().map(|(t, c)| serde_json::json!({"type": t, "count": c})).collect::<Vec<_>>(),
        "durability": {
            "wal_enabled": durability.wal_enabled,
            "wal_entries": durability.wal_entries,
            "parquet_enabled": durability.parquet_enabled,
            "parquet_files": durability.parquet_files,
            "durable": durability.durable,
        },
    }))
}

async fn exec_get_snapshot(core: &EmbeddedCore, args: &Value) -> Result<Value> {
    let entity_id = args
        .get("entity_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("entity_id is required"))?;

    // Try common projection names
    let projections = ["default", "state", "snapshot"];
    for proj_name in &projections {
        if let Some(state) = core.projection(proj_name, entity_id) {
            return Ok(serde_json::json!({
                "entity_id": entity_id,
                "projection": proj_name,
                "state": state,
            }));
        }
    }

    // Fall back to reconstructing from events
    exec_reconstruct_state(core, args).await
}

async fn exec_event_timeline(core: &EmbeddedCore, args: &Value) -> Result<Value> {
    let entity_id = args
        .get("entity_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("entity_id is required"))?;
    let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(100) as usize;

    let events = core
        .query(Query::new().entity_id(entity_id).limit(limit))
        .await?;

    let timeline: Vec<Value> = events
        .iter()
        .map(|e| {
            let summary = summarize_payload(&e.payload);
            serde_json::json!({
                "timestamp": e.timestamp.to_rfc3339(),
                "event_type": e.event_type,
                "version": e.version,
                "summary": summary,
            })
        })
        .collect();

    Ok(serde_json::json!({
        "entity_id": entity_id,
        "timeline": timeline,
        "event_count": timeline.len(),
    }))
}

async fn exec_explain_entity(core: &EmbeddedCore, args: &Value) -> Result<Value> {
    let entity_id = args
        .get("entity_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("entity_id is required"))?;

    let events = core
        .query(Query::new().entity_id(entity_id).limit(1000))
        .await?;

    if events.is_empty() {
        return Ok(serde_json::json!({
            "entity_id": entity_id,
            "explanation": "No events found for this entity.",
        }));
    }

    let first = &events[0];
    let last = &events[events.len() - 1];

    let mut type_counts: std::collections::HashMap<&str, usize> = std::collections::HashMap::new();
    for e in &events {
        *type_counts.entry(&e.event_type).or_default() += 1;
    }

    let mut phases: Vec<String> = Vec::new();
    let mut prev_type = "";
    for e in &events {
        if e.event_type != prev_type {
            phases.push(format!(
                "{} ({})",
                e.event_type,
                e.timestamp.format("%Y-%m-%d %H:%M:%S")
            ));
            prev_type = &e.event_type;
        }
    }

    Ok(serde_json::json!({
        "entity_id": entity_id,
        "total_events": events.len(),
        "created": first.timestamp.to_rfc3339(),
        "last_activity": last.timestamp.to_rfc3339(),
        "event_types": type_counts,
        "lifecycle_phases": phases,
    }))
}

async fn exec_reconstruct_state(core: &EmbeddedCore, args: &Value) -> Result<Value> {
    let entity_id = args
        .get("entity_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("entity_id is required"))?;

    let events = core
        .query(Query::new().entity_id(entity_id).limit(10000))
        .await?;

    if events.is_empty() {
        return Ok(serde_json::json!({
            "entity_id": entity_id,
            "state": null,
            "message": "No events found for this entity.",
        }));
    }

    // Fold events: merge all payloads (last write wins per field)
    let mut state = serde_json::Map::new();
    for e in &events {
        state.insert(
            "_last_event_type".to_string(),
            Value::String(e.event_type.clone()),
        );
        state.insert(
            "_last_updated".to_string(),
            Value::String(e.timestamp.to_rfc3339()),
        );
        state.insert("_version".to_string(), serde_json::json!(e.version));

        if let Some(obj) = e.payload.as_object() {
            for (k, v) in obj {
                state.insert(k.clone(), v.clone());
            }
        }
    }

    Ok(serde_json::json!({
        "entity_id": entity_id,
        "events_folded": events.len(),
        "state": Value::Object(state),
    }))
}

async fn exec_analyze_changes(core: &EmbeddedCore, args: &Value) -> Result<Value> {
    let entity_id = args
        .get("entity_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("entity_id is required"))?;

    let mut query = Query::new().entity_id(entity_id).limit(10000);
    if let Some(since) = args.get("since").and_then(|v| v.as_str()) {
        if let Ok(t) = since.parse() {
            query = query.since(t);
        }
    }
    if let Some(until) = args.get("until").and_then(|v| v.as_str()) {
        if let Ok(t) = until.parse() {
            query = query.until(t);
        }
    }

    let events = core.query(query).await?;

    let changes: Vec<Value> = events
        .iter()
        .map(|e| {
            serde_json::json!({
                "timestamp": e.timestamp.to_rfc3339(),
                "event_type": e.event_type,
                "changed_fields": e.payload.as_object().map(|o| o.keys().collect::<Vec<_>>()),
                "payload": e.payload,
            })
        })
        .collect();

    Ok(serde_json::json!({
        "entity_id": entity_id,
        "changes": changes,
        "total_changes": changes.len(),
    }))
}

/// Summarize a payload to a short string for timeline display.
fn summarize_payload(payload: &Value) -> String {
    match payload {
        Value::Object(map) => {
            let keys: Vec<&String> = map.keys().take(5).collect();
            if keys.is_empty() {
                "{}".to_string()
            } else {
                let mut s = keys
                    .iter()
                    .map(|k| k.as_str())
                    .collect::<Vec<_>>()
                    .join(", ");
                if map.len() > 5 {
                    s.push_str(&format!(" (+{} more)", map.len() - 5));
                }
                s
            }
        }
        _ => payload.to_string().chars().take(100).collect(),
    }
}
