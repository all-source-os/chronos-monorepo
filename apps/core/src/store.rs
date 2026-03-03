#[cfg(feature = "server")]
use crate::application::services::webhook::WebhookRegistry;
#[cfg(feature = "server")]
use crate::infrastructure::observability::metrics::MetricsRegistry;
#[cfg(feature = "server")]
use crate::infrastructure::web::websocket::WebSocketManager;
use crate::{
    application::{
        dto::QueryEventsRequest,
        services::{
            exactly_once::{ExactlyOnceConfig, ExactlyOnceRegistry},
            pipeline::PipelineManager,
            projection::{EntitySnapshotProjection, EventCounterProjection, ProjectionManager},
            replay::ReplayManager,
            schema::{SchemaRegistry, SchemaRegistryConfig},
            schema_evolution::SchemaEvolutionManager,
        },
    },
    domain::entities::Event,
    error::{AllSourceError, Result},
    infrastructure::{
        persistence::{
            compaction::{CompactionConfig, CompactionManager},
            index::{EventIndex, IndexEntry},
            snapshot::{SnapshotConfig, SnapshotManager, SnapshotType},
            storage::ParquetStorage,
            wal::{WALConfig, WriteAheadLog},
        },
        query::geospatial::GeoIndex,
    },
};
use chrono::{DateTime, Utc};
use dashmap::DashMap;
use parking_lot::RwLock;
use std::{path::PathBuf, sync::Arc};
#[cfg(feature = "server")]
use tokio::sync::mpsc;

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
    #[cfg(feature = "server")]
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
    #[cfg(feature = "server")]
    metrics: Arc<MetricsRegistry>,

    /// Total events ingested (for metrics)
    total_ingested: Arc<RwLock<u64>>,

    /// Projection state cache for Query Service integration (v0.7 feature)
    /// Key format: "{projection_name}:{entity_id}"
    /// This DashMap provides O(1) access with ~11.9 μs latency
    projection_state_cache: Arc<DashMap<String, serde_json::Value>>,

    /// Projection status overrides (v0.13 feature)
    /// Tracks pause/start state per projection name: "running" or "paused"
    projection_status: Arc<DashMap<String, String>>,

    /// Webhook registry for outbound event delivery (v0.11 feature)
    #[cfg(feature = "server")]
    webhook_registry: Arc<WebhookRegistry>,

    /// Channel sender for async webhook delivery tasks
    #[cfg(feature = "server")]
    webhook_tx: Arc<RwLock<Option<mpsc::UnboundedSender<WebhookDeliveryTask>>>>,

    /// Geospatial index for coordinate-based queries (v2.0 feature)
    geo_index: Arc<GeoIndex>,

    /// Exactly-once processing registry (v2.0 feature)
    exactly_once: Arc<ExactlyOnceRegistry>,

    /// Autonomous schema evolution manager (v2.0 feature)
    schema_evolution: Arc<SchemaEvolutionManager>,
}

