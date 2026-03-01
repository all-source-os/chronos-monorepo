use std::sync::Arc;

use crate::{
    application::dto::QueryEventsRequest,
    domain::entities::Event,
    error::Result,
    infrastructure::{
        cluster::{
            crdt::{ConflictResolution, CrdtResolver, ReplicatedEvent},
            hlc::{HlcTimestamp, HybridLogicalClock},
        },
        persistence::{compaction::CompactionConfig, snapshot::SnapshotConfig, wal::WALConfig},
    },
    store::{EventStore, EventStoreConfig, StoreStats},
};

use super::{
    config::EmbeddedConfig,
    types::{EventView, IngestEvent, Query},
};

/// High-level facade over [`EventStore`] for embedded (library) use.
///
/// Provides a simplified API that accepts plain strings and returns plain
/// types — no value object construction required.
///
/// Multiple instances can coexist in the same process with full isolation.
///
/// When configured with a `node_id`, each instance gets an HLC clock and
/// CRDT resolver for bidirectional sync via [`sync_to`](Self::sync_to).
///
/// All blocking store operations are offloaded to `spawn_blocking` so they
/// don't block the Tokio runtime's async worker threads.
///
/// # Example
/// ```rust,no_run
/// use allsource_core::embedded::{Config, EmbeddedCore, IngestEvent, Query};
/// use serde_json::json;
///
/// # #[tokio::main]
/// # async fn main() -> allsource_core::error::Result<()> {
/// let core = EmbeddedCore::open(Config::builder().build()?).await?;
///
/// core.ingest(IngestEvent {
///     entity_id: "order-1",
///     event_type: "order.placed",
///     payload: json!({"total": 99.99}),
///     metadata: None,
///     tenant_id: None,
/// }).await?;
///
/// let events = core.query(Query::new().entity_id("order-1")).await?;
/// core.shutdown().await?;
/// # Ok(())
/// # }
/// ```
pub struct EmbeddedCore {
    store: Arc<EventStore>,
    config: EmbeddedConfig,
    /// HLC clock for this node (initialized when node_id is set).
    hlc: Option<Arc<HybridLogicalClock>>,
    /// CRDT resolver for deduplication during sync.
    resolver: Option<Arc<CrdtResolver>>,
    /// Background interval-based fsync task handle.
    sync_handle: Option<tokio::task::JoinHandle<()>>,
}

impl EmbeddedCore {
    /// Open an `EmbeddedCore` instance.
    ///
    /// When `config.data_dir` is set, WAL and Parquet persistence are enabled.
    /// Otherwise runs in-memory only.
    ///
    /// When `config.node_id` is set, HLC and CRDT resolver are initialized
    /// for bidirectional sync support.
    pub async fn open(config: EmbeddedConfig) -> Result<Self> {
        let store_config = Self::build_store_config(&config);
        let store = Arc::new(EventStore::with_config(store_config));

        // Register replicant worker projections when the feature is enabled
        #[cfg(feature = "embedded-replicant")]
        {
            use super::replicant::{
                ReplicantRegistryProjection, TaskQueueProjection, WorkflowStatusProjection,
            };
            store.register_projection(Arc::new(WorkflowStatusProjection::new()));
            store.register_projection(Arc::new(ReplicantRegistryProjection::new()));
            store.register_projection(Arc::new(TaskQueueProjection::new()));
        }

        // Register AI projection templates when the feature is enabled
        #[cfg(feature = "embedded-projections")]
        {
            use super::ai_projections::{
                AgentUtilizationProjection, HumanInLoopQueueProjection, TokenUsageProjection,
                ToolCallAuditProjection,
            };
            store.register_projection(Arc::new(TokenUsageProjection::new()));
            store.register_projection(Arc::new(ToolCallAuditProjection::new()));
            store.register_projection(Arc::new(HumanInLoopQueueProjection::new()));
            store.register_projection(Arc::new(AgentUtilizationProjection::new()));
        }

        let (hlc, resolver) = match config.node_id() {
            Some(node_id) => {
                let hlc = Arc::new(HybridLogicalClock::new(node_id));
                let resolver = if config.merge_strategies().is_empty() {
                    Arc::new(CrdtResolver::new())
                } else {
                    Arc::new(CrdtResolver::with_strategies(
                        config.merge_strategies().to_vec(),
                    ))
                };
                (Some(hlc), Some(resolver))
            }
            None => (None, None),
        };

        // Spawn background fsync task if interval is configured and WAL exists
        let sync_handle = if let Some(interval_ms) = config.wal_fsync_interval_ms() {
            if let Some(wal) = store.wal() {
                let wal = Arc::clone(wal);
                let interval = std::time::Duration::from_millis(interval_ms);
                tracing::info!(
                    "🔄 Starting background WAL fsync task (interval: {}ms)",
                    interval_ms
                );
                Some(tokio::spawn(async move {
                    let mut ticker = tokio::time::interval(interval);
                    ticker.tick().await; // first tick completes immediately
                    loop {
                        ticker.tick().await;
                        if let Err(e) = wal.sync() {
                            tracing::warn!("Background WAL fsync failed: {e}");
                        }
                    }
                }))
            } else {
                None
            }
        } else {
            None
        };

        Ok(Self {
            store,
            config,
            hlc,
            resolver,
            sync_handle,
        })
    }

