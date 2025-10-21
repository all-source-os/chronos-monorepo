use crate::compaction::{CompactionConfig, CompactionManager};
use crate::error::{AllSourceError, Result};
use crate::event::{Event, QueryEventsRequest};
use crate::index::{EventIndex, IndexEntry};
use crate::metrics::MetricsRegistry;
use crate::pipeline::PipelineManager;
use crate::projection::{
    EntitySnapshotProjection, EventCounterProjection, ProjectionManager,
};
use crate::replay::ReplayManager;
use crate::schema::{SchemaRegistry, SchemaRegistryConfig};
use crate::snapshot::{SnapshotConfig, SnapshotManager, SnapshotType};
use crate::storage::ParquetStorage;
use crate::wal::{WALConfig, WriteAheadLog};
use crate::websocket::WebSocketManager;
use chrono::{DateTime, Utc};
use parking_lot::RwLock;
use std::path::PathBuf;
use std::sync::Arc;

/// High-performance event store with columnar storage
pub struct EventStore {
    /// In-memory event storage
    events: Arc<RwLock<Vec<Event>>>,

    /// High-performance concurrent index
    index: Arc<EventIndex>,

    /// Projection manager for real-time aggregations
    pub(crate) projections: Arc<RwLock<ProjectionManager>>,

    /// Optional persistent storage (v0.2 feature)
    storage: Option<Arc<RwLock<ParquetStorage>>>,

    /// WebSocket manager for real-time event streaming (v0.2 feature)
    websocket_manager: Arc<WebSocketManager>,

    /// Snapshot manager for fast state recovery (v0.2 feature)
    snapshot_manager: Arc<SnapshotManager>,

    /// Write-Ahead Log for durability (v0.2 feature)
    wal: Option<Arc<WriteAheadLog>>,

    /// Compaction manager for Parquet optimization (v0.2 feature)
    compaction_manager: Option<Arc<CompactionManager>>,

    /// Schema registry for event validation (v0.5 feature)
    schema_registry: Arc<SchemaRegistry>,

    /// Replay manager for event replay and projection rebuilding (v0.5 feature)
    replay_manager: Arc<ReplayManager>,

    /// Pipeline manager for stream processing (v0.5 feature)
    pipeline_manager: Arc<PipelineManager>,

    /// Prometheus metrics registry (v0.6 feature)
    metrics: Arc<MetricsRegistry>,

    /// Total events ingested (for metrics)
    total_ingested: Arc<RwLock<u64>>,
}

impl EventStore {
    /// Create a new in-memory event store
    pub fn new() -> Self {
        Self::with_config(EventStoreConfig::default())
    }

