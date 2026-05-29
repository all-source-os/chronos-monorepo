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
    let router = Router::new()
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
        // Declarative projections + per-field provenance.
        // These mirror the MCP tools (prime_define_projection,
        // prime_list_projections, prime_project_node, prime_node_provenance)
        // so SDK/REST callers reach the same Neotoma-parity primitives the
        // MCP layer already exposes.
        .route("/projections", post(define_projection))
        .route("/projections", get(list_projections))
        .route("/nodes/{id}/project", post(project_node))
        .route(
            "/nodes/{id}/fields/{field}/provenance",
            get(field_provenance),
        )
        // Stats (always available)
        .route("/stats", get(get_stats));

    // Vector and recall routes require prime-vectors feature
    #[cfg(feature = "prime-vectors")]
    let router = router
        .route("/vectors", post(store_vector))
        .route("/vectors/search", post(search_vectors))
        .route("/vectors/{id}", delete(delete_vector))
        .route("/recall", post(recall));

    router
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

#[cfg(feature = "prime-vectors")]
#[derive(Deserialize)]
struct StoreVectorReq {
    id: String,
    text: Option<String>,
    vector: Vec<f32>,
    metadata: Option<Value>,
}

#[cfg(feature = "prime-vectors")]
#[derive(Deserialize)]
struct VectorSearchReq {
    vector: Vec<f32>,
    top_k: Option<usize>,
}

#[cfg(feature = "prime-vectors")]
#[derive(Deserialize)]
struct RecallReq {
    vector: Option<Vec<f32>>,
    node_type: Option<String>,
    depth: Option<usize>,
    top_k: Option<usize>,
}

/// Body for `POST /projections`. `field_policies` maps a field name to one
/// of the four declarative merge policies, as a snake_case string
/// (`last_write`, `highest_priority`, `most_specific`, `merge_array`) — the
/// same wire shape the MCP tool accepts.
#[derive(Deserialize)]
struct DefineProjectionReq {
    entity_type: String,
    field_policies: std::collections::BTreeMap<String, String>,
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
            let entity_id = crate::prime::EntityId::node(&req.node_type, id.as_str()).to_wire();
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
        Err(e) => (StatusCode::NOT_FOUND, Json(json!({"error": e.to_string()}))),
    }
}

async fn delete_node(
    State(state): State<Arc<PrimeState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match state.prime.delete_node(&id).await {
        Ok(()) => (StatusCode::OK, Json(json!({"deleted": true}))),
        Err(e) => (StatusCode::NOT_FOUND, Json(json!({"error": e.to_string()}))),
    }
}

async fn get_neighbors(
    State(state): State<Arc<PrimeState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let nodes = state.prime.neighbors(&id, None, Direction::Both);
    let nodes_json: Vec<Value> = nodes
        .iter()
        .map(|n| json!({"id": n.id.as_str(), "type": n.node_type, "properties": n.properties}))
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
        Ok(id) => (StatusCode::CREATED, Json(json!({"edge_id": id.as_str()}))),
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
        Err(e) => (StatusCode::NOT_FOUND, Json(json!({"error": e.to_string()}))),
    }
}

#[cfg(feature = "prime-vectors")]
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

#[cfg(feature = "prime-vectors")]
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

#[cfg(feature = "prime-vectors")]
async fn delete_vector(
    State(state): State<Arc<PrimeState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match state.prime.delete_vector(&id).await {
        Ok(()) => (StatusCode::OK, Json(json!({"deleted": true}))),
        Err(e) => (StatusCode::NOT_FOUND, Json(json!({"error": e.to_string()}))),
    }
}

#[cfg(feature = "prime-vectors")]
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

// =============================================================================
// Declarative projections + provenance
// =============================================================================

/// Parse a node entity_id of the form `node:{type}:{id}` and return the
/// type segment. Mirrors `entity_type_from_node_id` in the MCP layer.
fn entity_type_from_node_id(node_id: &str) -> Option<&str> {
    let mut parts = node_id.splitn(3, ':');
    if parts.next()? != "node" {
        return None;
    }
    let entity_type = parts.next()?;
    parts.next()?; // id segment must exist
    Some(entity_type)
}

