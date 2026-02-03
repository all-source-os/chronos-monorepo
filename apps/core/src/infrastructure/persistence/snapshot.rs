use crate::domain::entities::Event;
use crate::error::Result;
use chrono::{DateTime, Utc};
use dashmap::DashMap;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

/// A point-in-time snapshot of entity state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Snapshot {
    /// Unique snapshot identifier
    pub id: Uuid,

    /// Entity this snapshot represents
    pub entity_id: String,

    /// The state data at this point in time
    pub state: serde_json::Value,

    /// Timestamp when this snapshot was created
    pub created_at: DateTime<Utc>,

    /// Last event timestamp included in this snapshot
    pub as_of: DateTime<Utc>,

    /// Number of events processed to create this snapshot
    pub event_count: usize,

    /// Metadata about the snapshot
    pub metadata: SnapshotMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotMetadata {
    /// Type of snapshot (manual, automatic, etc.)
    pub snapshot_type: SnapshotType,

    /// Size of the snapshot in bytes (approximate)
    pub size_bytes: usize,

    /// Version of the snapshot format
    pub version: u32,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum SnapshotType {
    Manual,
    Automatic,
    OnDemand,
}

impl Snapshot {
    /// Create a new snapshot from entity state
    pub fn new(
        entity_id: String,
        state: serde_json::Value,
        as_of: DateTime<Utc>,
        event_count: usize,
        snapshot_type: SnapshotType,
    ) -> Self {
        let state_json = serde_json::to_string(&state).unwrap_or_default();
        let size_bytes = state_json.len();

        Self {
            id: Uuid::new_v4(),
            entity_id,
            state,
            created_at: Utc::now(),
            as_of,
            event_count,
            metadata: SnapshotMetadata {
                snapshot_type,
                size_bytes,
                version: 1,
            },
        }
    }

    /// Merge this snapshot with subsequent events to get current state
    pub fn merge_with_events(&self, events: &[Event]) -> serde_json::Value {
        let mut merged = self.state.clone();

        for event in events {
            // Only process events after the snapshot
            if event.timestamp > self.as_of {
                if let serde_json::Value::Object(ref mut state_map) = merged {
                    if let serde_json::Value::Object(ref payload_map) = event.payload {
                        for (key, value) in payload_map {
                            state_map.insert(key.clone(), value.clone());
                        }
                    }
                }
            }
        }

        merged
    }
}

/// Configuration for snapshot creation
#[derive(Debug, Clone)]
pub struct SnapshotConfig {
    /// Create snapshot after this many events for an entity
    pub event_threshold: usize,

    /// Maximum age before creating a new snapshot
    pub time_threshold_seconds: i64,

    /// Maximum number of snapshots to keep per entity
    pub max_snapshots_per_entity: usize,

    /// Enable automatic snapshot creation
    pub auto_snapshot: bool,
}

impl Default for SnapshotConfig {
    fn default() -> Self {
        Self {
            event_threshold: 100,
            time_threshold_seconds: 3600, // 1 hour
            max_snapshots_per_entity: 10,
            auto_snapshot: true,
        }
    }
}

/// Manages entity snapshots for fast state recovery
pub struct SnapshotManager {
    /// Snapshots organized by entity_id - using DashMap for lock-free concurrent access
    snapshots: Arc<DashMap<String, Vec<Snapshot>>>,

    /// Configuration
    config: SnapshotConfig,

    /// Statistics
    stats: Arc<RwLock<SnapshotStats>>,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct SnapshotStats {
    pub total_snapshots: usize,
    pub total_entities: usize,
    pub total_size_bytes: usize,
    pub snapshots_created: u64,
    pub snapshots_pruned: u64,
}

impl SnapshotManager {
    /// Create a new snapshot manager
    pub fn new(config: SnapshotConfig) -> Self {
        Self {
            snapshots: Arc::new(DashMap::new()),
            config,
            stats: Arc::new(RwLock::new(SnapshotStats::default())),
        }
    }

    /// Create a new snapshot for an entity
    pub fn create_snapshot(
        &self,
        entity_id: String,
        state: serde_json::Value,
        as_of: DateTime<Utc>,
        event_count: usize,
        snapshot_type: SnapshotType,
    ) -> Result<Snapshot> {
        let snapshot = Snapshot::new(entity_id.clone(), state, as_of, event_count, snapshot_type);

        let mut entity_snapshots = self.snapshots.entry(entity_id.clone()).or_default();

        // Add new snapshot
        entity_snapshots.push(snapshot.clone());

        // Sort by timestamp (newest first)
        entity_snapshots.sort_by_key(|x| std::cmp::Reverse(x.as_of));

        // Prune old snapshots if over limit
        let mut pruned = 0;
        if entity_snapshots.len() > self.config.max_snapshots_per_entity {
            let to_remove = entity_snapshots.len() - self.config.max_snapshots_per_entity;
            entity_snapshots.truncate(self.config.max_snapshots_per_entity);
            pruned = to_remove;
        }

        // Drop the entry guard before updating stats
        drop(entity_snapshots);

        // Update statistics
        let mut stats = self.stats.write();
        stats.snapshots_created += 1;
        stats.snapshots_pruned += pruned as u64;
        stats.total_snapshots = self.snapshots.iter().map(|entry| entry.value().len()).sum();
        stats.total_entities = self.snapshots.len();
        stats.total_size_bytes = self.snapshots
            .iter()
            .map(|entry| entry.value().iter().map(|s| s.metadata.size_bytes).sum::<usize>())
            .sum();

        tracing::info!(
            "📸 Created {} snapshot for entity: {} (events: {}, size: {} bytes)",
            match snapshot_type {
                SnapshotType::Manual => "manual",
                SnapshotType::Automatic => "automatic",
                SnapshotType::OnDemand => "on-demand",
            },
            entity_id,
            event_count,
            snapshot.metadata.size_bytes
        );

        Ok(snapshot)
    }

    /// Get the most recent snapshot for an entity
    pub fn get_latest_snapshot(&self, entity_id: &str) -> Option<Snapshot> {
        self.snapshots
            .get(entity_id)
            .and_then(|entry| entry.value().first().cloned())
    }

    /// Get the best snapshot to use for reconstruction as of a specific time
    pub fn get_snapshot_as_of(&self, entity_id: &str, as_of: DateTime<Utc>) -> Option<Snapshot> {
        self.snapshots.get(entity_id).and_then(|entry| {
            entry.value()
                .iter()
                .filter(|s| s.as_of <= as_of)
                .max_by_key(|s| s.as_of)
                .cloned()
        })
    }

    /// Get all snapshots for an entity
    pub fn get_all_snapshots(&self, entity_id: &str) -> Vec<Snapshot> {
        self.snapshots.get(entity_id).map(|entry| entry.value().clone()).unwrap_or_default()
    }

    /// Check if a new snapshot should be created for an entity
    pub fn should_create_snapshot(
        &self,
        entity_id: &str,
        current_event_count: usize,
        last_event_time: DateTime<Utc>,
    ) -> bool {
        if !self.config.auto_snapshot {
            return false;
        }

        match self.snapshots.get(entity_id) {
            None => {
                // No snapshots exist, create one if we have enough events
                current_event_count >= self.config.event_threshold
            }
            Some(entry) => {
                let snaps = entry.value();
                if let Some(latest) = snaps.first() {
                    // Check event count threshold
                    let events_since_snapshot = current_event_count - latest.event_count;
                    if events_since_snapshot >= self.config.event_threshold {
                        return true;
                    }

                    // Check time threshold
                    let time_since_snapshot = (last_event_time - latest.as_of).num_seconds();
                    if time_since_snapshot >= self.config.time_threshold_seconds {
                        return true;
                    }
                }
                false
            }
        }
    }

    /// Delete all snapshots for an entity
    pub fn delete_snapshots(&self, entity_id: &str) -> Result<usize> {
        let removed = self.snapshots.remove(entity_id).map(|(_, v)| v.len()).unwrap_or(0);

        // Update stats
        let mut stats = self.stats.write();
        stats.total_snapshots = stats.total_snapshots.saturating_sub(removed);
        stats.total_entities = self.snapshots.len();

        tracing::info!("🗑️ Deleted {} snapshots for entity: {}", removed, entity_id);

        Ok(removed)
    }

    /// Delete a specific snapshot by ID
    pub fn delete_snapshot(&self, entity_id: &str, snapshot_id: Uuid) -> Result<bool> {
        if let Some(mut entity_snapshots) = self.snapshots.get_mut(entity_id) {
            let initial_len = entity_snapshots.len();
            entity_snapshots.retain(|s| s.id != snapshot_id);
            let removed = initial_len != entity_snapshots.len();

            if removed {
                // Update stats
                let mut stats = self.stats.write();
                stats.total_snapshots = stats.total_snapshots.saturating_sub(1);
                tracing::debug!("Deleted snapshot {} for entity {}", snapshot_id, entity_id);
            }

            return Ok(removed);
        }

        Ok(false)
    }

    /// Get snapshot statistics
    pub fn stats(&self) -> SnapshotStats {
        (*self.stats.read()).clone()
    }

    /// Clear all snapshots
    pub fn clear_all(&self) {
        self.snapshots.clear();

        let mut stats = self.stats.write();
        *stats = SnapshotStats::default();

        tracing::info!("🧹 Cleared all snapshots");
    }

    /// Get configuration
    pub fn config(&self) -> &SnapshotConfig {
        &self.config
    }

    /// List all entities with snapshots
    pub fn list_entities(&self) -> Vec<String> {
        self.snapshots.iter().map(|entry| entry.key().clone()).collect()
    }
}

/// Request to create a manual snapshot
#[derive(Debug, Deserialize)]
pub struct CreateSnapshotRequest {
    pub entity_id: String,
}

/// Response after creating a snapshot
#[derive(Debug, Serialize)]
pub struct CreateSnapshotResponse {
    pub snapshot_id: Uuid,
    pub entity_id: String,
    pub created_at: DateTime<Utc>,
    pub event_count: usize,
    pub size_bytes: usize,
}

/// Request to list snapshots
#[derive(Debug, Deserialize)]
pub struct ListSnapshotsRequest {
    pub entity_id: Option<String>,
}

/// Response containing snapshot list
#[derive(Debug, Serialize)]
pub struct ListSnapshotsResponse {
    pub snapshots: Vec<SnapshotInfo>,
    pub total: usize,
}

#[derive(Debug, Serialize)]
pub struct SnapshotInfo {
    pub id: Uuid,
    pub entity_id: String,
    pub created_at: DateTime<Utc>,
    pub as_of: DateTime<Utc>,
    pub event_count: usize,
    pub size_bytes: usize,
    pub snapshot_type: SnapshotType,
}

impl From<Snapshot> for SnapshotInfo {
    fn from(snapshot: Snapshot) -> Self {
        Self {
            id: snapshot.id,
            entity_id: snapshot.entity_id,
            created_at: snapshot.created_at,
            as_of: snapshot.as_of,
            event_count: snapshot.event_count,
            size_bytes: snapshot.metadata.size_bytes,
            snapshot_type: snapshot.metadata.snapshot_type,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;
    use serde_json::json;

    fn create_test_snapshot(entity_id: &str, event_count: usize) -> Snapshot {
        Snapshot::new(
            entity_id.to_string(),
            json!({"count": event_count}),
            Utc::now(),
            event_count,
            SnapshotType::Automatic,
        )
    }

    #[test]
    fn test_snapshot_creation() {
        let snapshot = create_test_snapshot("entity-1", 100);
        assert_eq!(snapshot.entity_id, "entity-1");
        assert_eq!(snapshot.event_count, 100);
        assert_eq!(snapshot.metadata.snapshot_type, SnapshotType::Automatic);
    }

    #[test]
    fn test_snapshot_manager() {
        let manager = SnapshotManager::new(SnapshotConfig::default());

        let result = manager.create_snapshot(
            "entity-1".to_string(),
            json!({"value": 42}),
            Utc::now(),
            100,
            SnapshotType::Manual,
        );

        assert!(result.is_ok());

        let latest = manager.get_latest_snapshot("entity-1");
        assert!(latest.is_some());
        assert_eq!(latest.unwrap().event_count, 100);
    }

    #[test]
    fn test_snapshot_pruning() {
        let config = SnapshotConfig {
            max_snapshots_per_entity: 3,
            ..Default::default()
        };
        let manager = SnapshotManager::new(config);

        // Create 5 snapshots
        for i in 0..5 {
            manager
                .create_snapshot(
                    "entity-1".to_string(),
                    json!({"count": i}),
                    Utc::now(),
                    i,
                    SnapshotType::Automatic,
                )
                .unwrap();
        }

        // Should only keep 3 most recent
        let snapshots = manager.get_all_snapshots("entity-1");
        assert_eq!(snapshots.len(), 3);
    }

    #[test]
    fn test_should_create_snapshot() {
        let config = SnapshotConfig {
            event_threshold: 100,
            time_threshold_seconds: 3600,
            auto_snapshot: true,
            ..Default::default()
        };
        let manager = SnapshotManager::new(config);

        // No snapshots, not enough events
        assert!(!manager.should_create_snapshot("entity-1", 50, Utc::now()));

        // No snapshots, enough events
        assert!(manager.should_create_snapshot("entity-1", 100, Utc::now()));

        // Create a snapshot
        manager
            .create_snapshot(
                "entity-1".to_string(),
                json!({"value": 1}),
                Utc::now(),
                100,
                SnapshotType::Automatic,
            )
            .unwrap();

        // Not enough new events
        assert!(!manager.should_create_snapshot("entity-1", 150, Utc::now()));

        // Enough new events
        assert!(manager.should_create_snapshot("entity-1", 200, Utc::now()));
    }

    #[test]
    fn test_merge_with_events() {
        let snapshot = Snapshot::new(
            "entity-1".to_string(),
            json!({"name": "Alice", "score": 10}),
            Utc::now(),
            5,
            SnapshotType::Automatic,
        );

        let event = Event::reconstruct_from_strings(
            Uuid::new_v4(),
            "score.updated".to_string(),
            "entity-1".to_string(),
            "default".to_string(),
            json!({"score": 20}),
            Utc::now(),
            None,
            1,
        );

        let merged = snapshot.merge_with_events(&[event]);
        assert_eq!(merged["name"], "Alice");
        assert_eq!(merged["score"], 20);
    }

    #[test]
    fn test_default_config() {
        let config = SnapshotConfig::default();
        assert_eq!(config.event_threshold, 100);
        assert_eq!(config.time_threshold_seconds, 3600);
        assert_eq!(config.max_snapshots_per_entity, 10);
        assert!(config.auto_snapshot);
    }

    #[test]
    fn test_snapshot_type_serde() {
        let types = vec![
            SnapshotType::Manual,
            SnapshotType::Automatic,
            SnapshotType::OnDemand,
        ];

        for snapshot_type in types {
            let json = serde_json::to_string(&snapshot_type).unwrap();
            let parsed: SnapshotType = serde_json::from_str(&json).unwrap();
            assert_eq!(parsed, snapshot_type);
        }
    }

    #[test]
    fn test_snapshot_serde() {
        let snapshot = create_test_snapshot("entity-1", 50);
        let json = serde_json::to_string(&snapshot).unwrap();
        let parsed: Snapshot = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.entity_id, snapshot.entity_id);
        assert_eq!(parsed.event_count, snapshot.event_count);
    }

    #[test]
    fn test_get_latest_snapshot_none() {
        let manager = SnapshotManager::new(SnapshotConfig::default());
        let latest = manager.get_latest_snapshot("non-existent");
        assert!(latest.is_none());
    }

    #[test]
    fn test_get_all_snapshots_empty() {
        let manager = SnapshotManager::new(SnapshotConfig::default());
        let snapshots = manager.get_all_snapshots("non-existent");
        assert!(snapshots.is_empty());
    }

    #[test]
    fn test_delete_snapshots() {
        let manager = SnapshotManager::new(SnapshotConfig::default());

        manager
            .create_snapshot(
                "entity-1".to_string(),
                json!({"value": 1}),
                Utc::now(),
                100,
                SnapshotType::Manual,
            )
            .unwrap();

        let deleted = manager.delete_snapshots("entity-1").unwrap();
        assert_eq!(deleted, 1);

        let latest = manager.get_latest_snapshot("entity-1");
        assert!(latest.is_none());
    }

    #[test]
    fn test_delete_single_snapshot() {
        let manager = SnapshotManager::new(SnapshotConfig::default());

        let snapshot = manager
            .create_snapshot(
                "entity-1".to_string(),
                json!({"value": 1}),
                Utc::now(),
                100,
                SnapshotType::Manual,
            )
            .unwrap();

        let deleted = manager.delete_snapshot("entity-1", snapshot.id).unwrap();
        assert!(deleted);

        let latest = manager.get_latest_snapshot("entity-1");
        assert!(latest.is_none());
    }

    #[test]
    fn test_delete_nonexistent_snapshot() {
        let manager = SnapshotManager::new(SnapshotConfig::default());
        let deleted = manager.delete_snapshot("entity-1", Uuid::new_v4()).unwrap();
        assert!(!deleted);
    }

    #[test]
    fn test_clear_all() {
        let manager = SnapshotManager::new(SnapshotConfig::default());

        manager
            .create_snapshot(
                "entity-1".to_string(),
                json!({"value": 1}),
                Utc::now(),
                100,
                SnapshotType::Manual,
            )
            .unwrap();
        manager
            .create_snapshot(
                "entity-2".to_string(),
                json!({"value": 2}),
                Utc::now(),
                200,
                SnapshotType::Manual,
            )
            .unwrap();

        manager.clear_all();

        let stats = manager.stats();
        assert_eq!(stats.total_snapshots, 0);
        assert_eq!(stats.total_entities, 0);
    }

    #[test]
    fn test_list_entities() {
        let manager = SnapshotManager::new(SnapshotConfig::default());

        manager
            .create_snapshot(
                "entity-1".to_string(),
                json!({"value": 1}),
                Utc::now(),
                100,
                SnapshotType::Manual,
            )
            .unwrap();
        manager
            .create_snapshot(
                "entity-2".to_string(),
                json!({"value": 2}),
                Utc::now(),
                200,
                SnapshotType::Manual,
            )
            .unwrap();

        let entities = manager.list_entities();
        assert_eq!(entities.len(), 2);
        assert!(entities.contains(&"entity-1".to_string()));
        assert!(entities.contains(&"entity-2".to_string()));
    }

    #[test]
    fn test_get_config() {
        let config = SnapshotConfig {
            event_threshold: 50,
            ..Default::default()
        };
        let manager = SnapshotManager::new(config);
        assert_eq!(manager.config().event_threshold, 50);
    }

    #[test]
    fn test_stats() {
        let manager = SnapshotManager::new(SnapshotConfig::default());

        manager
            .create_snapshot(
                "entity-1".to_string(),
                json!({"value": 1}),
                Utc::now(),
                100,
                SnapshotType::Manual,
            )
            .unwrap();

        let stats = manager.stats();
        assert_eq!(stats.total_snapshots, 1);
        assert_eq!(stats.total_entities, 1);
        assert_eq!(stats.snapshots_created, 1);
    }

    #[test]
    fn test_snapshot_as_of() {
        let manager = SnapshotManager::new(SnapshotConfig::default());
        let now = Utc::now();
        let past = now - Duration::hours(2);
        let future = now + Duration::hours(2);

        // Create snapshot in the past
        manager
            .create_snapshot(
                "entity-1".to_string(),
                json!({"value": 1}),
                past,
                100,
                SnapshotType::Manual,
            )
            .unwrap();

        // Should find snapshot as of now
        let snapshot = manager.get_snapshot_as_of("entity-1", now);
        assert!(snapshot.is_some());

        // Should not find snapshot as of before it was created
        let very_past = past - Duration::hours(1);
        let snapshot = manager.get_snapshot_as_of("entity-1", very_past);
        assert!(snapshot.is_none());
    }

    #[test]
    fn test_should_create_snapshot_time_threshold() {
        let config = SnapshotConfig {
            event_threshold: 1000,     // High threshold so only time triggers
            time_threshold_seconds: 1, // 1 second
            auto_snapshot: true,
            ..Default::default()
        };
        let manager = SnapshotManager::new(config);

        // Create initial snapshot in the past
        let past = Utc::now() - Duration::seconds(2);
        manager
            .create_snapshot(
                "entity-1".to_string(),
                json!({"value": 1}),
                past,
                100,
                SnapshotType::Manual,
            )
            .unwrap();

        // Time threshold should trigger
        assert!(manager.should_create_snapshot("entity-1", 101, Utc::now()));
    }

    #[test]
    fn test_should_create_snapshot_disabled() {
        let config = SnapshotConfig {
            auto_snapshot: false,
            ..Default::default()
        };
        let manager = SnapshotManager::new(config);

        // Even with many events, should not create if disabled
        assert!(!manager.should_create_snapshot("entity-1", 1000, Utc::now()));
    }

    #[test]
    fn test_snapshot_info_from() {
        let snapshot = create_test_snapshot("entity-1", 50);
        let info: SnapshotInfo = snapshot.clone().into();

        assert_eq!(info.id, snapshot.id);
        assert_eq!(info.entity_id, snapshot.entity_id);
        assert_eq!(info.event_count, snapshot.event_count);
        assert_eq!(info.snapshot_type, snapshot.metadata.snapshot_type);
    }

    #[test]
    fn test_snapshot_metadata() {
        let snapshot = create_test_snapshot("entity-1", 100);
        assert_eq!(snapshot.metadata.version, 1);
        assert!(snapshot.metadata.size_bytes > 0);
    }

    #[test]
    fn test_multiple_entities() {
        let manager = SnapshotManager::new(SnapshotConfig::default());

        for i in 0..5 {
            manager
                .create_snapshot(
                    format!("entity-{}", i),
                    json!({"value": i}),
                    Utc::now(),
                    100 + i,
                    SnapshotType::Automatic,
                )
                .unwrap();
        }

        let stats = manager.stats();
        assert_eq!(stats.total_entities, 5);
        assert_eq!(stats.total_snapshots, 5);
    }

    #[test]
    fn test_snapshot_stats_default() {
        let stats = SnapshotStats::default();
        assert_eq!(stats.total_snapshots, 0);
        assert_eq!(stats.total_entities, 0);
        assert_eq!(stats.total_size_bytes, 0);
        assert_eq!(stats.snapshots_created, 0);
        assert_eq!(stats.snapshots_pruned, 0);
    }
}