    /// Create event store with custom configuration
    pub fn with_config(config: EventStoreConfig) -> Self {
        let mut projections = ProjectionManager::new();

        // Register built-in projections
        projections.register(Arc::new(EntitySnapshotProjection::new("entity_snapshots")));
        projections.register(Arc::new(EventCounterProjection::new("event_counters")));

        // Initialize persistent storage if configured
        let storage = config.storage_dir.as_ref().and_then(|dir| {
            match ParquetStorage::new(dir) {
                Ok(storage) => {
                    tracing::info!("✅ Parquet persistence enabled at: {}", dir.display());
                    Some(Arc::new(RwLock::new(storage)))
                }
                Err(e) => {
                    tracing::error!("❌ Failed to initialize Parquet storage: {}", e);
                    None
                }
            }
        });

        // Initialize WAL if configured (v0.2 feature)
        let wal = config.wal_dir.as_ref().and_then(|dir| {
            match WriteAheadLog::new(dir, config.wal_config.clone()) {
                Ok(wal) => {
                    tracing::info!("✅ WAL enabled at: {}", dir.display());
                    Some(Arc::new(wal))
                }
                Err(e) => {
                    tracing::error!("❌ Failed to initialize WAL: {}", e);
                    None
                }
            }
        });

        // Initialize compaction manager if Parquet storage is enabled (v0.2 feature)
        let compaction_manager = config.storage_dir.as_ref().map(|dir| {
            let manager = CompactionManager::new(dir, config.compaction_config.clone());
            Arc::new(manager)
        });

        // Initialize schema registry (v0.5 feature)
        let schema_registry = Arc::new(SchemaRegistry::new(config.schema_registry_config.clone()));
        tracing::info!("✅ Schema registry enabled");

        // Initialize replay manager (v0.5 feature)
        let replay_manager = Arc::new(ReplayManager::new());
        tracing::info!("✅ Replay manager enabled");

        // Initialize pipeline manager (v0.5 feature)
        let pipeline_manager = Arc::new(PipelineManager::new());
        tracing::info!("✅ Pipeline manager enabled");

        // Initialize metrics registry (v0.6 feature)
        let metrics = MetricsRegistry::new();
        tracing::info!("✅ Prometheus metrics registry initialized");

        let store = Self {
            events: Arc::new(RwLock::new(Vec::new())),
            index: Arc::new(EventIndex::new()),
            projections: Arc::new(RwLock::new(projections)),
            storage,
            websocket_manager: Arc::new(WebSocketManager::new()),
            snapshot_manager: Arc::new(SnapshotManager::new(config.snapshot_config)),
            wal,
            compaction_manager,
            schema_registry,
            replay_manager,
            pipeline_manager,
            metrics,
            total_ingested: Arc::new(RwLock::new(0)),
        };

        // Recover from WAL first (most recent data)
        let mut wal_recovered = false;
        if let Some(ref wal) = store.wal {
            match wal.recover() {
                Ok(recovered_events) if !recovered_events.is_empty() => {
                    tracing::info!("🔄 Recovering {} events from WAL...", recovered_events.len());

                    for event in recovered_events {
                        // Re-index and process events from WAL
                        let offset = store.events.read().len();
                        if let Err(e) = store.index.index_event(
                            event.id,
                            &event.entity_id,
                            &event.event_type,
                            event.timestamp,
                            offset,
                        ) {
                            tracing::error!("Failed to re-index WAL event {}: {}", event.id, e);
                        }

                        if let Err(e) = store.projections.read().process_event(&event) {
                            tracing::error!("Failed to re-process WAL event {}: {}", event.id, e);
                        }

                        store.events.write().push(event);
                    }

                    let total = store.events.read().len();
                    *store.total_ingested.write() = total as u64;
                    tracing::info!("✅ Successfully recovered {} events from WAL", total);

                    // After successful recovery, checkpoint to Parquet if enabled
                    if store.storage.is_some() {
                        tracing::info!("📸 Checkpointing WAL to Parquet storage...");
                        if let Err(e) = store.flush_storage() {
                            tracing::error!("Failed to checkpoint to Parquet: {}", e);
                        } else if let Err(e) = wal.truncate() {
                            tracing::error!("Failed to truncate WAL after checkpoint: {}", e);
                        } else {
                            tracing::info!("✅ WAL checkpointed and truncated");
                        }
                    }

                    wal_recovered = true;
                }
                Ok(_) => {
                    tracing::debug!("No events to recover from WAL");
                }
                Err(e) => {
                    tracing::error!("❌ WAL recovery failed: {}", e);
                }
            }
        }

        // Load persisted events from Parquet only if we didn't recover from WAL
        // (to avoid loading the same events twice after WAL checkpoint)
        if !wal_recovered {
            if let Some(ref storage) = store.storage {
                if let Ok(persisted_events) = storage.read().load_all_events() {
                    tracing::info!("📂 Loading {} persisted events...", persisted_events.len());

                    for event in persisted_events {
                        // Re-index loaded events
                        let offset = store.events.read().len();
                        if let Err(e) = store.index.index_event(
                            event.id,
                            &event.entity_id,
                            &event.event_type,
                            event.timestamp,
                            offset,
                        ) {
                            tracing::error!("Failed to re-index event {}: {}", event.id, e);
                        }

                        // Re-process through projections
                        if let Err(e) = store.projections.read().process_event(&event) {
                            tracing::error!("Failed to re-process event {}: {}", event.id, e);
                        }

                        store.events.write().push(event);
                    }

                    let total = store.events.read().len();
                    *store.total_ingested.write() = total as u64;
                    tracing::info!("✅ Successfully loaded {} events from storage", total);
                }
            }
        }

        store
    }

