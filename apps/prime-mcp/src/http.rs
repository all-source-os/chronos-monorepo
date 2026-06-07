//! HTTP REST API server for Prime (axum-based).
//!
//! Exposes the full Prime API as REST endpoints, consistent with Core's HTTP stack.
//! Activated via `--mode http --port 3905`.

use std::sync::Arc;

use allsource_core::prime::hosted::HostedPrime;
use allsource_core::prime::{Direction, Prime, recall::RecallEngine};
use axum::{
    Json, Router,
    body::Bytes,
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::{Html, IntoResponse},
    routing::{delete, get, patch, post},
};
use serde::Deserialize;
use serde_json::{Value, json};
use tower_http::trace::TraceLayer;

/// Shared state for HTTP handlers.
pub struct AppState {
    pub prime: Arc<Prime>,
    /// Recall engine — used by the `/recall` endpoint and kept for future index endpoints.
    #[allow(dead_code)]
    pub recall: RecallEngine,
    /// Stateless, tenant-scoped engine over a remote Core. Present only in the
    /// hosted deployment (constructed when `CORE_URL` is set). When `Some` and
    /// the `/mcp` request carries a trusted `X-Tenant-Id`, the MCP dispatch is
    /// routed through this instead of the embedded `prime` — see [`mcp_handler`].
    pub hosted: Option<Arc<HostedPrime>>,
}

/// Start the HTTP server on the given port.
pub async fn serve(state: Arc<AppState>, port: u16) -> anyhow::Result<()> {
    let app = Router::new()
        // Node endpoints
        .route("/api/v1/prime/nodes", post(create_node))
        .route("/api/v1/prime/nodes/{id}", get(get_node))
        .route("/api/v1/prime/nodes/{id}", patch(update_node))
        .route("/api/v1/prime/nodes/{id}", delete(delete_node))
        .route("/api/v1/prime/nodes/{id}/neighbors", get(get_neighbors))
        .route("/api/v1/prime/nodes/{id}/subgraph", get(get_subgraph))
        .route("/api/v1/prime/nodes/{id}/history", get(get_history))
        // Edge endpoints
        .route("/api/v1/prime/edges", post(create_edge))
        .route("/api/v1/prime/edges/{id}", delete(delete_edge))
        // Vector endpoints
        .route("/api/v1/prime/vectors", post(store_vector))
        .route("/api/v1/prime/vectors/search", post(search_vectors))
        .route("/api/v1/prime/vectors/{id}", delete(delete_vector))
        // Query endpoints
        .route("/api/v1/prime/shortest-path", post(shortest_path))
        .route("/api/v1/prime/recall", post(recall))
        .route("/api/v1/prime/diff", get(get_diff))
        // Status endpoints
        .route("/api/v1/prime/stats", get(get_stats))
        .route("/api/v1/prime/graph", get(get_full_graph))
        // Self-contained local graph viewer (single HTML page, no CDN, offline).
        // Fetches /api/v1/prime/graph from the same origin and renders the
        // local store as a bubble graph + detail list.
        .route("/api/v1/prime/graph.html", get(graph_viewer))
        // MCP-over-HTTP (Streamable HTTP transport) — lets MCP clients connect
        // to the hosted Prime with no local binary. POST a JSON-RPC request,
        // get a JSON-RPC response (or 202 for notifications).
        .route("/mcp", post(mcp_handler))
        .route("/health", get(health))
        .merge(crate::profiling::routes())
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    // Bind to the IPv6 wildcard so Fly's internal network (which resolves
    // .internal names to IPv6) can reach us. `[::]` accepts both IPv4 and
    // IPv6 connections on Linux (dual-stack), matching what Core does via
    // the ALLSOURCE_HOST=":: " env var. A plain `0.0.0.0` bind is IPv4-only
    // and caused "connection refused" on Fly's private network even though
    // the public hostname worked via Fly's edge proxy.
    let addr = format!("[::]:{port}");
    tracing::info!("HTTP server listening on {addr}");
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

// =============================================================================
// MCP-over-HTTP (Streamable HTTP transport)
// =============================================================================

/// Optional bearer/API-key gate for the `/mcp` endpoint.
///
/// If `PRIME_API_KEY` is set (and non-empty), the request must carry a matching
/// key in `Authorization: Bearer <key>` (or a bare `Authorization`/`X-API-Key`).
/// If the env var is unset, the endpoint is open — matching the existing REST
/// handlers' behavior. Full per-tenant key validation through the gateway is a
/// follow-up (see bead t-dbee53 notes).
fn mcp_authorized(headers: &HeaderMap) -> bool {
    let expected = match std::env::var("PRIME_API_KEY") {
        Ok(k) if !k.is_empty() => k,
        _ => return true, // no key configured → open
    };
    let provided = headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.strip_prefix("Bearer ").unwrap_or(s))
        .or_else(|| headers.get("x-api-key").and_then(|v| v.to_str().ok()));
    provided == Some(expected.as_str())
}

