use crate::error::Result;
use crate::domain::entities::Event;
use crate::metrics::MetricsRegistry;
use dashmap::DashMap;
use serde_json::Value;
use std::sync::Arc;

/// A projection aggregates events into a queryable view
pub trait Projection: Send + Sync {
    /// Get the name of this projection
    fn name(&self) -> &str;

    /// Process an event and update the projection state
    fn process(&self, event: &Event) -> Result<()>;

    /// Get the current state of the projection for an entity
    fn get_state(&self, entity_id: &str) -> Option<Value>;

    /// Clear all projection state
    fn clear(&self);
}

/// Entity snapshot projection - maintains current state of each entity
pub struct EntitySnapshotProjection {
    name: String,
    /// entity_id -> latest state
    states: Arc<DashMap<String, Value>>,
}

impl EntitySnapshotProjection {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            states: Arc::new(DashMap::new()),
        }
    }

    /// Get all entity states
    pub fn get_all_states(&self) -> Vec<(String, Value)> {
        self.states
            .iter()
            .map(|entry| (entry.key().clone(), entry.value().clone()))
            .collect()
    }
}

impl Projection for EntitySnapshotProjection {
    fn name(&self) -> &str {
        &self.name
    }

    fn process(&self, event: &Event) -> Result<()> {
        // Simple merge strategy: update or insert
        self.states
            .entry(event.entity_id_str().to_string())
            .and_modify(|state| {
                // Merge the event payload into existing state
                if let Value::Object(ref mut map) = state {
                    if let Value::Object(ref payload_map) = event.payload {
                        for (key, value) in payload_map {
                            map.insert(key.clone(), value.clone());
                        }
                    }
                }
            })
            .or_insert_with(|| event.payload.clone());

        Ok(())
    }

    fn get_state(&self, entity_id: &str) -> Option<Value> {
        self.states.get(entity_id).map(|v| v.clone())
    }

    fn clear(&self) {
        self.states.clear();
    }
}

/// Event counter projection - counts events by type
pub struct EventCounterProjection {
    name: String,
    /// event_type -> count
    counts: Arc<DashMap<String, u64>>,
}

impl EventCounterProjection {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            counts: Arc::new(DashMap::new()),
        }
    }

    /// Get count for a specific event type
    pub fn get_count(&self, event_type: &str) -> u64 {
        self.counts
            .get(event_type)
            .map(|v| *v)
            .unwrap_or(0)
    }

    /// Get all event type counts
    pub fn get_all_counts(&self) -> Vec<(String, u64)> {
        self.counts
            .iter()
            .map(|entry| (entry.key().clone(), *entry.value()))
            .collect()
    }
}

impl Projection for EventCounterProjection {
    fn name(&self) -> &str {
        &self.name
    }

    fn process(&self, event: &Event) -> Result<()> {
        self.counts
            .entry(event.event_type_str().to_string())
            .and_modify(|count| *count += 1)
            .or_insert(1);

        Ok(())
    }

    fn get_state(&self, event_type: &str) -> Option<Value> {
        self.counts
            .get(event_type)
            .map(|count| serde_json::json!({ "count": *count }))
    }

    fn clear(&self) {
        self.counts.clear();
    }
}

/// Projection manager handles multiple projections
pub struct ProjectionManager {
    projections: Vec<Arc<dyn Projection>>,
    metrics: Arc<MetricsRegistry>,
}

impl ProjectionManager {
    pub fn new() -> Self {
        Self::with_metrics(MetricsRegistry::new())
    }

    pub fn with_metrics(metrics: Arc<MetricsRegistry>) -> Self {
        Self {
            projections: Vec::new(),
            metrics,
        }
    }

    /// Register a new projection
    pub fn register(&mut self, projection: Arc<dyn Projection>) {
        let name = projection.name();
        tracing::info!("Registering projection: {}", name);
        self.projections.push(projection);
        self.metrics.projections_total.set(self.projections.len() as i64);
    }

    /// Process an event through all projections
    pub fn process_event(&self, event: &Event) -> Result<()> {
        let timer = self.metrics.projection_duration_seconds.start_timer();

        for projection in &self.projections {
            let name = projection.name();

            match projection.process(event) {
                Ok(_) => {
                    self.metrics.projection_events_processed
                        .with_label_values(&[name])
                        .inc();
                }
                Err(e) => {
                    self.metrics.projection_errors_total
                        .with_label_values(&[name])
                        .inc();
                    tracing::error!(
                        "Projection '{}' failed to process event {}: {}",
                        name,
                        event.id,
                        e
                    );
                    // Continue processing other projections even if one fails
                }
            }
        }

        timer.observe_duration();
        Ok(())
    }

    /// Get a projection by name
    pub fn get_projection(&self, name: &str) -> Option<Arc<dyn Projection>> {
        self.projections.iter().find(|p| p.name() == name).cloned()
    }

    /// List all projections
    pub fn list_projections(&self) -> Vec<(String, Arc<dyn Projection>)> {
        self.projections
            .iter()
            .map(|p| (p.name().to_string(), Arc::clone(p)))
            .collect()
    }

    /// Clear all projections
    pub fn clear_all(&self) {
        for projection in &self.projections {
            projection.clear();
        }
    }
}

impl Default for ProjectionManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    fn create_test_event(entity_id: &str, event_type: &str) -> Event {
        Event::reconstruct_from_strings(
            Uuid::new_v4(),
            event_type.to_string(),
            entity_id.to_string(),
            "default".to_string(),
            serde_json::json!({
                "name": "Test User",
                "email": "test@example.com"
            }),
            chrono::Utc::now(),
            None,
            1,
        )
    }

    #[test]
    fn test_entity_snapshot_projection() {
        let projection = EntitySnapshotProjection::new("test");
        let event = create_test_event("user-123", "user.created");

        projection.process(&event).unwrap();

        let state = projection.get_state("user-123").unwrap();
        assert_eq!(state["name"], "Test User");
    }

    #[test]
    fn test_event_counter_projection() {
        let projection = EventCounterProjection::new("counter");

        let event1 = create_test_event("user-123", "user.created");
        let event2 = create_test_event("user-456", "user.created");
        let event3 = create_test_event("user-123", "user.updated");

        projection.process(&event1).unwrap();
        projection.process(&event2).unwrap();
        projection.process(&event3).unwrap();

        assert_eq!(projection.get_count("user.created"), 2);
        assert_eq!(projection.get_count("user.updated"), 1);
    }

    #[test]
    fn test_projection_manager() {
        let mut manager = ProjectionManager::new();

        let snapshot = Arc::new(EntitySnapshotProjection::new("snapshot"));
        let counter = Arc::new(EventCounterProjection::new("counter"));

        manager.register(snapshot.clone());
        manager.register(counter.clone());

        let event = create_test_event("user-123", "user.created");
        manager.process_event(&event).unwrap();

        assert!(snapshot.get_state("user-123").is_some());
        assert_eq!(counter.get_count("user.created"), 1);
    }
}
