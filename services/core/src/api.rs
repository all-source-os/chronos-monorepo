use crate::error::{AllSourceError, Result};
use crate::event::{Event, IngestEventRequest, IngestEventResponse, QueryEventsRequest, QueryEventsResponse};
use crate::store::EventStore;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use serde::Deserialize;
use std::sync::Arc;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;

type SharedStore = Arc<EventStore>;

pub async fn serve(store: SharedStore, addr: &str) -> anyhow::Result<()> {
    let app = Router::new()
        .route("/health", get(health))
        .route("/api/v1/events", post(ingest_event))
        .route("/api/v1/events/query", get(query_events))
        .route("/api/v1/entities/:entity_id/state", get(get_entity_state))
        .route("/api/v1/entities/:entity_id/snapshot", get(get_entity_snapshot))
        .route("/api/v1/stats", get(get_stats))
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any),
        )
        .layer(TraceLayer::new_for_http())
        .with_state(store);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

async fn health() -> impl IntoResponse {
    Json(serde_json::json!({
        "status": "healthy",
        "service": "allsource-core",
        "version": env!("CARGO_PKG_VERSION")
    }))
}

async fn ingest_event(
    State(store): State<SharedStore>,
    Json(req): Json<IngestEventRequest>,
) -> Result<Json<IngestEventResponse>> {
    let mut event = Event::new(req.event_type, req.entity_id, req.payload);
    event.metadata = req.metadata;

    let event_id = event.id;
    let timestamp = event.timestamp;

    store.ingest(event)?;

    tracing::info!("Event ingested: {}", event_id);

    Ok(Json(IngestEventResponse {
        event_id,
        timestamp,
    }))
}

async fn query_events(
    State(store): State<SharedStore>,
    Query(req): Query<QueryEventsRequest>,
) -> Result<Json<QueryEventsResponse>> {
    let events = store.query(req)?;
    let count = events.len();

    tracing::debug!("Query returned {} events", count);

    Ok(Json(QueryEventsResponse { events, count }))
}

#[derive(Deserialize)]
struct EntityStateParams {
    as_of: Option<chrono::DateTime<chrono::Utc>>,
}

async fn get_entity_state(
    State(store): State<SharedStore>,
    Path(entity_id): Path<String>,
    Query(params): Query<EntityStateParams>,
) -> Result<Json<serde_json::Value>> {
    let state = store.reconstruct_state(&entity_id, params.as_of)?;

    tracing::info!("State reconstructed for entity: {}", entity_id);

    Ok(Json(state))
}

async fn get_entity_snapshot(
    State(store): State<SharedStore>,
    Path(entity_id): Path<String>,
) -> Result<Json<serde_json::Value>> {
    let snapshot = store.get_snapshot(&entity_id)?;

    tracing::debug!("Snapshot retrieved for entity: {}", entity_id);

    Ok(Json(snapshot))
}

async fn get_stats(State(store): State<SharedStore>) -> impl IntoResponse {
    let stats = store.stats();
    Json(stats)
}