/// Extract the trusted tenant id from the `X-Tenant-Id` header (case-insensitive
/// — `HeaderMap` lookups are already case-insensitive). Returns `None` when the
/// header is absent or empty.
///
/// TRUST MODEL (this slice): the Control-Plane → prime hop is internal, so we
/// trust the tenant id the gateway stamps in this header. A shared-secret gate
/// — reusing the existing `PRIME_API_KEY` bearer via [`mcp_authorized`] so only
/// the gateway can set the tenant — is the follow-up hardening, not done here.
fn tenant_from_headers(headers: &HeaderMap) -> Option<String> {
    headers
        .get("x-tenant-id")
        .and_then(|v| v.to_str().ok())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(String::from)
}

/// MCP Streamable-HTTP endpoint. Accepts a single JSON-RPC request and returns
/// the JSON-RPC response as `application/json`, or `202 Accepted` for
/// notifications (which have no reply).
///
/// Two dispatch paths share the exact same request/response shaping:
/// - **Hosted, tenant-scoped:** when a [`HostedPrime`] engine is configured AND
///   the request carries a trusted `X-Tenant-Id`, the call is routed through
///   [`crate::hosted_dispatch::handle_request_hosted`] against the remote Core,
///   isolated per tenant.
/// - **Embedded (default/fallback):** otherwise, dispatch goes to the local
///   single-store [`Prime`] via [`crate::dispatch::handle_request`] — identical
///   to the stdio transport, so neither path can drift in methods/tools.
async fn mcp_handler(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    body: Bytes,
) -> impl IntoResponse {
    if !mcp_authorized(&headers) {
        return (
            StatusCode::UNAUTHORIZED,
            Json(json!({ "error": "invalid or missing API key" })),
        )
            .into_response();
    }

    let req: crate::protocol::Request = match serde_json::from_slice(&body) {
        Ok(r) => r,
        Err(e) => {
            // JSON-RPC parse error: reply 200 with an error envelope (null id).
            let resp = crate::protocol::Response::error(None, -32700, format!("Parse error: {e}"));
            return (StatusCode::OK, Json(resp)).into_response();
        }
    };

    // Hosted, tenant-scoped path: only when both a HostedPrime engine and a
    // trusted tenant id are present. Falls back to the embedded path otherwise.
    if let (Some(hosted), Some(tenant)) = (state.hosted.as_ref(), tenant_from_headers(&headers)) {
        return match crate::hosted_dispatch::handle_request_hosted(hosted, &tenant, &req).await {
            Some(resp) => (StatusCode::OK, Json(resp)).into_response(),
            None => StatusCode::ACCEPTED.into_response(),
        };
    }

    // HTTP transport runs without auto-inject (that's a stdio system-prompt
    // feature); pass the same defaults the stdio binary uses when disabled.
    match crate::dispatch::handle_request(&state.prime, &state.recall, false, 1000, &req).await {
        Some(resp) => (StatusCode::OK, Json(resp)).into_response(),
        None => StatusCode::ACCEPTED.into_response(),
    }
}

// =============================================================================
// Request/Response types
// =============================================================================

#[derive(Deserialize)]
struct CreateNodeRequest {
    #[serde(rename = "type")]
    node_type: String,
    properties: Value,
}

#[derive(Deserialize)]
struct UpdateNodeRequest {
    properties: Value,
}

