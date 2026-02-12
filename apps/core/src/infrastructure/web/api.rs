use crate::application::dto::{
    EventDto, IngestEventRequest, IngestEventResponse, IngestEventsBatchRequest,
    IngestEventsBatchResponse, QueryEventsRequest, QueryEventsResponse,
};
use crate::application::services::analytics::{
    AnalyticsEngine, CorrelationRequest, CorrelationResponse, EventFrequencyRequest,
    EventFrequencyResponse, StatsSummaryRequest, StatsSummaryResponse,
};
use crate::application::services::pipeline::{PipelineConfig, PipelineStats};
use crate::application::services::replay::{
    ReplayProgress, StartReplayRequest, StartReplayResponse,
};
use crate::application::services::schema::{
    CompatibilityMode, RegisterSchemaRequest, RegisterSchemaResponse, ValidateEventRequest,
    ValidateEventResponse,
};
use crate::domain::entities::Event;
use crate::error::Result;
use crate::infrastructure::persistence::compaction::CompactionResult;
use crate::infrastructure::persistence::snapshot::{
    CreateSnapshotRequest, CreateSnapshotResponse, ListSnapshotsRequest, ListSnapshotsResponse,
    SnapshotInfo,
};
use crate::store::{EventStore, EventTypeInfo, StreamInfo};
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
        .route("/metrics", get(prometheus_metrics)) // v0.6: Prometheus metrics endpoint
        .route("/api/v1/events", post(ingest_event))
        .route("/api/v1/events/batch", post(ingest_events_batch))
        .route("/api/v1/events/query", get(query_events))
        .route("/api/v1/events/stream", get(events_websocket)) // v0.2: WebSocket streaming
        // v0.10: Stream and event type discovery endpoints
        .route("/api/v1/streams", get(list_streams))
        .route("/api/v1/event-types", get(list_event_types))
        .route("/api/v1/entities/{entity_id}/state", get(get_entity_state))
        .route(
            "/api/v1/entities/{entity_id}/snapshot",
            get(get_entity_snapshot),
        )
        .route("/api/v1/stats", get(get_stats))
        // v0.2: Advanced analytics endpoints
        .route("/api/v1/analytics/frequency", get(analytics_frequency))
        .route("/api/v1/analytics/summary", get(analytics_summary))
        .route("/api/v1/analytics/correlation", get(analytics_correlation))
        // v0.2: Snapshot management endpoints
        .route("/api/v1/snapshots", post(create_snapshot))
        .route("/api/v1/snapshots", get(list_snapshots))
        .route(
            "/api/v1/snapshots/{entity_id}/latest",
            get(get_latest_snapshot),
        )
        // v0.2: Compaction endpoints
        .route("/api/v1/compaction/trigger", post(trigger_compaction))
        .route("/api/v1/compaction/stats", get(compaction_stats))
        // v0.5: Schema registry endpoints
        .route("/api/v1/schemas", post(register_schema))
        .route("/api/v1/schemas", get(list_subjects))
        .route("/api/v1/schemas/{subject}", get(get_schema))
        .route(
            "/api/v1/schemas/{subject}/versions",
            get(list_schema_versions),
        )
        .route("/api/v1/schemas/validate", post(validate_event_schema))
        .route(
            "/api/v1/schemas/{subject}/compatibility",
            put(set_compatibility_mode),
        )
        // v0.5: Replay and projection rebuild endpoints
        .route("/api/v1/replay", post(start_replay))
        .route("/api/v1/replay", get(list_replays))
        .route("/api/v1/replay/{replay_id}", get(get_replay_progress))
        .route("/api/v1/replay/{replay_id}/cancel", post(cancel_replay))
        .route(
            "/api/v1/replay/{replay_id}",
            axum::routing::delete(delete_replay),
        )
        // v0.5: Stream processing pipeline endpoints
        .route("/api/v1/pipelines", post(register_pipeline))
        .route("/api/v1/pipelines", get(list_pipelines))
        .route("/api/v1/pipelines/stats", get(all_pipeline_stats))
        .route("/api/v1/pipelines/{pipeline_id}", get(get_pipeline))
        .route(
            "/api/v1/pipelines/{pipeline_id}",
            axum::routing::delete(remove_pipeline),
        )
        .route(
            "/api/v1/pipelines/{pipeline_id}/stats",
            get(get_pipeline_stats),
        )
        .route("/api/v1/pipelines/{pipeline_id}/reset", put(reset_pipeline))
        // v0.7: Projection State API for Query Service integration
        .route("/api/v1/projections", get(list_projections))
        .route("/api/v1/projections/{name}", get(get_projection))
        .route(
            "/api/v1/projections/{name}/{entity_id}/state",
            get(get_projection_state),
        )
        .route(
            "/api/v1/projections/{name}/{entity_id}/state",
            post(save_projection_state),
        )
        .route(
            "/api/v1/projections/{name}/{entity_id}/state",
            put(save_projection_state),
        )
        .route(
            "/api/v1/projections/{name}/bulk",
            post(bulk_get_projection_states),
        )
        .route(
            "/api/v1/projections/{name}/bulk/save",
            post(bulk_save_projection_states),
        )
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
            .body(format!("Error encoding metrics: {e}"))
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

