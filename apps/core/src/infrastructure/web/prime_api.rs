//! Prime API endpoints — exposes graph, vector, and recall operations
//! through Core's existing HTTP server.
//!
//! Feature-gated behind `prime`. When enabled, adds `/api/v1/prime/*` routes
//! to the Core server. Prime runs embedded (same process, no HTTP hop).

use std::sync::Arc;

use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{delete, get, patch, post},
};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::prime::{Direction, Prime};

/// Shared Prime state, initialized once during server startup.
pub struct PrimeState {
    pub prime: Prime,
}

/// Build the Prime API router.
///
/// All routes are nested under `/api/v1/prime` by the caller.
pub fn prime_router() -> Router<Arc<PrimeState>> {
    Router::new()
        // Nodes
        .route("/nodes", post(create_node))
        .route("/nodes/{id}", get(get_node))
        .route("/nodes/{id}", patch(update_node))
        .route("/nodes/{id}", delete(delete_node))
        .route("/nodes/{id}/neighbors", get(get_neighbors))
        .route("/nodes/{id}/history", get(get_history))
        // Edges
        .route("/edges", post(create_edge))
        .route("/edges/{id}", delete(delete_edge_handler))
        // Vectors
        .route("/vectors", post(store_vector))
        .route("/vectors/search", post(search_vectors))
        .route("/vectors/{id}", delete(delete_vector))
        // Queries
        .route("/recall", post(recall))
        .route("/stats", get(get_stats))
}

// =============================================================================
// Request types
// =============================================================================

#[derive(Deserialize)]
struct CreateNodeReq {
    #[serde(rename = "type")]
    node_type: String,
    properties: Value,
}

#[derive(Deserialize)]
struct UpdateNodeReq {
    properties: Value,
}

#[derive(Deserialize)]
struct CreateEdgeReq {
    source: String,
    target: String,
    relation: String,
    properties: Option<Value>,
    weight: Option<f64>,
}

#[derive(Deserialize)]
struct StoreVectorReq {
    id: String,
    text: Option<String>,
    vector: Vec<f32>,
    metadata: Option<Value>,
}

#[derive(Deserialize)]
struct VectorSearchReq {
    vector: Vec<f32>,
    top_k: Option<usize>,
}

#[derive(Deserialize)]
struct RecallReq {
    vector: Option<Vec<f32>>,
    node_type: Option<String>,
    depth: Option<usize>,
    top_k: Option<usize>,
}

// =============================================================================
// Handlers
// =============================================================================

async fn create_node(
    State(state): State<Arc<PrimeState>>,
    Json(req): Json<CreateNodeReq>,
) -> impl IntoResponse {
    match state.prime.add_node(&req.node_type, req.properties).await {
        Ok(id) => {
            let entity_id =
                crate::prime::EntityId::node(&req.node_type, id.as_str()).to_wire();
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

async fn get_node(
    State(state): State<Arc<PrimeState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match state.prime.get_node(&id) {
        Some(node) => (
            StatusCode::OK,
            Json(json!({
                "id": node.id.as_str(),
                "type": node.node_type,
                "properties": node.properties,
                "domain": node.domain,
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
    State(state): State<Arc<PrimeState>>,
    Path(id): Path<String>,
    Json(req): Json<UpdateNodeReq>,
) -> impl IntoResponse {
    match state.prime.update_node(&id, req.properties).await {
        Ok(()) => (StatusCode::OK, Json(json!({"updated": true}))),
        Err(e) => (
            StatusCode::NOT_FOUND,
            Json(json!({"error": e.to_string()})),
        ),
    }
}

async fn delete_node(
    State(state): State<Arc<PrimeState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match state.prime.delete_node(&id).await {
        Ok(()) => (StatusCode::OK, Json(json!({"deleted": true}))),
        Err(e) => (
            StatusCode::NOT_FOUND,
            Json(json!({"error": e.to_string()})),
        ),
    }
}

async fn get_neighbors(
    State(state): State<Arc<PrimeState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let nodes = state.prime.neighbors(&id, None, Direction::Both);
    let nodes_json: Vec<Value> = nodes
        .iter()
        .map(|n| {
            json!({"id": n.id.as_str(), "type": n.node_type, "properties": n.properties})
        })
        .collect();
    Json(json!({"nodes": nodes_json}))
}

async fn get_history(
    State(state): State<Arc<PrimeState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match state.prime.history(&id).await {
        Ok(entries) => {
            let events: Vec<Value> = entries
                .iter()
                .map(|e| {
                    json!({
                        "type": e.event_type,
                        "timestamp": e.timestamp.to_rfc3339(),
                        "payload": e.payload
                    })
                })
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
    State(state): State<Arc<PrimeState>>,
    Json(req): Json<CreateEdgeReq>,
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
        Ok(id) => (
            StatusCode::CREATED,
            Json(json!({"edge_id": id.as_str()})),
        ),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": e.to_string()})),
        ),
    }
}

async fn delete_edge_handler(
    State(state): State<Arc<PrimeState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match state.prime.delete_edge(&id).await {
        Ok(()) => (StatusCode::OK, Json(json!({"deleted": true}))),
        Err(e) => (
            StatusCode::NOT_FOUND,
            Json(json!({"error": e.to_string()})),
        ),
    }
}

async fn store_vector(
    State(state): State<Arc<PrimeState>>,
    Json(req): Json<StoreVectorReq>,
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
    State(state): State<Arc<PrimeState>>,
    Json(req): Json<VectorSearchReq>,
) -> impl IntoResponse {
    let results = state
        .prime
        .vector_search(&req.vector, req.top_k.unwrap_or(10));
    let results_json: Vec<Value> = results
        .iter()
        .map(|r| json!({"id": r.id, "score": r.score, "text": r.text}))
        .collect();
    Json(json!({"results": results_json}))
}

async fn delete_vector(
    State(state): State<Arc<PrimeState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match state.prime.delete_vector(&id).await {
        Ok(()) => (StatusCode::OK, Json(json!({"deleted": true}))),
        Err(e) => (
            StatusCode::NOT_FOUND,
            Json(json!({"error": e.to_string()})),
        ),
    }
}

async fn recall(
    State(state): State<Arc<PrimeState>>,
    Json(req): Json<RecallReq>,
) -> impl IntoResponse {
    use crate::prime::types::RecallQuery;

    let query = RecallQuery {
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
                        "properties": sn.node.properties,
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

async fn get_stats(State(state): State<Arc<PrimeState>>) -> impl IntoResponse {
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