    /// Ingest an event. Accepts plain strings — no value object construction needed.
    ///
    /// In single-tenant mode, "default" is used as the tenant_id regardless of
    /// the `tenant_id` field on `IngestEvent`. In multi-tenant mode, the
    /// `tenant_id` field is used if provided, otherwise "default".
    ///
    /// The blocking store write is offloaded to `spawn_blocking`.
    pub async fn ingest(&self, event: IngestEvent<'_>) -> Result<()> {
        let tenant_id = self.effective_tenant_id(event.tenant_id);
        let domain_event = Event::from_strings(
            event.event_type.to_string(),
            event.entity_id.to_string(),
            tenant_id,
            event.payload,
            event.metadata,
        )?;

        // If sync is enabled, stamp the event and mark it as seen
        if let (Some(hlc), Some(resolver)) = (&self.hlc, &self.resolver) {
            let ts = hlc.now();
            let replicated = ReplicatedEvent {
                event_id: domain_event.id().to_string(),
                hlc_timestamp: ts,
                origin_region: format!("node-{}", hlc.node_id()),
                event_data: serde_json::to_value(EventView::from(&domain_event))
                    .unwrap_or_default(),
            };
            resolver.accept(&replicated);
        }

        let store = Arc::clone(&self.store);
        tokio::task::spawn_blocking(move || store.ingest(domain_event))
            .await
            .map_err(|e| {
                crate::error::AllSourceError::InvalidInput(format!("spawn_blocking failed: {e}"))
            })??;
        Ok(())
    }

    /// Ingest a batch of events with a single write lock acquisition.
    ///
    /// All events are validated (converted to domain events) before any are
    /// ingested. If validation fails for any event, no events are stored.
    /// The store-level batch uses a single write lock for all events,
    /// minimizing lock contention for high-throughput streaming.
    pub async fn ingest_batch(&self, events: Vec<IngestEvent<'_>>) -> Result<()> {
        // Phase 1: Build and validate all domain events before acquiring any locks
        let mut domain_events = Vec::with_capacity(events.len());
        for event in events {
            let tenant_id = self.effective_tenant_id(event.tenant_id);
            let domain_event = Event::from_strings(
                event.event_type.to_string(),
                event.entity_id.to_string(),
                tenant_id,
                event.payload,
                event.metadata,
            )?;
            domain_events.push(domain_event);
        }

        // Phase 2: Stamp all events for sync (if enabled)
        if let (Some(hlc), Some(resolver)) = (&self.hlc, &self.resolver) {
            for domain_event in &domain_events {
                let ts = hlc.now();
                let replicated = ReplicatedEvent {
                    event_id: domain_event.id().to_string(),
                    hlc_timestamp: ts,
                    origin_region: format!("node-{}", hlc.node_id()),
                    event_data: serde_json::to_value(EventView::from(domain_event))
                        .unwrap_or_default(),
                };
                resolver.accept(&replicated);
            }
        }

        // Phase 3: Ingest entire batch on the blocking threadpool (single lock)
        let store = Arc::clone(&self.store);
        tokio::task::spawn_blocking(move || store.ingest_batch(domain_events))
            .await
            .map_err(|e| {
                crate::error::AllSourceError::InvalidInput(format!("spawn_blocking failed: {e}"))
            })??;

        Ok(())
    }

