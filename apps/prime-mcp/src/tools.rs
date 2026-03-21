//! MCP tool definitions and dispatch for Prime.
//!
//! Each tool maps to a Prime facade method. Tool descriptions are written for
//! AI agent consumption — they explain *when* to use each tool.

use allsource_core::prime::Prime;
use allsource_core::prime::recall::RecallEngine;
use serde_json::{Value, json};

/// Return MCP tool definitions (for `tools/list`).
pub fn tool_definitions() -> Value {
    json!([
        {
            "name": "prime_add_node",
            "description": "Create a new node in the knowledge graph. Use when the agent learns about a new entity (person, concept, project, etc.).",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "type": { "type": "string", "description": "Node type (e.g. 'person', 'concept', 'project')" },
                    "properties": { "type": "object", "description": "Node properties (e.g. {\"name\": \"Alice\", \"role\": \"engineer\"})" }
                },
                "required": ["type", "properties"]
            }
        },
        {
            "name": "prime_add_edge",
            "description": "Create a directed relationship between two nodes. Use when the agent discovers a connection between entities.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "source": { "type": "string", "description": "Source node entity_id" },
                    "target": { "type": "string", "description": "Target node entity_id" },
                    "relation": { "type": "string", "description": "Relationship type (e.g. 'works_on', 'knows', 'authored')" },
                    "properties": { "type": "object", "description": "Optional edge properties" },
                    "weight": { "type": "number", "description": "Optional edge weight (0.0-1.0)" }
                },
                "required": ["source", "target", "relation"]
            }
        },
        {
            "name": "prime_neighbors",
            "description": "Find nodes connected to a given node. Use to explore the knowledge graph around an entity.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "node_id": { "type": "string", "description": "The node entity_id to find neighbors of" },
                    "relation": { "type": "string", "description": "Optional: filter by relation type" },
                    "direction": { "type": "string", "enum": ["incoming", "outgoing", "both"], "description": "Edge direction (default: both)" },
                    "depth": { "type": "integer", "description": "Max traversal depth (default: 1)" }
                },
                "required": ["node_id"]
            }
        },
        {
            "name": "prime_search",
            "description": "Search nodes by type. Use to find all entities of a given type.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "type": { "type": "string", "description": "Node type to search for" }
                },
                "required": ["type"]
            }
        },
        {
            "name": "prime_shortest_path",
            "description": "Find the shortest path between two nodes. Use to discover how entities are connected.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "from": { "type": "string", "description": "Source node entity_id" },
                    "to": { "type": "string", "description": "Target node entity_id" },
                    "relation": { "type": "string", "description": "Optional: restrict to edges of this relation type" }
                },
                "required": ["from", "to"]
            }
        },
        {
            "name": "prime_forget",
            "description": "Soft-delete a node and all its connected edges. The data is preserved in history for audit.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "node_id": { "type": "string", "description": "The node entity_id to delete" }
                },
                "required": ["node_id"]
            }
        },
        {
            "name": "prime_history",
            "description": "Get the full audit trail for any entity. Use to see when and how knowledge was added or changed.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "entity_id": { "type": "string", "description": "The entity_id to get history for (node, edge, or vector)" }
                },
                "required": ["entity_id"]
            }
        },
        {
            "name": "prime_stats",
            "description": "Get graph statistics: total nodes, edges, types, relations.",
            "inputSchema": {
                "type": "object",
                "properties": {}
            }
        },
        {
            "name": "prime_index",
            "description": "Get a compressed summary of everything stored in memory, organized by domain with cross-references. Use this to understand the shape of your knowledge before searching for specifics.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "agent_id": { "type": "string", "description": "Optional agent ID for scoping" }
                }
            }
        },
        {
            "name": "prime_context",
            "description": "Search memory with hybrid recall: compressed index + semantic vectors + graph + temporal. Use instead of prime_search when you want the compressed index included for cross-domain reasoning.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "query": { "type": "string", "description": "Natural language query" },
                    "agent_id": { "type": "string", "description": "Optional agent ID for scoping" },
                    "top_k": { "type": "integer", "description": "Max vector results (default: 5)" },
                    "include_index": { "type": "boolean", "description": "Include compressed index excerpt (default: true)" },
                    "max_tokens": { "type": "integer", "description": "Max total tokens in response" }
                },
                "required": ["query"]
            }
        },
        {
            "name": "prime_embed",
            "description": "Store a vector embedding for a piece of knowledge. Use when the agent has computed an embedding and wants to make content searchable by semantic similarity.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "id": { "type": "string", "description": "Unique ID for this embedding (or entity_id of an existing node)" },
                    "text": { "type": "string", "description": "Source text that was embedded" },
                    "vector": { "type": "array", "items": { "type": "number" }, "description": "The embedding vector (float array)" },
                    "metadata": { "type": "object", "description": "Optional metadata (tags, source, etc.)" }
                },
                "required": ["id", "vector"]
            }
        },
        {
            "name": "prime_similar",
            "description": "Find the most similar stored embeddings to a given ID. Use after prime_embed to find related knowledge.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "id": { "type": "string", "description": "ID of the stored embedding to find similar items for" },
                    "top_k": { "type": "integer", "description": "Number of results (default: 5)" }
                },
                "required": ["id"]
            }
        },
        {
            "name": "prime_recall",
            "description": "Hybrid recall: combines vector similarity, graph proximity, and temporal recency to find the most relevant knowledge. Use this as the primary 'what do I know about X?' tool.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "vector": { "type": "array", "items": { "type": "number" }, "description": "Query embedding vector" },
                    "node_type": { "type": "string", "description": "Optional: filter results to this node type" },
                    "depth": { "type": "integer", "description": "Graph expansion depth from vector matches (default: 1)" },
                    "top_k": { "type": "integer", "description": "Max results (default: 10)" },
                    "text": { "type": "string", "description": "Optional text description for logging" }
                },
                "required": ["vector"]
            }
        }
    ])
}

