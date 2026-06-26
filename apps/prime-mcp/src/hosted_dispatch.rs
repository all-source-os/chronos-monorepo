//! Tenant-scoped MCP dispatch over a stateless [`HostedPrime`].
//!
//! This is a SEPARATE dispatch path from [`crate::dispatch`]: the embedded
//! stdio/HTTP path stays untouched. When the hosted `/mcp` handler has a
//! [`HostedPrime`] engine and a trusted tenant id, it routes `tools/call` here
//! so every operation is threaded through the tenant and executed against the
//! remote Core — rather than the local single-store embedded `Prime`.
//!
//! Only the subset of tools [`HostedPrime`] implements is supported; the rest
//! return a clear "not yet available on the hosted backend" tool-error so a
//! caller never silently gets a wrong answer or a crash. `initialize` and
//! `tools/list` reuse the shared `protocol::server_info` + `tools::tool_definitions`
//! so the advertised surface matches the embedded path exactly.

use allsource_core::prime::{Direction, hosted::HostedPrime};
use serde_json::{Value, json};

use crate::{
    protocol::{self, Request, Response},
    tools,
};

/// Dispatch one JSON-RPC request against a tenant-scoped [`HostedPrime`].
///
/// Returns `None` for notifications (no reply), `Some(Response)` otherwise —
/// the same contract as [`crate::dispatch::handle_request`], so the HTTP
/// transport shapes both identically (200 JSON / 202 notification).
pub async fn handle_request_hosted(
    hosted: &HostedPrime,
    tenant: &str,
    req: &Request,
) -> Option<Response> {
    match req.method.as_str() {
        // Auto-inject is a stdio system-prompt feature; the hosted HTTP path
        // never enables it, matching the embedded `/mcp` handler's `false`.
        "initialize" => Some(Response::success(
            req.id.clone(),
            protocol::server_info(false),
        )),
        "notifications/initialized" => None,

        "tools/list" => {
            let defs = tools::tool_definitions();
            Some(Response::success(req.id.clone(), json!({ "tools": defs })))
        }

        "tools/call" => {
            let params = req.params.as_ref();
            let tool_name = params
                .and_then(|p| p.get("name"))
                .and_then(Value::as_str)
                .unwrap_or("");
            let args = params
                .and_then(|p| p.get("arguments"))
                .cloned()
                .unwrap_or_else(|| json!({}));

            let result = call_tool_hosted(hosted, tenant, tool_name, &args).await;
            Some(Response::success(req.id.clone(), result))
        }

        _ => Some(Response::error(
            req.id.clone(),
            -32601,
            format!("Method not found: {}", req.method),
        )),
    }
}

/// Dispatch a single tool call to the matching [`HostedPrime`] method, threading
/// `tenant`. Tools the hosted engine does not yet implement return a tool-error
/// rather than panicking or silently no-op'ing.
async fn call_tool_hosted(hosted: &HostedPrime, tenant: &str, name: &str, args: &Value) -> Value {
    match name {
        "prime_add_node" => call_add_node(hosted, tenant, args).await,
        "prime_add_edge" => call_add_edge(hosted, tenant, args).await,
        "prime_neighbors" => call_neighbors(hosted, tenant, args).await,
        "prime_search" => call_search(hosted, tenant, args).await,
        "prime_stats" => call_stats(hosted, tenant).await,
        "prime_recall" => call_recall(hosted, tenant, args).await,
        "prime_embed" => call_embed(hosted, tenant, args).await,
        "prime_forget" => call_forget(hosted, tenant, args).await,
        "prime_history" => call_history(hosted, tenant, args).await,

        // Tools the embedded path serves but HostedPrime does not implement yet.
        // Return a clear, non-fatal error so an agent learns the capability is
        // unavailable on this backend rather than getting a wrong/empty answer.
        "prime_index"
        | "prime_context"
        | "prime_similar"
        | "prime_shortest_path"
        | "prime_list_templates"
        | "prime_load_template"
        | "prime_define_projection"
        | "prime_list_projections"
        | "prime_project_node"
        | "prime_node_provenance" => tool_error(&format!(
            "tool {name} not yet available on the hosted backend"
        )),

        // Hound is local-first: ingest runs where the source lives, so the whole
        // code-graph toolset is served on the embedded stdio path, not here.
        "hound_ingest" | "hound_impact" | "hound_report" => tool_error(
            "Hound runs in local mode where your source lives, not on the hosted backend. \
             Run prime-mcp locally (stdio) or `allsource-prime --mode hound <path>` to build \
             a code graph.",
        ),

        _ => tool_error(&format!("Unknown tool: {name}")),
    }
}

// ── Result envelope helpers ──────────────────────────────────────────────
//
// Minimal duplicates of `tools.rs`'s envelope helpers. We do NOT reuse
// `tools::tool_result` because that applies the global TOON/JSON format
// switch; the hosted HTTP transport always emits plain JSON. Keeping these
// local also keeps this dispatch fully independent of `tools::call_tool`.