    /// Ingest a new event into the store
    pub fn ingest(&self, event: Event) -> Result<()> {
        // Start metrics timer (v0.6 feature)
        let timer = self.metrics.ingestion_duration_seconds.start_timer();

        // Validate event
        let validation_result = self.validate_event(&event);
        if let Err(e) = validation_result {
            // Record ingestion error
            self.metrics.ingestion_errors_total.inc();
            timer.observe_duration();
            return Err(e);
        }

        // Write to WAL FIRST for durability (v0.2 feature)
        // This ensures event is persisted before processing
        if let Some(ref wal) = self.wal {
            if let Err(e) = wal.append(event.clone()) {
                self.metrics.ingestion_errors_total.inc();
                timer.observe_duration();
                return Err(e);
            }
        }

        let mut events = self.events.write();
        let offset = events.len();

        // Index the event
        self.index.index_event(
            event.id,
            &event.entity_id,
            &event.event_type,
            event.timestamp,
            offset,
        )?;

        // Process through projections
        let projections = self.projections.read();
        projections.process_event(&event)?;
        drop(projections); // Release lock

        // Process through pipelines (v0.5 feature)
        // Pipelines can transform, filter, and aggregate events in real-time
        let pipeline_results = self.pipeline_manager.process_event(&event);
        if !pipeline_results.is_empty() {
            tracing::debug!(
                "Event {} processed by {} pipeline(s)",
                event.id,
                pipeline_results.len()
            );
            // Pipeline results could be stored, emitted, or forwarded elsewhere
            // For now, we just log them for observability
            for (pipeline_id, result) in pipeline_results {
                tracing::trace!("Pipeline {} result: {:?}", pipeline_id, result);
            }
        }

        // Persist to Parquet storage if enabled (v0.2)
        if let Some(ref storage) = self.storage {
            let mut storage = storage.write();
            storage.append_event(event.clone())?;
        }

        // Store the event in memory
        events.push(event.clone());
        let total_events = events.len();
        drop(events); // Release lock early

        // Broadcast to WebSocket clients (v0.2 feature)
        self.websocket_manager.broadcast_event(Arc::new(event.clone()));

        // Check if automatic snapshot should be created (v0.2 feature)
        self.check_auto_snapshot(&event.entity_id, &event);

        // Update metrics (v0.6 feature)
        self.metrics.events_ingested_total.inc();
        self.metrics.events_ingested_by_type
            .with_label_values(&[&event.event_type])
            .inc();
        self.metrics.storage_events_total.set(total_events as i64);

        // Update legacy total counter
        let mut total = self.total_ingested.write();
        *total += 1;

        timer.observe_duration();

        tracing::debug!(
            "Event ingested: {} (offset: {})",
            event.id,
            offset
        );

        Ok(())
    }

    /// Get the WebSocket manager for this store
    pub fn websocket_manager(&self) -> Arc<WebSocketManager> {
        Arc::clone(&self.websocket_manager)
    }

    /// Get the snapshot manager for this store
    pub fn snapshot_manager(&self) -> Arc<SnapshotManager> {
        Arc::clone(&self.snapshot_manager)
    }

    /// Get the compaction manager for this store
    pub fn compaction_manager(&self) -> Option<Arc<CompactionManager>> {
        self.compaction_manager.as_ref().map(Arc::clone)
    }

    /// Get the schema registry for this store (v0.5 feature)
    pub fn schema_registry(&self) -> Arc<SchemaRegistry> {
        Arc::clone(&self.schema_registry)
    }

    /// Get the replay manager for this store (v0.5 feature)
    pub fn replay_manager(&self) -> Arc<ReplayManager> {
        Arc::clone(&self.replay_manager)
    }

    /// Get the pipeline manager for this store (v0.5 feature)
    pub fn pipeline_manager(&self) -> Arc<PipelineManager> {
        Arc::clone(&self.pipeline_manager)
    }

    /// Get the metrics registry for this store (v0.6 feature)
    pub fn metrics(&self) -> Arc<MetricsRegistry> {
        Arc::clone(&self.metrics)
    }

