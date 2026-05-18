//! MCP tool definitions and dispatch for Prime.
//!
//! Each tool maps to a Prime facade method. Tool descriptions are written for
//! AI agent consumption — they explain *when* to use each tool.

use std::sync::OnceLock;

use allsource_core::prime::{Prime, recall::RecallEngine};
use serde_json::{Value, json};

/// Encoding for the text payload of MCP tool results.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResultFormat {
    /// Pretty-printed JSON.
    Json,
    /// TOON — compact, token-efficient (see the `toon` module).
    Toon,
}

static RESULT_FORMAT: OnceLock<ResultFormat> = OnceLock::new();

/// Set the tool-result payload format. Called once at startup; later calls
/// are ignored. Defaults to [`ResultFormat::Json`] if never set.
pub fn set_result_format(fmt: ResultFormat) {
    let _ = RESULT_FORMAT.set(fmt);
}

fn result_format() -> ResultFormat {
    RESULT_FORMAT.get().copied().unwrap_or(ResultFormat::Json)
}

/// Return MCP tool definitions (for `tools/list`).
pub fn tool_definitions() -> Value {
    json!([
        // ─── Graph CRUD ─────────────────────────────────────────────
        {
            "name": "prime_add_node",
            "description": "Create a node in the knowledge graph. Use whenever you learn a new fact, meet a new entity, or discover a concept. Always include a 'domain' property to enable cross-domain reasoning. Pair with prime_embed to make the node searchable by meaning, and prime_add_edge to connect it to existing knowledge.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "type": { "type": "string", "description": "Node type: 'person', 'concept', 'project', 'metric', 'decision', 'event', 'insight'" },
                    "properties": { "type": "object", "description": "Node data. Always include 'name' and 'domain'. Example: {\"name\": \"Alice\", \"role\": \"engineer\", \"domain\": \"engineering\"}" }
                },
                "required": ["type", "properties"]
            }
        },
        {
            "name": "prime_add_edge",
            "description": "Create a directed relationship between two nodes. Use when you discover how entities connect: causes, depends_on, works_on, impacts, requires. Cross-domain edges (connecting nodes in different domains) are especially valuable — they power the compressed index's cross-reference section.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "source": { "type": "string", "description": "Source node entity_id (format: node:{type}:{id})" },
                    "target": { "type": "string", "description": "Target node entity_id" },
                    "relation": { "type": "string", "description": "Relationship type: 'works_on', 'impacts', 'requires', 'depends_on', 'causes', 'authored', 'manages'" },
                    "properties": { "type": "object", "description": "Optional edge properties (e.g. {\"since\": \"2026-01\"})" },
                    "weight": { "type": "number", "description": "Optional confidence/strength (0.0-1.0)" }
                },
                "required": ["source", "target", "relation"]
            }
        },
        // ─── Graph Queries ───────────────────────────────────────────
        {
            "name": "prime_neighbors",
            "description": "Explore the graph around a node. Use to find what's connected to an entity: who works on a project, what a person manages, what depends on a service. Set depth > 1 for multi-hop exploration (e.g. 'who are the teammates of Alice's manager?'). Prefer this over prime_search when you already know a starting node.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "node_id": { "type": "string", "description": "Starting node entity_id" },
                    "relation": { "type": "string", "description": "Filter to edges of this type only" },
                    "direction": { "type": "string", "enum": ["incoming", "outgoing", "both"], "description": "Edge direction (default: both)" },
                    "depth": { "type": "integer", "description": "BFS depth: 1 = immediate neighbors, 2+ = multi-hop (default: 1)" }
                },
                "required": ["node_id"]
            }
        },
        {
            "name": "prime_search",
            "description": "Find all nodes of a given type. Use for broad queries like 'list all projects' or 'show me every person'. For semantic queries ('find things related to X'), use prime_recall instead.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "type": { "type": "string", "description": "Node type to search for (e.g. 'person', 'project')" }
                },
                "required": ["type"]
            }
        },
        {
            "name": "prime_shortest_path",
            "description": "Find how two entities are connected through the graph. Returns the chain of nodes linking them. Use to answer 'how does X relate to Y?' when you need the specific path of relationships.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "from": { "type": "string", "description": "Start node entity_id" },
                    "to": { "type": "string", "description": "End node entity_id" },
                    "relation": { "type": "string", "description": "Restrict path to this relation type only" }
                },
                "required": ["from", "to"]
            }
        },
        // ─── Memory Lifecycle ────────────────────────────────────────
        {
            "name": "prime_forget",
            "description": "Soft-delete a node and all its edges. The node becomes invisible to queries but its full history is preserved — use prime_history to see what was forgotten and why. Use when knowledge is outdated or incorrect. Prefer updating over forgetting when the entity still exists but facts changed.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "node_id": { "type": "string", "description": "Node entity_id to forget" }
                },
                "required": ["node_id"]
            }
        },
        {
            "name": "prime_history",
            "description": "Get the complete audit trail for any entity: every creation, update, and deletion with timestamps. Use to answer 'when did I learn this?', 'what changed?', or 'who was responsible before?' Returns events in chronological order.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "entity_id": { "type": "string", "description": "Any entity_id: node, edge, or vector" }
                },
                "required": ["entity_id"]
            }
        },
        {
            "name": "prime_stats",
            "description": "Quick overview of memory state: total nodes, edges, types, relations. Call this at conversation start to orient yourself. Low cost, no parameters needed.",
            "inputSchema": {
                "type": "object",
                "properties": {}
            }
        },
        // ─── Compressed Index & Recall ───────────────────────────────
        {
            "name": "prime_index",
            "description": "Get your compressed knowledge index — a token-efficient markdown summary of everything you know, organized by domain with cross-references. Call this FIRST at the start of every conversation to orient yourself. The index shows: which domains exist, how many facts per domain, and which domains are connected. Use it as navigational scaffolding before searching for specifics.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "agent_id": { "type": "string", "description": "Scope to a specific agent's knowledge" }
                }
            }
        },
        {
            "name": "prime_context",
            "description": "Combined retrieval with tiered depth control. L0: stats only (~100 tokens, use for orientation). L1: recent conversation context (~500-1500 tokens, use for follow-ups in same conversation). L2: full hybrid recall with compressed index + vectors + graph expansion (~2000-5000 tokens, use for cross-domain questions or new topics). Default: L2.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "query": { "type": "string", "description": "Natural language question" },
                    "agent_id": { "type": "string", "description": "Scope to a specific agent" },
                    "top_k": { "type": "integer", "description": "Max vector results (default: 5, L2 only)" },
                    "include_index": { "type": "boolean", "description": "Prepend compressed index excerpt (default: true, L2 only)" },
                    "max_tokens": { "type": "integer", "description": "Cap total response tokens" },
                    "tier": { "type": "string", "enum": ["L0", "L1", "L2"], "description": "Retrieval depth. L0=stats only. L1=recent conversation context. L2=full hybrid recall. Default: L2." },
                    "conversation_id": { "type": "string", "description": "Scope L1 retrieval to this conversation's nodes. Ignored for L0/L2." }
                },
                "required": ["query"]
            }
        },
        // ─── Vector Operations ───────────────────────────────────────
        {
            "name": "prime_embed",
            "description": "Store a vector embedding for semantic search. Always pair with prime_add_node — create the node first, then embed it using the node's entity_id. This makes the node findable by meaning, not just by type or graph position. Without an embedding, prime_recall won't find the node.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "id": { "type": "string", "description": "Entity_id of the node to embed (use the entity_id returned by prime_add_node)" },
                    "text": { "type": "string", "description": "Source text that was embedded (stored for display in search results)" },
                    "vector": { "type": "array", "items": { "type": "number" }, "description": "Embedding vector (float array from your embedding model)" },
                    "metadata": { "type": "object", "description": "Optional: tags, source URL, confidence score" }
                },
                "required": ["id", "vector"]
            }
        },
        {
            "name": "prime_similar",
            "description": "Find the most similar embeddings to a stored vector. Use to discover related knowledge: 'what else do I know that's similar to this?' Requires the target to have been embedded with prime_embed first.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "id": { "type": "string", "description": "Entity_id of the embedded node to find similar items for" },
                    "top_k": { "type": "integer", "description": "Number of results (default: 5)" }
                },
                "required": ["id"]
            }
        },
        {
            "name": "prime_recall",
            "description": "Hybrid recall: vectors + graph + temporal recency. Your PRIMARY tool for 'what do I know about X?' questions. Finds semantically similar facts via embedding, then expands through graph connections to discover related context. Set depth=0 for vector-only, depth=1+ to include graph neighbors. For cross-domain questions, use prime_context instead (it adds the compressed index).",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "vector": { "type": "array", "items": { "type": "number" }, "description": "Query embedding vector" },
                    "node_type": { "type": "string", "description": "Filter to this node type only" },
                    "depth": { "type": "integer", "description": "Graph expansion hops from vector matches (0=vector-only, 1+=include neighbors, default: 1)" },
                    "top_k": { "type": "integer", "description": "Max results (default: 10)" },
                    "text": { "type": "string", "description": "Query text for logging/debugging" }
                },
                "required": ["vector"]
            }
        }
    ])
}

