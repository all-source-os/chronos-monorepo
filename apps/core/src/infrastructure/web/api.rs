#[cfg(feature = "replication")]
use crate::infrastructure::replication::ReplicationMode;
use crate::{
    application::{
        dto::{
            AckRequest, ConsumerEventDto, ConsumerEventsResponse, ConsumerResponse,
            DetectDuplicatesRequest, DetectDuplicatesResponse, DuplicateGroup, EntitySummary,
            EventDto, IngestEventRequest, IngestEventResponse, IngestEventsBatchRequest,
            IngestEventsBatchResponse, ListEntitiesRequest, ListEntitiesResponse,
            QueryEventsRequest, QueryEventsResponse, RegisterConsumerRequest,
        },
        services::{
            analytics::{
                AnalyticsEngine, CorrelationRequest, CorrelationResponse, EventFrequencyRequest,
                EventFrequencyResponse, StatsSummaryRequest, StatsSummaryResponse,
            },
            pipeline::{PipelineConfig, PipelineStats},
            replay::{ReplayProgress, StartReplayRequest, StartReplayResponse},
            schema::{
                CompatibilityMode, RegisterSchemaRequest, RegisterSchemaResponse,
                ValidateEventRequest, ValidateEventResponse,
            },
            webhook::{RegisterWebhookRequest, UpdateWebhookRequest},
        },
    },
    domain::entities::Event,
    error::Result,
    infrastructure::{
        persistence::{
            compaction::CompactionResult,
            snapshot::{
                CreateSnapshotRequest, CreateSnapshotResponse, ListSnapshotsRequest,
                ListSnapshotsResponse, SnapshotInfo,
            },
        },
        query::{
            geospatial::GeoQueryRequest,
            graphql::{GraphQLError, GraphQLRequest, GraphQLResponse},
        },
        security::middleware::OptionalAuth,
        web::api_v1::AppState,
    },
    store::{EventStore, EventTypeInfo, StreamInfo},
};
use axum::{
    Json, Router,
    extract::{Path, Query, State, WebSocketUpgrade},
    response::{IntoResponse, Response},
    routing::{get, post, put},
};
use serde::Deserialize;
use std::sync::Arc;
use tower_http::{
    cors::{Any, CorsLayer},
    trace::TraceLayer,
};

type SharedStore = Arc<EventStore>;

/// Wait for follower ACK(s) in semi-sync/sync replication modes.
///
/// In async mode (default), returns immediately. In semi-sync mode, waits for
/// at least 1 follower to ACK the current WAL offset. In sync mode, waits for
/// all followers. If the timeout expires, logs a warning and continues (degraded mode).
#[cfg(feature = "replication")]
async fn await_replication_ack(state: &AppState) {
    let shipper_guard = state.wal_shipper.read().await;
    if let Some(ref shipper) = *shipper_guard {
        let mode = shipper.replication_mode();
        if mode == ReplicationMode::Async {
            return;
        }

        let target_offset = shipper.current_leader_offset();
        if target_offset == 0 {
            return;
        }

        let shipper = Arc::clone(shipper);
        // Drop the read guard before the async wait to avoid holding it across await
        drop(shipper_guard);

        let timer = state
            .store
            .metrics()
            .replication_ack_wait_seconds
            .start_timer();
        let acked = shipper.wait_for_ack(target_offset).await;
        timer.observe_duration();

        if !acked {
            tracing::warn!(
                "Replication ACK timeout in {} mode (offset {}). \
                 Write succeeded locally but follower confirmation pending.",
                mode,
                target_offset,
            );
        }
    }
}
#[cfg(not(feature = "replication"))]
async fn await_replication_ack(_state: &AppState) {
    // No-op in community edition
}

pub async fn serve(store: SharedStore, addr: &str) -> anyhow::Result<()> {
    let app = Router::new()
        .route("/health", get(health))
        .route("/metrics", get(prometheus_metrics)) // v0.6: Prometheus metrics endpoint
        .route("/api/v1/events", post(ingest_event))
        .route("/api/v1/events/batch", post(ingest_events_batch))
        .route("/api/v1/events/query", get(query_events))
        .route("/api/v1/events/{event_id}", get(get_event_by_id))
        .route("/api/v1/events/stream", get(events_websocket)) // v0.2: WebSocket streaming
        // v0.10: Stream and event type discovery endpoints
        .route("/api/v1/streams", get(list_streams))
        .route("/api/v1/event-types", get(list_event_types))
        .route("/api/v1/entities/duplicates", get(detect_duplicates))
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
            "/api/v1/projections/{name}",
            axum::routing::delete(delete_projection),
        )
        .route(
            "/api/v1/projections/{name}/state",
            get(get_projection_state_summary),
        )
        .route("/api/v1/projections/{name}/reset", post(reset_projection))
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
        // v0.11: Webhook management endpoints
        .route("/api/v1/webhooks", post(register_webhook))
        .route("/api/v1/webhooks", get(list_webhooks))
        .route("/api/v1/webhooks/{webhook_id}", get(get_webhook))
        .route("/api/v1/webhooks/{webhook_id}", put(update_webhook))
        .route(
            "/api/v1/webhooks/{webhook_id}",
            axum::routing::delete(delete_webhook),
        )
        .route(
            "/api/v1/webhooks/{webhook_id}/deliveries",
            get(list_webhook_deliveries),
        )
        // v2.0: Advanced query features
        .route("/api/v1/graphql", post(graphql_query))
        .route("/api/v1/geospatial/query", post(geo_query))
        .route("/api/v1/geospatial/stats", get(geo_stats))
        .route("/api/v1/exactly-once/stats", get(exactly_once_stats))
        .route(
            "/api/v1/schema-evolution/history/{event_type}",
            get(schema_evolution_history),
        )
        .route(
            "/api/v1/schema-evolution/schema/{event_type}",
            get(schema_evolution_schema),
        )
        .route(
            "/api/v1/schema-evolution/stats",
            get(schema_evolution_stats),
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
    let expected_version = req.expected_version;

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

    let new_version = store.ingest_with_expected_version(&event, expected_version)?;

    tracing::info!("Event ingested: {}", event_id);

    Ok(Json(IngestEventResponse {
        event_id,
        timestamp,
        version: Some(new_version),
    }))
}