    /// Compact token events for an entity into a single `workflow.output.complete` event.
    ///
    /// Finds all `workflow.token` events for the given `entity_id`, concatenates
    /// their `token` fields in index order, replaces them with a single
    /// `workflow.output.complete` event, and preserves all non-token events.
    ///
    /// The merged event inherits the tenant ID from the first token event,
    /// preserving multi-tenant correctness.
    ///
    /// Returns `Ok(())` regardless of whether compaction was needed.
    pub async fn compact_tokens(&self, entity_id: &str) -> Result<()> {
        let store = Arc::clone(&self.store);
        let entity_id = entity_id.to_string();

        tokio::task::spawn_blocking(move || {
            // Query all token events for this entity
            let all_events = store.query(QueryEventsRequest {
                entity_id: Some(entity_id.clone()),
                event_type: Some("workflow.token".to_string()),
                tenant_id: None,
                as_of: None,
                since: None,
                until: None,
                limit: None,
                event_type_prefix: None,
                payload_filter: None,
            })?;

            if all_events.is_empty() {
                return Ok(());
            }

            // Inherit tenant from the original events (not the config default).
            // Validate all events share the same tenant to prevent cross-tenant merging.
            let tenant_id = all_events[0].tenant_id_str().to_string();
            if all_events.iter().any(|e| e.tenant_id_str() != tenant_id) {
                return Err(crate::error::AllSourceError::InvalidInput(
                    format!(
                        "compact_tokens: entity '{}' has token events across multiple tenants — cannot merge",
                        entity_id
                    ),
                ));
            }

            // Sort by index and concatenate tokens
            let mut tokens: Vec<(u64, String)> = all_events
                .iter()
                .map(|e| {
                    let idx = e.payload.get("index").and_then(|v| v.as_u64()).unwrap_or(0);
                    let token = e
                        .payload
                        .get("token")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    (idx, token)
                })
                .collect();
            tokens.sort_by_key(|(idx, _)| *idx);

            let merged_text: String = tokens
                .iter()
                .map(|(_, t)| t.as_str())
                .collect();

            let merged_event = Event::from_strings(
                "workflow.output.complete".to_string(),
                entity_id.clone(),
                tenant_id,
                serde_json::json!({ "text": merged_text, "token_count": tokens.len() }),
                None,
            )?;

            store.compact_entity_tokens(&entity_id, "workflow.token", merged_event)?;

            Ok(())
        })
        .await
        .map_err(|e| crate::error::AllSourceError::InvalidInput(format!("spawn_blocking failed: {e}")))?
    }