#[allow(clippy::needless_pass_by_value)]
fn tool_result(content: Value) -> Value {
    let text = serde_json::to_string_pretty(&content).unwrap_or_default();
    json!({
        "content": [{ "type": "text", "text": text }]
    })
}

fn tool_error(msg: &str) -> Value {
    json!({
        "isError": true,
        "content": [{ "type": "text", "text": msg }]
    })
}

/// Shape a [`Node`](allsource_core::prime::Node) the same way the embedded
/// tools do (`{id, type, properties}`), so hosted results are wire-identical.
fn node_json(n: &allsource_core::prime::Node) -> Value {
    json!({ "id": n.id.as_str(), "type": n.node_type, "properties": n.properties })
}

// ── Tool implementations (mirroring tools.rs arg parsing + result shaping) ──

async fn call_add_node(hosted: &HostedPrime, tenant: &str, args: &Value) -> Value {
    let node_type = args
        .get("type")
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    let properties = args.get("properties").cloned().unwrap_or_else(|| json!({}));

    match hosted.add_node(tenant, node_type, properties).await {
        Ok(id) => {
            let entity_id = allsource_core::prime::EntityId::node(node_type, id.as_str()).to_wire();
            tool_result(json!({ "node_id": id.as_str(), "entity_id": entity_id }))
        }
        Err(e) => tool_error(&e.to_string()),
    }
}

async fn call_add_edge(hosted: &HostedPrime, tenant: &str, args: &Value) -> Value {
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
    let weight = args.get("weight").and_then(Value::as_f64);

    let result = if let Some(w) = weight {
        hosted
            .add_edge_weighted(tenant, source, target, relation, w, properties)
            .await
    } else {
        hosted
            .add_edge(tenant, source, target, relation, properties)
            .await
    };

    match result {
        Ok(id) => tool_result(json!({ "edge_id": id.as_str() })),
        Err(e) => tool_error(&e.to_string()),
    }
}

async fn call_neighbors(hosted: &HostedPrime, tenant: &str, args: &Value) -> Value {
    let Some(node_id) = args.get("node_id").and_then(Value::as_str) else {
        return tool_error("missing 'node_id'");
    };
    let relation = args.get("relation").and_then(Value::as_str);
    let direction = match args.get("direction").and_then(Value::as_str) {
        Some("incoming") => Direction::Incoming,
        Some("outgoing") => Direction::Outgoing,
        _ => Direction::Both,
    };

    // HostedPrime serves single-hop neighbors; depth>1 is part of the embedded
    // graph traversal and is not available on the hosted backend yet.
    match hosted.neighbors(tenant, node_id, relation, direction).await {
        Ok(nodes) => {
            let nodes_json: Vec<Value> = nodes.iter().map(node_json).collect();
            tool_result(json!({ "nodes": nodes_json }))
        }
        Err(e) => tool_error(&e.to_string()),
    }
}

async fn call_search(hosted: &HostedPrime, tenant: &str, args: &Value) -> Value {
    let Some(node_type) = args.get("type").and_then(Value::as_str) else {
        return tool_error("missing 'type'");
    };
    match hosted.search(tenant, node_type).await {
        Ok(nodes) => {
            let nodes_json: Vec<Value> = nodes.iter().map(node_json).collect();
            tool_result(json!({ "nodes": nodes_json }))
        }
        Err(e) => tool_error(&e.to_string()),
    }
}

async fn call_stats(hosted: &HostedPrime, tenant: &str) -> Value {
    match hosted.stats(tenant).await {
        Ok(stats) => tool_result(json!({
            "total_nodes": stats.total_nodes,
            "total_edges": stats.total_edges,
            "deleted_nodes": stats.deleted_nodes,
            "deleted_edges": stats.deleted_edges,
            "event_count": stats.event_count,
            "nodes_by_type": stats.nodes_by_type,
            "edges_by_relation": stats.edges_by_relation,
        })),
        Err(e) => tool_error(&e.to_string()),
    }
}

async fn call_recall(hosted: &HostedPrime, tenant: &str, args: &Value) -> Value {
    // The hosted recall takes a precomputed query vector. The in-process
    // text→vector embedder (`embed_text`) downloads a model on first use and is
    // never exercised on the hosted path here, so we require an explicit vector.
    let Some(vector) = args.get("vector").and_then(|v| v.as_array()) else {
        return tool_error(
            "missing 'vector' — the hosted backend requires a precomputed query embedding",
        );
    };
    let query_vector: Vec<f32> = vector
        .iter()
        .filter_map(|v| v.as_f64().map(|f| f as f32))
        .collect();
    let top_k = args
        .get("top_k")
        .and_then(Value::as_u64)
        .map_or(10, |v| v as usize);

    match hosted.recall(tenant, query_vector, top_k).await {
        Ok(results) => {
            let nodes_json: Vec<Value> = results
                .iter()
                .map(|(n, score)| {
                    json!({
                        "id": n.id.as_str(),
                        "type": n.node_type,
                        "properties": n.properties,
                        "score": score,
                    })
                })
                .collect();
            tool_result(json!({ "nodes": nodes_json }))
        }
        Err(e) => tool_error(&e.to_string()),
    }
}