#[derive(Deserialize)]
struct CreateEdgeRequest {
    source: String,
    target: String,
    relation: String,
    properties: Option<Value>,
    weight: Option<f64>,
}

#[derive(Deserialize)]
struct StoreVectorRequest {
    id: String,
    text: Option<String>,
    /// Optional when `text` is supplied — the server embeds it in-process
    /// via the bundled fastembed model.
    vector: Option<Vec<f32>>,
    metadata: Option<Value>,
}

#[derive(Deserialize)]
struct VectorSearchRequest {
    /// Optional when `text` is supplied — the server embeds it in-process.
    vector: Option<Vec<f32>>,
    /// Natural-language query. Embedded server-side when `vector` is absent.
    text: Option<String>,
    top_k: Option<usize>,
}

#[derive(Deserialize)]
struct ShortestPathRequest {
    from: String,
    to: String,
    relation: Option<String>,
}

/// Query parameters for `GET /api/v1/prime/graph`.
///
/// Mirrors Core's contract. prime-mcp is a LOCAL single-store with no tenant,
/// so `tenant_id` is accepted for shape-parity but normally omitted.
#[derive(Deserialize)]
struct GraphQuery {
    tenant_id: Option<String>,
    node_type: Option<String>,
    limit: Option<usize>,
}

#[derive(Deserialize)]
struct RecallRequest {
    vector: Option<Vec<f32>>,
    node_type: Option<String>,
    depth: Option<usize>,
    top_k: Option<usize>,
    text: Option<String>,
}

// =============================================================================
// Handlers
// =============================================================================

async fn health() -> impl IntoResponse {
    Json(json!({"status": "ok"}))
}

/// `GET /api/v1/prime/graph.html` — the self-contained local graph viewer.
///
/// The page is compiled into the binary via `include_str!` (same idiom as the
/// entity templates in `templates.rs`), so it needs no static-file server and
/// no companion assets on disk. It has no external `<script src>`/`<link href>`,
/// so it renders offline; on load it fetches the same-origin
/// `/api/v1/prime/graph` and draws the store as a bubble graph + detail list.
async fn graph_viewer() -> impl IntoResponse {
    Html(include_str!("../static/graph.html"))
}

async fn get_stats(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let stats = state.prime.stats();
    Json(json!({
        "total_nodes": stats.total_nodes,
        "total_edges": stats.total_edges,
        "deleted_nodes": stats.deleted_nodes,
        "deleted_edges": stats.deleted_edges,
        "event_count": stats.event_count,
        "nodes_by_type": stats.nodes_by_type,
        "edges_by_relation": stats.edges_by_relation,
        "sync": crate::tools::sync_status_json(),
    }))
}

/// `GET /api/v1/prime/graph` — full materialized knowledge graph from the
/// local store, identical contract to Core's `/api/v1/prime/graph` (see the
/// doc comment on Core's `get_full_graph` for the verbatim JSON shape).
/// prime-mcp is local + single-store, so there is no tenant boundary here.
async fn get_full_graph(
    State(state): State<Arc<AppState>>,
    Query(q): Query<GraphQuery>,
) -> impl IntoResponse {
    let graph = state.prime.full_graph(
        q.tenant_id.as_deref(),
        q.node_type.as_deref(),
        q.limit,
    );
    (
        StatusCode::OK,
        Json(serde_json::to_value(&graph).unwrap_or(Value::Null)),
    )
}

async fn create_node(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateNodeRequest>,
) -> impl IntoResponse {
    match state.prime.add_node(&req.node_type, req.properties).await {
        Ok(id) => {
            let entity_id =
                allsource_core::prime::EntityId::node(&req.node_type, id.as_str()).to_wire();
            (
                StatusCode::CREATED,
                Json(json!({"node_id": id.as_str(), "entity_id": entity_id})),
            )
        }
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": e.to_string()})),
        ),
    }
}