    /// Sync events from this instance to another instance.
    ///
    /// Sends all events that the peer hasn't seen yet, using CRDT resolution
    /// to prevent duplicates. Both instances must have `node_id` configured.
    ///
    /// For full bidirectional sync, call `a.sync_to(&b)` then `b.sync_to(&a)`.
    pub async fn sync_to(&self, peer: &EmbeddedCore) -> Result<()> {
        let (Some(self_hlc), Some(_self_resolver)) = (&self.hlc, &self.resolver) else {
            return Err(crate::error::AllSourceError::InvalidInput(
                "sync requires node_id to be configured".to_string(),
            ));
        };

        let (Some(peer_hlc), Some(peer_resolver)) = (&peer.hlc, &peer.resolver) else {
            return Err(crate::error::AllSourceError::InvalidInput(
                "peer sync requires node_id to be configured".to_string(),
            ));
        };

        let self_region = format!("node-{}", self_hlc.node_id());

        // Use version vector for delta exchange: only query events newer than
        // what the peer has already seen from our region.
        let since = peer_resolver
            .version_vector_for(&self_region)
            .and_then(|vv| vv.get(&self_region).copied())
            .map(|ts| {
                chrono::DateTime::from_timestamp_millis(ts.physical_ms as i64).unwrap_or_default()
            });

        // Get events from this store, filtered by version vector threshold
        let self_store = Arc::clone(&self.store);
        let all_events = tokio::task::spawn_blocking(move || {
            self_store.query(QueryEventsRequest {
                entity_id: None,
                event_type: None,
                tenant_id: None,
                as_of: None,
                since,
                until: None,
                limit: None,
                event_type_prefix: None,
                payload_filter: None,
            })
        })
        .await
        .map_err(|e| {
            crate::error::AllSourceError::InvalidInput(format!("spawn_blocking failed: {e}"))
        })??;
        let self_node_id = self_hlc.node_id();

        // Collect events to sync (CRDT resolution is cheap, do it on the async thread)
        let mut events_to_sync = Vec::new();
        let mut last_ms = 0u64;
        let mut logical = 0u32;
        for event in &all_events {
            // Use the event's original creation timestamp for causal ordering.
            // Minting fresh timestamps via hlc.now() would make sync order
            // determine LWW winners instead of creation order.
            // Logical counter distinguishes events within the same millisecond.
            let event_ms = event.timestamp().timestamp_millis() as u64;
            if event_ms == last_ms {
                logical += 1;
            } else {
                last_ms = event_ms;
                logical = 0;
            }
            let ts = HlcTimestamp::new(event_ms, logical, self_node_id);

            let replicated = ReplicatedEvent {
                event_id: event.id().to_string(),
                hlc_timestamp: ts,
                origin_region: self_region.clone(),
                event_data: serde_json::json!({}),
            };

            // CRDT resolver on the peer determines if this event is new
            let resolution = peer_resolver.resolve(&replicated);
            if resolution == ConflictResolution::Accept {
                peer_resolver.accept(&replicated);

                // Update peer's HLC with our timestamp for causal ordering
                let _ = peer_hlc.receive(&ts);

                events_to_sync.push(event.clone());
            }
        }

        // Ingest accepted events into peer store (blocking)
        if !events_to_sync.is_empty() {
            let peer_store = Arc::clone(&peer.store);
            tokio::task::spawn_blocking(move || {
                for event in events_to_sync {
                    peer_store.ingest(event)?;
                }
                Ok::<(), crate::error::AllSourceError>(())
            })
            .await
            .map_err(|e| {
                crate::error::AllSourceError::InvalidInput(format!("spawn_blocking failed: {e}"))
            })??;
        }

        Ok(())
    }

    /// Query events. Returns `Vec<EventView>` with plain string fields.
    pub async fn query(&self, query: Query) -> Result<Vec<EventView>> {
        let request = QueryEventsRequest {
            entity_id: query.entity_id,
            event_type: query.event_type,
            tenant_id: query.tenant_id,
            as_of: None,
            since: query.since,
            until: query.until,
            limit: query.limit,
            event_type_prefix: query.event_type_prefix,
            payload_filter: None,
        };

        let store = Arc::clone(&self.store);
        let events = tokio::task::spawn_blocking(move || store.query(request))
            .await
            .map_err(|e| {
                crate::error::AllSourceError::InvalidInput(format!("spawn_blocking failed: {e}"))
            })??;
        Ok(events.iter().map(EventView::from).collect())
    }

