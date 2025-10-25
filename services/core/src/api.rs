use crate::analytics::{
    AnalyticsEngine, CorrelationRequest, CorrelationResponse, EventFrequencyRequest,
    EventFrequencyResponse, StatsSummaryRequest, StatsSummaryResponse,
};
use crate::compaction::CompactionResult;
use crate::domain::entities::Event;
use crate::error::Result;
use crate::application::dto::{IngestEventRequest, IngestEventResponse, QueryEventsRequest, QueryEventsResponse, EventDto};
use crate::pipeline::{PipelineConfig, PipelineStats};
use crate::replay::{ReplayProgress, StartReplayRequest, StartReplayResponse};
use crate::schema::{
    CompatibilityMode, RegisterSchemaRequest, RegisterSchemaResponse, ValidateEventRequest,
    ValidateEventResponse,
};
use crate::snapshot::{
    CreateSnapshotRequest, CreateSnapshotResponse, ListSnapshotsRequest, ListSnapshotsResponse,
    SnapshotInfo,
};
use crate::store::EventStore;
use axum::{
    extract::{Path, Query, State, WebSocketUpgrade},
    response::{IntoResponse, Response},
    routing::{get, post, put},
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
        .route("/metrics", get(prometheus_metrics))  // v0.6: Prometheus metrics endpoint
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
        // v0.5: Schema registry endpoints
        .route("/api/v1/schemas", post(register_schema))
        .route("/api/v1/schemas", get(list_subjects))
        .route("/api/v1/schemas/:subject", get(get_schema))
        .route("/api/v1/schemas/:subject/versions", get(list_schema_versions))
        .route("/api/v1/schemas/validate", post(validate_event_schema))
        .route("/api/v1/schemas/:subject/compatibility", put(set_compatibility_mode))
        // v0.5: Replay and projection rebuild endpoints
        .route("/api/v1/replay", post(start_replay))
        .route("/api/v1/replay", get(list_replays))
        .route("/api/v1/replay/:replay_id", get(get_replay_progress))
        .route("/api/v1/replay/:replay_id/cancel", post(cancel_replay))
        .route("/api/v1/replay/:replay_id", axum::routing::delete(delete_replay))
        // v0.5: Stream processing pipeline endpoints
        .route("/api/v1/pipelines", post(register_pipeline))
        .route("/api/v1/pipelines", get(list_pipelines))
        .route("/api/v1/pipelines/stats", get(all_pipeline_stats))
        .route("/api/v1/pipelines/:pipeline_id", get(get_pipeline))
        .route("/api/v1/pipelines/:pipeline_id", axum::routing::delete(remove_pipeline))
        .route("/api/v1/pipelines/:pipeline_id/stats", get(get_pipeline_stats))
        .route("/api/v1/pipelines/:pipeline_id/reset", put(reset_pipeline))
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

pub async fn health() -> impl IntoResponse {
    Json(serde_json::json!({
        "status": "healthy",
        "service": "allsource-core",
        "version": env!("CARGO_PKG_VERSION")
    }))
}

// v0.6: Prometheus metrics endpoint
pub async fn prometheus_metrics(State(store): State<SharedStore>) -> impl IntoResponse {
    let metrics = store.metrics();

    match metrics.encode() {
        Ok(encoded) => Response::builder()
            .status(200)
            .header("Content-Type", "text/plain; version=0.0.4")
            .body(encoded)
            .unwrap()
            .into_response(),
        Err(e) => Response::builder()
            .status(500)
            .body(format!("Error encoding metrics: {}", e))
            .unwrap()
            .into_response(),
    }
}

pub async fn ingest_event(
    State(store): State<SharedStore>,
    Json(req): Json<IngestEventRequest>,
) -> Result<Json<IngestEventResponse>> {
    // Create event using from_strings with default tenant
    let event = Event::from_strings(
        req.event_type,
        req.entity_id,
        "default".to_string(),
        req.payload,
        req.metadata,
    )?;

    let event_id = event.id;
    let timestamp = event.timestamp;

    store.ingest(event)?;

    tracing::info!("Event ingested: {}", event_id);

    Ok(Json(IngestEventResponse {
        event_id,
        timestamp,
    }))
}

pub async fn query_events(
    State(store): State<SharedStore>,
    Query(req): Query<QueryEventsRequest>,
) -> Result<Json<QueryEventsResponse>> {
    let domain_events = store.query(req)?;
    let events: Vec<EventDto> = domain_events.iter().map(EventDto::from).collect();
    let count = events.len();

    tracing::debug!("Query returned {} events", count);

    Ok(Json(QueryEventsResponse { events, count }))
}

#[derive(Deserialize)]
pub struct EntityStateParams {
    as_of: Option<chrono::DateTime<chrono::Utc>>,
}

pub async fn get_entity_state(
    State(store): State<SharedStore>,
    Path(entity_id): Path<String>,
    Query(params): Query<EntityStateParams>,
) -> Result<Json<serde_json::Value>> {
    let state = store.reconstruct_state(&entity_id, params.as_of)?;

    tracing::info!("State reconstructed for entity: {}", entity_id);

    Ok(Json(state))
}

pub async fn get_entity_snapshot(
    State(store): State<SharedStore>,
    Path(entity_id): Path<String>,
) -> Result<Json<serde_json::Value>> {
    let snapshot = store.get_snapshot(&entity_id)?;

    tracing::debug!("Snapshot retrieved for entity: {}", entity_id);

    Ok(Json(snapshot))
}

pub async fn get_stats(State(store): State<SharedStore>) -> impl IntoResponse {
    let stats = store.stats();
    Json(stats)
}

// v0.2: WebSocket endpoint for real-time event streaming
pub async fn events_websocket(
    ws: WebSocketUpgrade,
    State(store): State<SharedStore>,
) -> Response {
    let websocket_manager = store.websocket_manager();

    ws.on_upgrade(move |socket| async move {
        websocket_manager.handle_socket(socket).await;
    })
}

// v0.2: Event frequency analytics endpoint
pub async fn analytics_frequency(
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
pub async fn analytics_summary(
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
pub async fn analytics_correlation(
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
pub async fn create_snapshot(
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
pub async fn list_snapshots(
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
pub async fn get_latest_snapshot(
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
pub async fn trigger_compaction(State(store): State<SharedStore>) -> Result<Json<CompactionResult>> {
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
pub async fn compaction_stats(State(store): State<SharedStore>) -> Result<Json<serde_json::Value>> {
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

// v0.5: Register a new schema
pub async fn register_schema(
    State(store): State<SharedStore>,
    Json(req): Json<RegisterSchemaRequest>,
) -> Result<Json<RegisterSchemaResponse>> {
    let schema_registry = store.schema_registry();

    let response = schema_registry.register_schema(
        req.subject,
        req.schema,
        req.description,
        req.tags,
    )?;

    tracing::info!("📋 Schema registered: v{} for '{}'", response.version, response.subject);

    Ok(Json(response))
}

// v0.5: Get a schema by subject and optional version
#[derive(Deserialize)]
pub struct GetSchemaParams {
    version: Option<u32>,
}

pub async fn get_schema(
    State(store): State<SharedStore>,
    Path(subject): Path<String>,
    Query(params): Query<GetSchemaParams>,
) -> Result<Json<serde_json::Value>> {
    let schema_registry = store.schema_registry();

    let schema = schema_registry.get_schema(&subject, params.version)?;

    tracing::debug!("Retrieved schema v{} for '{}'", schema.version, subject);

    Ok(Json(serde_json::json!({
        "id": schema.id,
        "subject": schema.subject,
        "version": schema.version,
        "schema": schema.schema,
        "created_at": schema.created_at,
        "description": schema.description,
        "tags": schema.tags
    })))
}

// v0.5: List all versions of a schema subject
pub async fn list_schema_versions(
    State(store): State<SharedStore>,
    Path(subject): Path<String>,
) -> Result<Json<serde_json::Value>> {
    let schema_registry = store.schema_registry();

    let versions = schema_registry.list_versions(&subject)?;

    Ok(Json(serde_json::json!({
        "subject": subject,
        "versions": versions
    })))
}

// v0.5: List all schema subjects
pub async fn list_subjects(State(store): State<SharedStore>) -> Json<serde_json::Value> {
    let schema_registry = store.schema_registry();

    let subjects = schema_registry.list_subjects();

    Json(serde_json::json!({
        "subjects": subjects,
        "total": subjects.len()
    }))
}

// v0.5: Validate an event against a schema
pub async fn validate_event_schema(
    State(store): State<SharedStore>,
    Json(req): Json<ValidateEventRequest>,
) -> Result<Json<ValidateEventResponse>> {
    let schema_registry = store.schema_registry();

    let response = schema_registry.validate(&req.subject, req.version, &req.payload)?;

    if response.valid {
        tracing::debug!("✅ Event validated against schema '{}' v{}", req.subject, response.schema_version);
    } else {
        tracing::warn!("❌ Event validation failed for '{}': {:?}", req.subject, response.errors);
    }

    Ok(Json(response))
}

// v0.5: Set compatibility mode for a subject
#[derive(Deserialize)]
pub struct SetCompatibilityRequest {
    compatibility: CompatibilityMode,
}

pub async fn set_compatibility_mode(
    State(store): State<SharedStore>,
    Path(subject): Path<String>,
    Json(req): Json<SetCompatibilityRequest>,
) -> Json<serde_json::Value> {
    let schema_registry = store.schema_registry();

    schema_registry.set_compatibility_mode(subject.clone(), req.compatibility);

    tracing::info!("🔧 Set compatibility mode for '{}' to {:?}", subject, req.compatibility);

    Json(serde_json::json!({
        "subject": subject,
        "compatibility": req.compatibility
    }))
}

// v0.5: Start a replay operation
pub async fn start_replay(
    State(store): State<SharedStore>,
    Json(req): Json<StartReplayRequest>,
) -> Result<Json<StartReplayResponse>> {
    let replay_manager = store.replay_manager();

    let response = replay_manager.start_replay(store, req)?;

    tracing::info!("🔄 Started replay {} with {} events", response.replay_id, response.total_events);

    Ok(Json(response))
}

// v0.5: Get replay progress
pub async fn get_replay_progress(
    State(store): State<SharedStore>,
    Path(replay_id): Path<uuid::Uuid>,
) -> Result<Json<ReplayProgress>> {
    let replay_manager = store.replay_manager();

    let progress = replay_manager.get_progress(replay_id)?;

    Ok(Json(progress))
}

// v0.5: List all replay operations
pub async fn list_replays(State(store): State<SharedStore>) -> Json<serde_json::Value> {
    let replay_manager = store.replay_manager();

    let replays = replay_manager.list_replays();

    Json(serde_json::json!({
        "replays": replays,
        "total": replays.len()
    }))
}

// v0.5: Cancel a running replay
pub async fn cancel_replay(
    State(store): State<SharedStore>,
    Path(replay_id): Path<uuid::Uuid>,
) -> Result<Json<serde_json::Value>> {
    let replay_manager = store.replay_manager();

    replay_manager.cancel_replay(replay_id)?;

    tracing::info!("🛑 Cancelled replay {}", replay_id);

    Ok(Json(serde_json::json!({
        "replay_id": replay_id,
        "status": "cancelled"
    })))
}

// v0.5: Delete a completed replay
pub async fn delete_replay(
    State(store): State<SharedStore>,
    Path(replay_id): Path<uuid::Uuid>,
) -> Result<Json<serde_json::Value>> {
    let replay_manager = store.replay_manager();

    let deleted = replay_manager.delete_replay(replay_id)?;

    if deleted {
        tracing::info!("🗑️  Deleted replay {}", replay_id);
    }

    Ok(Json(serde_json::json!({
        "replay_id": replay_id,
        "deleted": deleted
    })))
}

// v0.5: Register a new pipeline
pub async fn register_pipeline(
    State(store): State<SharedStore>,
    Json(config): Json<PipelineConfig>,
) -> Result<Json<serde_json::Value>> {
    let pipeline_manager = store.pipeline_manager();

    let pipeline_id = pipeline_manager.register(config.clone());

    tracing::info!(
        "🔀 Pipeline registered: {} (name: {})",
        pipeline_id,
        config.name
    );

    Ok(Json(serde_json::json!({
        "pipeline_id": pipeline_id,
        "name": config.name,
        "enabled": config.enabled
    })))
}

// v0.5: List all pipelines
pub async fn list_pipelines(State(store): State<SharedStore>) -> Json<serde_json::Value> {
    let pipeline_manager = store.pipeline_manager();

    let pipelines = pipeline_manager.list();

    tracing::debug!("Listed {} pipelines", pipelines.len());

    Json(serde_json::json!({
        "pipelines": pipelines,
        "total": pipelines.len()
    }))
}

// v0.5: Get a specific pipeline
pub async fn get_pipeline(
    State(store): State<SharedStore>,
    Path(pipeline_id): Path<uuid::Uuid>,
) -> Result<Json<PipelineConfig>> {
    let pipeline_manager = store.pipeline_manager();

    let pipeline = pipeline_manager
        .get(pipeline_id)
        .ok_or_else(|| crate::error::AllSourceError::ValidationError(
            format!("Pipeline not found: {}", pipeline_id)
        ))?;

    Ok(Json(pipeline.config().clone()))
}

// v0.5: Remove a pipeline
pub async fn remove_pipeline(
    State(store): State<SharedStore>,
    Path(pipeline_id): Path<uuid::Uuid>,
) -> Result<Json<serde_json::Value>> {
    let pipeline_manager = store.pipeline_manager();

    let removed = pipeline_manager.remove(pipeline_id);

    if removed {
        tracing::info!("🗑️  Removed pipeline {}", pipeline_id);
    }

    Ok(Json(serde_json::json!({
        "pipeline_id": pipeline_id,
        "removed": removed
    })))
}

// v0.5: Get statistics for all pipelines
pub async fn all_pipeline_stats(State(store): State<SharedStore>) -> Json<serde_json::Value> {
    let pipeline_manager = store.pipeline_manager();

    let stats = pipeline_manager.all_stats();

    Json(serde_json::json!({
        "stats": stats,
        "total": stats.len()
    }))
}

// v0.5: Get statistics for a specific pipeline
pub async fn get_pipeline_stats(
    State(store): State<SharedStore>,
    Path(pipeline_id): Path<uuid::Uuid>,
) -> Result<Json<PipelineStats>> {
    let pipeline_manager = store.pipeline_manager();

    let pipeline = pipeline_manager
        .get(pipeline_id)
        .ok_or_else(|| crate::error::AllSourceError::ValidationError(
            format!("Pipeline not found: {}", pipeline_id)
        ))?;

    Ok(Json(pipeline.stats()))
}

// v0.5: Reset a pipeline's state
pub async fn reset_pipeline(
    State(store): State<SharedStore>,
    Path(pipeline_id): Path<uuid::Uuid>,
) -> Result<Json<serde_json::Value>> {
    let pipeline_manager = store.pipeline_manager();

    let pipeline = pipeline_manager
        .get(pipeline_id)
        .ok_or_else(|| crate::error::AllSourceError::ValidationError(
            format!("Pipeline not found: {}", pipeline_id)
        ))?;

    pipeline.reset();

    tracing::info!("🔄 Reset pipeline {}", pipeline_id);

    Ok(Json(serde_json::json!({
        "pipeline_id": pipeline_id,
        "reset": true
    })))
}