async fn get_node(State(state): State<Arc<AppState>>, Path(id): Path<String>) -> impl IntoResponse {
    // Try both raw id and as entity_id
    match state.prime.get_node(&id) {
        Some(node) => (
            StatusCode::OK,
            Json(json!({
                "id": node.id.as_str(),
                "type": node.node_type,
                "properties": node.properties,
                "domain": node.domain,
                "labels": node.labels,
                "created_at": node.created_at.to_rfc3339(),
                "updated_at": node.updated_at.to_rfc3339(),
            })),
        ),
        None => (
            StatusCode::NOT_FOUND,
            Json(json!({"error": format!("node not found: {id}")})),
        ),
    }
}

async fn update_node(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(req): Json<UpdateNodeRequest>,
) -> impl IntoResponse {
    match state.prime.update_node(&id, req.properties).await {
        Ok(()) => (StatusCode::OK, Json(json!({"updated": true}))),
        Err(e) => (StatusCode::NOT_FOUND, Json(json!({"error": e.to_string()}))),
    }
}

async fn delete_node(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match state.prime.delete_node(&id).await {
        Ok(()) => (StatusCode::OK, Json(json!({"deleted": true}))),
        Err(e) => (StatusCode::NOT_FOUND, Json(json!({"error": e.to_string()}))),
    }
}

async fn get_neighbors(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let nodes = state.prime.neighbors(&id, None, Direction::Both);
    let nodes_json: Vec<Value> = nodes
        .iter()
        .map(|n| json!({"id": n.id.as_str(), "type": n.node_type, "properties": n.properties}))
        .collect();
    Json(json!({"nodes": nodes_json}))
}

async fn get_subgraph(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let sg = state.prime.subgraph(&id, 2);
    let nodes_json: Vec<Value> = sg
        .nodes
        .iter()
        .map(|n| json!({"id": n.id.as_str(), "type": n.node_type, "properties": n.properties}))
        .collect();
    let edges_json: Vec<Value> = sg
        .edges
        .iter()
        .map(|e| json!({"id": e.id.as_str(), "source": e.source.as_str(), "target": e.target.as_str(), "relation": e.relation}))
        .collect();
    Json(json!({"nodes": nodes_json, "edges": edges_json}))
}

async fn get_history(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match state.prime.history(&id).await {
        Ok(entries) => {
            let events: Vec<Value> = entries
                .iter()
                .map(|e| json!({"type": e.event_type, "timestamp": e.timestamp.to_rfc3339(), "payload": e.payload}))
                .collect();
            (StatusCode::OK, Json(json!({"events": events})))
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": e.to_string()})),
        ),
    }
}

async fn create_edge(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateEdgeRequest>,
) -> impl IntoResponse {
    let result = if let Some(w) = req.weight {
        state
            .prime
            .add_edge_weighted(&req.source, &req.target, &req.relation, w, req.properties)
            .await
    } else {
        state
            .prime
            .add_edge(&req.source, &req.target, &req.relation, req.properties)
            .await
    };

    match result {
        Ok(id) => (StatusCode::CREATED, Json(json!({"edge_id": id.as_str()}))),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": e.to_string()})),
        ),
    }
}

async fn delete_edge(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match state.prime.delete_edge(&id).await {
        Ok(()) => (StatusCode::OK, Json(json!({"deleted": true}))),
        Err(e) => (StatusCode::NOT_FOUND, Json(json!({"error": e.to_string()}))),
    }
}

async fn store_vector(
    State(state): State<Arc<AppState>>,
    Json(req): Json<StoreVectorRequest>,
) -> impl IntoResponse {
    let vector = match req.vector {
        Some(v) => v,
        None => match req.text.as_deref() {
            Some(t) => match state.prime.embed_text(t) {
                Ok(v) => v,
                Err(e) => {
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(json!({"error": format!("server-side embedding failed: {e}")})),
                    );
                }
            },
            None => {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(json!({"error": "missing 'vector' or 'text' — supply at least one"})),
                );
            }
        },
    };

    match state
        .prime
        .embed_with_metadata(&req.id, req.text.as_deref(), vector, req.metadata)
        .await
    {
        Ok(()) => (
            StatusCode::CREATED,
            Json(json!({"stored": true, "id": req.id})),
        ),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": e.to_string()})),
        ),
    }
}

