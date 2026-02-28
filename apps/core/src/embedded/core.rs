use std::sync::Arc;

use crate::{
    application::dto::QueryEventsRequest,
    domain::entities::Event,
    error::Result,
    infrastructure::{
        cluster::{
            crdt::{ConflictResolution, CrdtResolver, ReplicatedEvent},
            hlc::HybridLogicalClock,
        },
        persistence::{
            compaction::CompactionConfig, snapshot::SnapshotConfig, wal::WALConfig,
        },
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
                let resolver = Arc::new(CrdtResolver::new());
                (Some(hlc), Some(resolver))
            }
            None => (None, None),
        };

        Ok(Self {
            store,
            config,
            hlc,
            resolver,
        })
    }

    /// Ingest an event. Accepts plain strings — no value object construction needed.
    ///
    /// In single-tenant mode, "default" is used as the tenant_id regardless of
    /// the `tenant_id` field on `IngestEvent`. In multi-tenant mode, the
    /// `tenant_id` field is used if provided, otherwise "default".
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
                event_data: serde_json::to_value(&EventView::from(&domain_event))
                    .unwrap_or_default(),
            };
            resolver.accept(&replicated);
        }

        self.store.ingest(domain_event)?;
        Ok(())
    }

    /// Ingest a batch of events atomically.
    ///
    /// All events are ingested in order. If any event fails validation,
    /// prior events in the batch are still stored (no rollback).
    pub async fn ingest_batch(&self, events: Vec<IngestEvent<'_>>) -> Result<()> {
        for event in events {
            self.ingest(event).await?;
        }
        Ok(())
    }

    /// Compact token events for an entity into a single `workflow.output.complete` event.
    ///
    /// Finds all `workflow.token` events for the given `entity_id`, concatenates
    /// their `token` fields in index order, replaces them with a single
    /// `workflow.output.complete` event, and preserves all non-token events.
    ///
    /// Returns `Ok(())` regardless of whether compaction was needed.
    pub async fn compact_tokens(&self, entity_id: &str) -> Result<()> {
        // Query all token events for this entity
        let all_events = self.store.query(QueryEventsRequest {
            entity_id: Some(entity_id.to_string()),
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
            .collect::<Vec<_>>()
            .join(" ");

        let tenant_id = self.effective_tenant_id(None);
        let merged_event = Event::from_strings(
            "workflow.output.complete".to_string(),
            entity_id.to_string(),
            tenant_id,
            serde_json::json!({ "text": merged_text, "token_count": tokens.len() }),
            None,
        )?;

        self.store
            .compact_entity_tokens(entity_id, "workflow.token", merged_event)?;

        Ok(())
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

        // Get all events from this store
        let all_events = self.store.query(QueryEventsRequest {
            entity_id: None,
            event_type: None,
            tenant_id: None,
            as_of: None,
            since: None,
            until: None,
            limit: None,
            event_type_prefix: None,
            payload_filter: None,
        })?;

        let self_region = format!("node-{}", self_hlc.node_id());

        for event in &all_events {
            // Use a per-event HLC timestamp for causal ordering
            let ts = self_hlc.now();

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

                // Clone the original event to preserve its UUID for dedup
                let cloned = event.clone();
                peer.store.ingest(cloned)?;
            }
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

        let events = self.store.query(request)?;
        Ok(events.iter().map(EventView::from).collect())
    }

    /// Get the current state of a named projection for a given entity.
    ///
    /// Returns `None` if the projection doesn't exist or has no state for the entity.
    pub fn projection(
        &self,
        projection_name: &str,
        entity_id: &str,
    ) -> Option<serde_json::Value> {
        let pm = self.store.projection_manager();
        let projection = pm.get_projection(projection_name)?;
        projection.get_state(entity_id)
    }

    /// Get basic statistics about this store instance.
    pub fn stats(&self) -> StoreStats {
        self.store.stats()
    }

    /// Get a reference to the underlying `EventStore`.
    ///
    /// Escape hatch for advanced use cases that need direct access.
    pub fn inner(&self) -> Arc<EventStore> {
        Arc::clone(&self.store)
    }

    /// Flush WAL and Parquet storage, then shut down cleanly.
    pub async fn shutdown(&self) -> Result<()> {
        self.store.flush_storage()
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
                let wal_config = WALConfig {
                    sync_on_write: config.wal_sync_on_write(),
                    ..WALConfig::default()
                };
                EventStoreConfig::production(
                    storage_dir,
                    wal_dir,
                    SnapshotConfig::default(),
                    wal_config,
                    CompactionConfig::default(),
                )
            }
            None => EventStoreConfig::default(),
        }
    }
}