    /// Manually flush any pending events to persistent storage
    pub fn flush_storage(&self) -> Result<()> {
        if let Some(ref storage) = self.storage {
            let mut storage = storage.write();
            storage.flush()?;
            tracing::info!("✅ Flushed events to persistent storage");
        }
        Ok(())
    }

    /// Manually create a snapshot for an entity
    pub fn create_snapshot(&self, entity_id: &str) -> Result<()> {
        // Get all events for this entity
        let events = self.query(QueryEventsRequest {
            entity_id: Some(entity_id.to_string()),
            event_type: None,
            as_of: None,
            since: None,
            until: None,
            limit: None,
        })?;

        if events.is_empty() {
            return Err(AllSourceError::EntityNotFound(entity_id.to_string()));
        }

        // Build current state
        let mut state = serde_json::json!({});
        for event in &events {
            if let serde_json::Value::Object(ref mut state_map) = state {
                if let serde_json::Value::Object(ref payload_map) = event.payload {
                    for (key, value) in payload_map {
                        state_map.insert(key.clone(), value.clone());
                    }
                }
            }
        }

        let last_event = events.last().unwrap();
        self.snapshot_manager.create_snapshot(
            entity_id.to_string(),
            state,
            last_event.timestamp,
            events.len(),
            SnapshotType::Manual,
        )?;

        Ok(())
    }

    /// Check and create automatic snapshots if needed
    fn check_auto_snapshot(&self, entity_id: &str, event: &Event) {
        // Count events for this entity
        let entity_event_count = self
            .index
            .get_by_entity(entity_id)
            .map(|entries| entries.len())
            .unwrap_or(0);

        if self.snapshot_manager.should_create_snapshot(
            entity_id,
            entity_event_count,
            event.timestamp,
        ) {
            // Create snapshot in background (don't block ingestion)
            if let Err(e) = self.create_snapshot(entity_id) {
                tracing::warn!(
                    "Failed to create automatic snapshot for {}: {}",
                    entity_id,
                    e
                );
            }
        }
    }

    /// Validate an event before ingestion
    fn validate_event(&self, event: &Event) -> Result<()> {
        if event.entity_id.is_empty() {
            return Err(AllSourceError::ValidationError(
                "entity_id cannot be empty".to_string(),
            ));
        }

        if event.event_type.is_empty() {
            return Err(AllSourceError::ValidationError(
                "event_type cannot be empty".to_string(),
            ));
        }

        Ok(())
    }

