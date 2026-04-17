//! HTTP REST API server for Prime (axum-based).
//!
//! Exposes the full Prime API as REST endpoints, consistent with Core's HTTP stack.
//! Activated via `--mode http --port 3905`.

use std::sync::Arc;

use allsource_core::prime::{Direction, Prime, recall::RecallEngine};
use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{delete, get, patch, post},
};
use serde::Deserialize;
use serde_json::{Value, json};
use tower_http::trace::TraceLayer;

/// Shared state for HTTP handlers.
pub struct AppState {
    pub prime: Prime,
    /// Recall engine — used by the `/recall` endpoint and kept for future index endpoints.
    #[allow(dead_code)]
    pub recall: RecallEngine,
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
        .route("/health", get(health))
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
    vector: Vec<f32>,
    metadata: Option<Value>,
}

#[derive(Deserialize)]
struct VectorSearchRequest {
    vector: Vec<f32>,
    top_k: Option<usize>,
}

#[derive(Deserialize)]
struct ShortestPathRequest {
    from: String,
    to: String,
    relation: Option<String>,
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
    }))
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
    match state
        .prime
        .embed_with_metadata(&req.id, req.text.as_deref(), req.vector, req.metadata)
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
    let results = state.prime.vector_search(&req.vector, top_k);
    let results_json: Vec<Value> = results
        .iter()
        .map(|r| json!({"id": r.id, "score": r.score, "text": r.text}))
        .collect();
    Json(json!({"results": results_json}))
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

    let query = RecallQuery {
        text: req.text,
        vector: req.vector,
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