    /// Query events and return the result in TOON (Token-Oriented Object Notation).
    ///
    /// TOON is a compact text format that uses ~50% fewer tokens than JSON for
    /// tabular data — ideal for LLM consumption. Falls back gracefully for
    /// non-tabular structures.
    #[cfg(feature = "embedded-toon")]
    pub async fn query_toon(&self, query: Query) -> Result<String> {
        let events = self.query(query).await?;
        let json_value = serde_json::to_value(&events).map_err(|e| {
            crate::error::AllSourceError::InvalidInput(format!("JSON serialization failed: {e}"))
        })?;
        let opts = toon_format::EncodeOptions::default();
        toon_format::encode(&json_value, &opts).map_err(|e| {
            crate::error::AllSourceError::InvalidInput(format!("TOON encoding failed: {e}"))
        })
    }

    /// Get the current state of a named projection for a given entity.
    ///
    /// Returns `None` if the projection doesn't exist or has no state for the entity.
    pub fn projection(&self, projection_name: &str, entity_id: &str) -> Option<serde_json::Value> {
        let pm = self.store.projection_manager();
        let projection = pm.get_projection(projection_name)?;
        projection.get_state(entity_id)
    }

    /// Get basic statistics about this store instance.
    pub fn stats(&self) -> StoreStats {
        self.store.stats()
    }

    /// Get the merged version vector across all known regions.
    ///
    /// Returns a map of `region_id → latest HLC timestamp`. Used by the
    /// sync transport to compute deltas.
    pub fn version_vector(
        &self,
    ) -> std::collections::BTreeMap<String, crate::infrastructure::cluster::hlc::HlcTimestamp> {
        match &self.resolver {
            Some(resolver) => {
                let all_vv = resolver.all_version_vectors();
                let mut merged = crate::infrastructure::cluster::crdt::VersionVector::new();
                for (_region, vv) in &all_vv {
                    merged.merge(vv);
                }
                merged.entries().clone()
            }
            None => std::collections::BTreeMap::new(),
        }
    }

    /// Get the node's region identifier (e.g., "node-1").
    ///
    /// Returns `None` if sync is not configured (no `node_id`).
    pub fn region_id(&self) -> Option<String> {
        self.hlc.as_ref().map(|hlc| format!("node-{}", hlc.node_id()))
    }

    /// Receive events from a remote sync push.
    ///
    /// Applies CRDT conflict resolution to each event and ingests
    /// accepted events. Returns `(accepted, skipped)` counts.
    pub async fn receive_sync_push(
        &self,
        events: Vec<ReplicatedEvent>,
    ) -> Result<(usize, usize)> {
        let (Some(_hlc), Some(resolver)) = (&self.hlc, &self.resolver) else {
            return Err(crate::error::AllSourceError::InvalidInput(
                "sync requires node_id to be configured".to_string(),
            ));
        };

        let mut accepted = Vec::new();
        let mut skipped = 0usize;

        for event in &events {
            let resolution = resolver.resolve(event);
            if resolution == ConflictResolution::Accept {
                resolver.accept(event);
                accepted.push(event.clone());
            } else {
                skipped += 1;
            }
        }

        let accepted_count = accepted.len();

        if !accepted.is_empty() {
            let store = Arc::clone(&self.store);
            tokio::task::spawn_blocking(move || {
                for rep_event in accepted {
                    // Convert ReplicatedEvent back to domain Event
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

                    let domain_event = Event::from_strings(
                        event_type,
                        entity_id,
                        tenant_id,
                        payload,
                        metadata,
                    )?;
                    store.ingest(domain_event)?;
                }
                Ok::<(), crate::error::AllSourceError>(())
            })
            .await
            .map_err(|e| {
                crate::error::AllSourceError::InvalidInput(format!("spawn_blocking failed: {e}"))
            })??;
        }

        Ok((accepted_count, skipped))
    }