/// Dispatch a tool call to the Prime facade or Recall engine.
pub async fn call_tool(prime: &Prime, recall: &RecallEngine, name: &str, args: &Value) -> Value {
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
    let text = match result_format() {
        // A leading `format: toon` line tells the consuming model how to read
        // the payload; it is itself a valid TOON field, so the result stays
        // well-formed TOON end to end.
        ResultFormat::Toon => format!("format: toon\n{}", crate::toon::encode(&content)),
        ResultFormat::Json => serde_json::to_string_pretty(&content).unwrap_or_default(),
    };
    json!({
        "content": [{
            "type": "text",
            "text": text
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
    let node_type = args
        .get("type")
        .and_then(Value::as_str)
        .unwrap_or("unknown");
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
        prime
            .add_edge_weighted(source, target, relation, w, properties)
            .await
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
                .map(|e| {
                    json!({
                        "type": e.event_type,
                        "timestamp": e.timestamp.to_rfc3339(),
                        "payload": e.payload,
                    })
                })
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
    let vector: Vec<f32> = vector
        .iter()
        .filter_map(|v| v.as_f64().map(|f| f as f32))
        .collect();
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
    let vector: Vec<f32> = vector
        .iter()
        .filter_map(|v| v.as_f64().map(|f| f as f32))
        .collect();

    let query = RecallQuery {
        text: args.get("text").and_then(Value::as_str).map(String::from),
        vector: Some(vector),
        node_type: args
            .get("node_type")
            .and_then(Value::as_str)
            .map(String::from),
        depth: args
            .get("depth")
            .and_then(Value::as_u64)
            .map_or(1, |v| v as usize),
        top_k: args
            .get("top_k")
            .and_then(Value::as_u64)
            .map_or(10, |v| v as usize),
        ..RecallQuery::default()
    };

    match prime.recall(query).await {
        Ok(result) => {
            let nodes_json: Vec<Value> = result
                .nodes
                .iter()
                .map(|sn| {
                    json!({
                        "id": sn.node.id.as_str(),
                        "type": sn.node.node_type,
                        "properties": sn.node.properties,
                        "score": sn.score,
                        "depth": sn.depth,
                    })
                })
                .collect();
            let vectors_json: Vec<Value> = result
                .vectors
                .iter()
                .map(|v| {
                    json!({
                        "id": v.id,
                        "score": v.score,
                        "text": v.text,
                    })
                })
                .collect();

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
    use allsource_core::prime::recall::{ContextTier, RecallContextQuery};

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

    let tier = match args.get("tier").and_then(Value::as_str) {
        Some("L0") => ContextTier::L0,
        Some("L1") => ContextTier::L1,
        Some("L2") | None => ContextTier::L2,
        Some(other) => return tool_error(&format!("invalid tier: {other}. Use L0, L1, or L2")),
    };

    let ctx_query = RecallContextQuery {
        query,
        agent_id: args
            .get("agent_id")
            .and_then(Value::as_str)
            .map(String::from),
        top_k,
        as_of: None,
        include_index: args
            .get("include_index")
            .and_then(Value::as_bool)
            .unwrap_or(true),
        max_tokens,
        tier,
        conversation_id: args
            .get("conversation_id")
            .and_then(Value::as_str)
            .map(String::from),
    };

    let ctx = recall.context(ctx_query).await;

    tool_result(json!({
        "tier": format!("{:?}", ctx.tier),
        "index": ctx.index,
        "vectors": ctx.vectors,
        "nodes": ctx.nodes,
        "edges": ctx.edges,
        "stats": ctx.stats,
        "token_count": ctx.token_count,
    }))
}