async fn search_vectors(
    State(state): State<Arc<AppState>>,
    Json(req): Json<VectorSearchRequest>,
) -> impl IntoResponse {
    let top_k = req.top_k.unwrap_or(10);
    let vector = match req.vector {
        Some(v) => v,
        None => match req.text.as_deref() {
            Some(t) => match state.prime.embed_text(t) {
                Ok(v) => v,
                Err(e) => {
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(json!({"error": format!("server-side embedding failed: {e}")})),
                    );
                }
            },
            None => {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(json!({"error": "missing 'vector' or 'text' — supply at least one"})),
                );
            }
        },
    };
    let results = state.prime.vector_search(&vector, top_k);
    let results_json: Vec<Value> = results
        .iter()
        .map(|r| json!({"id": r.id, "score": r.score, "text": r.text}))
        .collect();
    (StatusCode::OK, Json(json!({"results": results_json})))
}

async fn delete_vector(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match state.prime.delete_vector(&id).await {
        Ok(()) => (StatusCode::OK, Json(json!({"deleted": true}))),
        Err(e) => (StatusCode::NOT_FOUND, Json(json!({"error": e.to_string()}))),
    }
}

async fn shortest_path(
    State(state): State<Arc<AppState>>,
    Json(req): Json<ShortestPathRequest>,
) -> impl IntoResponse {
    match state
        .prime
        .shortest_path(&req.from, &req.to, req.relation.as_deref())
    {
        Some(path) => {
            let nodes: Vec<Value> = path
                .iter()
                .map(|n| json!({"id": n.id.as_str(), "type": n.node_type, "properties": n.properties}))
                .collect();
            Json(json!({"path": nodes}))
        }
        None => Json(json!({"path": null, "message": "No path found"})),
    }
}

async fn recall(
    State(state): State<Arc<AppState>>,
    Json(req): Json<RecallRequest>,
) -> impl IntoResponse {
    use allsource_core::prime::types::RecallQuery;

    let vector = match req.vector {
        Some(v) => Some(v),
        None => match req.text.as_deref() {
            Some(t) => match state.prime.embed_text(t) {
                Ok(v) => Some(v),
                Err(e) => {
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(json!({"error": format!("server-side embedding failed: {e}")})),
                    );
                }
            },
            None => None,
        },
    };

    let query = RecallQuery {
        text: req.text,
        vector,
        node_type: req.node_type,
        depth: req.depth.unwrap_or(1),
        top_k: req.top_k.unwrap_or(10),
        ..RecallQuery::default()
    };

    match state.prime.recall(query).await {
        Ok(result) => {
            let nodes: Vec<Value> = result
                .nodes
                .iter()
                .map(|sn| {
                    json!({
                        "id": sn.node.id.as_str(),
                        "type": sn.node.node_type,
                        "score": sn.score,
                        "depth": sn.depth,
                    })
                })
                .collect();
            (StatusCode::OK, Json(json!({"nodes": nodes})))
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": e.to_string()})),
        ),
    }
}