    /// Get all events as `ReplicatedEvent`s for sync push.
    ///
    /// Optionally filtered by a version vector threshold (only events
    /// newer than the threshold are returned).
    pub async fn events_for_sync(
        &self,
        since_vv: &std::collections::BTreeMap<String, crate::infrastructure::cluster::hlc::HlcTimestamp>,
    ) -> Result<Vec<ReplicatedEvent>> {
        let Some(hlc) = &self.hlc else {
            return Err(crate::error::AllSourceError::InvalidInput(
                "sync requires node_id to be configured".to_string(),
            ));
        };

        let self_region = format!("node-{}", hlc.node_id());
        let since = since_vv
            .get(&self_region)
            .map(|ts| {
                chrono::DateTime::from_timestamp_millis(ts.physical_ms as i64).unwrap_or_default()
            });

        let store = Arc::clone(&self.store);
        let all_events = tokio::task::spawn_blocking(move || {
            store.query(QueryEventsRequest {
                entity_id: None,
                event_type: None,
                tenant_id: None,
                as_of: None,
                since,
                until: None,
                limit: None,
                event_type_prefix: None,
                payload_filter: None,
            })
        })
        .await
        .map_err(|e| {
            crate::error::AllSourceError::InvalidInput(format!("spawn_blocking failed: {e}"))
        })??;

        let node_id = hlc.node_id();
        let mut replicated = Vec::with_capacity(all_events.len());
        let mut last_ms = 0u64;
        let mut logical = 0u32;

        for event in &all_events {
            let event_ms = event.timestamp().timestamp_millis() as u64;
            if event_ms == last_ms {
                logical += 1;
            } else {
                last_ms = event_ms;
                logical = 0;
            }
            let ts = HlcTimestamp::new(event_ms, logical, node_id);

            replicated.push(ReplicatedEvent {
                event_id: event.id().to_string(),
                hlc_timestamp: ts,
                origin_region: self_region.clone(),
                event_data: serde_json::to_value(EventView::from(event)).unwrap_or_default(),
            });
        }

        Ok(replicated)
    }

    /// Get a reference to the underlying `EventStore`.
    ///
    /// Escape hatch for advanced use cases that need direct access.
    pub fn inner(&self) -> Arc<EventStore> {
        Arc::clone(&self.store)
    }

    /// Flush WAL and Parquet storage, then shut down cleanly.
    ///
    /// Aborts the background fsync task (if running) and performs a final
    /// sync before flushing storage.
    pub async fn shutdown(&self) -> Result<()> {
        // Stop background fsync task
        if let Some(handle) = &self.sync_handle {
            handle.abort();
        }

        // Final sync if WAL exists (ensure last writes are durable)
        if let Some(wal) = self.store.wal() {
            wal.sync()?;
        }

        let store = Arc::clone(&self.store);
        tokio::task::spawn_blocking(move || store.flush_storage())
            .await
            .map_err(|e| {
                crate::error::AllSourceError::InvalidInput(format!("spawn_blocking failed: {e}"))
            })?
    }

    fn effective_tenant_id(&self, explicit: Option<&str>) -> String {
        if self.config.single_tenant() {
            "default".to_string()
        } else {
            explicit.unwrap_or("default").to_string()
        }
    }

    fn build_store_config(config: &EmbeddedConfig) -> EventStoreConfig {
        match config.data_dir() {
            Some(dir) => {
                let storage_dir = dir.join("storage");
                let wal_dir = dir.join("wal");
                // When interval-based fsync is configured, force sync_on_write off
                // to prevent double-fsync (the background task handles durability).
                let sync_on_write = if config.wal_fsync_interval_ms().is_some() {
                    false
                } else {
                    config.wal_sync_on_write()
                };
                let wal_config = WALConfig {
                    sync_on_write,
                    fsync_interval_ms: config.wal_fsync_interval_ms(),
                    ..WALConfig::default()
                };
                let snapshot_config = SnapshotConfig {
                    time_threshold_seconds: config.parquet_flush_interval_secs() as i64,
                    ..SnapshotConfig::default()
                };
                EventStoreConfig::production(
                    storage_dir,
                    wal_dir,
                    snapshot_config,
                    wal_config,
                    CompactionConfig::default(),
                )
            }
            None => EventStoreConfig::default(),
        }
    }
}