    /// Query events based on filters (optimized with indices)
    pub fn query(&self, request: QueryEventsRequest) -> Result<Vec<Event>> {
        // Determine query type for metrics (v0.6 feature)
        let query_type = if request.entity_id.is_some() {
            "entity"
        } else if request.event_type.is_some() {
            "type"
        } else {
            "full_scan"
        };

        // Start metrics timer (v0.6 feature)
        let timer = self.metrics.query_duration_seconds
            .with_label_values(&[query_type])
            .start_timer();

        // Increment query counter (v0.6 feature)
        self.metrics.queries_total
            .with_label_values(&[query_type])
            .inc();

        let events = self.events.read();

        // Use index for fast lookups
        let offsets: Vec<usize> = if let Some(entity_id) = &request.entity_id {
            // Use entity index
            self.index
                .get_by_entity(entity_id)
                .map(|entries| self.filter_entries(entries, &request))
                .unwrap_or_default()
        } else if let Some(event_type) = &request.event_type {
            // Use type index
            self.index
                .get_by_type(event_type)
                .map(|entries| self.filter_entries(entries, &request))
                .unwrap_or_default()
        } else {
            // Full scan (less efficient but necessary for complex queries)
            (0..events.len()).collect()
        };

        // Fetch events and apply remaining filters
        let mut results: Vec<Event> = offsets
            .iter()
            .filter_map(|&offset| events.get(offset).cloned())
            .filter(|event| self.apply_filters(event, &request))
            .collect();

        // Sort by timestamp (ascending)
        results.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));

        // Apply limit
        if let Some(limit) = request.limit {
            results.truncate(limit);
        }

        // Record query results count (v0.6 feature)
        self.metrics.query_results_total
            .with_label_values(&[query_type])
            .inc_by(results.len() as u64);

        timer.observe_duration();

        Ok(results)
    }

    /// Filter index entries based on query parameters
    fn filter_entries(&self, entries: Vec<IndexEntry>, request: &QueryEventsRequest) -> Vec<usize> {
        entries
            .into_iter()
            .filter(|entry| {
                // Time filters
                if let Some(as_of) = request.as_of {
                    if entry.timestamp > as_of {
                        return false;
                    }
                }
                if let Some(since) = request.since {
                    if entry.timestamp < since {
                        return false;
                    }
                }
                if let Some(until) = request.until {
                    if entry.timestamp > until {
                        return false;
                    }
                }
                true
            })
            .map(|entry| entry.offset)
            .collect()
    }

    /// Apply filters to an event
    fn apply_filters(&self, event: &Event, request: &QueryEventsRequest) -> bool {
        // Additional type filter if entity was primary
        if request.entity_id.is_some() {
            if let Some(ref event_type) = request.event_type {
                if &event.event_type != event_type {
                    return false;
                }
            }
        }

        true
    }

    /// Reconstruct entity state as of a specific timestamp
    /// v0.2: Now uses snapshots for fast reconstruction
    pub fn reconstruct_state(
        &self,
        entity_id: &str,
        as_of: Option<DateTime<Utc>>,
    ) -> Result<serde_json::Value> {
        // Try to find a snapshot to use as a base (v0.2 optimization)
        let (merged_state, since_timestamp) = if let Some(as_of_time) = as_of {
            // Get snapshot closest to requested time
            if let Some(snapshot) = self.snapshot_manager.get_snapshot_as_of(entity_id, as_of_time) {
                tracing::debug!(
                    "Using snapshot from {} for entity {} (saved {} events)",
                    snapshot.as_of,
                    entity_id,
                    snapshot.event_count
                );
                (snapshot.state.clone(), Some(snapshot.as_of))
            } else {
                (serde_json::json!({}), None)
            }
        } else {
            // Get latest snapshot for current state
            if let Some(snapshot) = self.snapshot_manager.get_latest_snapshot(entity_id) {
                tracing::debug!(
                    "Using latest snapshot from {} for entity {}",
                    snapshot.as_of,
                    entity_id
                );
                (snapshot.state.clone(), Some(snapshot.as_of))
            } else {
                (serde_json::json!({}), None)
            }
        };

        // Query events after the snapshot (or all if no snapshot)
        let events = self.query(QueryEventsRequest {
            entity_id: Some(entity_id.to_string()),
            event_type: None,
            as_of,
            since: since_timestamp,
            until: None,
            limit: None,
        })?;

        // If no events and no snapshot, entity not found
        if events.is_empty() && since_timestamp.is_none() {
            return Err(AllSourceError::EntityNotFound(entity_id.to_string()));
        }

        // Merge events on top of snapshot (or from scratch if no snapshot)
        let mut merged_state = merged_state;
        for event in &events {
            if let serde_json::Value::Object(ref mut state_map) = merged_state {
                if let serde_json::Value::Object(ref payload_map) = event.payload {
                    for (key, value) in payload_map {
                        state_map.insert(key.clone(), value.clone());
                    }
                }
            }
        }

        // Wrap with metadata
        let state = serde_json::json!({
            "entity_id": entity_id,
            "last_updated": events.last().map(|e| e.timestamp),
            "event_count": events.len(),
            "as_of": as_of,
            "current_state": merged_state,
            "history": events.iter().map(|e| {
                serde_json::json!({
                    "event_id": e.id,
                    "type": e.event_type,
                    "timestamp": e.timestamp,
                    "payload": e.payload
                })
            }).collect::<Vec<_>>()
        });

        Ok(state)
    }

    /// Get snapshot from projection (faster than reconstructing)
    pub fn get_snapshot(&self, entity_id: &str) -> Result<serde_json::Value> {
        let projections = self.projections.read();

        if let Some(snapshot_projection) = projections.get_projection("entity_snapshots") {
            if let Some(state) = snapshot_projection.get_state(entity_id) {
                return Ok(serde_json::json!({
                    "entity_id": entity_id,
                    "snapshot": state,
                    "from_projection": "entity_snapshots"
                }));
            }
        }

        Err(AllSourceError::EntityNotFound(entity_id.to_string()))
    }

    /// Get statistics about the event store
    pub fn stats(&self) -> StoreStats {
        let events = self.events.read();
        let index_stats = self.index.stats();

        StoreStats {
            total_events: events.len(),
            total_entities: index_stats.total_entities,
            total_event_types: index_stats.total_event_types,
            total_ingested: *self.total_ingested.read(),
        }
    }
}