/// Ingest a single event with semi-sync/sync replication ACK waiting.
///
/// Used by the v1 API (with auth and replication support).
pub async fn ingest_event_v1(
    State(state): State<AppState>,
    Json(req): Json<IngestEventRequest>,
) -> Result<Json<IngestEventResponse>> {
    let expected_version = req.expected_version;

    let event = Event::from_strings(
        req.event_type,
        req.entity_id,
        "default".to_string(),
        req.payload,
        req.metadata,
    )?;

    let event_id = event.id;
    let timestamp = event.timestamp;

    let new_version = state
        .store
        .ingest_with_expected_version(&event, expected_version)?;

    // Semi-sync/sync: wait for follower ACK(s) before returning
    await_replication_ack(&state).await;

    tracing::info!("Event ingested: {}", event_id);

    Ok(Json(IngestEventResponse {
        event_id,
        timestamp,
        version: Some(new_version),
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
        let expected_version = event_req.expected_version;

        let event = Event::from_strings(
            event_req.event_type,
            event_req.entity_id,
            tenant_id,
            event_req.payload,
            event_req.metadata,
        )?;

        let event_id = event.id;
        let timestamp = event.timestamp;

        let new_version = store.ingest_with_expected_version(&event, expected_version)?;

        ingested_events.push(IngestEventResponse {
            event_id,
            timestamp,
            version: Some(new_version),
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

/// Batch ingest with semi-sync/sync replication ACK waiting.
///
/// Used by the v1 API (with auth and replication support).
pub async fn ingest_events_batch_v1(
    State(state): State<AppState>,
    Json(req): Json<IngestEventsBatchRequest>,
) -> Result<Json<IngestEventsBatchResponse>> {
    let total = req.events.len();
    let mut ingested_events = Vec::with_capacity(total);

    for event_req in req.events {
        let tenant_id = event_req.tenant_id.unwrap_or_else(|| "default".to_string());
        let expected_version = event_req.expected_version;

        let event = Event::from_strings(
            event_req.event_type,
            event_req.entity_id,
            tenant_id,
            event_req.payload,
            event_req.metadata,
        )?;

        let event_id = event.id;
        let timestamp = event.timestamp;

        let new_version = state
            .store
            .ingest_with_expected_version(&event, expected_version)?;

        ingested_events.push(IngestEventResponse {
            event_id,
            timestamp,
            version: Some(new_version),
        });
    }

    // Semi-sync/sync: wait for follower ACK(s) after all events are ingested
    await_replication_ack(&state).await;

    let ingested = ingested_events.len();
    tracing::info!("Batch ingested {} events", ingested);

    Ok(Json(IngestEventsBatchResponse {
        total,
        ingested,
        events: ingested_events,
    }))
}

pub async fn query_events(
    OptionalAuth(auth): OptionalAuth,
    Query(req): Query<QueryEventsRequest>,
    State(store): State<SharedStore>,
) -> Result<Json<QueryEventsResponse>> {
    let requested_limit = req.limit;
    let queried_entity_id = req.entity_id.clone();

    // Enforce tenant isolation: authenticated users can only see their own
    // tenant's events. Unauthenticated (skipped-auth paths) use request param.
    let enforced_tenant = auth
        .as_ref()
        .map(|a| a.tenant_id().to_string())
        .or(req.tenant_id.clone());

    // Query without limit to get total count
    let unlimited_req = QueryEventsRequest {
        entity_id: req.entity_id,
        event_type: req.event_type,
        tenant_id: enforced_tenant,
        as_of: req.as_of,
        since: req.since,
        until: req.until,
        limit: None,
        event_type_prefix: req.event_type_prefix,
        payload_filter: req.payload_filter,
    };
    let all_events = store.query(&unlimited_req)?;
    let total_count = all_events.len();

    // Apply limit
    let limited_events: Vec<Event> = if let Some(limit) = requested_limit {
        all_events.into_iter().take(limit).collect()
    } else {
        all_events
    };

    let count = limited_events.len();
    let has_more = count < total_count;
    let events: Vec<EventDto> = limited_events.iter().map(EventDto::from).collect();

    // Include entity_version only when filtering by a single entity_id
    let entity_version = queried_entity_id
        .as_deref()
        .map(|eid| store.get_entity_version(eid));

    tracing::debug!("Query returned {} events (total: {})", count, total_count);

    Ok(Json(QueryEventsResponse {
        events,
        count,
        total_count,
        has_more,
        entity_version,
    }))
}

pub async fn list_entities(
    State(store): State<SharedStore>,
    Query(req): Query<ListEntitiesRequest>,
) -> Result<Json<ListEntitiesResponse>> {
    use std::collections::HashMap;

    // Get all events matching the filters
    let query_req = QueryEventsRequest {
        entity_id: None,
        event_type: None,
        tenant_id: None,
        as_of: None,
        since: None,
        until: None,
        limit: None,
        event_type_prefix: req.event_type_prefix,
        payload_filter: req.payload_filter,
    };
    let events = store.query(&query_req)?;

    // Group by entity_id
    let mut entity_map: HashMap<String, Vec<&Event>> = HashMap::new();
    for event in &events {
        entity_map
            .entry(event.entity_id().to_string())
            .or_default()
            .push(event);
    }

    // Build entity summaries sorted by last event time (descending)
    let mut summaries: Vec<EntitySummary> = entity_map
        .into_iter()
        .map(|(entity_id, events)| {
            let last = events.iter().max_by_key(|e| e.timestamp()).unwrap();
            EntitySummary {
                entity_id,
                event_count: events.len(),
                last_event_type: last.event_type_str().to_string(),
                last_event_at: last.timestamp(),
            }
        })
        .collect();
    summaries.sort_by(|a, b| b.last_event_at.cmp(&a.last_event_at));

    let total = summaries.len();

    // Apply offset and limit
    let offset = req.offset.unwrap_or(0);
    let summaries: Vec<EntitySummary> = summaries.into_iter().skip(offset).collect::<Vec<_>>();
    let summaries = if let Some(limit) = req.limit {
        let has_more = summaries.len() > limit;
        let truncated: Vec<EntitySummary> = summaries.into_iter().take(limit).collect();
        return Ok(Json(ListEntitiesResponse {
            entities: truncated,
            total,
            has_more,
        }));
    } else {
        summaries
    };

    Ok(Json(ListEntitiesResponse {
        entities: summaries,
        total,
        has_more: false,
    }))
}

pub async fn detect_duplicates(
    State(store): State<SharedStore>,
    Query(req): Query<DetectDuplicatesRequest>,
) -> Result<Json<DetectDuplicatesResponse>> {
    use std::collections::HashMap;

    let group_by_fields: Vec<&str> = req.group_by.split(',').map(str::trim).collect();

    // Query events scoped by the required prefix
    let query_req = QueryEventsRequest {
        entity_id: None,
        event_type: None,
        tenant_id: None,
        as_of: None,
        since: None,
        until: None,
        limit: None,
        event_type_prefix: Some(req.event_type_prefix),
        payload_filter: None,
    };
    let events = store.query(&query_req)?;

    // For each entity, extract the latest event's payload fields specified by group_by
    // Then group entities by those field values
    let mut entity_latest: HashMap<String, &Event> = HashMap::new();
    for event in &events {
        let eid = event.entity_id().to_string();
        entity_latest
            .entry(eid)
            .and_modify(|existing| {
                if event.timestamp() > existing.timestamp() {
                    *existing = event;
                }
            })
            .or_insert(event);
    }

    // Group entities by their payload field values
    let mut groups: HashMap<String, Vec<String>> = HashMap::new();
    for (entity_id, event) in &entity_latest {
        let payload = event.payload();
        let mut key_parts = serde_json::Map::new();
        for field in &group_by_fields {
            let value = payload
                .get(*field)
                .cloned()
                .unwrap_or(serde_json::Value::Null);
            key_parts.insert((*field).to_string(), value);
        }
        let key_str = serde_json::to_string(&key_parts).unwrap_or_default();
        groups.entry(key_str).or_default().push(entity_id.clone());
    }

    // Filter to groups with count > 1 (actual duplicates)
    let mut duplicate_groups: Vec<DuplicateGroup> = groups
        .into_iter()
        .filter(|(_, ids)| ids.len() > 1)
        .map(|(key_str, mut ids)| {
            ids.sort();
            let key: serde_json::Value =
                serde_json::from_str(&key_str).unwrap_or(serde_json::Value::Null);
            let count = ids.len();
            DuplicateGroup {
                key,
                entity_ids: ids,
                count,
            }
        })
        .collect();

    // Sort by count descending for consistent output
    duplicate_groups.sort_by(|a, b| b.count.cmp(&a.count));

    let total = duplicate_groups.len();

    // Apply offset and limit
    let offset = req.offset.unwrap_or(0);
    let duplicate_groups: Vec<DuplicateGroup> = duplicate_groups.into_iter().skip(offset).collect();

    if let Some(limit) = req.limit {
        let has_more = duplicate_groups.len() > limit;
        let truncated: Vec<DuplicateGroup> = duplicate_groups.into_iter().take(limit).collect();
        return Ok(Json(DetectDuplicatesResponse {
            duplicates: truncated,
            total,
            has_more,
        }));
    }

    Ok(Json(DetectDuplicatesResponse {
        duplicates: duplicate_groups,
        total,
        has_more: false,
    }))
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
#[derive(Debug, Deserialize)]
pub struct WebSocketParams {
    pub consumer_id: Option<String>,
}

pub async fn events_websocket(
    ws: WebSocketUpgrade,
    State(store): State<SharedStore>,
    Query(params): Query<WebSocketParams>,
) -> Response {
    let websocket_manager = store.websocket_manager();

    ws.on_upgrade(move |socket| async move {
        if let Some(consumer_id) = params.consumer_id {
            websocket_manager
                .handle_socket_with_consumer(socket, consumer_id, store)
                .await;
        } else {
            websocket_manager.handle_socket(socket).await;
        }
    })
}

// v0.2: Event frequency analytics endpoint
pub async fn analytics_frequency(
    State(store): State<SharedStore>,
    Query(req): Query<EventFrequencyRequest>,
) -> Result<Json<EventFrequencyResponse>> {
    let response = AnalyticsEngine::event_frequency(&store, &req)?;

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
    let response = AnalyticsEngine::stats_summary(&store, &req)?;

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
        crate::error::AllSourceError::ValidationError(format!("Pipeline not found: {pipeline_id}"))
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
        crate::error::AllSourceError::ValidationError(format!("Pipeline not found: {pipeline_id}"))
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
        crate::error::AllSourceError::ValidationError(format!("Pipeline not found: {pipeline_id}"))
    })?;

    pipeline.reset();

    tracing::info!("🔄 Reset pipeline {}", pipeline_id);

    Ok(Json(serde_json::json!({
        "pipeline_id": pipeline_id,
        "reset": true
    })))
}

// =============================================================================
// v0.11: Single Event Lookup by ID
// =============================================================================

/// Get a single event by UUID
pub async fn get_event_by_id(
    State(store): State<SharedStore>,
    Path(event_id): Path<uuid::Uuid>,
) -> Result<Json<serde_json::Value>> {
    let event = store.get_event_by_id(&event_id)?.ok_or_else(|| {
        crate::error::AllSourceError::EntityNotFound(format!("Event '{event_id}' not found"))
    })?;

    let dto = EventDto::from(&event);

    tracing::debug!("Event retrieved by ID: {}", event_id);

    Ok(Json(serde_json::json!({
        "event": dto,
        "found": true
    })))
}

// =============================================================================
// v0.7: Projection State API for Query Service Integration
// =============================================================================

/// List all registered projections
pub async fn list_projections(State(store): State<SharedStore>) -> Json<serde_json::Value> {
    let projection_manager = store.projection_manager();
    let status_map = store.projection_status();

    let projections: Vec<serde_json::Value> = projection_manager
        .list_projections()
        .iter()
        .map(|(name, projection)| {
            let status = status_map
                .get(name)
                .map_or_else(|| "running".to_string(), |s| s.value().clone());
            serde_json::json!({
                "name": name,
                "type": format!("{:?}", projection.name()),
                "status": status,
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

/// Delete (clear) a projection by name
///
/// Removes all state from the projection. The projection definition remains
/// registered but its accumulated state is cleared.
pub async fn delete_projection(
    State(store): State<SharedStore>,
    Path(name): Path<String>,
) -> Result<Json<serde_json::Value>> {
    let projection_manager = store.projection_manager();

    let projection = projection_manager.get_projection(&name).ok_or_else(|| {
        crate::error::AllSourceError::EntityNotFound(format!("Projection '{name}' not found"))
    })?;

    projection.clear();

    // Also clear any cached state for this projection
    let cache = store.projection_state_cache();
    let prefix = format!("{name}:");
    let keys_to_remove: Vec<String> = cache
        .iter()
        .filter(|entry| entry.key().starts_with(&prefix))
        .map(|entry| entry.key().clone())
        .collect();
    for key in keys_to_remove {
        cache.remove(&key);
    }

    tracing::info!("Projection deleted (cleared): {}", name);

    Ok(Json(serde_json::json!({
        "projection": name,
        "deleted": true
    })))
}

/// Get aggregate projection state (all entities)
///
/// Returns summary information about a projection's state across all entities.
pub async fn get_projection_state_summary(
    State(store): State<SharedStore>,
    Path(name): Path<String>,
) -> Result<Json<serde_json::Value>> {
    let projection_manager = store.projection_manager();

    let _projection = projection_manager.get_projection(&name).ok_or_else(|| {
        crate::error::AllSourceError::EntityNotFound(format!("Projection '{name}' not found"))
    })?;

    // Collect cached states for this projection
    let cache = store.projection_state_cache();
    let prefix = format!("{name}:");
    let states: Vec<serde_json::Value> = cache
        .iter()
        .filter(|entry| entry.key().starts_with(&prefix))
        .map(|entry| {
            let entity_id = entry.key().strip_prefix(&prefix).unwrap_or(entry.key());
            serde_json::json!({
                "entity_id": entity_id,
                "state": entry.value().clone()
            })
        })
        .collect();

    let total = states.len();

    tracing::debug!("Projection state summary: {} ({} entities)", name, total);

    Ok(Json(serde_json::json!({
        "projection": name,
        "states": states,
        "total": total
    })))
}

/// Reset a projection to its initial state
///
/// Clears all accumulated state and reprocesses events from the beginning.
pub async fn reset_projection(
    State(store): State<SharedStore>,
    Path(name): Path<String>,
) -> Result<Json<serde_json::Value>> {
    let reprocessed = store.reset_projection(&name)?;

    tracing::info!(
        "Projection reset: {} ({} events reprocessed)",
        name,
        reprocessed
    );

    Ok(Json(serde_json::json!({
        "projection": name,
        "reset": true,
        "events_reprocessed": reprocessed
    })))
}

/// Pause a projection
///
/// Sets the projection status to "paused" so it stops processing new events.
pub async fn pause_projection(
    State(store): State<SharedStore>,
    Path(name): Path<String>,
) -> Result<Json<serde_json::Value>> {
    let projection_manager = store.projection_manager();

    // Verify projection exists
    let _projection = projection_manager.get_projection(&name).ok_or_else(|| {
        crate::error::AllSourceError::EntityNotFound(format!("Projection '{name}' not found"))
    })?;

    store
        .projection_status()
        .insert(name.clone(), "paused".to_string());

    tracing::info!("Projection paused: {}", name);

    Ok(Json(serde_json::json!({
        "projection": name,
        "status": "paused"
    })))
}

/// Start (resume) a projection
///
/// Sets the projection status to "running" so it resumes processing events.
pub async fn start_projection(
    State(store): State<SharedStore>,
    Path(name): Path<String>,
) -> Result<Json<serde_json::Value>> {
    let projection_manager = store.projection_manager();

    // Verify projection exists
    let _projection = projection_manager.get_projection(&name).ok_or_else(|| {
        crate::error::AllSourceError::EntityNotFound(format!("Projection '{name}' not found"))
    })?;

    store
        .projection_status()
        .insert(name.clone(), "running".to_string());

    tracing::info!("Projection started: {}", name);

    Ok(Json(serde_json::json!({
        "projection": name,
        "status": "running"
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

// =============================================================================
// v0.11: Webhook Management API
// =============================================================================

/// Query parameters for listing webhooks
#[derive(Debug, Deserialize)]
pub struct ListWebhooksParams {
    pub tenant_id: Option<String>,
}

/// Register a new webhook subscription
pub async fn register_webhook(
    State(store): State<SharedStore>,
    Json(req): Json<RegisterWebhookRequest>,
) -> Json<serde_json::Value> {
    let registry = store.webhook_registry();
    let webhook = registry.register(req);

    tracing::info!("Webhook registered: {} -> {}", webhook.id, webhook.url);

    Json(serde_json::json!({
        "webhook": webhook,
        "created": true
    }))
}

/// List webhooks, optionally filtered by tenant_id
pub async fn list_webhooks(
    State(store): State<SharedStore>,
    Query(params): Query<ListWebhooksParams>,
) -> Json<serde_json::Value> {
    let registry = store.webhook_registry();

    let webhooks = if let Some(tenant_id) = params.tenant_id {
        registry.list_by_tenant(&tenant_id)
    } else {
        // Without tenant filter, return empty (tenants should always filter)
        vec![]
    };

    let total = webhooks.len();

    Json(serde_json::json!({
        "webhooks": webhooks,
        "total": total
    }))
}

/// Get a specific webhook by ID
pub async fn get_webhook(
    State(store): State<SharedStore>,
    Path(webhook_id): Path<uuid::Uuid>,
) -> Result<Json<serde_json::Value>> {
    let registry = store.webhook_registry();

    let webhook = registry.get(webhook_id).ok_or_else(|| {
        crate::error::AllSourceError::EntityNotFound(format!("Webhook '{webhook_id}' not found"))
    })?;

    Ok(Json(serde_json::json!({
        "webhook": webhook,
        "found": true
    })))
}

/// Update a webhook subscription
pub async fn update_webhook(
    State(store): State<SharedStore>,
    Path(webhook_id): Path<uuid::Uuid>,
    Json(req): Json<UpdateWebhookRequest>,
) -> Result<Json<serde_json::Value>> {
    let registry = store.webhook_registry();

    let webhook = registry.update(webhook_id, req).ok_or_else(|| {
        crate::error::AllSourceError::EntityNotFound(format!("Webhook '{webhook_id}' not found"))
    })?;

    tracing::info!("Webhook updated: {}", webhook_id);

    Ok(Json(serde_json::json!({
        "webhook": webhook,
        "updated": true
    })))
}

/// Delete a webhook subscription
pub async fn delete_webhook(
    State(store): State<SharedStore>,
    Path(webhook_id): Path<uuid::Uuid>,
) -> Result<Json<serde_json::Value>> {
    let registry = store.webhook_registry();

    let webhook = registry.delete(webhook_id).ok_or_else(|| {
        crate::error::AllSourceError::EntityNotFound(format!("Webhook '{webhook_id}' not found"))
    })?;

    tracing::info!("Webhook deleted: {} ({})", webhook_id, webhook.url);

    Ok(Json(serde_json::json!({
        "webhook_id": webhook_id,
        "deleted": true
    })))
}

/// Query parameters for listing webhook deliveries
#[derive(Debug, Deserialize)]
pub struct ListDeliveriesParams {
    pub limit: Option<usize>,
}

/// List delivery history for a webhook
pub async fn list_webhook_deliveries(
    State(store): State<SharedStore>,
    Path(webhook_id): Path<uuid::Uuid>,
    Query(params): Query<ListDeliveriesParams>,
) -> Result<Json<serde_json::Value>> {
    let registry = store.webhook_registry();

    // Verify webhook exists
    registry.get(webhook_id).ok_or_else(|| {
        crate::error::AllSourceError::EntityNotFound(format!("Webhook '{webhook_id}' not found"))
    })?;

    let limit = params.limit.unwrap_or(50);
    let deliveries = registry.get_deliveries(webhook_id, limit);
    let total = deliveries.len();

    Ok(Json(serde_json::json!({
        "webhook_id": webhook_id,
        "deliveries": deliveries,
        "total": total
    })))
}

// =============================================================================
// v2.0: Advanced Query Features
// =============================================================================

/// EventQL: Execute SQL queries over events using DataFusion
#[cfg(feature = "analytics")]
pub async fn eventql_query(
    State(store): State<SharedStore>,
    Json(req): Json<crate::infrastructure::query::eventql::EventQLRequest>,
) -> Result<Json<serde_json::Value>> {
    let events = store.snapshot_events();
    match crate::infrastructure::query::eventql::execute_eventql(&events, &req).await {
        Ok(response) => Ok(Json(serde_json::json!({
            "columns": response.columns,
            "rows": response.rows,
            "row_count": response.row_count,
        }))),
        Err(e) => Err(crate::error::AllSourceError::InvalidQuery(e)),
    }
}

/// GraphQL: Execute GraphQL queries
pub async fn graphql_query(
    State(store): State<SharedStore>,
    Json(req): Json<GraphQLRequest>,
) -> Json<serde_json::Value> {
    let fields = match crate::infrastructure::query::graphql::parse_query(&req.query) {
        Ok(f) => f,
        Err(e) => {
            return Json(
                serde_json::to_value(GraphQLResponse {
                    data: None,
                    errors: vec![GraphQLError { message: e }],
                })
                .unwrap(),
            );
        }
    };

    let mut data = serde_json::Map::new();
    let mut errors = Vec::new();

    for field in &fields {
        match field.name.as_str() {
            "events" => {
                let request = crate::application::dto::QueryEventsRequest {
                    entity_id: field.arguments.get("entity_id").cloned(),
                    event_type: field.arguments.get("event_type").cloned(),
                    tenant_id: field.arguments.get("tenant_id").cloned(),
                    limit: field.arguments.get("limit").and_then(|l| l.parse().ok()),
                    as_of: None,
                    since: None,
                    until: None,
                    event_type_prefix: None,
                    payload_filter: None,
                };
                match store.query(&request) {
                    Ok(events) => {
                        let json_events: Vec<serde_json::Value> = events
                            .iter()
                            .map(|e| {
                                crate::infrastructure::query::graphql::event_to_json(
                                    e,
                                    &field.fields,
                                )
                            })
                            .collect();
                        data.insert("events".to_string(), serde_json::Value::Array(json_events));
                    }
                    Err(e) => errors.push(GraphQLError {
                        message: format!("events query failed: {e}"),
                    }),
                }
            }
            "event" => {
                if let Some(id_str) = field.arguments.get("id") {
                    if let Ok(id) = uuid::Uuid::parse_str(id_str) {
                        match store.get_event_by_id(&id) {
                            Ok(Some(event)) => {
                                data.insert(
                                    "event".to_string(),
                                    crate::infrastructure::query::graphql::event_to_json(
                                        &event,
                                        &field.fields,
                                    ),
                                );
                            }
                            Ok(None) => {
                                data.insert("event".to_string(), serde_json::Value::Null);
                            }
                            Err(e) => errors.push(GraphQLError {
                                message: format!("event lookup failed: {e}"),
                            }),
                        }
                    } else {
                        errors.push(GraphQLError {
                            message: format!("Invalid UUID: {id_str}"),
                        });
                    }
                } else {
                    errors.push(GraphQLError {
                        message: "event query requires 'id' argument".to_string(),
                    });
                }
            }
            "projections" => {
                let pm = store.projection_manager();
                let names: Vec<serde_json::Value> = pm
                    .list_projections()
                    .iter()
                    .map(|(name, _)| serde_json::Value::String(name.clone()))
                    .collect();
                data.insert("projections".to_string(), serde_json::Value::Array(names));
            }
            "stats" => {
                let stats = store.stats();
                data.insert(
                    "stats".to_string(),
                    serde_json::json!({
                        "total_events": stats.total_events,
                        "total_entities": stats.total_entities,
                        "total_event_types": stats.total_event_types,
                    }),
                );
            }
            "__schema" => {
                data.insert(
                    "__schema".to_string(),
                    crate::infrastructure::query::graphql::introspection_schema(),
                );
            }
            other => {
                errors.push(GraphQLError {
                    message: format!("Unknown field: {other}"),
                });
            }
        }
    }

    Json(
        serde_json::to_value(GraphQLResponse {
            data: Some(serde_json::Value::Object(data)),
            errors,
        })
        .unwrap(),
    )
}

/// Geospatial: Query events by location
pub async fn geo_query(
    State(store): State<SharedStore>,
    Json(req): Json<GeoQueryRequest>,
) -> Json<serde_json::Value> {
    let events = store.snapshot_events();
    let geo_index = store.geo_index();
    let results =
        crate::infrastructure::query::geospatial::execute_geo_query(&events, &geo_index, &req);
    let total = results.len();
    Json(serde_json::json!({
        "results": results,
        "total": total,
    }))
}

/// Geospatial index stats
pub async fn geo_stats(State(store): State<SharedStore>) -> Json<serde_json::Value> {
    let stats = store.geo_index().stats();
    Json(serde_json::json!(stats))
}

/// Exactly-once processing stats
pub async fn exactly_once_stats(State(store): State<SharedStore>) -> Json<serde_json::Value> {
    let stats = store.exactly_once().stats();
    Json(serde_json::json!(stats))
}

/// Schema evolution history for an event type
pub async fn schema_evolution_history(
    State(store): State<SharedStore>,
    Path(event_type): Path<String>,
) -> Json<serde_json::Value> {
    let mgr = store.schema_evolution();
    let history = mgr.get_history(&event_type);
    let version = mgr.get_version(&event_type);
    Json(serde_json::json!({
        "event_type": event_type,
        "current_version": version,
        "history": history,
    }))
}

/// Current inferred schema for an event type
pub async fn schema_evolution_schema(
    State(store): State<SharedStore>,
    Path(event_type): Path<String>,
) -> Json<serde_json::Value> {
    let mgr = store.schema_evolution();
    if let Some(schema) = mgr.get_schema(&event_type) {
        let json_schema = crate::application::services::schema_evolution::to_json_schema(&schema);
        Json(serde_json::json!({
            "event_type": event_type,
            "version": mgr.get_version(&event_type),
            "inferred_schema": schema,
            "json_schema": json_schema,
        }))
    } else {
        Json(serde_json::json!({
            "event_type": event_type,
            "error": "No schema inferred for this event type"
        }))
    }
}

/// Schema evolution stats
pub async fn schema_evolution_stats(State(store): State<SharedStore>) -> Json<serde_json::Value> {
    let stats = store.schema_evolution().stats();
    let event_types = store.schema_evolution().list_event_types();
    Json(serde_json::json!({
        "stats": stats,
        "tracked_event_types": event_types,
    }))
}

// =============================================================================
// Sync Protocol Endpoints (v0.11: embedded↔server bidirectional sync)
// =============================================================================

/// POST /api/v1/sync/pull — Client sends version vector, server returns delta events.
#[cfg(feature = "embedded-sync")]
pub async fn sync_pull_handler(
    State(state): State<AppState>,
    Json(request): Json<crate::embedded::sync_types::SyncPullRequest>,
) -> Result<Json<crate::embedded::sync_types::SyncPullResponse>> {
    use crate::infrastructure::cluster::{crdt::ReplicatedEvent, hlc::HlcTimestamp};

    let store = &state.store;

    // Compute "since" threshold from the client's version vector
    // We return all events the client hasn't seen yet
    let since = request
        .version_vector
        .values()
        .map(|ts| ts.physical_ms)
        .min()
        .and_then(|ms| chrono::DateTime::from_timestamp_millis(ms as i64));

    let events = store.query(&crate::application::dto::QueryEventsRequest {
        entity_id: None,
        event_type: None,
        tenant_id: None,
        as_of: None,
        since,
        until: None,
        limit: None,
        event_type_prefix: None,
        payload_filter: None,
    })?;

    // Convert domain events to ReplicatedEvent wire format
    let mut replicated = Vec::with_capacity(events.len());
    let mut last_ms = 0u64;
    let mut logical = 0u32;

    for event in &events {
        let event_ms = event.timestamp().timestamp_millis() as u64;
        if event_ms == last_ms {
            logical += 1;
        } else {
            last_ms = event_ms;
            logical = 0;
        }

        replicated.push(ReplicatedEvent {
            event_id: event.id().to_string(),
            hlc_timestamp: HlcTimestamp::new(event_ms, logical, 0),
            origin_region: "server".to_string(),
            event_data: serde_json::json!({
                "event_type": event.event_type_str(),
                "entity_id": event.entity_id_str(),
                "tenant_id": event.tenant_id_str(),
                "payload": event.payload,
                "metadata": event.metadata,
            }),
        });
    }

    Ok(Json(crate::embedded::sync_types::SyncPullResponse {
        events: replicated,
        version_vector: std::collections::BTreeMap::new(),
    }))
}

/// POST /api/v1/sync/push — Client pushes events, server applies CRDT resolution.
#[cfg(feature = "embedded-sync")]
pub async fn sync_push_handler(
    State(state): State<AppState>,
    Json(request): Json<crate::embedded::sync_types::SyncPushRequest>,
) -> Result<Json<crate::embedded::sync_types::SyncPushResponse>> {
    let store = &state.store;

    let mut accepted = 0usize;
    let mut skipped = 0usize;

    for rep_event in &request.events {
        let event_data = &rep_event.event_data;
        let event_type = event_data
            .get("event_type")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string();
        let entity_id = event_data
            .get("entity_id")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string();
        let tenant_id = event_data
            .get("tenant_id")
            .and_then(|v| v.as_str())
            .unwrap_or("default")
            .to_string();
        let payload = event_data
            .get("payload")
            .cloned()
            .unwrap_or(serde_json::json!({}));
        let metadata = event_data.get("metadata").cloned();

        match Event::from_strings(event_type, entity_id, tenant_id, payload, metadata) {
            Ok(domain_event) => {
                store.ingest(&domain_event)?;
                accepted += 1;
            }
            Err(_) => {
                skipped += 1;
            }
        }
    }

    Ok(Json(crate::embedded::sync_types::SyncPushResponse {
        accepted,
        skipped,
        version_vector: std::collections::BTreeMap::new(),
    }))
}

// =============================================================================
// Consumer endpoints for durable subscriptions (v0.14)
// =============================================================================

/// POST /api/v1/consumers — Register a durable consumer
pub async fn register_consumer(
    State(store): State<SharedStore>,
    Json(req): Json<RegisterConsumerRequest>,
) -> Result<Json<ConsumerResponse>> {
    let consumer = store
        .consumer_registry()
        .register(&req.consumer_id, &req.event_type_filters);

    Ok(Json(ConsumerResponse {
        consumer_id: consumer.consumer_id,
        event_type_filters: consumer.event_type_filters,
        cursor_position: consumer.cursor_position,
    }))
}

/// GET /api/v1/consumers/{consumer_id} — Get consumer metadata and cursor position
pub async fn get_consumer(
    State(store): State<SharedStore>,
    Path(consumer_id): Path<String>,
) -> Result<Json<ConsumerResponse>> {
    let consumer = store.consumer_registry().get_or_create(&consumer_id);

    Ok(Json(ConsumerResponse {
        consumer_id: consumer.consumer_id,
        event_type_filters: consumer.event_type_filters,
        cursor_position: consumer.cursor_position,
    }))
}

/// GET /api/v1/consumers/{consumer_id}/events — Poll for events since last ack
#[derive(Debug, Deserialize)]
pub struct ConsumerPollQuery {
    pub limit: Option<usize>,
}

pub async fn poll_consumer_events(
    State(store): State<SharedStore>,
    Path(consumer_id): Path<String>,
    Query(query): Query<ConsumerPollQuery>,
) -> Result<Json<ConsumerEventsResponse>> {
    let consumer = store.consumer_registry().get_or_create(&consumer_id);
    let offset = consumer.cursor_position.unwrap_or(0);
    let limit = query.limit.unwrap_or(100);

    let events = store.events_after_offset(offset, &consumer.event_type_filters, limit);
    let count = events.len();

    let consumer_events: Vec<ConsumerEventDto> = events
        .into_iter()
        .map(|(position, event)| ConsumerEventDto {
            position,
            event: EventDto::from(&event),
        })
        .collect();

    Ok(Json(ConsumerEventsResponse {
        events: consumer_events,
        count,
    }))
}

/// POST /api/v1/consumers/{consumer_id}/ack — Acknowledge processed events
pub async fn ack_consumer(
    State(store): State<SharedStore>,
    Path(consumer_id): Path<String>,
    Json(req): Json<AckRequest>,
) -> Result<Json<serde_json::Value>> {
    let max_offset = store.total_events() as u64;

    store
        .consumer_registry()
        .ack(&consumer_id, req.position, max_offset)
        .map_err(crate::error::AllSourceError::InvalidInput)?;

    Ok(Json(serde_json::json!({
        "status": "ok",
        "consumer_id": consumer_id,
        "position": req.position,
    })))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{domain::entities::Event, store::EventStore};

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
    async fn test_query_events_has_more_and_total_count() {
        let store = create_test_store();

        // Ingest 50 events
        for i in 0..50 {
            store
                .ingest(&create_test_event(&format!("entity-{i}"), "user.created"))
                .unwrap();
        }

        // Query with limit=10 — should get has_more=true, total_count=50
        let req = QueryEventsRequest {
            entity_id: None,
            event_type: None,
            tenant_id: None,
            as_of: None,
            since: None,
            until: None,
            limit: Some(10),
            event_type_prefix: None,
            payload_filter: None,
        };

        let requested_limit = req.limit;
        let unlimited_req = QueryEventsRequest {
            limit: None,
            ..QueryEventsRequest {
                entity_id: req.entity_id,
                event_type: req.event_type,
                tenant_id: req.tenant_id,
                as_of: req.as_of,
                since: req.since,
                until: req.until,
                limit: None,
                event_type_prefix: req.event_type_prefix,
                payload_filter: req.payload_filter,
            }
        };
        let all_events = store.query(&unlimited_req).unwrap();
        let total_count = all_events.len();
        let limited_events: Vec<Event> = if let Some(limit) = requested_limit {
            all_events.into_iter().take(limit).collect()
        } else {
            all_events
        };
        let count = limited_events.len();
        let has_more = count < total_count;

        assert_eq!(count, 10);
        assert_eq!(total_count, 50);
        assert!(has_more);
    }

    #[tokio::test]
    async fn test_query_events_no_more_results() {
        let store = create_test_store();

        // Ingest 5 events
        for i in 0..5 {
            store
                .ingest(&create_test_event(&format!("entity-{i}"), "user.created"))
                .unwrap();
        }

        // Query with limit=100 — should get has_more=false, total_count=5
        let all_events = store
            .query(&QueryEventsRequest {
                entity_id: None,
                event_type: None,
                tenant_id: None,
                as_of: None,
                since: None,
                until: None,
                limit: None,
                event_type_prefix: None,
                payload_filter: None,
            })
            .unwrap();
        let total_count = all_events.len();
        let limited_events: Vec<Event> = all_events.into_iter().take(100).collect();
        let count = limited_events.len();
        let has_more = count < total_count;

        assert_eq!(count, 5);
        assert_eq!(total_count, 5);
        assert!(!has_more);
    }

    #[tokio::test]
    async fn test_list_entities_by_type_prefix() {
        let store = create_test_store();

        // 3 index entities
        store
            .ingest(&create_test_event("idx-1", "index.created"))
            .unwrap();
        store
            .ingest(&create_test_event("idx-1", "index.updated"))
            .unwrap();
        store
            .ingest(&create_test_event("idx-2", "index.created"))
            .unwrap();
        store
            .ingest(&create_test_event("idx-3", "index.created"))
            .unwrap();
        // 2 trade entities
        store
            .ingest(&create_test_event("trade-1", "trade.created"))
            .unwrap();
        store
            .ingest(&create_test_event("trade-2", "trade.created"))
            .unwrap();

        // List entities for index.*
        let req = ListEntitiesRequest {
            event_type_prefix: Some("index.".to_string()),
            payload_filter: None,
            limit: None,
            offset: None,
        };
        let query_req = QueryEventsRequest {
            entity_id: None,
            event_type: None,
            tenant_id: None,
            as_of: None,
            since: None,
            until: None,
            limit: None,
            event_type_prefix: req.event_type_prefix,
            payload_filter: req.payload_filter,
        };
        let events = store.query(&query_req).unwrap();

        // Group and verify
        let mut entity_map: std::collections::HashMap<String, Vec<&Event>> =
            std::collections::HashMap::new();
        for event in &events {
            entity_map
                .entry(event.entity_id().to_string())
                .or_default()
                .push(event);
        }

        assert_eq!(entity_map.len(), 3); // idx-1, idx-2, idx-3
        assert_eq!(entity_map["idx-1"].len(), 2); // 2 events for idx-1
        assert_eq!(entity_map["idx-2"].len(), 1);
        assert_eq!(entity_map["idx-3"].len(), 1);
    }

    fn create_test_event_with_payload(
        entity_id: &str,
        event_type: &str,
        payload: serde_json::Value,
    ) -> Event {
        Event::from_strings(
            event_type.to_string(),
            entity_id.to_string(),
            "test-stream".to_string(),
            payload,
            None,
        )
        .unwrap()
    }

    #[tokio::test]
    async fn test_detect_duplicates_by_payload_fields() {
        let store = create_test_store();

        // Create entities with duplicate "name" field values
        store
            .ingest(&create_test_event_with_payload(
                "idx-1",
                "index.created",
                serde_json::json!({"name": "S&P 500", "user_id": "alice"}),
            ))
            .unwrap();
        store
            .ingest(&create_test_event_with_payload(
                "idx-2",
                "index.created",
                serde_json::json!({"name": "S&P 500", "user_id": "bob"}),
            ))
            .unwrap();
        store
            .ingest(&create_test_event_with_payload(
                "idx-3",
                "index.created",
                serde_json::json!({"name": "NASDAQ", "user_id": "alice"}),
            ))
            .unwrap();
        store
            .ingest(&create_test_event_with_payload(
                "idx-4",
                "index.created",
                serde_json::json!({"name": "NASDAQ", "user_id": "carol"}),
            ))
            .unwrap();
        store
            .ingest(&create_test_event_with_payload(
                "idx-5",
                "index.created",
                serde_json::json!({"name": "DAX", "user_id": "dave"}),
            ))
            .unwrap();

        // Group by name — should find 2 groups: "S&P 500" (idx-1, idx-2) and "NASDAQ" (idx-3, idx-4)
        let query_req = QueryEventsRequest {
            entity_id: None,
            event_type: None,
            tenant_id: None,
            as_of: None,
            since: None,
            until: None,
            limit: None,
            event_type_prefix: Some("index.".to_string()),
            payload_filter: None,
        };
        let events = store.query(&query_req).unwrap();

        // Manually replicate the handler logic for testing
        let group_by_fields = vec!["name"];
        let mut entity_latest: std::collections::HashMap<String, &Event> =
            std::collections::HashMap::new();
        for event in &events {
            let eid = event.entity_id().to_string();
            entity_latest
                .entry(eid)
                .and_modify(|existing| {
                    if event.timestamp() > existing.timestamp() {
                        *existing = event;
                    }
                })
                .or_insert(event);
        }

        let mut groups: std::collections::HashMap<String, Vec<String>> =
            std::collections::HashMap::new();
        for (entity_id, event) in &entity_latest {
            let payload = event.payload();
            let mut key_parts = serde_json::Map::new();
            for field in &group_by_fields {
                let value = payload
                    .get(*field)
                    .cloned()
                    .unwrap_or(serde_json::Value::Null);
                key_parts.insert((*field).to_string(), value);
            }
            let key_str = serde_json::to_string(&key_parts).unwrap_or_default();
            groups.entry(key_str).or_default().push(entity_id.clone());
        }

        let duplicate_groups: Vec<_> = groups
            .into_iter()
            .filter(|(_, ids)| ids.len() > 1)
            .collect();

        assert_eq!(duplicate_groups.len(), 2); // S&P 500 and NASDAQ groups
        for (_, ids) in &duplicate_groups {
            assert_eq!(ids.len(), 2);
        }
    }

    #[tokio::test]
    async fn test_detect_duplicates_no_duplicates() {
        let store = create_test_store();

        // All unique names
        store
            .ingest(&create_test_event_with_payload(
                "idx-1",
                "index.created",
                serde_json::json!({"name": "A"}),
            ))
            .unwrap();
        store
            .ingest(&create_test_event_with_payload(
                "idx-2",
                "index.created",
                serde_json::json!({"name": "B"}),
            ))
            .unwrap();

        let query_req = QueryEventsRequest {
            entity_id: None,
            event_type: None,
            tenant_id: None,
            as_of: None,
            since: None,
            until: None,
            limit: None,
            event_type_prefix: Some("index.".to_string()),
            payload_filter: None,
        };
        let events = store.query(&query_req).unwrap();

        let mut entity_latest: std::collections::HashMap<String, &Event> =
            std::collections::HashMap::new();
        for event in &events {
            entity_latest
                .entry(event.entity_id().to_string())
                .or_insert(event);
        }

        let mut groups: std::collections::HashMap<String, Vec<String>> =
            std::collections::HashMap::new();
        for (entity_id, event) in &entity_latest {
            let key_str =
                serde_json::to_string(&serde_json::json!({"name": event.payload().get("name")}))
                    .unwrap();
            groups.entry(key_str).or_default().push(entity_id.clone());
        }

        let duplicate_groups: Vec<_> = groups
            .into_iter()
            .filter(|(_, ids)| ids.len() > 1)
            .collect();

        assert_eq!(duplicate_groups.len(), 0); // No duplicates
    }

    #[tokio::test]
    async fn test_detect_duplicates_multi_field_group_by() {
        let store = create_test_store();

        // Two entities with same name AND user_id = true duplicate
        store
            .ingest(&create_test_event_with_payload(
                "idx-1",
                "index.created",
                serde_json::json!({"name": "S&P 500", "user_id": "alice"}),
            ))
            .unwrap();
        store
            .ingest(&create_test_event_with_payload(
                "idx-2",
                "index.created",
                serde_json::json!({"name": "S&P 500", "user_id": "alice"}),
            ))
            .unwrap();
        // Same name but different user_id = NOT a duplicate in multi-field group
        store
            .ingest(&create_test_event_with_payload(
                "idx-3",
                "index.created",
                serde_json::json!({"name": "S&P 500", "user_id": "bob"}),
            ))
            .unwrap();

        let query_req = QueryEventsRequest {
            entity_id: None,
            event_type: None,
            tenant_id: None,
            as_of: None,
            since: None,
            until: None,
            limit: None,
            event_type_prefix: Some("index.".to_string()),
            payload_filter: None,
        };
        let events = store.query(&query_req).unwrap();

        let group_by_fields = vec!["name", "user_id"];
        let mut entity_latest: std::collections::HashMap<String, &Event> =
            std::collections::HashMap::new();
        for event in &events {
            entity_latest
                .entry(event.entity_id().to_string())
                .and_modify(|existing| {
                    if event.timestamp() > existing.timestamp() {
                        *existing = event;
                    }
                })
                .or_insert(event);
        }

        let mut groups: std::collections::HashMap<String, Vec<String>> =
            std::collections::HashMap::new();
        for (entity_id, event) in &entity_latest {
            let payload = event.payload();
            let mut key_parts = serde_json::Map::new();
            for field in &group_by_fields {
                let value = payload
                    .get(*field)
                    .cloned()
                    .unwrap_or(serde_json::Value::Null);
                key_parts.insert((*field).to_string(), value);
            }
            let key_str = serde_json::to_string(&key_parts).unwrap_or_default();
            groups.entry(key_str).or_default().push(entity_id.clone());
        }

        let duplicate_groups: Vec<_> = groups
            .into_iter()
            .filter(|(_, ids)| ids.len() > 1)
            .collect();

        // Only 1 duplicate group: name=S&P 500, user_id=alice (idx-1, idx-2)
        assert_eq!(duplicate_groups.len(), 1);
        let (_, ref ids) = duplicate_groups[0];
        assert_eq!(ids.len(), 2);
        let mut sorted_ids = ids.clone();
        sorted_ids.sort();
        assert_eq!(sorted_ids, vec!["idx-1", "idx-2"]);
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
        store.ingest(&event).unwrap();

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
                format!("entity_snapshots:entity-{i}"),
                serde_json::json!({"id": i, "status": "active"}),
            );
        }

        // Verify all insertions
        assert_eq!(cache.len(), 10);

        // Verify each entity
        for i in 0..10 {
            let key = format!("entity_snapshots:entity-{i}");
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
            .ingest(&create_test_event("user-1", "user.created"))
            .unwrap();
        store
            .ingest(&create_test_event("user-2", "user.created"))
            .unwrap();
        store
            .ingest(&create_test_event("user-1", "user.updated"))
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
                        format!("concurrent:entity-{i}"),
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
            cache.insert(format!("iter:entity-{i}"), serde_json::json!({"index": i}));
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
            let event = create_test_event(&format!("bulk-user-{i}"), "user.created");
            store.ingest(&event).unwrap();
        }

        // Get projection and verify bulk access
        let projection_manager = store.projection_manager();
        let snapshot_projection = projection_manager
            .get_projection("entity_snapshots")
            .unwrap();

        // Verify we can access all entities
        for i in 0..5 {
            let state = snapshot_projection.get_state(&format!("bulk-user-{i}"));
            assert!(state.is_some(), "Entity bulk-user-{i} should have state");
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
                format!("volume_test:entity-{i}"),
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

        for proj in &projections {
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
        for proj in &projections {
            let state = cache.get(&format!("{proj}:entity-0")).unwrap();
            assert_eq!(state["projection"], *proj);
        }
    }
}