async fn get_diff(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    // Without from/to params, return a summary of all events
    let stats = state.prime.stats();
    Json(json!({
        "total_nodes": stats.total_nodes,
        "total_edges": stats.total_edges,
        "event_count": stats.event_count,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use allsource_core::prime::EntityId;
    use allsource_core::prime::recall::{IndexConfig, RecallEngine};
    use axum::body::{Body, to_bytes};
    use axum::http::Request;
    use tower::ServiceExt; // for `oneshot`

    async fn test_state() -> Arc<AppState> {
        let prime = Prime::open_in_memory().await.unwrap();
        let recall = RecallEngine::with_deps(prime.recall_deps(), &IndexConfig::default());
        Arc::new(AppState {
            prime: Arc::new(prime),
            recall,
            hosted: None,
        })
    }

    fn graph_router(state: Arc<AppState>) -> Router {
        Router::new()
            .route("/api/v1/prime/graph", get(get_full_graph))
            .with_state(state)
    }

    /// A node + edge written to the local store appear in `/api/v1/prime/graph`
    /// with full properties and the same contract Core serves.
    #[tokio::test]
    async fn graph_endpoint_returns_written_node_and_edge() {
        let state = test_state().await;

        let org_id = state
            .prime
            .add_node("organization", json!({ "name": "Acme" }))
            .await
            .unwrap();
        let org = EntityId::node("organization", org_id.as_str()).to_wire();
        let contact_id = state
            .prime
            .add_node("contact", json!({ "name": "Alice" }))
            .await
            .unwrap();
        let contact = EntityId::node("contact", contact_id.as_str()).to_wire();
        state
            .prime
            .add_edge(&contact, &org, "works_at", None)
            .await
            .unwrap();

        let app = graph_router(state);
        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/api/v1/prime/graph")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        let body: Value = serde_json::from_slice(&bytes).unwrap();

        let nodes = body["nodes"].as_array().unwrap();
        assert_eq!(nodes.len(), 2);
        let org_node = nodes.iter().find(|n| n["id"] == org).unwrap();
        assert_eq!(org_node["properties"]["name"], "Acme");
        assert_eq!(org_node["has_vector"], false);

        let edges = body["edges"].as_array().unwrap();
        assert_eq!(edges.len(), 1);
        assert_eq!(edges[0]["source"], contact);
        assert_eq!(edges[0]["target"], org);
        assert_eq!(edges[0]["relation"], "works_at");

        assert_eq!(body["stats"]["node_count"], 2);
        assert_eq!(body["stats"]["edge_count"], 1);
        assert_eq!(body["has_more"], false);
    }

    // ─── MCP-over-HTTP (Streamable HTTP transport) ────────────────────────

    fn mcp_router(state: Arc<AppState>) -> Router {
        Router::new().route("/mcp", post(mcp_handler)).with_state(state)
    }

    async fn mcp_post(app: &Router, payload: Value) -> (StatusCode, Value) {
        let resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/mcp")
                    .header("content-type", "application/json")
                    .body(Body::from(payload.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        let status = resp.status();
        let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        let body = if bytes.is_empty() {
            Value::Null
        } else {
            serde_json::from_slice(&bytes).unwrap()
        };
        (status, body)
    }

    /// A client can complete the full MCP handshake over HTTP: initialize,
    /// tools/list, and a tools/call — the bead's acceptance criterion.
    #[tokio::test]
    async fn mcp_http_initialize_list_and_call() {
        let app = mcp_router(test_state().await);

        let (status, body) =
            mcp_post(&app, json!({ "jsonrpc": "2.0", "id": 1, "method": "initialize" })).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["result"]["serverInfo"]["name"], "allsource-prime");
        assert!(body["result"]["protocolVersion"].is_string());

        let (status, body) =
            mcp_post(&app, json!({ "jsonrpc": "2.0", "id": 2, "method": "tools/list" })).await;
        assert_eq!(status, StatusCode::OK);
        let tools = body["result"]["tools"].as_array().unwrap();
        assert!(!tools.is_empty(), "tools/list must return the Prime tools");

        let (status, body) = mcp_post(
            &app,
            json!({
                "jsonrpc": "2.0", "id": 3, "method": "tools/call",
                "params": { "name": "prime_stats", "arguments": {} }
            }),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert!(!body["result"].is_null(), "tools/call must return a result envelope");
    }

    /// Notifications (no id, no reply) get 202 Accepted with an empty body.
    #[tokio::test]
    async fn mcp_http_notification_returns_202() {
        let app = mcp_router(test_state().await);
        let (status, body) = mcp_post(
            &app,
            json!({ "jsonrpc": "2.0", "method": "notifications/initialized" }),
        )
        .await;
        assert_eq!(status, StatusCode::ACCEPTED);
        assert_eq!(body, Value::Null);
    }

    /// Malformed JSON yields a JSON-RPC parse-error envelope (not a 500).
    #[tokio::test]
    async fn mcp_http_parse_error_is_jsonrpc_envelope() {
        let app = mcp_router(test_state().await);
        let resp = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/mcp")
                    .header("content-type", "application/json")
                    .body(Body::from("not json"))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        let body: Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(body["error"]["code"], -32700);
    }

    // ─── Hosted, tenant-scoped /mcp dispatch ──────────────────────────────

    use wiremock::matchers::{method as wm_method, path as wm_path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    /// Mount the standard empty-Core pair (GET query → `{events:[]}`, POST
    /// ingest → 200) and return an `AppState` whose `hosted` engine points at it.
    async fn hosted_state(server: &MockServer) -> Arc<AppState> {
        Mock::given(wm_method("GET"))
            .and(wm_path("/api/v1/events/query"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "events": [] })))
            .mount(server)
            .await;
        Mock::given(wm_method("POST"))
            .and(wm_path("/api/v1/events"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "ok": true })))
            .mount(server)
            .await;

        let prime = Prime::open_in_memory().await.unwrap();
        let recall = RecallEngine::with_deps(prime.recall_deps(), &IndexConfig::default());
        let hosted = HostedPrime::connect(
            server.uri(),
            None,
            8,
            std::time::Duration::from_secs(60),
        );
        Arc::new(AppState {
            prime: Arc::new(prime),
            recall,
            hosted: Some(Arc::new(hosted)),
        })
    }

    async fn mcp_post_with_tenant(
        app: &Router,
        tenant: Option<&str>,
        payload: Value,
    ) -> (StatusCode, Value) {
        let mut builder = Request::builder()
            .method("POST")
            .uri("/mcp")
            .header("content-type", "application/json");
        if let Some(t) = tenant {
            builder = builder.header("x-tenant-id", t);
        }
        let resp = app
            .clone()
            .oneshot(builder.body(Body::from(payload.to_string())).unwrap())
            .await
            .unwrap();
        let status = resp.status();
        let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        let body = if bytes.is_empty() {
            Value::Null
        } else {
            serde_json::from_slice(&bytes).unwrap()
        };
        (status, body)
    }

    /// With a `HostedPrime` engine and an `X-Tenant-Id`, a `prime_add_node` write
    /// followed by a `prime_search` round-trips through the hosted path, proving
    /// both the hosted dispatch and tenant threading end-to-end.
    #[tokio::test]
    async fn mcp_hosted_path_add_node_then_search() {
        let server = MockServer::start().await;
        let app = mcp_router(hosted_state(&server).await);

        let (status, body) = mcp_post_with_tenant(
            &app,
            Some("tenant-a"),
            json!({
                "jsonrpc": "2.0", "id": 1, "method": "tools/call",
                "params": { "name": "prime_add_node", "arguments": { "type": "contact", "properties": { "name": "Alice" } } }
            }),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_ne!(body["result"]["isError"], json!(true), "add_node should succeed on hosted path");

        let (status, body) = mcp_post_with_tenant(
            &app,
            Some("tenant-a"),
            json!({
                "jsonrpc": "2.0", "id": 2, "method": "tools/call",
                "params": { "name": "prime_search", "arguments": { "type": "contact" } }
            }),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        let text = body["result"]["content"][0]["text"].as_str().unwrap();
        assert!(text.contains("Alice"), "hosted search should return the added node; got {text}");
    }

    /// A tool `HostedPrime` does not implement returns a clear tool-error (not a
    /// crash) on the hosted path.
    #[tokio::test]
    async fn mcp_hosted_path_unsupported_tool_errors() {
        let server = MockServer::start().await;
        let app = mcp_router(hosted_state(&server).await);

        let (status, body) = mcp_post_with_tenant(
            &app,
            Some("tenant-a"),
            json!({
                "jsonrpc": "2.0", "id": 1, "method": "tools/call",
                "params": { "name": "prime_index", "arguments": {} }
            }),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["result"]["isError"], json!(true));
        let text = body["result"]["content"][0]["text"].as_str().unwrap();
        assert!(text.contains("not yet available on the hosted backend"));
    }

    /// Without `X-Tenant-Id`, the request falls back to the embedded dispatch
    /// even when a `HostedPrime` engine is configured — initialize still works.
    #[tokio::test]
    async fn mcp_no_tenant_falls_back_to_embedded() {
        let server = MockServer::start().await;
        let app = mcp_router(hosted_state(&server).await);

        let (status, body) = mcp_post_with_tenant(
            &app,
            None,
            json!({ "jsonrpc": "2.0", "id": 1, "method": "initialize" }),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["result"]["serverInfo"]["name"], "allsource-prime");
    }
}