/// Dispatch a tool call to the Prime facade or Recall engine.
pub async fn call_tool(
    prime: &Prime,
    recall: &RecallEngine,
    name: &str,
    args: &Value,
) -> Value {
    match name {
        "prime_add_node" => call_add_node(prime, args).await,
        "prime_add_edge" => call_add_edge(prime, args).await,
        "prime_neighbors" => call_neighbors(prime, args),
        "prime_search" => call_search(prime, args),
        "prime_shortest_path" => call_shortest_path(prime, args),
        "prime_forget" => call_forget(prime, args).await,
        "prime_history" => call_history(prime, args).await,
        "prime_stats" => call_stats(prime),
        "prime_index" => call_index(recall).await,
        "prime_context" => call_context(recall, args).await,
        "prime_embed" => call_embed(prime, args).await,
        "prime_similar" => call_similar(prime, args),
        "prime_recall" => call_recall(prime, args).await,
        _ => tool_error(&format!("Unknown tool: {name}")),
    }
}

#[allow(clippy::needless_pass_by_value)]
fn tool_result(content: Value) -> Value {
    json!({
        "content": [{
            "type": "text",
            "text": serde_json::to_string_pretty(&content).unwrap_or_default()
        }]
    })
}

fn tool_error(msg: &str) -> Value {
    json!({
        "isError": true,
        "content": [{
            "type": "text",
            "text": msg
        }]
    })
}

async fn call_add_node(prime: &Prime, args: &Value) -> Value {
    let node_type = args.get("type").and_then(Value::as_str).unwrap_or("unknown");
    let properties = args.get("properties").cloned().unwrap_or(json!({}));

    match prime.add_node(node_type, properties).await {
        Ok(id) => {
            let entity_id = allsource_core::prime::EntityId::node(node_type, id.as_str()).to_wire();
            tool_result(json!({ "node_id": id.as_str(), "entity_id": entity_id }))
        }
        Err(e) => tool_error(&e.to_string()),
    }
}

async fn call_add_edge(prime: &Prime, args: &Value) -> Value {
    let Some(source) = args.get("source").and_then(Value::as_str) else {
        return tool_error("missing 'source'");
    };
    let Some(target) = args.get("target").and_then(Value::as_str) else {
        return tool_error("missing 'target'");
    };
    let Some(relation) = args.get("relation").and_then(Value::as_str) else {
        return tool_error("missing 'relation'");
    };
    let properties = args.get("properties").cloned();
    let weight = args.get("weight").and_then(serde_json::Value::as_f64);

    let result = if let Some(w) = weight {
        prime.add_edge_weighted(source, target, relation, w, properties).await
    } else {
        prime.add_edge(source, target, relation, properties).await
    };

    match result {
        Ok(id) => tool_result(json!({ "edge_id": id.as_str() })),
        Err(e) => tool_error(&e.to_string()),
    }
}