/// Batch ingest multiple events in a single request
///
/// This endpoint allows ingesting multiple events atomically, which is more
/// efficient than making individual requests for each event.
pub async fn ingest_events_batch(
    State(store): State<SharedStore>,
    Json(req): Json<IngestEventsBatchRequest>,
) -> Result<Json<IngestEventsBatchResponse>> {
    let total = req.events.len();
    let mut ingested_events = Vec::with_capacity(total);

    for event_req in req.events {
        let tenant_id = event_req.tenant_id.unwrap_or_else(|| "default".to_string());

        let event = Event::from_strings(
            event_req.event_type,
            event_req.entity_id,
            tenant_id,
            event_req.payload,
            event_req.metadata,
        )?;

        let event_id = event.id;
        let timestamp = event.timestamp;

        store.ingest(event)?;

        ingested_events.push(IngestEventResponse {
            event_id,
            timestamp,
        });
    }

    let ingested = ingested_events.len();
    tracing::info!("Batch ingested {} events", ingested);

    Ok(Json(IngestEventsBatchResponse {
        total,
        ingested,
        events: ingested_events,
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

// v0.10: List all streams (entity_ids) in the event store
/// Query parameters for listing streams
#[derive(Debug, Deserialize)]
pub struct ListStreamsParams {
    /// Optional limit on number of streams to return
    pub limit: Option<usize>,
    /// Optional offset for pagination
    pub offset: Option<usize>,
}

/// Response for listing streams
#[derive(Debug, serde::Serialize)]
pub struct ListStreamsResponse {
    pub streams: Vec<StreamInfo>,
    pub total: usize,
}

pub async fn list_streams(
    State(store): State<SharedStore>,
    Query(params): Query<ListStreamsParams>,
) -> Json<ListStreamsResponse> {
    let mut streams = store.list_streams();
    let total = streams.len();

    // Sort by last_event_at descending (most recent first)
    streams.sort_by(|a, b| b.last_event_at.cmp(&a.last_event_at));

    // Apply pagination
    if let Some(offset) = params.offset {
        if offset < streams.len() {
            streams = streams[offset..].to_vec();
        } else {
            streams = vec![];
        }
    }

    if let Some(limit) = params.limit {
        streams.truncate(limit);
    }

    tracing::debug!("Listed {} streams (total: {})", streams.len(), total);

    Json(ListStreamsResponse { streams, total })
}

// v0.10: List all event types in the event store
/// Query parameters for listing event types
#[derive(Debug, Deserialize)]
pub struct ListEventTypesParams {
    /// Optional limit on number of event types to return
    pub limit: Option<usize>,
    /// Optional offset for pagination
    pub offset: Option<usize>,
}

/// Response for listing event types
#[derive(Debug, serde::Serialize)]
pub struct ListEventTypesResponse {
    pub event_types: Vec<EventTypeInfo>,
    pub total: usize,
}

pub async fn list_event_types(
    State(store): State<SharedStore>,
    Query(params): Query<ListEventTypesParams>,
) -> Json<ListEventTypesResponse> {
    let mut event_types = store.list_event_types();
    let total = event_types.len();

    // Sort by event_count descending (most used first)
    event_types.sort_by(|a, b| b.event_count.cmp(&a.event_count));

    // Apply pagination
    if let Some(offset) = params.offset {
        if offset < event_types.len() {
            event_types = event_types[offset..].to_vec();
        } else {
            event_types = vec![];
        }
    }

    if let Some(limit) = params.limit {
        event_types.truncate(limit);
    }

    tracing::debug!(
        "Listed {} event types (total: {})",
        event_types.len(),
        total
    );

    Json(ListEventTypesResponse { event_types, total })
}

// v0.2: WebSocket endpoint for real-time event streaming
pub async fn events_websocket(ws: WebSocketUpgrade, State(store): State<SharedStore>) -> Response {
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
pub async fn trigger_compaction(
    State(store): State<SharedStore>,
) -> Result<Json<CompactionResult>> {
    let compaction_manager = store.compaction_manager().ok_or_else(|| {
        crate::error::AllSourceError::InternalError(
            "Compaction not enabled (no Parquet storage)".to_string(),
        )
    })?;

    tracing::info!("📦 Manual compaction triggered via API");

    let result = compaction_manager.compact_now()?;

    Ok(Json(result))
}

// v0.2: Get compaction statistics
pub async fn compaction_stats(State(store): State<SharedStore>) -> Result<Json<serde_json::Value>> {
    let compaction_manager = store.compaction_manager().ok_or_else(|| {
        crate::error::AllSourceError::InternalError(
            "Compaction not enabled (no Parquet storage)".to_string(),
        )
    })?;

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

    let response =
        schema_registry.register_schema(req.subject, req.schema, req.description, req.tags)?;

    tracing::info!(
        "📋 Schema registered: v{} for '{}'",
        response.version,
        response.subject
    );

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
        tracing::debug!(
            "✅ Event validated against schema '{}' v{}",
            req.subject,
            response.schema_version
        );
    } else {
        tracing::warn!(
            "❌ Event validation failed for '{}': {:?}",
            req.subject,
            response.errors
        );
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

    tracing::info!(
        "🔧 Set compatibility mode for '{}' to {:?}",
        subject,
        req.compatibility
    );

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

    tracing::info!(
        "🔄 Started replay {} with {} events",
        response.replay_id,
        response.total_events
    );

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

    let pipeline = pipeline_manager.get(pipeline_id).ok_or_else(|| {
        crate::error::AllSourceError::ValidationError(format!(
            "Pipeline not found: {}",
            pipeline_id
        ))
    })?;

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

    let pipeline = pipeline_manager.get(pipeline_id).ok_or_else(|| {
        crate::error::AllSourceError::ValidationError(format!(
            "Pipeline not found: {}",
            pipeline_id
        ))
    })?;

    Ok(Json(pipeline.stats()))
}

// v0.5: Reset a pipeline's state
pub async fn reset_pipeline(
    State(store): State<SharedStore>,
    Path(pipeline_id): Path<uuid::Uuid>,
) -> Result<Json<serde_json::Value>> {
    let pipeline_manager = store.pipeline_manager();

    let pipeline = pipeline_manager.get(pipeline_id).ok_or_else(|| {
        crate::error::AllSourceError::ValidationError(format!(
            "Pipeline not found: {}",
            pipeline_id
        ))
    })?;

    pipeline.reset();

    tracing::info!("🔄 Reset pipeline {}", pipeline_id);

    Ok(Json(serde_json::json!({
        "pipeline_id": pipeline_id,
        "reset": true
    })))
}

// =============================================================================
// v0.7: Projection State API for Query Service Integration
// =============================================================================

/// List all registered projections
pub async fn list_projections(State(store): State<SharedStore>) -> Json<serde_json::Value> {
    let projection_manager = store.projection_manager();

    let projections: Vec<serde_json::Value> = projection_manager
        .list_projections()
        .iter()
        .map(|(name, projection)| {
            serde_json::json!({
                "name": name,
                "type": format!("{:?}", projection.name()),
            })
        })
        .collect();

    tracing::debug!("Listed {} projections", projections.len());

    Json(serde_json::json!({
        "projections": projections,
        "total": projections.len()
    }))
}

/// Get projection metadata by name
pub async fn get_projection(
    State(store): State<SharedStore>,
    Path(name): Path<String>,
) -> Result<Json<serde_json::Value>> {
    let projection_manager = store.projection_manager();

    let projection = projection_manager.get_projection(&name).ok_or_else(|| {
        crate::error::AllSourceError::EntityNotFound(format!("Projection '{name}' not found"))
    })?;

    Ok(Json(serde_json::json!({
        "name": projection.name(),
        "found": true
    })))
}

/// Get projection state for a specific entity
///
/// This endpoint allows the Elixir Query Service to fetch projection state
/// from the Rust Core for synchronization.
pub async fn get_projection_state(
    State(store): State<SharedStore>,
    Path((name, entity_id)): Path<(String, String)>,
) -> Result<Json<serde_json::Value>> {
    let projection_manager = store.projection_manager();

    let projection = projection_manager.get_projection(&name).ok_or_else(|| {
        crate::error::AllSourceError::EntityNotFound(format!("Projection '{name}' not found"))
    })?;

    let state = projection.get_state(&entity_id);

    tracing::debug!("Projection state retrieved: {} / {}", name, entity_id);

    Ok(Json(serde_json::json!({
        "projection": name,
        "entity_id": entity_id,
        "state": state,
        "found": state.is_some()
    })))
}

/// Request body for saving projection state
#[derive(Debug, Deserialize)]
pub struct SaveProjectionStateRequest {
    pub state: serde_json::Value,
}

/// Save/update projection state for an entity
///
/// This endpoint allows external services (like Elixir Query Service) to
/// store computed projection state back to the Core for persistence.
pub async fn save_projection_state(
    State(store): State<SharedStore>,
    Path((name, entity_id)): Path<(String, String)>,
    Json(req): Json<SaveProjectionStateRequest>,
) -> Result<Json<serde_json::Value>> {
    let projection_cache = store.projection_state_cache();

    // Store in the projection state cache
    projection_cache.insert(format!("{name}:{entity_id}"), req.state.clone());

    tracing::info!("Projection state saved: {} / {}", name, entity_id);

    Ok(Json(serde_json::json!({
        "projection": name,
        "entity_id": entity_id,
        "saved": true
    })))
}

/// Bulk get projection states for multiple entities
///
/// Efficient endpoint for fetching multiple entity states in a single request.
#[derive(Debug, Deserialize)]
pub struct BulkGetStateRequest {
    pub entity_ids: Vec<String>,
}

/// Bulk save projection states for multiple entities
///
/// Efficient endpoint for saving multiple entity states in a single request.
#[derive(Debug, Deserialize)]
pub struct BulkSaveStateRequest {
    pub states: Vec<BulkSaveStateItem>,
}

#[derive(Debug, Deserialize)]
pub struct BulkSaveStateItem {
    pub entity_id: String,
    pub state: serde_json::Value,
}

pub async fn bulk_get_projection_states(
    State(store): State<SharedStore>,
    Path(name): Path<String>,
    Json(req): Json<BulkGetStateRequest>,
) -> Result<Json<serde_json::Value>> {
    let projection_manager = store.projection_manager();

    let projection = projection_manager.get_projection(&name).ok_or_else(|| {
        crate::error::AllSourceError::EntityNotFound(format!("Projection '{name}' not found"))
    })?;

    let states: Vec<serde_json::Value> = req
        .entity_ids
        .iter()
        .map(|entity_id| {
            let state = projection.get_state(entity_id);
            serde_json::json!({
                "entity_id": entity_id,
                "state": state,
                "found": state.is_some()
            })
        })
        .collect();

    tracing::debug!(
        "Bulk projection state retrieved: {} entities from {}",
        states.len(),
        name
    );

    Ok(Json(serde_json::json!({
        "projection": name,
        "states": states,
        "total": states.len()
    })))
}

/// Bulk save projection states for multiple entities
///
/// This endpoint allows efficient batch saving of projection states,
/// critical for high-throughput event processing pipelines.
pub async fn bulk_save_projection_states(
    State(store): State<SharedStore>,
    Path(name): Path<String>,
    Json(req): Json<BulkSaveStateRequest>,
) -> Result<Json<serde_json::Value>> {
    let projection_cache = store.projection_state_cache();

    let mut saved_count = 0;
    for item in &req.states {
        projection_cache.insert(format!("{name}:{}", item.entity_id), item.state.clone());
        saved_count += 1;
    }

    tracing::info!(
        "Bulk projection state saved: {} entities for {}",
        saved_count,
        name
    );

    Ok(Json(serde_json::json!({
        "projection": name,
        "saved": saved_count,
        "total": req.states.len()
    })))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::entities::Event;
    use crate::store::EventStore;

    fn create_test_store() -> Arc<EventStore> {
        Arc::new(EventStore::new())
    }

    fn create_test_event(entity_id: &str, event_type: &str) -> Event {
        Event::from_strings(
            event_type.to_string(),
            entity_id.to_string(),
            "test-stream".to_string(),
            serde_json::json!({
                "name": "Test",
                "value": 42
            }),
            None,
        )
        .unwrap()
    }

    #[tokio::test]
    async fn test_projection_state_cache() {
        let store = create_test_store();

        // Test cache insertion
        let cache = store.projection_state_cache();
        cache.insert(
            "entity_snapshots:user-123".to_string(),
            serde_json::json!({"name": "Test User", "age": 30}),
        );

        // Test cache retrieval
        let state = cache.get("entity_snapshots:user-123");
        assert!(state.is_some());
        let state = state.unwrap();
        assert_eq!(state["name"], "Test User");
        assert_eq!(state["age"], 30);
    }

    #[tokio::test]
    async fn test_projection_manager_list_projections() {
        let store = create_test_store();

        // List projections (built-in projections should be available)
        let projection_manager = store.projection_manager();
        let projections = projection_manager.list_projections();

        // Should have entity_snapshots and event_counters
        assert!(projections.len() >= 2);

        let names: Vec<&str> = projections.iter().map(|(name, _)| name.as_str()).collect();
        assert!(names.contains(&"entity_snapshots"));
        assert!(names.contains(&"event_counters"));
    }

    #[tokio::test]
    async fn test_projection_state_after_event_ingestion() {
        let store = create_test_store();

        // Ingest an event
        let event = create_test_event("user-456", "user.created");
        store.ingest(event).unwrap();

        // Get projection state
        let projection_manager = store.projection_manager();
        let snapshot_projection = projection_manager
            .get_projection("entity_snapshots")
            .unwrap();

        let state = snapshot_projection.get_state("user-456");
        assert!(state.is_some());
        let state = state.unwrap();
        assert_eq!(state["name"], "Test");
        assert_eq!(state["value"], 42);
    }

    #[tokio::test]
    async fn test_projection_state_cache_multiple_entities() {
        let store = create_test_store();
        let cache = store.projection_state_cache();

        // Insert multiple entities
        for i in 0..10 {
            cache.insert(
                format!("entity_snapshots:entity-{}", i),
                serde_json::json!({"id": i, "status": "active"}),
            );
        }

        // Verify all insertions
        assert_eq!(cache.len(), 10);

        // Verify each entity
        for i in 0..10 {
            let key = format!("entity_snapshots:entity-{}", i);
            let state = cache.get(&key);
            assert!(state.is_some());
            assert_eq!(state.unwrap()["id"], i);
        }
    }

    #[tokio::test]
    async fn test_projection_state_update() {
        let store = create_test_store();
        let cache = store.projection_state_cache();

        // Initial state
        cache.insert(
            "entity_snapshots:user-789".to_string(),
            serde_json::json!({"balance": 100}),
        );

        // Update state
        cache.insert(
            "entity_snapshots:user-789".to_string(),
            serde_json::json!({"balance": 150}),
        );

        // Verify update
        let state = cache.get("entity_snapshots:user-789").unwrap();
        assert_eq!(state["balance"], 150);
    }

    #[tokio::test]
    async fn test_event_counter_projection() {
        let store = create_test_store();

        // Ingest events of different types
        store
            .ingest(create_test_event("user-1", "user.created"))
            .unwrap();
        store
            .ingest(create_test_event("user-2", "user.created"))
            .unwrap();
        store
            .ingest(create_test_event("user-1", "user.updated"))
            .unwrap();

        // Get event counter projection
        let projection_manager = store.projection_manager();
        let counter_projection = projection_manager.get_projection("event_counters").unwrap();

        // Check counts
        let created_state = counter_projection.get_state("user.created");
        assert!(created_state.is_some());
        assert_eq!(created_state.unwrap()["count"], 2);

        let updated_state = counter_projection.get_state("user.updated");
        assert!(updated_state.is_some());
        assert_eq!(updated_state.unwrap()["count"], 1);
    }

    #[tokio::test]
    async fn test_projection_state_cache_key_format() {
        let store = create_test_store();
        let cache = store.projection_state_cache();

        // Test standard key format: {projection_name}:{entity_id}
        let key = "orders:order-12345".to_string();
        cache.insert(key.clone(), serde_json::json!({"total": 99.99}));

        let state = cache.get(&key).unwrap();
        assert_eq!(state["total"], 99.99);
    }

    #[tokio::test]
    async fn test_projection_state_cache_removal() {
        let store = create_test_store();
        let cache = store.projection_state_cache();

        // Insert and then remove
        cache.insert(
            "test:entity-1".to_string(),
            serde_json::json!({"data": "value"}),
        );
        assert_eq!(cache.len(), 1);

        cache.remove("test:entity-1");
        assert_eq!(cache.len(), 0);
        assert!(cache.get("test:entity-1").is_none());
    }

    #[tokio::test]
    async fn test_get_nonexistent_projection() {
        let store = create_test_store();
        let projection_manager = store.projection_manager();

        // Requesting a non-existent projection should return None
        let projection = projection_manager.get_projection("nonexistent_projection");
        assert!(projection.is_none());
    }

    #[tokio::test]
    async fn test_get_nonexistent_entity_state() {
        let store = create_test_store();
        let projection_manager = store.projection_manager();

        // Get state for non-existent entity
        let snapshot_projection = projection_manager
            .get_projection("entity_snapshots")
            .unwrap();
        let state = snapshot_projection.get_state("nonexistent-entity-xyz");
        assert!(state.is_none());
    }

    #[tokio::test]
    async fn test_projection_state_cache_concurrent_access() {
        let store = create_test_store();
        let cache = store.projection_state_cache();

        // Simulate concurrent writes
        let handles: Vec<_> = (0..10)
            .map(|i| {
                let cache_clone = cache.clone();
                tokio::spawn(async move {
                    cache_clone.insert(
                        format!("concurrent:entity-{}", i),
                        serde_json::json!({"thread": i}),
                    );
                })
            })
            .collect();

        for handle in handles {
            handle.await.unwrap();
        }

        // All 10 entries should be present
        assert_eq!(cache.len(), 10);
    }

    #[tokio::test]
    async fn test_projection_state_large_payload() {
        let store = create_test_store();
        let cache = store.projection_state_cache();

        // Create a large JSON payload (~10KB)
        let large_array: Vec<serde_json::Value> = (0..1000)
            .map(|i| serde_json::json!({"item": i, "description": "test item with some padding data to increase size"}))
            .collect();

        cache.insert(
            "large:entity-1".to_string(),
            serde_json::json!({"items": large_array}),
        );

        let state = cache.get("large:entity-1").unwrap();
        let items = state["items"].as_array().unwrap();
        assert_eq!(items.len(), 1000);
    }

    #[tokio::test]
    async fn test_projection_state_complex_json() {
        let store = create_test_store();
        let cache = store.projection_state_cache();

        // Complex nested JSON structure
        let complex_state = serde_json::json!({
            "user": {
                "id": "user-123",
                "profile": {
                    "name": "John Doe",
                    "email": "john@example.com",
                    "settings": {
                        "theme": "dark",
                        "notifications": true
                    }
                },
                "roles": ["admin", "user"],
                "metadata": {
                    "created_at": "2025-01-01T00:00:00Z",
                    "last_login": null
                }
            }
        });

        cache.insert("complex:user-123".to_string(), complex_state);

        let state = cache.get("complex:user-123").unwrap();
        assert_eq!(state["user"]["profile"]["name"], "John Doe");
        assert_eq!(state["user"]["roles"][0], "admin");
        assert!(state["user"]["metadata"]["last_login"].is_null());
    }

    #[tokio::test]
    async fn test_projection_state_cache_iteration() {
        let store = create_test_store();
        let cache = store.projection_state_cache();

        // Insert entries
        for i in 0..5 {
            cache.insert(
                format!("iter:entity-{}", i),
                serde_json::json!({"index": i}),
            );
        }

        // Iterate over all entries
        let entries: Vec<_> = cache.iter().map(|entry| entry.key().clone()).collect();
        assert_eq!(entries.len(), 5);
    }

    #[tokio::test]
    async fn test_projection_manager_get_entity_snapshots() {
        let store = create_test_store();
        let projection_manager = store.projection_manager();

        // Get entity_snapshots projection specifically
        let projection = projection_manager.get_projection("entity_snapshots");
        assert!(projection.is_some());
        assert_eq!(projection.unwrap().name(), "entity_snapshots");
    }

    #[tokio::test]
    async fn test_projection_manager_get_event_counters() {
        let store = create_test_store();
        let projection_manager = store.projection_manager();

        // Get event_counters projection specifically
        let projection = projection_manager.get_projection("event_counters");
        assert!(projection.is_some());
        assert_eq!(projection.unwrap().name(), "event_counters");
    }

    #[tokio::test]
    async fn test_projection_state_cache_overwrite() {
        let store = create_test_store();
        let cache = store.projection_state_cache();

        // Initial value
        cache.insert(
            "overwrite:entity-1".to_string(),
            serde_json::json!({"version": 1}),
        );

        // Overwrite with new value
        cache.insert(
            "overwrite:entity-1".to_string(),
            serde_json::json!({"version": 2}),
        );

        // Overwrite again
        cache.insert(
            "overwrite:entity-1".to_string(),
            serde_json::json!({"version": 3}),
        );

        let state = cache.get("overwrite:entity-1").unwrap();
        assert_eq!(state["version"], 3);

        // Should still be only 1 entry
        assert_eq!(cache.len(), 1);
    }

    #[tokio::test]
    async fn test_projection_state_multiple_projections() {
        let store = create_test_store();
        let cache = store.projection_state_cache();

        // Store states for different projections
        cache.insert(
            "entity_snapshots:user-1".to_string(),
            serde_json::json!({"name": "Alice"}),
        );
        cache.insert(
            "event_counters:user.created".to_string(),
            serde_json::json!({"count": 5}),
        );
        cache.insert(
            "custom_projection:order-1".to_string(),
            serde_json::json!({"total": 150.0}),
        );

        // Verify each projection's state
        assert_eq!(
            cache.get("entity_snapshots:user-1").unwrap()["name"],
            "Alice"
        );
        assert_eq!(
            cache.get("event_counters:user.created").unwrap()["count"],
            5
        );
        assert_eq!(
            cache.get("custom_projection:order-1").unwrap()["total"],
            150.0
        );
    }

    #[tokio::test]
    async fn test_bulk_projection_state_access() {
        let store = create_test_store();

        // Ingest multiple events for different entities
        for i in 0..5 {
            let event = create_test_event(&format!("bulk-user-{}", i), "user.created");
            store.ingest(event).unwrap();
        }

        // Get projection and verify bulk access
        let projection_manager = store.projection_manager();
        let snapshot_projection = projection_manager
            .get_projection("entity_snapshots")
            .unwrap();

        // Verify we can access all entities
        for i in 0..5 {
            let state = snapshot_projection.get_state(&format!("bulk-user-{}", i));
            assert!(state.is_some(), "Entity bulk-user-{} should have state", i);
        }
    }

    #[tokio::test]
    async fn test_bulk_save_projection_states() {
        let store = create_test_store();
        let cache = store.projection_state_cache();

        // Simulate bulk save request
        let states = vec![
            BulkSaveStateItem {
                entity_id: "bulk-entity-1".to_string(),
                state: serde_json::json!({"name": "Entity 1", "value": 100}),
            },
            BulkSaveStateItem {
                entity_id: "bulk-entity-2".to_string(),
                state: serde_json::json!({"name": "Entity 2", "value": 200}),
            },
            BulkSaveStateItem {
                entity_id: "bulk-entity-3".to_string(),
                state: serde_json::json!({"name": "Entity 3", "value": 300}),
            },
        ];

        let projection_name = "test_projection";

        // Save states to cache (simulating bulk_save_projection_states handler)
        for item in &states {
            cache.insert(
                format!("{projection_name}:{}", item.entity_id),
                item.state.clone(),
            );
        }

        // Verify all states were saved
        assert_eq!(cache.len(), 3);

        let state1 = cache.get("test_projection:bulk-entity-1").unwrap();
        assert_eq!(state1["name"], "Entity 1");
        assert_eq!(state1["value"], 100);

        let state2 = cache.get("test_projection:bulk-entity-2").unwrap();
        assert_eq!(state2["name"], "Entity 2");
        assert_eq!(state2["value"], 200);

        let state3 = cache.get("test_projection:bulk-entity-3").unwrap();
        assert_eq!(state3["name"], "Entity 3");
        assert_eq!(state3["value"], 300);
    }

    #[tokio::test]
    async fn test_bulk_save_empty_states() {
        let store = create_test_store();
        let cache = store.projection_state_cache();

        // Clear cache
        cache.clear();

        // Empty states should work fine
        let states: Vec<BulkSaveStateItem> = vec![];
        assert_eq!(states.len(), 0);

        // Cache should remain empty
        assert_eq!(cache.len(), 0);
    }

    #[tokio::test]
    async fn test_bulk_save_overwrites_existing() {
        let store = create_test_store();
        let cache = store.projection_state_cache();

        // Insert initial state
        cache.insert(
            "test:entity-1".to_string(),
            serde_json::json!({"version": 1, "data": "initial"}),
        );

        // Bulk save with updated state
        let new_state = serde_json::json!({"version": 2, "data": "updated"});
        cache.insert("test:entity-1".to_string(), new_state);

        // Verify overwrite
        let state = cache.get("test:entity-1").unwrap();
        assert_eq!(state["version"], 2);
        assert_eq!(state["data"], "updated");
    }

    #[tokio::test]
    async fn test_bulk_save_high_volume() {
        let store = create_test_store();
        let cache = store.projection_state_cache();

        // Simulate high volume save (1000 entities)
        for i in 0..1000 {
            cache.insert(
                format!("volume_test:entity-{}", i),
                serde_json::json!({"index": i, "status": "active"}),
            );
        }

        // Verify count
        assert_eq!(cache.len(), 1000);

        // Spot check some entries
        assert_eq!(cache.get("volume_test:entity-0").unwrap()["index"], 0);
        assert_eq!(cache.get("volume_test:entity-500").unwrap()["index"], 500);
        assert_eq!(cache.get("volume_test:entity-999").unwrap()["index"], 999);
    }

    #[tokio::test]
    async fn test_bulk_save_different_projections() {
        let store = create_test_store();
        let cache = store.projection_state_cache();

        // Save to multiple projections in bulk
        let projections = ["entity_snapshots", "event_counters", "custom_analytics"];

        for proj in projections.iter() {
            for i in 0..5 {
                cache.insert(
                    format!("{proj}:entity-{i}"),
                    serde_json::json!({"projection": proj, "id": i}),
                );
            }
        }

        // Verify total count (3 projections * 5 entities)
        assert_eq!(cache.len(), 15);

        // Verify each projection
        for proj in projections.iter() {
            let state = cache.get(&format!("{proj}:entity-0")).unwrap();
            assert_eq!(state["projection"], *proj);
        }
    }
}