/// Configuration for EventStore
#[derive(Debug, Clone)]
pub struct EventStoreConfig {
    /// Optional directory for persistent Parquet storage (v0.2 feature)
    pub storage_dir: Option<PathBuf>,

    /// Snapshot configuration (v0.2 feature)
    pub snapshot_config: SnapshotConfig,

    /// Optional directory for WAL (Write-Ahead Log) (v0.2 feature)
    pub wal_dir: Option<PathBuf>,

    /// WAL configuration (v0.2 feature)
    pub wal_config: WALConfig,

    /// Compaction configuration (v0.2 feature)
    pub compaction_config: CompactionConfig,

    /// Schema registry configuration (v0.5 feature)
    pub schema_registry_config: SchemaRegistryConfig,
}

impl Default for EventStoreConfig {
    fn default() -> Self {
        Self {
            storage_dir: None,
            snapshot_config: SnapshotConfig::default(),
            wal_dir: None,
            wal_config: WALConfig::default(),
            compaction_config: CompactionConfig::default(),
            schema_registry_config: SchemaRegistryConfig::default(),
        }
    }
}

impl EventStoreConfig {
    /// Create config with persistent storage enabled
    pub fn with_persistence(storage_dir: impl Into<PathBuf>) -> Self {
        Self {
            storage_dir: Some(storage_dir.into()),
            snapshot_config: SnapshotConfig::default(),
            wal_dir: None,
            wal_config: WALConfig::default(),
            compaction_config: CompactionConfig::default(),
            schema_registry_config: SchemaRegistryConfig::default(),
        }
    }

    /// Create config with custom snapshot settings
    pub fn with_snapshots(snapshot_config: SnapshotConfig) -> Self {
        Self {
            storage_dir: None,
            snapshot_config,
            wal_dir: None,
            wal_config: WALConfig::default(),
            compaction_config: CompactionConfig::default(),
            schema_registry_config: SchemaRegistryConfig::default(),
        }
    }

    /// Create config with WAL enabled
    pub fn with_wal(wal_dir: impl Into<PathBuf>, wal_config: WALConfig) -> Self {
        Self {
            storage_dir: None,
            snapshot_config: SnapshotConfig::default(),
            wal_dir: Some(wal_dir.into()),
            wal_config,
            compaction_config: CompactionConfig::default(),
            schema_registry_config: SchemaRegistryConfig::default(),
        }
    }

    /// Create config with both persistence and snapshots
    pub fn with_all(storage_dir: impl Into<PathBuf>, snapshot_config: SnapshotConfig) -> Self {
        Self {
            storage_dir: Some(storage_dir.into()),
            snapshot_config,
            wal_dir: None,
            wal_config: WALConfig::default(),
            compaction_config: CompactionConfig::default(),
            schema_registry_config: SchemaRegistryConfig::default(),
        }
    }

    /// Create production config with all features enabled
    pub fn production(
        storage_dir: impl Into<PathBuf>,
        wal_dir: impl Into<PathBuf>,
        snapshot_config: SnapshotConfig,
        wal_config: WALConfig,
        compaction_config: CompactionConfig,
    ) -> Self {
        Self {
            storage_dir: Some(storage_dir.into()),
            snapshot_config,
            wal_dir: Some(wal_dir.into()),
            wal_config,
            compaction_config,
            schema_registry_config: SchemaRegistryConfig::default(),
        }
    }
}

#[derive(Debug, serde::Serialize)]
pub struct StoreStats {
    pub total_events: usize,
    pub total_entities: usize,
    pub total_event_types: usize,
    pub total_ingested: u64,
}

impl Default for EventStore {
    fn default() -> Self {
        Self::new()
    }
}

// Tests for store are covered in integration tests