fn call_neighbors(prime: &Prime, args: &Value) -> Value {
    let Some(node_id) = args.get("node_id").and_then(Value::as_str) else {
        return tool_error("missing 'node_id'");
    };
    let relation = args.get("relation").and_then(Value::as_str);
    let direction = match args.get("direction").and_then(Value::as_str) {
        Some("incoming") => allsource_core::prime::Direction::Incoming,
        Some("outgoing") => allsource_core::prime::Direction::Outgoing,
        _ => allsource_core::prime::Direction::Both,
    };
    let depth = args.get("depth").and_then(Value::as_u64).unwrap_or(1) as usize;

    if depth <= 1 {
        let nodes = prime.neighbors(node_id, relation, direction);
        let nodes_json: Vec<Value> = nodes
            .iter()
            .map(|n| json!({"id": n.id.as_str(), "type": n.node_type, "properties": n.properties}))
            .collect();
        tool_result(json!({ "nodes": nodes_json }))
    } else {
        let results = prime.neighbors_within(node_id, depth, relation, direction);
        let nodes_json: Vec<Value> = results
            .iter()
            .map(|(n, d)| json!({"id": n.id.as_str(), "type": n.node_type, "properties": n.properties, "depth": d}))
            .collect();
        tool_result(json!({ "nodes": nodes_json }))
    }
}

fn call_search(prime: &Prime, args: &Value) -> Value {
    let Some(node_type) = args.get("type").and_then(Value::as_str) else {
        return tool_error("missing 'type'");
    };
    let nodes = prime.nodes_by_type(node_type);
    let nodes_json: Vec<Value> = nodes
        .iter()
        .map(|n| json!({"id": n.id.as_str(), "type": n.node_type, "properties": n.properties}))
        .collect();
    tool_result(json!({ "nodes": nodes_json }))
}

fn call_shortest_path(prime: &Prime, args: &Value) -> Value {
    let Some(from) = args.get("from").and_then(Value::as_str) else {
        return tool_error("missing 'from'");
    };
    let Some(to) = args.get("to").and_then(Value::as_str) else {
        return tool_error("missing 'to'");
    };
    let relation = args.get("relation").and_then(Value::as_str);

    match prime.shortest_path(from, to, relation) {
        Some(path) => {
            let path_json: Vec<Value> = path
                .iter()
                .map(|n| json!({"id": n.id.as_str(), "type": n.node_type, "properties": n.properties}))
                .collect();
            tool_result(json!({ "path": path_json }))
        }
        None => tool_result(json!({ "path": null, "message": "No path found" })),
    }
}

async fn call_forget(prime: &Prime, args: &Value) -> Value {
    let Some(node_id) = args.get("node_id").and_then(Value::as_str) else {
        return tool_error("missing 'node_id'");
    };

    match prime.delete_node(node_id).await {
        Ok(()) => tool_result(json!({ "deleted": true })),
        Err(e) => tool_error(&e.to_string()),
    }
}

async fn call_history(prime: &Prime, args: &Value) -> Value {
    let Some(entity_id) = args.get("entity_id").and_then(Value::as_str) else {
        return tool_error("missing 'entity_id'");
    };

    match prime.history(entity_id).await {
        Ok(entries) => {
            let events_json: Vec<Value> = entries
                .iter()
                .map(|e| json!({
                    "type": e.event_type,
                    "timestamp": e.timestamp.to_rfc3339(),
                    "payload": e.payload,
                }))
                .collect();
            tool_result(json!({ "events": events_json }))
        }
        Err(e) => tool_error(&e.to_string()),
    }
}

fn call_stats(prime: &Prime) -> Value {
    let stats = prime.stats();
    tool_result(json!({
        "total_nodes": stats.total_nodes,
        "total_edges": stats.total_edges,
        "deleted_nodes": stats.deleted_nodes,
        "deleted_edges": stats.deleted_edges,
        "event_count": stats.event_count,
        "nodes_by_type": stats.nodes_by_type,
        "edges_by_relation": stats.edges_by_relation,
    }))
}

// =========================================================================
// Recall tools
// =========================================================================

