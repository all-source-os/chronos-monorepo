use crate::analytics::{
    AnalyticsEngine, CorrelationRequest, CorrelationResponse, EventFrequencyRequest,
    EventFrequencyResponse, StatsSummaryRequest, StatsSummaryResponse,
};
use crate::compaction::CompactionResult;
use crate::error::Result;
use crate::event::{Event, IngestEventRequest, IngestEventResponse, QueryEventsRequest, QueryEventsResponse};
use crate::snapshot::{
    CreateSnapshotRequest, CreateSnapshotResponse, ListSnapshotsRequest, ListSnapshotsResponse,
    SnapshotInfo,
};
use crate::store::EventStore;
use axum::{
    extract::{Path, Query, State, WebSocketUpgrade},
    response::{IntoResponse, Response},
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
        .route("/api/v1/events/stream", get(events_websocket)) // v0.2: WebSocket streaming
        .route("/api/v1/entities/:entity_id/state", get(get_entity_state))
        .route("/api/v1/entities/:entity_id/snapshot", get(get_entity_snapshot))
        .route("/api/v1/stats", get(get_stats))
        // v0.2: Advanced analytics endpoints
        .route("/api/v1/analytics/frequency", get(analytics_frequency))
        .route("/api/v1/analytics/summary", get(analytics_summary))
        .route("/api/v1/analytics/correlation", get(analytics_correlation))
        // v0.2: Snapshot management endpoints
        .route("/api/v1/snapshots", post(create_snapshot))
        .route("/api/v1/snapshots", get(list_snapshots))
        .route("/api/v1/snapshots/:entity_id/latest", get(get_latest_snapshot))
        // v0.2: Compaction endpoints
        .route("/api/v1/compaction/trigger", post(trigger_compaction))
        .route("/api/v1/compaction/stats", get(compaction_stats))
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

// v0.2: WebSocket endpoint for real-time event streaming
async fn events_websocket(
    ws: WebSocketUpgrade,
    State(store): State<SharedStore>,
) -> Response {
    let websocket_manager = store.websocket_manager();

    ws.on_upgrade(move |socket| async move {
        websocket_manager.handle_socket(socket).await;
    })
}

// v0.2: Event frequency analytics endpoint
async fn analytics_frequency(
    State(store): State<SharedStore>,
    Query(req): Query<EventFrequencyRequest>,
) -> Result<Json<EventFrequencyResponse>> {
    let response = AnalyticsEngine::event_frequency(&store, req)?;

    tracing::debug!(
        "Frequency analysis returned {} buckets",
        response.buckets.len()
    );

    Ok(Json(response))
}

// v0.2: Statistical summary endpoint
async fn analytics_summary(
    State(store): State<SharedStore>,
    Query(req): Query<StatsSummaryRequest>,
) -> Result<Json<StatsSummaryResponse>> {
    let response = AnalyticsEngine::stats_summary(&store, req)?;

    tracing::debug!(
        "Stats summary: {} events across {} entities",
        response.total_events,
        response.unique_entities
    );

    Ok(Json(response))
}

// v0.2: Event correlation analysis endpoint
async fn analytics_correlation(
    State(store): State<SharedStore>,
    Query(req): Query<CorrelationRequest>,
) -> Result<Json<CorrelationResponse>> {
    let response = AnalyticsEngine::analyze_correlation(&store, req)?;

    tracing::debug!(
        "Correlation analysis: {}/{} correlated pairs ({:.2}%)",
        response.correlated_pairs,
        response.total_a,
        response.correlation_percentage
    );

    Ok(Json(response))
}

// v0.2: Create a snapshot for an entity
async fn create_snapshot(
    State(store): State<SharedStore>,
    Json(req): Json<CreateSnapshotRequest>,
) -> Result<Json<CreateSnapshotResponse>> {
    store.create_snapshot(&req.entity_id)?;

    let snapshot_manager = store.snapshot_manager();
    let snapshot = snapshot_manager
        .get_latest_snapshot(&req.entity_id)
        .ok_or_else(|| crate::error::AllSourceError::EntityNotFound(req.entity_id.clone()))?;

    tracing::info!("📸 Created snapshot for entity: {}", req.entity_id);

    Ok(Json(CreateSnapshotResponse {
        snapshot_id: snapshot.id,
        entity_id: snapshot.entity_id,
        created_at: snapshot.created_at,
        event_count: snapshot.event_count,
        size_bytes: snapshot.metadata.size_bytes,
    }))
}

// v0.2: List snapshots
async fn list_snapshots(
    State(store): State<SharedStore>,
    Query(req): Query<ListSnapshotsRequest>,
) -> Result<Json<ListSnapshotsResponse>> {
    let snapshot_manager = store.snapshot_manager();

    let snapshots: Vec<SnapshotInfo> = if let Some(entity_id) = req.entity_id {
        snapshot_manager
            .get_all_snapshots(&entity_id)
            .into_iter()
            .map(SnapshotInfo::from)
            .collect()
    } else {
        // List all entities with snapshots
        let entities = snapshot_manager.list_entities();
        entities
            .iter()
            .flat_map(|entity_id| {
                snapshot_manager
                    .get_all_snapshots(entity_id)
                    .into_iter()
                    .map(SnapshotInfo::from)
            })
            .collect()
    };

    let total = snapshots.len();

    tracing::debug!("Listed {} snapshots", total);

    Ok(Json(ListSnapshotsResponse { snapshots, total }))
}

// v0.2: Get latest snapshot for an entity
async fn get_latest_snapshot(
    State(store): State<SharedStore>,
    Path(entity_id): Path<String>,
) -> Result<Json<serde_json::Value>> {
    let snapshot_manager = store.snapshot_manager();

    let snapshot = snapshot_manager
        .get_latest_snapshot(&entity_id)
        .ok_or_else(|| crate::error::AllSourceError::EntityNotFound(entity_id.clone()))?;

    tracing::debug!("Retrieved latest snapshot for entity: {}", entity_id);

    Ok(Json(serde_json::json!({
        "snapshot_id": snapshot.id,
        "entity_id": snapshot.entity_id,
        "created_at": snapshot.created_at,
        "as_of": snapshot.as_of,
        "event_count": snapshot.event_count,
        "size_bytes": snapshot.metadata.size_bytes,
        "snapshot_type": snapshot.metadata.snapshot_type,
        "state": snapshot.state
    })))
}

// v0.2: Trigger manual compaction
async fn trigger_compaction(State(store): State<SharedStore>) -> Result<Json<CompactionResult>> {
    let compaction_manager = store
        .compaction_manager()
        .ok_or_else(|| crate::error::AllSourceError::InternalError(
            "Compaction not enabled (no Parquet storage)".to_string()
        ))?;

    tracing::info!("📦 Manual compaction triggered via API");

    let result = compaction_manager.compact_now()?;

    Ok(Json(result))
}

// v0.2: Get compaction statistics
async fn compaction_stats(State(store): State<SharedStore>) -> Result<Json<serde_json::Value>> {
    let compaction_manager = store
        .compaction_manager()
        .ok_or_else(|| crate::error::AllSourceError::InternalError(
            "Compaction not enabled (no Parquet storage)".to_string()
        ))?;

    let stats = compaction_manager.stats();
    let config = compaction_manager.config();

    Ok(Json(serde_json::json!({
        "stats": stats,
        "config": {
            "min_files_to_compact": config.min_files_to_compact,
            "target_file_size": config.target_file_size,
            "max_file_size": config.max_file_size,
            "small_file_threshold": config.small_file_threshold,
            "compaction_interval_seconds": config.compaction_interval_seconds,
            "auto_compact": config.auto_compact,
            "strategy": config.strategy
        }
    })))
}