/// Load the latest projection definition for `entity_type` by replaying the
/// `prime.projection.defined` event stream. Core keeps no in-memory registry
/// (unlike the MCP process); the durable event log is the source of truth.
async fn load_def(
    state: &Arc<PrimeState>,
    entity_type: &str,
) -> Result<Option<crate::prime::projections::declarative::ProjectionDef>, String> {
    let defs = state
        .prime
        .load_projection_defs()
        .await
        .map_err(|e| e.to_string())?;
    Ok(defs.into_iter().find(|d| d.entity_type == entity_type))
}

async fn define_projection(
    State(state): State<Arc<PrimeState>>,
    Json(req): Json<DefineProjectionReq>,
) -> impl IntoResponse {
    use crate::prime::projections::declarative::{MergePolicy, ProjectionDef};

    let mut field_policies = std::collections::BTreeMap::new();
    for (field, policy_str) in &req.field_policies {
        match serde_json::from_value::<MergePolicy>(json!(policy_str)) {
            Ok(p) => {
                field_policies.insert(field.clone(), p);
            }
            Err(_) => {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(json!({"error": format!(
                        "field_policies.{field}: unknown policy '{policy_str}' — valid: last_write, highest_priority, most_specific, merge_array"
                    )})),
                );
            }
        }
    }

    let def = ProjectionDef {
        entity_type: req.entity_type.clone(),
        field_policies,
    };

    match state.prime.define_projection(&def).await {
        Ok(()) => (
            StatusCode::CREATED,
            Json(json!({"entity_type": req.entity_type, "persisted": true})),
        ),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": e.to_string()})),
        ),
    }
}

async fn list_projections(State(state): State<Arc<PrimeState>>) -> impl IntoResponse {
    match state.prime.load_projection_defs().await {
        Ok(defs) => {
            let projections: Vec<Value> = defs
                .iter()
                .map(|d| {
                    let policies: serde_json::Map<String, Value> = d
                        .field_policies
                        .iter()
                        .map(|(field, policy)| {
                            (field.clone(), serde_json::to_value(policy).unwrap_or(Value::Null))
                        })
                        .collect();
                    json!({"entity_type": d.entity_type, "field_policies": policies})
                })
                .collect();
            (
                StatusCode::OK,
                Json(json!({"projections": projections, "total": defs.len()})),
            )
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": e.to_string()})),
        ),
    }
}

async fn project_node(
    State(state): State<Arc<PrimeState>>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    use crate::prime::projections::declarative::fold;

    let Some(entity_type) = entity_type_from_node_id(&id) else {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": format!("node_id must be 'node:{{type}}:{{id}}', got: {id}")})),
        );
    };
    let def = match load_def(&state, entity_type).await {
        Ok(Some(d)) => d,
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Json(json!({"error": format!(
                    "no projection defined for entity_type '{entity_type}' — POST /projections first"
                )})),
            );
        }
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": e}))),
    };
    let observations = match state.prime.observations(&id).await {
        Ok(o) => o,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": format!("failed to load observations: {e}")})),
            );
        }
    };
    let snapshot = fold(&observations, &def);
    (
        StatusCode::OK,
        Json(json!({
            "entity_type": snapshot.entity_type,
            "fields": snapshot.fields,
            "observation_count": observations.len(),
        })),
    )
}