async fn call_index(recall: &RecallEngine) -> Value {
    let index = recall.index().await;
    tool_result(json!({
        "index": index.markdown,
        "token_count": index.token_count,
        "domains": index.domains,
        "cross_references": index.cross_references,
        "last_updated": index.last_updated.to_rfc3339(),
    }))
}

async fn call_embed(prime: &Prime, args: &Value) -> Value {
    let Some(id) = args.get("id").and_then(Value::as_str) else {
        return tool_error("missing 'id'");
    };
    let text = args.get("text").and_then(Value::as_str);
    let Some(vector) = args.get("vector").and_then(|v| v.as_array()) else {
        return tool_error("missing 'vector'");
    };
    let vector: Vec<f32> = vector.iter().filter_map(|v| v.as_f64().map(|f| f as f32)).collect();
    let metadata = args.get("metadata").cloned();

    match prime.embed_with_metadata(id, text, vector, metadata).await {
        Ok(()) => tool_result(json!({ "stored": true, "id": id })),
        Err(e) => tool_error(&e.to_string()),
    }
}

fn call_similar(prime: &Prime, args: &Value) -> Value {
    let Some(id) = args.get("id").and_then(Value::as_str) else {
        return tool_error("missing 'id'");
    };
    let top_k = args
        .get("top_k")
        .and_then(Value::as_u64)
        .map_or(5, |v| v as usize);

    match prime.similar(id, top_k) {
        Ok(results) => {
            let results_json: Vec<Value> = results
                .iter()
                .map(|r| json!({ "id": r.id, "score": r.score, "text": r.text }))
                .collect();
            tool_result(json!({ "results": results_json }))
        }
        Err(e) => tool_error(&e.to_string()),
    }
}

async fn call_recall(prime: &Prime, args: &Value) -> Value {
    use allsource_core::prime::types::RecallQuery;

    let Some(vector) = args.get("vector").and_then(|v| v.as_array()) else {
        return tool_error("missing 'vector'");
    };
    let vector: Vec<f32> = vector.iter().filter_map(|v| v.as_f64().map(|f| f as f32)).collect();

    let query = RecallQuery {
        text: args.get("text").and_then(Value::as_str).map(String::from),
        vector: Some(vector),
        node_type: args.get("node_type").and_then(Value::as_str).map(String::from),
        depth: args.get("depth").and_then(Value::as_u64).map_or(1, |v| v as usize),
        top_k: args.get("top_k").and_then(Value::as_u64).map_or(10, |v| v as usize),
        ..RecallQuery::default()
    };

    match prime.recall(query).await {
        Ok(result) => {
            let nodes_json: Vec<Value> = result.nodes.iter().map(|sn| json!({
                "id": sn.node.id.as_str(),
                "type": sn.node.node_type,
                "properties": sn.node.properties,
                "score": sn.score,
                "depth": sn.depth,
            })).collect();
            let vectors_json: Vec<Value> = result.vectors.iter().map(|v| json!({
                "id": v.id,
                "score": v.score,
                "text": v.text,
            })).collect();

            tool_result(json!({
                "nodes": nodes_json,
                "vectors": vectors_json,
                "edges": result.edges.len(),
            }))
        }
        Err(e) => tool_error(&e.to_string()),
    }
}

async fn call_context(recall: &RecallEngine, args: &Value) -> Value {
    use allsource_core::prime::recall::RecallContextQuery;

    let Some(query) = args.get("query").and_then(Value::as_str) else {
        return tool_error("missing 'query'");
    };
    let query = query.to_string();

    let top_k = args
        .get("top_k")
        .and_then(Value::as_u64)
        .map_or(5, |v| usize::try_from(v).unwrap_or(5));
    let max_tokens = args
        .get("max_tokens")
        .and_then(Value::as_u64)
        .and_then(|v| usize::try_from(v).ok());

    let ctx_query = RecallContextQuery {
        query,
        agent_id: args.get("agent_id").and_then(Value::as_str).map(String::from),
        top_k,
        as_of: None,
        include_index: args
            .get("include_index")
            .and_then(Value::as_bool)
            .unwrap_or(true),
        max_tokens,
    };

    let ctx = recall.context(ctx_query).await;

    tool_result(json!({
        "index": ctx.index,
        "vectors": ctx.vectors,
        "nodes": ctx.nodes,
        "edges": ctx.edges,
        "token_count": ctx.token_count,
    }))
}