async fn call_embed(hosted: &HostedPrime, tenant: &str, args: &Value) -> Value {
    let Some(id) = args.get("id").and_then(Value::as_str) else {
        return tool_error("missing 'id'");
    };
    // As with recall, the hosted path takes a precomputed vector to avoid the
    // first-use model download. Text-only embedding is a follow-up.
    let Some(arr) = args.get("vector").and_then(|v| v.as_array()) else {
        return tool_error(
            "missing 'vector' — the hosted backend requires a precomputed embedding vector",
        );
    };
    let vector: Vec<f32> = arr
        .iter()
        .filter_map(|v| v.as_f64().map(|f| f as f32))
        .collect();
    let metadata = args.get("metadata").cloned();

    match hosted.embed(tenant, id, vector, metadata).await {
        Ok(()) => tool_result(json!({ "stored": true, "id": id })),
        Err(e) => tool_error(&e.to_string()),
    }
}

async fn call_forget(hosted: &HostedPrime, tenant: &str, args: &Value) -> Value {
    let Some(node_id) = args.get("node_id").and_then(Value::as_str) else {
        return tool_error("missing 'node_id'");
    };
    match hosted.delete_node(tenant, node_id).await {
        Ok(()) => tool_result(json!({ "deleted": true })),
        Err(e) => tool_error(&e.to_string()),
    }
}

async fn call_history(hosted: &HostedPrime, tenant: &str, args: &Value) -> Value {
    let Some(entity_id) = args.get("entity_id").and_then(Value::as_str) else {
        return tool_error("missing 'entity_id'");
    };
    match hosted.history(tenant, entity_id).await {
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use wiremock::{
        Mock, MockServer, ResponseTemplate,
        matchers::{method, path},
    };

    async fn mount_empty_core(server: &MockServer) {
        Mock::given(method("GET"))
            .and(path("/api/v1/events/query"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "events": [] })))
            .mount(server)
            .await;
        Mock::given(method("POST"))
            .and(path("/api/v1/events"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "ok": true })))
            .mount(server)
            .await;
    }

    #[allow(clippy::needless_pass_by_value)] // test helper; clarity over &Value
    fn req(id: i64, method: &str, params: Value) -> Request {
        // Build a Request via JSON so we don't depend on field visibility.
        serde_json::from_value(json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params,
        }))
        .unwrap()
    }

    #[tokio::test]
    async fn initialize_returns_server_info() {
        let server = MockServer::start().await;
        let hosted = HostedPrime::connect(server.uri(), None, 8, Duration::from_secs(60));
        let r = req(1, "initialize", json!({}));
        let resp = handle_request_hosted(&hosted, "tenant-a", &r)
            .await
            .unwrap();
        let v = serde_json::to_value(&resp).unwrap();
        assert_eq!(v["result"]["serverInfo"]["name"], "allsource-prime");
    }

    #[tokio::test]
    async fn notifications_initialized_has_no_reply() {
        let server = MockServer::start().await;
        let hosted = HostedPrime::connect(server.uri(), None, 8, Duration::from_secs(60));
        let r = req(0, "notifications/initialized", json!({}));
        assert!(
            handle_request_hosted(&hosted, "tenant-a", &r)
                .await
                .is_none()
        );
    }

    #[tokio::test]
    async fn add_node_then_search_round_trips_with_tenant() {
        let server = MockServer::start().await;
        mount_empty_core(&server).await;
        let hosted = HostedPrime::connect(server.uri(), None, 8, Duration::from_secs(60));

        let add = req(
            1,
            "tools/call",
            json!({ "name": "prime_add_node", "arguments": { "type": "contact", "properties": { "name": "Alice" } } }),
        );
        let resp = handle_request_hosted(&hosted, "tenant-a", &add)
            .await
            .unwrap();
        let v = serde_json::to_value(&resp).unwrap();
        assert_ne!(
            v["result"]["isError"],
            json!(true),
            "add_node should succeed"
        );

        let search = req(
            2,
            "tools/call",
            json!({ "name": "prime_search", "arguments": { "type": "contact" } }),
        );
        let resp = handle_request_hosted(&hosted, "tenant-a", &search)
            .await
            .unwrap();
        let v = serde_json::to_value(&resp).unwrap();
        let text = v["result"]["content"][0]["text"].as_str().unwrap();
        assert!(
            text.contains("Alice"),
            "search should return the added node; got {text}"
        );
    }

    #[tokio::test]
    async fn unsupported_tool_errors_without_crashing() {
        let server = MockServer::start().await;
        let hosted = HostedPrime::connect(server.uri(), None, 8, Duration::from_secs(60));
        let r = req(
            1,
            "tools/call",
            json!({ "name": "prime_index", "arguments": {} }),
        );
        let resp = handle_request_hosted(&hosted, "tenant-a", &r)
            .await
            .unwrap();
        let v = serde_json::to_value(&resp).unwrap();
        assert_eq!(v["result"]["isError"], json!(true));
        let text = v["result"]["content"][0]["text"].as_str().unwrap();
        assert!(text.contains("not yet available on the hosted backend"));
    }
}