async fn field_provenance(
    State(state): State<Arc<PrimeState>>,
    Path((id, field)): Path<(String, String)>,
) -> impl IntoResponse {
    use crate::prime::projections::declarative::provenance_for_field;

    let Some(entity_type) = entity_type_from_node_id(&id) else {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"error": format!("node_id must be 'node:{{type}}:{{id}}', got: {id}")})),
        );
    };
    let def = match load_def(&state, entity_type).await {
        Ok(Some(d)) => d,
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Json(json!({"error": format!(
                    "no projection defined for entity_type '{entity_type}' — POST /projections first"
                )})),
            );
        }
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": e}))),
    };
    let observations = match state.prime.observations(&id).await {
        Ok(o) => o,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": format!("failed to load observations: {e}")})),
            );
        }
    };
    match provenance_for_field(&observations, &def, &field) {
        Some(p) => (
            StatusCode::OK,
            Json(json!({
                "field": p.field,
                "value": p.value,
                "source_event_id": p.source_event_id,
                "source_event_at": p.source_event_at,
                "merge_policy_applied": p.merge_policy_applied,
            })),
        ),
        None => (
            StatusCode::NOT_FOUND,
            Json(json!({"error": format!(
                "no provenance for field '{field}' on {id} (missing, not in projection, or merge_array policy — union has no single source)"
            )})),
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

// =============================================================================
// Tests — exercise the REST routes end-to-end through the real router, proving
// SDK/REST callers reach the same projection + provenance primitives the MCP
// tools expose (bead t-2ac8 acceptance: REST returns what the MCP path does).
// =============================================================================
#[cfg(test)]
mod tests {
    use super::*;
    use crate::prime::{EntityId, Prime};
    use axum::body::{Body, to_bytes};
    use axum::http::Request;
    use tower::ServiceExt; // for `oneshot`

    async fn test_app() -> (Router, Arc<PrimeState>) {
        let prime = Prime::open_in_memory().await.unwrap();
        let state = Arc::new(PrimeState { prime });
        (prime_router().with_state(state.clone()), state)
    }

    async fn send(app: &Router, method: &str, uri: &str, body: Option<Value>) -> (StatusCode, Value) {
        let builder = Request::builder().method(method).uri(uri);
        let req = match body {
            Some(b) => builder
                .header("content-type", "application/json")
                .body(Body::from(b.to_string()))
                .unwrap(),
            None => builder.body(Body::empty()).unwrap(),
        };
        let resp = app.clone().oneshot(req).await.unwrap();
        let status = resp.status();
        let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        let json: Value = if bytes.is_empty() {
            Value::Null
        } else {
            serde_json::from_slice(&bytes).unwrap()
        };
        (status, json)
    }

    #[tokio::test]
    async fn define_list_project_and_provenance_roundtrip() {
        let (app, state) = test_app().await;

        // Define a projection for entity_type "contact".
        let (status, _) = send(
            &app,
            "POST",
            "/projections",
            Some(json!({
                "entity_type": "contact",
                "field_policies": { "status": "last_write", "tags": "merge_array" }
            })),
        )
        .await;
        assert_eq!(status, StatusCode::CREATED);

        // It shows up in the list (replayed from the durable event log).
        let (status, body) = send(&app, "GET", "/projections", None).await;
        assert_eq!(status, StatusCode::OK);
        let listed = body["projections"].as_array().unwrap();
        assert!(listed.iter().any(|p| p["entity_type"] == "contact"));
        assert_eq!(body["projections"][0]["field_policies"]["status"], "last_write");

        // Seed a node (one observation) so project/provenance have something.
        let node_id = state
            .prime
            .add_node("contact", json!({ "status": "active", "tags": ["a"] }))
            .await
            .unwrap();
        let entity_id = EntityId::node("contact", node_id.as_str()).to_wire();

        // Project the node through the REST route.
        let (status, body) = send(&app, "POST", &format!("/nodes/{entity_id}/project"), None).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["entity_type"], "contact");
        assert_eq!(body["fields"]["status"], "active");
        assert_eq!(body["observation_count"], 1);

        // Per-field provenance for a single-source field.
        let (status, body) = send(
            &app,
            "GET",
            &format!("/nodes/{entity_id}/fields/status/provenance"),
            None,
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body["value"], "active");
        assert_eq!(body["merge_policy_applied"], "last_write");
        assert!(body["source_event_id"].as_str().is_some_and(|s| !s.is_empty()));

        // merge_array fields have no single source → 404.
        let (status, _) = send(
            &app,
            "GET",
            &format!("/nodes/{entity_id}/fields/tags/provenance"),
            None,
        )
        .await;
        assert_eq!(status, StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn project_unknown_entity_type_is_404() {
        let (app, _state) = test_app().await;
        let (status, _) = send(&app, "POST", "/nodes/node:ghost:1/project", None).await;
        assert_eq!(status, StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn define_rejects_unknown_policy() {
        let (app, _state) = test_app().await;
        let (status, body) = send(
            &app,
            "POST",
            "/projections",
            Some(json!({ "entity_type": "x", "field_policies": { "f": "nonsense" } })),
        )
        .await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert!(body["error"].as_str().unwrap().contains("unknown policy"));
    }

    #[tokio::test]
    async fn malformed_node_id_is_400() {
        let (app, _state) = test_app().await;
        let (status, _) = send(&app, "POST", "/nodes/not-a-node-id/project", None).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
    }
}
