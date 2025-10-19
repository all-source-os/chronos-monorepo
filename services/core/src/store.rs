use crate::error::{AllSourceError, Result};
use crate::event::{Event, QueryEventsRequest};
use crate::index::{EventIndex, IndexEntry};
use crate::projection::{
    EntitySnapshotProjection, EventCounterProjection, Projection, ProjectionManager,
};
use chrono::{DateTime, Utc};
use parking_lot::RwLock;
use std::sync::Arc;

/// High-performance event store with columnar storage
pub struct EventStore {
    /// In-memory event storage (for demo - would be Arrow/Parquet in production)
    events: Arc<RwLock<Vec<Event>>>,

    /// High-performance concurrent index
    index: Arc<EventIndex>,

    /// Projection manager for real-time aggregations
    projections: Arc<RwLock<ProjectionManager>>,

    /// Total events ingested (for metrics)
    total_ingested: Arc<RwLock<u64>>,
}

impl EventStore {
    pub fn new() -> Self {
        let mut projections = ProjectionManager::new();

        // Register built-in projections
        projections.register(Arc::new(EntitySnapshotProjection::new("entity_snapshots")));
        projections.register(Arc::new(EventCounterProjection::new("event_counters")));

        Self {
            events: Arc::new(RwLock::new(Vec::new())),
            index: Arc::new(EventIndex::new()),
            projections: Arc::new(RwLock::new(projections)),
            total_ingested: Arc::new(RwLock::new(0)),
        }
    }

    /// Ingest a new event into the store
    pub fn ingest(&self, event: Event) -> Result<()> {
        // Validate event
        self.validate_event(&event)?;

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

        // Store the event
        events.push(event.clone());
        drop(events); // Release lock early

        // Update metrics
        let mut total = self.total_ingested.write();
        *total += 1;

        tracing::debug!(
            "Event ingested: {} (offset: {})",
            event.id,
            offset
        );

        Ok(())
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
    pub fn reconstruct_state(
        &self,
        entity_id: &str,
        as_of: Option<DateTime<Utc>>,
    ) -> Result<serde_json::Value> {
        let events = self.query(QueryEventsRequest {
            entity_id: Some(entity_id.to_string()),
            event_type: None,
            as_of,
            since: None,
            until: None,
            limit: None,
        })?;

        if events.is_empty() {
            return Err(AllSourceError::EntityNotFound(entity_id.to_string()));
        }

        // Build comprehensive state from event stream
        let mut merged_state = serde_json::json!({});

        // Merge all event payloads in order
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