/// A task queued for async webhook delivery
#[cfg(feature = "server")]
#[derive(Debug, Clone)]
pub struct WebhookDeliveryTask {
    pub webhook: crate::application::services::webhook::WebhookSubscription,
    pub event: Event,
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
        let storage = config
            .storage_dir
            .as_ref()
            .and_then(|dir| match ParquetStorage::new(dir) {
                Ok(storage) => {
                    tracing::info!("✅ Parquet persistence enabled at: {}", dir.display());
                    Some(Arc::new(RwLock::new(storage)))
                }
                Err(e) => {
                    tracing::error!("❌ Failed to initialize Parquet storage: {}", e);
                    None
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
        #[cfg(feature = "server")]
        let metrics = {
            let m = MetricsRegistry::new();
            tracing::info!("✅ Prometheus metrics registry initialized");
            m
        };

        // Initialize projection state cache (v0.7 feature)
        let projection_state_cache = Arc::new(DashMap::new());
        tracing::info!("✅ Projection state cache initialized");

        // Initialize webhook registry (v0.11 feature)
        #[cfg(feature = "server")]
        let webhook_registry = {
            let w = Arc::new(WebhookRegistry::new());
            tracing::info!("✅ Webhook registry initialized");
            w
        };

        let store = Self {
            events: Arc::new(RwLock::new(Vec::new())),
            index: Arc::new(EventIndex::new()),
            projections: Arc::new(RwLock::new(projections)),
            storage,
            #[cfg(feature = "server")]
            websocket_manager: Arc::new(WebSocketManager::new()),
            snapshot_manager: Arc::new(SnapshotManager::new(config.snapshot_config)),
            wal,
            compaction_manager,
            schema_registry,
            replay_manager,
            pipeline_manager,
            #[cfg(feature = "server")]
            metrics,
            total_ingested: Arc::new(RwLock::new(0)),
            projection_state_cache,
            projection_status: Arc::new(DashMap::new()),
            #[cfg(feature = "server")]
            webhook_registry,
            #[cfg(feature = "server")]
            webhook_tx: Arc::new(RwLock::new(None)),
            geo_index: Arc::new(GeoIndex::new()),
            exactly_once: Arc::new(ExactlyOnceRegistry::new(ExactlyOnceConfig::default())),
            schema_evolution: Arc::new(SchemaEvolutionManager::new()),
        };

        // Step 1: Load persisted events from Parquet (the durable baseline)
        if let Some(ref storage) = store.storage
            && let Ok(persisted_events) = storage.read().load_all_events()
            && !persisted_events.is_empty()
        {
            tracing::info!("📂 Loading {} persisted events...", persisted_events.len());

            for event in persisted_events {
                let offset = store.events.read().len();
                if let Err(e) = store.index.index_event(
                    event.id,
                    event.entity_id_str(),
                    event.event_type_str(),
                    event.timestamp,
                    offset,
                ) {
                    tracing::error!("Failed to re-index event {}: {}", event.id, e);
                }

                if let Err(e) = store.projections.read().process_event(&event) {
                    tracing::error!("Failed to re-process event {}: {}", event.id, e);
                }

                store.events.write().push(event);
            }

            let total = store.events.read().len();
            *store.total_ingested.write() = total as u64;
            tracing::info!("✅ Successfully loaded {} events from storage", total);
        }

        // Step 2: Recover WAL events (written after last Parquet checkpoint)
        if let Some(ref wal) = store.wal {
            match wal.recover() {
                Ok(recovered_events) if !recovered_events.is_empty() => {
                    // Collect IDs already loaded from Parquet to skip duplicates
                    let existing_ids: std::collections::HashSet<uuid::Uuid> =
                        store.events.read().iter().map(|e| e.id).collect();

                    let mut wal_new = 0usize;
                    for event in recovered_events {
                        if existing_ids.contains(&event.id) {
                            continue; // already loaded from Parquet
                        }

                        let offset = store.events.read().len();
                        if let Err(e) = store.index.index_event(
                            event.id,
                            event.entity_id_str(),
                            event.event_type_str(),
                            event.timestamp,
                            offset,
                        ) {
                            tracing::error!("Failed to re-index WAL event {}: {}", event.id, e);
                        }

                        if let Err(e) = store.projections.read().process_event(&event) {
                            tracing::error!("Failed to re-process WAL event {}: {}", event.id, e);
                        }

                        store.events.write().push(event);
                        wal_new += 1;
                    }

                    if wal_new > 0 {
                        let total = store.events.read().len();
                        *store.total_ingested.write() = total as u64;
                        tracing::info!(
                            "✅ Recovered {} new events from WAL ({} total)",
                            wal_new,
                            total
                        );

                        // Checkpoint WAL events to Parquet — buffer them into
                        // the Parquet batch first, then flush. Without this,
                        // flush_storage() finds an empty current_batch and
                        // silently no-ops, then we truncate the WAL and the
                        // events exist only in memory (lost on next restart).
                        if let Some(ref storage) = store.storage {
                            tracing::info!(
                                "📸 Checkpointing {} WAL events to Parquet storage...",
                                wal_new
                            );
                            let parquet = storage.read();
                            let events = store.events.read();
                            let mut buffered = 0usize;
                            for event in events.iter().skip(events.len() - wal_new) {
                                if let Err(e) = parquet.append_event(event.clone()) {
                                    tracing::error!(
                                        "Failed to buffer WAL event for Parquet: {}",
                                        e
                                    );
                                } else {
                                    buffered += 1;
                                }
                            }
                            drop(events);
                            drop(parquet);

                            if buffered > 0 {
                                if let Err(e) = store.flush_storage() {
                                    tracing::error!("Failed to checkpoint to Parquet: {}", e);
                                } else if let Err(e) = wal.truncate() {
                                    tracing::error!(
                                        "Failed to truncate WAL after checkpoint: {}",
                                        e
                                    );
                                } else {
                                    tracing::info!(
                                        "✅ WAL checkpointed and truncated ({} events)",
                                        buffered
                                    );
                                }
                            }
                        }
                    }
                }
                Ok(_) => {
                    tracing::debug!("No events to recover from WAL");
                }
                Err(e) => {
                    tracing::error!("❌ WAL recovery failed: {}", e);
                }
            }
        }

        store
    }

    /// Ingest a new event into the store
    pub fn ingest(&self, event: Event) -> Result<()> {
        // Start metrics timer (v0.6 feature)
        #[cfg(feature = "server")]
        let timer = self.metrics.ingestion_duration_seconds.start_timer();

        // Validate event
        let validation_result = self.validate_event(&event);
        if let Err(e) = validation_result {
            #[cfg(feature = "server")]
            {
                self.metrics.ingestion_errors_total.inc();
                timer.observe_duration();
            }
            return Err(e);
        }

        // Write to WAL FIRST for durability (v0.2 feature)
        // This ensures event is persisted before processing
        if let Some(ref wal) = self.wal
            && let Err(e) = wal.append(event.clone())
        {
            #[cfg(feature = "server")]
            {
                self.metrics.ingestion_errors_total.inc();
                timer.observe_duration();
            }
            return Err(e);
        }

        let mut events = self.events.write();
        let offset = events.len();

        // Index the event
        self.index.index_event(
            event.id,
            event.entity_id_str(),
            event.event_type_str(),
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
            let storage = storage.read();
            storage.append_event(event.clone())?;
        }

        // Store the event in memory
        events.push(event.clone());
        let total_events = events.len();
        drop(events); // Release lock early

        // Broadcast to WebSocket clients (v0.2 feature)
        #[cfg(feature = "server")]
        self.websocket_manager
            .broadcast_event(Arc::new(event.clone()));

        // Dispatch to matching webhook subscriptions (v0.11 feature)
        #[cfg(feature = "server")]
        self.dispatch_webhooks(&event);

        // Update geospatial index (v2.0 feature)
        self.geo_index.index_event(&event);

        // Autonomous schema evolution (v2.0 feature)
        self.schema_evolution
            .analyze_event(event.event_type_str(), &event.payload);

        // Check if automatic snapshot should be created (v0.2 feature)
        self.check_auto_snapshot(event.entity_id_str(), &event);

        // Update metrics (v0.6 feature)
        #[cfg(feature = "server")]
        {
            self.metrics.events_ingested_total.inc();
            self.metrics
                .events_ingested_by_type
                .with_label_values(&[event.event_type_str()])
                .inc();
            self.metrics.storage_events_total.set(total_events as i64);
        }

        // Update legacy total counter
        let mut total = self.total_ingested.write();
        *total += 1;

        #[cfg(feature = "server")]
        timer.observe_duration();

        tracing::debug!("Event ingested: {} (offset: {})", event.id, offset);

        Ok(())
    }

    /// Ingest a batch of events with a single write lock acquisition.
    ///
    /// All events are validated first. If any event fails validation, no
    /// events are stored (all-or-nothing validation). Events are then written
    /// to WAL, indexed, processed through projections, and pushed to the
    /// events vector under a single write lock.
    pub fn ingest_batch(&self, batch: Vec<Event>) -> Result<()> {
        if batch.is_empty() {
            return Ok(());
        }

        // Phase 1: Validate all events before acquiring any locks
        for event in &batch {
            self.validate_event(event)?;
        }

        // Phase 2: Write all events to WAL (before write lock, for durability)
        if let Some(ref wal) = self.wal {
            for event in &batch {
                wal.append(event.clone())?;
            }
        }

        // Phase 3: Single write lock for index + projections + push
        let mut events = self.events.write();
        let projections = self.projections.read();

        for event in batch {
            let offset = events.len();

            self.index.index_event(
                event.id,
                event.entity_id_str(),
                event.event_type_str(),
                event.timestamp,
                offset,
            )?;

            projections.process_event(&event)?;
            self.pipeline_manager.process_event(&event);

            if let Some(ref storage) = self.storage {
                let storage = storage.read();
                storage.append_event(event.clone())?;
            }

            self.geo_index.index_event(&event);
            self.schema_evolution
                .analyze_event(event.event_type_str(), &event.payload);

            events.push(event);
        }

        let total_events = events.len();
        drop(projections);
        drop(events);

        let mut total = self.total_ingested.write();
        *total += total_events as u64;

        Ok(())
    }

    /// Ingest a replicated event from the leader (follower mode).
    ///
    /// Unlike `ingest()`, this method:
    /// - Skips WAL writing (the follower's WalReceiver manages its own local WAL)
    /// - Skips schema validation (the leader already validated)
    /// - Still indexes, processes projections/pipelines, and broadcasts to WebSocket clients
    pub fn ingest_replicated(&self, event: Event) -> Result<()> {
        #[cfg(feature = "server")]
        let timer = self.metrics.ingestion_duration_seconds.start_timer();

        let mut events = self.events.write();
        let offset = events.len();

        // Index the event
        self.index.index_event(
            event.id,
            event.entity_id_str(),
            event.event_type_str(),
            event.timestamp,
            offset,
        )?;

        // Process through projections
        let projections = self.projections.read();
        projections.process_event(&event)?;
        drop(projections);

        // Process through pipelines
        let pipeline_results = self.pipeline_manager.process_event(&event);
        if !pipeline_results.is_empty() {
            tracing::debug!(
                "Replicated event {} processed by {} pipeline(s)",
                event.id,
                pipeline_results.len()
            );
        }

        // Store the event in memory
        events.push(event.clone());
        let total_events = events.len();
        drop(events);

        // Broadcast to WebSocket clients
        #[cfg(feature = "server")]
        self.websocket_manager
            .broadcast_event(Arc::new(event.clone()));

        // Update metrics
        #[cfg(feature = "server")]
        {
            self.metrics.events_ingested_total.inc();
            self.metrics
                .events_ingested_by_type
                .with_label_values(&[event.event_type_str()])
                .inc();
            self.metrics.storage_events_total.set(total_events as i64);
        }

        let mut total = self.total_ingested.write();
        *total += 1;

        #[cfg(feature = "server")]
        timer.observe_duration();

        tracing::debug!(
            "Replicated event ingested: {} (offset: {})",
            event.id,
            offset
        );

        Ok(())
    }

    /// Get the WebSocket manager for this store
    #[cfg(feature = "server")]
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
    #[cfg(feature = "server")]
    pub fn metrics(&self) -> Arc<MetricsRegistry> {
        Arc::clone(&self.metrics)
    }

    /// Get the projection manager for this store (v0.7 feature)
    pub fn projection_manager(&self) -> parking_lot::RwLockReadGuard<'_, ProjectionManager> {
        self.projections.read()
    }

    /// Register a custom projection at runtime.
    ///
    /// The projection will receive all future events via `process()`.
    /// Historical events are **not** replayed — only events ingested after
    /// registration will be processed by this projection.
    ///
    /// See [`register_projection_with_backfill`](Self::register_projection_with_backfill)
    /// to also process historical events.
    pub fn register_projection(
        &self,
        projection: Arc<dyn crate::application::services::projection::Projection>,
    ) {
        let mut pm = self.projections.write();
        pm.register(projection);
    }

    /// Register a custom projection and replay all existing events through it.
    ///
    /// After registration, the projection will also receive all future events.
    /// Historical events are replayed under a read lock — the projection's
    /// internal state (typically DashMap) handles concurrent access.
    pub fn register_projection_with_backfill(
        &self,
        projection: Arc<dyn crate::application::services::projection::Projection>,
    ) -> Result<()> {
        // First register so future events are processed
        {
            let mut pm = self.projections.write();
            pm.register(Arc::clone(&projection));
        }

        // Then replay existing events under read lock
        let events = self.events.read();
        for event in events.iter() {
            projection.process(event)?;
        }

        Ok(())
    }

    /// Get the projection state cache for this store (v0.7 feature)
    /// Used by Elixir Query Service for state synchronization
    pub fn projection_state_cache(&self) -> Arc<DashMap<String, serde_json::Value>> {
        Arc::clone(&self.projection_state_cache)
    }

    /// Get the projection status map (v0.13 feature)
    pub fn projection_status(&self) -> Arc<DashMap<String, String>> {
        Arc::clone(&self.projection_status)
    }

    /// Get the webhook registry for this store (v0.11 feature)
    /// Geospatial index for coordinate-based queries (v2.0 feature)
    pub fn geo_index(&self) -> Arc<GeoIndex> {
        self.geo_index.clone()
    }

    /// Exactly-once processing registry (v2.0 feature)
    pub fn exactly_once(&self) -> Arc<ExactlyOnceRegistry> {
        self.exactly_once.clone()
    }

    /// Schema evolution manager (v2.0 feature)
    pub fn schema_evolution(&self) -> Arc<SchemaEvolutionManager> {
        self.schema_evolution.clone()
    }

    /// Get a read-locked snapshot of all events (for EventQL/GraphQL queries).
    ///
    /// Returns an `Arc` reference to the internal events vec, avoiding a full
    /// clone. The caller holds a read lock for the duration of the `Arc`
    /// lifetime — prefer short-lived usage.
    pub fn snapshot_events(&self) -> Vec<Event> {
        self.events.read().clone()
    }

    /// Compact token events for an entity by replacing matching events with a
    /// single merged event. Used by the embedded streaming feature.
    ///
    /// Returns `Ok(true)` if compaction was performed, `Ok(false)` if no
    /// matching events were found.
    ///
    /// **Note:** The merged event is processed through projections *without*
    /// clearing the removed events' projection state first. Projections that
    /// accumulate state (e.g., counters) should be designed to handle this
    /// (the merged event replaces individual tokens, not adds to them).
    ///
    /// **Crash safety:** The WAL append happens *after* the in-memory swap
    /// under the write lock. If the process crashes before the WAL write,
    /// no change is persisted — WAL replay restores the pre-compaction state.
    /// If the process crashes after the WAL write, replay sees the merged
    /// event (and the original tokens, which are idempotent to replay since
    /// the merged event supersedes them).
    ///
    /// The write lock is held for the swap + WAL write + index rebuild.
    /// The index rebuild is O(N) over all events, which is acceptable for
    /// embedded workloads but should not be called in hot paths for large stores.
    pub fn compact_entity_tokens(
        &self,
        entity_id: &str,
        token_event_type: &str,
        merged_event: Event,
    ) -> Result<bool> {
        // Phase 1: Read-only check — do we have anything to compact?
        {
            let events = self.events.read();
            let has_tokens = events
                .iter()
                .any(|e| e.entity_id_str() == entity_id && e.event_type_str() == token_event_type);
            if !has_tokens {
                return Ok(false);
            }
        }

        // Phase 2: Process merged event through projections (no write lock held)
        let projections = self.projections.read();
        projections.process_event(&merged_event)?;
        drop(projections);

        // Phase 3: Acquire write lock for the swap + WAL + index rebuild
        let mut events = self.events.write();

        events.retain(|e| {
            !(e.entity_id_str() == entity_id && e.event_type_str() == token_event_type)
        });

        events.push(merged_event.clone());

        // WAL append inside write lock: crash before this line = no change persisted.
        // Crash after = merged event in WAL, original tokens also in WAL but
        // superseded by the merged event's entity_id + event_type.
        if let Some(ref wal) = self.wal {
            wal.append(merged_event)?;
        }

        // Rebuild entire index since retain() shifted event positions.
        // Errors here indicate a corrupt event (missing entity_id/event_type)
        // which should not happen for well-formed events. Log and continue
        // rather than failing the entire compaction.
        self.index.clear();
        for (offset, event) in events.iter().enumerate() {
            if let Err(e) = self.index.index_event(
                event.id,
                event.entity_id_str(),
                event.event_type_str(),
                event.timestamp,
                offset,
            ) {
                tracing::warn!(
                    event_id = %event.id,
                    offset,
                    "Failed to re-index event during compaction: {e}"
                );
            }
        }

        Ok(true)
    }

    #[cfg(feature = "server")]
    pub fn webhook_registry(&self) -> Arc<WebhookRegistry> {
        Arc::clone(&self.webhook_registry)
    }

    /// Set the channel for async webhook delivery.
    /// Called during server startup to wire the delivery worker.
    #[cfg(feature = "server")]
    pub fn set_webhook_tx(&self, tx: mpsc::UnboundedSender<WebhookDeliveryTask>) {
        *self.webhook_tx.write() = Some(tx);
        tracing::info!("Webhook delivery channel connected");
    }

    /// Dispatch matching webhooks for a given event (non-blocking).
    #[cfg(feature = "server")]
    fn dispatch_webhooks(&self, event: &Event) {
        let matching = self.webhook_registry.find_matching(event);
        if matching.is_empty() {
            return;
        }

        let tx_guard = self.webhook_tx.read();
        if let Some(ref tx) = *tx_guard {
            for webhook in matching {
                let task = WebhookDeliveryTask {
                    webhook,
                    event: event.clone(),
                };
                if let Err(e) = tx.send(task) {
                    tracing::warn!("Failed to queue webhook delivery: {}", e);
                }
            }
        }
    }

    /// Manually flush any pending events to persistent storage
    pub fn flush_storage(&self) -> Result<()> {
        if let Some(ref storage) = self.storage {
            let storage = storage.read();
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
            tenant_id: None,
            as_of: None,
            since: None,
            until: None,
            limit: None,
            event_type_prefix: None,
            payload_filter: None,
        })?;

        if events.is_empty() {
            return Err(AllSourceError::EntityNotFound(entity_id.to_string()));
        }

        // Build current state
        let mut state = serde_json::json!({});
        for event in &events {
            if let serde_json::Value::Object(ref mut state_map) = state
                && let serde_json::Value::Object(ref payload_map) = event.payload
            {
                for (key, value) in payload_map {
                    state_map.insert(key.clone(), value.clone());
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
        // EntityId and EventType value objects already validate non-empty in their constructors
        // So these checks are now redundant, but we keep them for explicit validation
        if event.entity_id_str().is_empty() {
            return Err(AllSourceError::ValidationError(
                "entity_id cannot be empty".to_string(),
            ));
        }

        if event.event_type_str().is_empty() {
            return Err(AllSourceError::ValidationError(
                "event_type cannot be empty".to_string(),
            ));
        }

        // Reject system namespace events from user-facing ingestion.
        // System events are written exclusively via SystemMetadataStore.
        if event.event_type().is_system() {
            return Err(AllSourceError::ValidationError(
                "Event types starting with '_system.' are reserved for internal use".to_string(),
            ));
        }

        Ok(())
    }

    /// Reset a projection by clearing its state and reprocessing all events
    pub fn reset_projection(&self, name: &str) -> Result<usize> {
        let projection_manager = self.projections.read();
        let projection = projection_manager.get_projection(name).ok_or_else(|| {
            AllSourceError::EntityNotFound(format!("Projection '{name}' not found"))
        })?;

        // Clear existing state
        projection.clear();

        // Clear cached state for this projection
        let prefix = format!("{name}:");
        let keys_to_remove: Vec<String> = self
            .projection_state_cache
            .iter()
            .filter(|entry| entry.key().starts_with(&prefix))
            .map(|entry| entry.key().clone())
            .collect();
        for key in keys_to_remove {
            self.projection_state_cache.remove(&key);
        }

        // Reprocess all events through this projection
        let events = self.events.read();
        let mut reprocessed = 0usize;
        for event in events.iter() {
            if projection.process(event).is_ok() {
                reprocessed += 1;
            }
        }

        Ok(reprocessed)
    }

    /// Get a single event by its UUID
    pub fn get_event_by_id(&self, event_id: &uuid::Uuid) -> Result<Option<Event>> {
        if let Some(offset) = self.index.get_by_id(event_id) {
            let events = self.events.read();
            Ok(events.get(offset).cloned())
        } else {
            Ok(None)
        }
    }

    /// Query events based on filters (optimized with indices)
    pub fn query(&self, request: QueryEventsRequest) -> Result<Vec<Event>> {
        // Determine query type for metrics (v0.6 feature)
        let query_type = if request.entity_id.is_some() {
            "entity"
        } else if request.event_type.is_some() {
            "type"
        } else if request.event_type_prefix.is_some() {
            "type_prefix"
        } else {
            "full_scan"
        };

        // Start metrics timer (v0.6 feature)
        #[cfg(feature = "server")]
        let timer = self
            .metrics
            .query_duration_seconds
            .with_label_values(&[query_type])
            .start_timer();

        // Increment query counter (v0.6 feature)
        #[cfg(feature = "server")]
        self.metrics
            .queries_total
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
            // Use type index (exact match)
            self.index
                .get_by_type(event_type)
                .map(|entries| self.filter_entries(entries, &request))
                .unwrap_or_default()
        } else if let Some(prefix) = &request.event_type_prefix {
            // Use type index (prefix match)
            let entries = self.index.get_by_type_prefix(prefix);
            self.filter_entries(entries, &request)
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
        results.sort_by_key(|x| x.timestamp);

        // Apply limit
        if let Some(limit) = request.limit {
            results.truncate(limit);
        }

        // Record query results count (v0.6 feature)
        #[cfg(feature = "server")]
        {
            self.metrics
                .query_results_total
                .with_label_values(&[query_type])
                .inc_by(results.len() as u64);
            timer.observe_duration();
        }

        Ok(results)
    }

    /// Filter index entries based on query parameters
    fn filter_entries(&self, entries: Vec<IndexEntry>, request: &QueryEventsRequest) -> Vec<usize> {
        entries
            .into_iter()
            .filter(|entry| {
                // Time filters
                if let Some(as_of) = request.as_of
                    && entry.timestamp > as_of
                {
                    return false;
                }
                if let Some(since) = request.since
                    && entry.timestamp < since
                {
                    return false;
                }
                if let Some(until) = request.until
                    && entry.timestamp > until
                {
                    return false;
                }
                true
            })
            .map(|entry| entry.offset)
            .collect()
    }

    /// Apply filters to an event
    fn apply_filters(&self, event: &Event, request: &QueryEventsRequest) -> bool {
        // Additional type filter if entity was primary
        if request.entity_id.is_some()
            && let Some(ref event_type) = request.event_type
            && event.event_type_str() != event_type
        {
            return false;
        }

        // Additional prefix filter if entity was primary
        if request.entity_id.is_some()
            && let Some(ref prefix) = request.event_type_prefix
            && !event.event_type_str().starts_with(prefix)
        {
            return false;
        }

        // Payload field filtering
        if let Some(ref filter_str) = request.payload_filter
            && let Ok(filter_obj) =
                serde_json::from_str::<serde_json::Map<String, serde_json::Value>>(filter_str)
        {
            let payload = event.payload();
            for (key, expected_value) in &filter_obj {
                match payload.get(key) {
                    Some(actual_value) if actual_value == expected_value => {}
                    _ => return false,
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
            if let Some(snapshot) = self
                .snapshot_manager
                .get_snapshot_as_of(entity_id, as_of_time)
            {
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
            tenant_id: None,
            as_of,
            since: since_timestamp,
            until: None,
            limit: None,
            event_type_prefix: None,
            payload_filter: None,
        })?;

        // If no events and no snapshot, entity not found
        if events.is_empty() && since_timestamp.is_none() {
            return Err(AllSourceError::EntityNotFound(entity_id.to_string()));
        }

        // Merge events on top of snapshot (or from scratch if no snapshot)
        let mut merged_state = merged_state;
        for event in &events {
            if let serde_json::Value::Object(ref mut state_map) = merged_state
                && let serde_json::Value::Object(ref payload_map) = event.payload
            {
                for (key, value) in payload_map {
                    state_map.insert(key.clone(), value.clone());
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

        if let Some(snapshot_projection) = projections.get_projection("entity_snapshots")
            && let Some(state) = snapshot_projection.get_state(entity_id)
        {
            return Ok(serde_json::json!({
                "entity_id": entity_id,
                "snapshot": state,
                "from_projection": "entity_snapshots"
            }));
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

    /// Get all unique streams (entity_ids) in the store
    pub fn list_streams(&self) -> Vec<StreamInfo> {
        self.index
            .get_all_entities()
            .into_iter()
            .map(|entity_id| {
                let event_count = self
                    .index
                    .get_by_entity(&entity_id)
                    .map(|entries| entries.len())
                    .unwrap_or(0);
                let last_event_at = self
                    .index
                    .get_by_entity(&entity_id)
                    .and_then(|entries| entries.last().map(|e| e.timestamp));
                StreamInfo {
                    stream_id: entity_id,
                    event_count,
                    last_event_at,
                }
            })
            .collect()
    }

    /// Get all unique event types in the store
    pub fn list_event_types(&self) -> Vec<EventTypeInfo> {
        self.index
            .get_all_types()
            .into_iter()
            .map(|event_type| {
                let event_count = self
                    .index
                    .get_by_type(&event_type)
                    .map(|entries| entries.len())
                    .unwrap_or(0);
                let last_event_at = self
                    .index
                    .get_by_type(&event_type)
                    .and_then(|entries| entries.last().map(|e| e.timestamp));
                EventTypeInfo {
                    event_type,
                    event_count,
                    last_event_at,
                }
            })
            .collect()
    }

    /// Attach a broadcast sender to the WAL for replication.
    ///
    /// Thread-safe: can be called through `Arc<EventStore>` at runtime.
    /// Used during initial setup and during follower → leader promotion.
    /// When set, every WAL append publishes the entry to the broadcast
    /// channel so the WAL shipper can stream it to followers.
    pub fn enable_wal_replication(
        &self,
        tx: tokio::sync::broadcast::Sender<crate::infrastructure::persistence::wal::WALEntry>,
    ) {
        if let Some(ref wal_arc) = self.wal {
            wal_arc.set_replication_tx(tx);
            tracing::info!("WAL replication broadcast enabled");
        } else {
            tracing::warn!("Cannot enable WAL replication: WAL is not configured");
        }
    }

    /// Get a reference to the WAL (if configured).
    /// Used by the replication catch-up protocol to determine oldest available offset.
    pub fn wal(&self) -> Option<&Arc<WriteAheadLog>> {
        self.wal.as_ref()
    }

    /// Get a reference to the Parquet storage (if configured).
    /// Used by the replication catch-up protocol to stream snapshot files to followers.
    pub fn parquet_storage(&self) -> Option<&Arc<RwLock<ParquetStorage>>> {
        self.storage.as_ref()
    }
}

/// Configuration for EventStore
#[derive(Debug, Clone, Default)]
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

    /// Optional directory for system metadata storage (dogfood feature).
    /// When set, operational metadata (tenants, config, audit) is stored
    /// using AllSource's own event store rather than an external database.
    /// Defaults to `{storage_dir}/__system/` when storage_dir is set.
    pub system_data_dir: Option<PathBuf>,

    /// Name of the default tenant to auto-create on first boot.
    pub bootstrap_tenant: Option<String>,
}

impl EventStoreConfig {
    /// Create config with persistent storage enabled
    pub fn with_persistence(storage_dir: impl Into<PathBuf>) -> Self {
        Self {
            storage_dir: Some(storage_dir.into()),
            ..Self::default()
        }
    }

    /// Create config with custom snapshot settings
    pub fn with_snapshots(snapshot_config: SnapshotConfig) -> Self {
        Self {
            snapshot_config,
            ..Self::default()
        }
    }

    /// Create config with WAL enabled
    pub fn with_wal(wal_dir: impl Into<PathBuf>, wal_config: WALConfig) -> Self {
        Self {
            wal_dir: Some(wal_dir.into()),
            wal_config,
            ..Self::default()
        }
    }

    /// Create config with both persistence and snapshots
    pub fn with_all(storage_dir: impl Into<PathBuf>, snapshot_config: SnapshotConfig) -> Self {
        Self {
            storage_dir: Some(storage_dir.into()),
            snapshot_config,
            ..Self::default()
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
        let storage_dir = storage_dir.into();
        let system_data_dir = storage_dir.join("__system");
        Self {
            storage_dir: Some(storage_dir),
            snapshot_config,
            wal_dir: Some(wal_dir.into()),
            wal_config,
            compaction_config,
            system_data_dir: Some(system_data_dir),
            ..Self::default()
        }
    }

    /// Resolve the effective system data directory.
    ///
    /// If explicitly set, returns that. Otherwise, derives from storage_dir.
    /// Returns None if neither is configured (in-memory mode).
    pub fn effective_system_data_dir(&self) -> Option<PathBuf> {
        self.system_data_dir
            .clone()
            .or_else(|| self.storage_dir.as_ref().map(|d| d.join("__system")))
    }

    /// Build config from environment variables.
    ///
    /// Reads `ALLSOURCE_DATA_DIR`, `ALLSOURCE_STORAGE_DIR`, `ALLSOURCE_WAL_DIR`,
    /// and `ALLSOURCE_WAL_ENABLED` to determine persistence mode.
    ///
    /// Returns `(config, description)` where description is a human-readable
    /// summary of the persistence mode for logging.
    pub fn from_env() -> (Self, &'static str) {
        Self::from_env_vars(
            std::env::var("ALLSOURCE_DATA_DIR")
                .ok()
                .filter(|s| !s.is_empty()),
            std::env::var("ALLSOURCE_STORAGE_DIR")
                .ok()
                .filter(|s| !s.is_empty()),
            std::env::var("ALLSOURCE_WAL_DIR")
                .ok()
                .filter(|s| !s.is_empty()),
            std::env::var("ALLSOURCE_WAL_ENABLED").ok(),
        )
    }

    /// Build config from explicit env-var values (testable without mutating process env).
    pub fn from_env_vars(
        data_dir: Option<String>,
        explicit_storage_dir: Option<String>,
        explicit_wal_dir: Option<String>,
        wal_enabled_var: Option<String>,
    ) -> (Self, &'static str) {
        let data_dir = data_dir.filter(|s| !s.is_empty());
        let storage_dir = explicit_storage_dir
            .filter(|s| !s.is_empty())
            .or_else(|| data_dir.as_ref().map(|d| format!("{}/storage", d)));
        let wal_dir = explicit_wal_dir
            .filter(|s| !s.is_empty())
            .or_else(|| data_dir.as_ref().map(|d| format!("{}/wal", d)));
        let wal_enabled = wal_enabled_var.map(|v| v == "true").unwrap_or(true);

        match (&storage_dir, &wal_dir) {
            (Some(sd), Some(wd)) if wal_enabled => {
                let config = Self::production(
                    sd,
                    wd,
                    SnapshotConfig::default(),
                    WALConfig::default(),
                    CompactionConfig::default(),
                );
                (config, "wal+parquet")
            }
            (Some(sd), _) => {
                let config = Self::with_persistence(sd);
                (config, "parquet-only")
            }
            (_, Some(wd)) if wal_enabled => {
                let config = Self::with_wal(wd, WALConfig::default());
                (config, "wal-only")
            }
            _ => (Self::default(), "in-memory"),
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

/// Information about a stream (entity_id)
#[derive(Debug, Clone, serde::Serialize)]
pub struct StreamInfo {
    /// The stream identifier (entity_id)
    pub stream_id: String,
    /// Total number of events in this stream
    pub event_count: usize,
    /// Timestamp of the last event in this stream
    pub last_event_at: Option<chrono::DateTime<chrono::Utc>>,
}

/// Information about an event type
#[derive(Debug, Clone, serde::Serialize)]
pub struct EventTypeInfo {
    /// The event type name
    pub event_type: String,
    /// Total number of events of this type
    pub event_count: usize,
    /// Timestamp of the last event of this type
    pub last_event_at: Option<chrono::DateTime<chrono::Utc>>,
}

impl Default for EventStore {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::entities::Event;
    use tempfile::TempDir;

    fn create_test_event(entity_id: &str, event_type: &str) -> Event {
        Event::from_strings(
            event_type.to_string(),
            entity_id.to_string(),
            "default".to_string(),
            serde_json::json!({"name": "Test", "value": 42}),
            None,
        )
        .unwrap()
    }

    fn create_test_event_with_payload(
        entity_id: &str,
        event_type: &str,
        payload: serde_json::Value,
    ) -> Event {
        Event::from_strings(
            event_type.to_string(),
            entity_id.to_string(),
            "default".to_string(),
            payload,
            None,
        )
        .unwrap()
    }

    #[test]
    fn test_event_store_new() {
        let store = EventStore::new();
        assert_eq!(store.stats().total_events, 0);
        assert_eq!(store.stats().total_entities, 0);
    }

    #[test]
    fn test_event_store_default() {
        let store = EventStore::default();
        assert_eq!(store.stats().total_events, 0);
    }

    #[test]
    fn test_ingest_single_event() {
        let store = EventStore::new();
        let event = create_test_event("entity-1", "user.created");

        store.ingest(event).unwrap();

        assert_eq!(store.stats().total_events, 1);
        assert_eq!(store.stats().total_ingested, 1);
    }

    #[test]
    fn test_ingest_multiple_events() {
        let store = EventStore::new();

        for i in 0..10 {
            let event = create_test_event(&format!("entity-{}", i), "user.created");
            store.ingest(event).unwrap();
        }

        assert_eq!(store.stats().total_events, 10);
        assert_eq!(store.stats().total_ingested, 10);
    }

    #[test]
    fn test_query_by_entity_id() {
        let store = EventStore::new();

        store
            .ingest(create_test_event("entity-1", "user.created"))
            .unwrap();
        store
            .ingest(create_test_event("entity-2", "user.created"))
            .unwrap();
        store
            .ingest(create_test_event("entity-1", "user.updated"))
            .unwrap();

        let results = store
            .query(QueryEventsRequest {
                entity_id: Some("entity-1".to_string()),
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

        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_query_by_event_type() {
        let store = EventStore::new();

        store
            .ingest(create_test_event("entity-1", "user.created"))
            .unwrap();
        store
            .ingest(create_test_event("entity-2", "user.updated"))
            .unwrap();
        store
            .ingest(create_test_event("entity-3", "user.created"))
            .unwrap();

        let results = store
            .query(QueryEventsRequest {
                entity_id: None,
                event_type: Some("user.created".to_string()),
                tenant_id: None,
                as_of: None,
                since: None,
                until: None,
                limit: None,
                event_type_prefix: None,
                payload_filter: None,
            })
            .unwrap();

        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_query_with_limit() {
        let store = EventStore::new();

        for i in 0..10 {
            let event = create_test_event(&format!("entity-{}", i), "user.created");
            store.ingest(event).unwrap();
        }

        let results = store
            .query(QueryEventsRequest {
                entity_id: None,
                event_type: None,
                tenant_id: None,
                as_of: None,
                since: None,
                until: None,
                limit: Some(5),
                event_type_prefix: None,
                payload_filter: None,
            })
            .unwrap();

        assert_eq!(results.len(), 5);
    }

    #[test]
    fn test_query_empty_store() {
        let store = EventStore::new();

        let results = store
            .query(QueryEventsRequest {
                entity_id: Some("non-existent".to_string()),
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

        assert!(results.is_empty());
    }

    #[test]
    fn test_reconstruct_state() {
        let store = EventStore::new();

        store
            .ingest(create_test_event("entity-1", "user.created"))
            .unwrap();

        let state = store.reconstruct_state("entity-1", None).unwrap();
        // The state is wrapped with metadata
        assert_eq!(state["current_state"]["name"], "Test");
        assert_eq!(state["current_state"]["value"], 42);
    }

    #[test]
    fn test_reconstruct_state_not_found() {
        let store = EventStore::new();

        let result = store.reconstruct_state("non-existent", None);
        assert!(result.is_err());
    }

    #[test]
    fn test_get_snapshot_empty() {
        let store = EventStore::new();

        let result = store.get_snapshot("non-existent");
        // Entity not found error is expected
        assert!(result.is_err());
    }

    #[test]
    fn test_create_snapshot() {
        let store = EventStore::new();

        store
            .ingest(create_test_event("entity-1", "user.created"))
            .unwrap();

        store.create_snapshot("entity-1").unwrap();

        // Verify snapshot was created
        let snapshot = store.get_snapshot("entity-1").unwrap();
        assert!(snapshot != serde_json::json!(null));
    }

    #[test]
    fn test_create_snapshot_entity_not_found() {
        let store = EventStore::new();

        let result = store.create_snapshot("non-existent");
        assert!(result.is_err());
    }

    #[test]
    fn test_websocket_manager() {
        let store = EventStore::new();
        let manager = store.websocket_manager();
        // Manager should be accessible
        assert!(Arc::strong_count(&manager) >= 1);
    }

    #[test]
    fn test_snapshot_manager() {
        let store = EventStore::new();
        let manager = store.snapshot_manager();
        assert!(Arc::strong_count(&manager) >= 1);
    }

    #[test]
    fn test_compaction_manager_none() {
        let store = EventStore::new();
        // Without storage_dir, compaction manager should be None
        assert!(store.compaction_manager().is_none());
    }

    #[test]
    fn test_schema_registry() {
        let store = EventStore::new();
        let registry = store.schema_registry();
        assert!(Arc::strong_count(&registry) >= 1);
    }

    #[test]
    fn test_replay_manager() {
        let store = EventStore::new();
        let manager = store.replay_manager();
        assert!(Arc::strong_count(&manager) >= 1);
    }

    #[test]
    fn test_pipeline_manager() {
        let store = EventStore::new();
        let manager = store.pipeline_manager();
        assert!(Arc::strong_count(&manager) >= 1);
    }

    #[test]
    fn test_projection_manager() {
        let store = EventStore::new();
        let manager = store.projection_manager();
        // Built-in projections should be registered
        let projections = manager.list_projections();
        assert!(projections.len() >= 2); // entity_snapshots and event_counters
    }

    #[test]
    fn test_projection_state_cache() {
        let store = EventStore::new();
        let cache = store.projection_state_cache();

        cache.insert("test:key".to_string(), serde_json::json!({"value": 123}));
        assert_eq!(cache.len(), 1);

        let value = cache.get("test:key").unwrap();
        assert_eq!(value["value"], 123);
    }

    #[test]
    fn test_metrics() {
        let store = EventStore::new();
        let metrics = store.metrics();
        assert!(Arc::strong_count(&metrics) >= 1);
    }

    #[test]
    fn test_store_stats() {
        let store = EventStore::new();

        store
            .ingest(create_test_event("entity-1", "user.created"))
            .unwrap();
        store
            .ingest(create_test_event("entity-2", "order.placed"))
            .unwrap();

        let stats = store.stats();
        assert_eq!(stats.total_events, 2);
        assert_eq!(stats.total_entities, 2);
        assert_eq!(stats.total_event_types, 2);
        assert_eq!(stats.total_ingested, 2);
    }

    #[test]
    fn test_event_store_config_default() {
        let config = EventStoreConfig::default();
        assert!(config.storage_dir.is_none());
        assert!(config.wal_dir.is_none());
    }

    #[test]
    fn test_event_store_config_with_persistence() {
        let temp_dir = TempDir::new().unwrap();
        let config = EventStoreConfig::with_persistence(temp_dir.path());

        assert!(config.storage_dir.is_some());
        assert!(config.wal_dir.is_none());
    }

    #[test]
    fn test_event_store_config_with_wal() {
        let temp_dir = TempDir::new().unwrap();
        let config = EventStoreConfig::with_wal(temp_dir.path(), WALConfig::default());

        assert!(config.storage_dir.is_none());
        assert!(config.wal_dir.is_some());
    }

    #[test]
    fn test_event_store_config_with_all() {
        let temp_dir = TempDir::new().unwrap();
        let config = EventStoreConfig::with_all(temp_dir.path(), SnapshotConfig::default());

        assert!(config.storage_dir.is_some());
    }

    #[test]
    fn test_event_store_config_production() {
        let storage_dir = TempDir::new().unwrap();
        let wal_dir = TempDir::new().unwrap();
        let config = EventStoreConfig::production(
            storage_dir.path(),
            wal_dir.path(),
            SnapshotConfig::default(),
            WALConfig::default(),
            CompactionConfig::default(),
        );

        assert!(config.storage_dir.is_some());
        assert!(config.wal_dir.is_some());
    }

    // -----------------------------------------------------------------------
    // from_env_vars tests — verifies the env-var-to-config wiring that
    // caused the durability bug (events lost on restart) in v0.10.3.
    // -----------------------------------------------------------------------

    #[test]
    fn test_from_env_vars_data_dir_enables_full_persistence() {
        let (config, mode) =
            EventStoreConfig::from_env_vars(Some("/app/data".to_string()), None, None, None);
        assert_eq!(mode, "wal+parquet");
        assert_eq!(
            config.storage_dir.unwrap().to_str().unwrap(),
            "/app/data/storage"
        );
        assert_eq!(config.wal_dir.unwrap().to_str().unwrap(), "/app/data/wal");
    }

    #[test]
    fn test_from_env_vars_explicit_dirs() {
        let (config, mode) = EventStoreConfig::from_env_vars(
            None,
            Some("/custom/storage".to_string()),
            Some("/custom/wal".to_string()),
            None,
        );
        assert_eq!(mode, "wal+parquet");
        assert_eq!(
            config.storage_dir.unwrap().to_str().unwrap(),
            "/custom/storage"
        );
        assert_eq!(config.wal_dir.unwrap().to_str().unwrap(), "/custom/wal");
    }

    #[test]
    fn test_from_env_vars_wal_disabled() {
        let (config, mode) = EventStoreConfig::from_env_vars(
            Some("/app/data".to_string()),
            None,
            None,
            Some("false".to_string()),
        );
        assert_eq!(mode, "parquet-only");
        assert!(config.storage_dir.is_some());
        assert!(config.wal_dir.is_none());
    }

    #[test]
    fn test_from_env_vars_no_dirs_is_in_memory() {
        let (config, mode) = EventStoreConfig::from_env_vars(None, None, None, None);
        assert_eq!(mode, "in-memory");
        assert!(config.storage_dir.is_none());
        assert!(config.wal_dir.is_none());
    }

    #[test]
    fn test_from_env_vars_empty_strings_treated_as_none() {
        let (_, mode) = EventStoreConfig::from_env_vars(
            Some("".to_string()),
            Some("".to_string()),
            Some("".to_string()),
            None,
        );
        assert_eq!(mode, "in-memory");
    }

    #[test]
    fn test_from_env_vars_explicit_overrides_data_dir() {
        let (config, mode) = EventStoreConfig::from_env_vars(
            Some("/app/data".to_string()),
            Some("/override/storage".to_string()),
            Some("/override/wal".to_string()),
            None,
        );
        assert_eq!(mode, "wal+parquet");
        assert_eq!(
            config.storage_dir.unwrap().to_str().unwrap(),
            "/override/storage"
        );
        assert_eq!(config.wal_dir.unwrap().to_str().unwrap(), "/override/wal");
    }

    #[test]
    fn test_from_env_vars_wal_only() {
        let (config, mode) =
            EventStoreConfig::from_env_vars(None, None, Some("/wal/only".to_string()), None);
        assert_eq!(mode, "wal-only");
        assert!(config.storage_dir.is_none());
        assert_eq!(config.wal_dir.unwrap().to_str().unwrap(), "/wal/only");
    }

    #[test]
    fn test_store_stats_serde() {
        let stats = StoreStats {
            total_events: 100,
            total_entities: 50,
            total_event_types: 10,
            total_ingested: 100,
        };

        let json = serde_json::to_string(&stats).unwrap();
        assert!(json.contains("\"total_events\":100"));
        assert!(json.contains("\"total_entities\":50"));
    }

    #[test]
    fn test_query_with_entity_and_type() {
        let store = EventStore::new();

        store
            .ingest(create_test_event("entity-1", "user.created"))
            .unwrap();
        store
            .ingest(create_test_event("entity-1", "user.updated"))
            .unwrap();
        store
            .ingest(create_test_event("entity-2", "user.created"))
            .unwrap();

        let results = store
            .query(QueryEventsRequest {
                entity_id: Some("entity-1".to_string()),
                event_type: Some("user.created".to_string()),
                tenant_id: None,
                as_of: None,
                since: None,
                until: None,
                limit: None,
                event_type_prefix: None,
                payload_filter: None,
            })
            .unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].event_type_str(), "user.created");
    }

    #[test]
    fn test_query_by_event_type_prefix() {
        let store = EventStore::new();

        // Ingest events with various types
        store
            .ingest(create_test_event("entity-1", "index.created"))
            .unwrap();
        store
            .ingest(create_test_event("entity-2", "index.updated"))
            .unwrap();
        store
            .ingest(create_test_event("entity-3", "trade.created"))
            .unwrap();
        store
            .ingest(create_test_event("entity-4", "trade.completed"))
            .unwrap();
        store
            .ingest(create_test_event("entity-5", "balance.updated"))
            .unwrap();

        // Query with prefix "index." should return exactly 2
        let results = store
            .query(QueryEventsRequest {
                entity_id: None,
                event_type: None,
                tenant_id: None,
                as_of: None,
                since: None,
                until: None,
                limit: None,
                event_type_prefix: Some("index.".to_string()),
                payload_filter: None,
            })
            .unwrap();

        assert_eq!(results.len(), 2);
        assert!(
            results
                .iter()
                .all(|e| e.event_type_str().starts_with("index."))
        );
    }

    #[test]
    fn test_query_by_event_type_prefix_empty_returns_all() {
        let store = EventStore::new();

        store
            .ingest(create_test_event("entity-1", "index.created"))
            .unwrap();
        store
            .ingest(create_test_event("entity-2", "trade.created"))
            .unwrap();

        // Empty prefix matches all types
        let results = store
            .query(QueryEventsRequest {
                entity_id: None,
                event_type: None,
                tenant_id: None,
                as_of: None,
                since: None,
                until: None,
                limit: None,
                event_type_prefix: Some("".to_string()),
                payload_filter: None,
            })
            .unwrap();

        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_query_by_event_type_prefix_no_match() {
        let store = EventStore::new();

        store
            .ingest(create_test_event("entity-1", "index.created"))
            .unwrap();

        let results = store
            .query(QueryEventsRequest {
                entity_id: None,
                event_type: None,
                tenant_id: None,
                as_of: None,
                since: None,
                until: None,
                limit: None,
                event_type_prefix: Some("nonexistent.".to_string()),
                payload_filter: None,
            })
            .unwrap();

        assert!(results.is_empty());
    }

    #[test]
    fn test_query_by_entity_with_type_prefix() {
        let store = EventStore::new();

        store
            .ingest(create_test_event("entity-1", "index.created"))
            .unwrap();
        store
            .ingest(create_test_event("entity-1", "trade.created"))
            .unwrap();
        store
            .ingest(create_test_event("entity-2", "index.updated"))
            .unwrap();

        // Query entity-1 with prefix "index." should return 1
        let results = store
            .query(QueryEventsRequest {
                entity_id: Some("entity-1".to_string()),
                event_type: None,
                tenant_id: None,
                as_of: None,
                since: None,
                until: None,
                limit: None,
                event_type_prefix: Some("index.".to_string()),
                payload_filter: None,
            })
            .unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].event_type_str(), "index.created");
    }

    #[test]
    fn test_query_prefix_with_limit() {
        let store = EventStore::new();

        for i in 0..5 {
            store
                .ingest(create_test_event(&format!("entity-{}", i), "index.created"))
                .unwrap();
        }

        let results = store
            .query(QueryEventsRequest {
                entity_id: None,
                event_type: None,
                tenant_id: None,
                as_of: None,
                since: None,
                until: None,
                limit: Some(3),
                event_type_prefix: Some("index.".to_string()),
                payload_filter: None,
            })
            .unwrap();

        assert_eq!(results.len(), 3);
    }

    #[test]
    fn test_query_prefix_alongside_existing_filters() {
        let store = EventStore::new();

        store
            .ingest(create_test_event("entity-1", "index.created"))
            .unwrap();
        // Sleep briefly to ensure different timestamps
        std::thread::sleep(std::time::Duration::from_millis(10));
        store
            .ingest(create_test_event("entity-2", "index.strategy.updated"))
            .unwrap();
        std::thread::sleep(std::time::Duration::from_millis(10));
        store
            .ingest(create_test_event("entity-3", "index.deleted"))
            .unwrap();

        // Prefix with limit
        let results = store
            .query(QueryEventsRequest {
                entity_id: None,
                event_type: None,
                tenant_id: None,
                as_of: None,
                since: None,
                until: None,
                limit: Some(2),
                event_type_prefix: Some("index.".to_string()),
                payload_filter: None,
            })
            .unwrap();

        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_query_with_payload_filter() {
        let store = EventStore::new();

        // Ingest 5 events with user_id=alice
        for i in 0..5 {
            store
                .ingest(create_test_event_with_payload(
                    &format!("entity-{}", i),
                    "user.action",
                    serde_json::json!({"user_id": "alice", "action": "click"}),
                ))
                .unwrap();
        }
        // Ingest 5 events with user_id=bob
        for i in 5..10 {
            store
                .ingest(create_test_event_with_payload(
                    &format!("entity-{}", i),
                    "user.action",
                    serde_json::json!({"user_id": "bob", "action": "view"}),
                ))
                .unwrap();
        }

        // Filter for alice
        let results = store
            .query(QueryEventsRequest {
                entity_id: None,
                event_type: Some("user.action".to_string()),
                tenant_id: None,
                as_of: None,
                since: None,
                until: None,
                limit: None,
                event_type_prefix: None,
                payload_filter: Some(r#"{"user_id":"alice"}"#.to_string()),
            })
            .unwrap();

        assert_eq!(results.len(), 5);
    }

    #[test]
    fn test_query_payload_filter_non_existent_field() {
        let store = EventStore::new();

        store
            .ingest(create_test_event_with_payload(
                "entity-1",
                "user.action",
                serde_json::json!({"user_id": "alice"}),
            ))
            .unwrap();

        // Filter for a field that doesn't exist — returns 0, not error
        let results = store
            .query(QueryEventsRequest {
                entity_id: None,
                event_type: None,
                tenant_id: None,
                as_of: None,
                since: None,
                until: None,
                limit: None,
                event_type_prefix: None,
                payload_filter: Some(r#"{"nonexistent":"value"}"#.to_string()),
            })
            .unwrap();

        assert!(results.is_empty());
    }

    #[test]
    fn test_query_payload_filter_with_prefix() {
        let store = EventStore::new();

        store
            .ingest(create_test_event_with_payload(
                "entity-1",
                "index.created",
                serde_json::json!({"status": "active"}),
            ))
            .unwrap();
        store
            .ingest(create_test_event_with_payload(
                "entity-2",
                "index.created",
                serde_json::json!({"status": "inactive"}),
            ))
            .unwrap();
        store
            .ingest(create_test_event_with_payload(
                "entity-3",
                "trade.created",
                serde_json::json!({"status": "active"}),
            ))
            .unwrap();

        // Combine prefix + payload filter
        let results = store
            .query(QueryEventsRequest {
                entity_id: None,
                event_type: None,
                tenant_id: None,
                as_of: None,
                since: None,
                until: None,
                limit: None,
                event_type_prefix: Some("index.".to_string()),
                payload_filter: Some(r#"{"status":"active"}"#.to_string()),
            })
            .unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].entity_id().to_string(), "entity-1");
    }

    #[test]
    fn test_flush_storage_no_storage() {
        let store = EventStore::new();
        // Without storage, flush should succeed (no-op)
        let result = store.flush_storage();
        assert!(result.is_ok());
    }

    #[test]
    fn test_state_evolution() {
        let store = EventStore::new();

        // Initial state
        store
            .ingest(
                Event::from_strings(
                    "user.created".to_string(),
                    "user-1".to_string(),
                    "default".to_string(),
                    serde_json::json!({"name": "Alice", "age": 25}),
                    None,
                )
                .unwrap(),
            )
            .unwrap();

        // Update state
        store
            .ingest(
                Event::from_strings(
                    "user.updated".to_string(),
                    "user-1".to_string(),
                    "default".to_string(),
                    serde_json::json!({"age": 26}),
                    None,
                )
                .unwrap(),
            )
            .unwrap();

        let state = store.reconstruct_state("user-1", None).unwrap();
        // The state is wrapped with metadata
        assert_eq!(state["current_state"]["name"], "Alice");
        assert_eq!(state["current_state"]["age"], 26);
    }

    #[test]
    fn test_reject_system_event_types() {
        let store = EventStore::new();

        // System event types should be rejected via user-facing ingestion
        let event = Event::reconstruct_from_strings(
            uuid::Uuid::new_v4(),
            "_system.tenant.created".to_string(),
            "_system:tenant:acme".to_string(),
            "_system".to_string(),
            serde_json::json!({"name": "ACME"}),
            chrono::Utc::now(),
            None,
            1,
        );

        let result = store.ingest(event);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            err.to_string().contains("reserved for internal use"),
            "Expected system namespace rejection, got: {}",
            err
        );
    }

    // -----------------------------------------------------------------------
    // Crash recovery: WAL events survive restart via Parquet checkpoint.
    // Regression test for GitHub issue #84 — flush_storage() was a no-op
    // during recovery because events were never buffered into Parquet's
    // current_batch before flushing.
    // -----------------------------------------------------------------------

    #[test]
    fn test_wal_recovery_checkpoints_to_parquet() {
        let data_dir = TempDir::new().unwrap();
        let storage_dir = data_dir.path().join("storage");
        let wal_dir = data_dir.path().join("wal");

        // Session 1: ingest events with WAL + Parquet
        {
            let config = EventStoreConfig::production(
                &storage_dir,
                &wal_dir,
                SnapshotConfig::default(),
                WALConfig {
                    sync_on_write: true,
                    ..WALConfig::default()
                },
                CompactionConfig::default(),
            );
            let store = EventStore::with_config(config);

            for i in 0..5 {
                let event = Event::from_strings(
                    "test.created".to_string(),
                    format!("entity-{}", i),
                    "default".to_string(),
                    serde_json::json!({"index": i}),
                    None,
                )
                .unwrap();
                store.ingest(event).unwrap();
            }

            assert_eq!(store.stats().total_events, 5);

            // Do NOT call flush_storage or shutdown — simulate a crash.
            // Events are in WAL (sync_on_write: true) but NOT in Parquet.
        }

        // Verify WAL file has data
        let wal_files: Vec<_> = std::fs::read_dir(&wal_dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().is_some_and(|ext| ext == "log"))
            .collect();
        assert!(!wal_files.is_empty(), "WAL file should exist");
        let wal_size = wal_files[0].metadata().unwrap().len();
        assert!(wal_size > 0, "WAL file should have data (got 0 bytes)");

        // Session 2: reopen — recovery should checkpoint WAL to Parquet, then truncate
        {
            let config = EventStoreConfig::production(
                &storage_dir,
                &wal_dir,
                SnapshotConfig::default(),
                WALConfig {
                    sync_on_write: true,
                    ..WALConfig::default()
                },
                CompactionConfig::default(),
            );
            let store = EventStore::with_config(config);

            // Events should be recovered
            assert_eq!(
                store.stats().total_events,
                5,
                "Session 2 should have all 5 events after WAL recovery"
            );

            // Parquet should now have files (checkpoint happened)
            let parquet_files: Vec<_> = std::fs::read_dir(&storage_dir)
                .unwrap()
                .filter_map(|e| e.ok())
                .filter(|e| e.path().extension().is_some_and(|ext| ext == "parquet"))
                .collect();
            assert!(
                !parquet_files.is_empty(),
                "Parquet file should exist after WAL checkpoint"
            );
        }

        // Session 3: reopen again — events should load from Parquet (WAL was truncated)
        {
            let config = EventStoreConfig::production(
                &storage_dir,
                &wal_dir,
                SnapshotConfig::default(),
                WALConfig {
                    sync_on_write: true,
                    ..WALConfig::default()
                },
                CompactionConfig::default(),
            );
            let store = EventStore::with_config(config);

            assert_eq!(
                store.stats().total_events,
                5,
                "Session 3 should still have all 5 events from Parquet"
            );
        }
    }
}
