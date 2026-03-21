//! Graph-aware offline sync for Prime.
//!
//! Wraps [`EmbeddedCore::sync_to`] with a sync report that categorizes
//! pushed/pulled events by graph type and surfaces graph-level conflicts
//! (e.g., node property conflicts, edge contradictions).

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::embedded::{EmbeddedCore, Query};

/// Report from a sync operation.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SyncReport {
    /// Number of events pushed to the peer.
    pub pushed: usize,
    /// Number of events pulled from the peer.
    pub pulled: usize,
    /// Detected conflicts.
    pub conflicts: Vec<SyncConflict>,
}

/// A conflict detected during sync.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncConflict {
    /// The entity that has conflicting state.
    pub entity_id: String,
    /// Type of conflict.
    pub conflict_type: ConflictType,
    /// Description of the conflict.
    pub description: String,
}

/// Types of graph-aware conflicts.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ConflictType {
    /// Same node updated with different properties on local and remote.
    NodePropertyConflict,
    /// Conflicting exclusive edges synced from remote.
    EdgeContradiction,
    /// Same logical entity exists with different IDs.
    DuplicateNode,
}

/// Preview of what a sync would do without applying changes.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SyncPreview {
    /// Events that would be pushed to the peer.
    pub will_push: usize,
    /// Events that would be pulled from the peer.
    pub will_pull: usize,
    /// Potential conflicts that would arise.
    pub potential_conflicts: Vec<SyncConflict>,
}

/// Sync Prime data between two embedded cores and produce a graph-aware report.
///
/// This performs a bidirectional sync:
/// 1. Push local events to peer (via `sync_to`)
/// 2. Pull peer events to local (via `peer.sync_to(local)`)
/// 3. Analyze the exchanged events for graph conflicts
pub async fn sync(
    local: &EmbeddedCore,
    peer: &EmbeddedCore,
) -> crate::error::Result<SyncReport> {
    // Count events before sync
    let local_before = count_prime_events(local).await?;
    let peer_before = count_prime_events(peer).await?;

    // Snapshot entity states before sync for conflict detection
    let pre_sync_states = snapshot_node_states(local).await?;

    // Bidirectional sync
    local.sync_to(peer).await?;
    peer.sync_to(local).await?;

    // Count events after sync
    let local_after = count_prime_events(local).await?;
    let peer_after = count_prime_events(peer).await?;

    let pushed = peer_after.saturating_sub(peer_before);
    let pulled = local_after.saturating_sub(local_before);

    // Detect conflicts by comparing pre/post sync state
    let conflicts = detect_conflicts(local, &pre_sync_states).await?;

    Ok(SyncReport {
        pushed,
        pulled,
        conflicts,
    })
}

/// Preview what a sync would do without applying changes.
pub async fn sync_preview(
    local: &EmbeddedCore,
    peer: &EmbeddedCore,
) -> crate::error::Result<SyncPreview> {
    // Count prime events on each side to estimate delta
    let local_count = count_prime_events(local).await?;
    let peer_count = count_prime_events(peer).await?;

    // Simple estimate: the difference in event counts approximates the delta.
    // For a precise preview we'd need to run the CRDT resolver without applying,
    // which is the sync_to logic minus the ingest step. For now, this gives a
    // directional estimate.
    Ok(SyncPreview {
        will_push: local_count.saturating_sub(peer_count),
        will_pull: peer_count.saturating_sub(local_count),
        potential_conflicts: vec![], // Full conflict detection requires applying events
    })
}

/// Count total prime.* events in a core instance.
async fn count_prime_events(core: &EmbeddedCore) -> crate::error::Result<usize> {
    let events = core
        .query(Query::new().event_type_prefix("prime."))
        .await?;
    Ok(events.len())
}

/// Snapshot the latest payload per node entity for pre-sync comparison.
async fn snapshot_node_states(
    core: &EmbeddedCore,
) -> crate::error::Result<std::collections::HashMap<String, Value>> {
    let events = core
        .query(Query::new().event_type_prefix("prime.node."))
        .await?;

    let mut states: std::collections::HashMap<String, Value> = std::collections::HashMap::new();
    for event in &events {
        // Keep the latest payload per entity (events are ordered)
        states.insert(event.entity_id.clone(), event.payload.clone());
    }
    Ok(states)
}

/// Detect conflicts by comparing entity state before and after sync.
///
/// An entity that had state before sync and now has additional update events
/// with different payloads indicates a concurrent modification that was merged.
async fn detect_conflicts(
    core: &EmbeddedCore,
    pre_sync_states: &std::collections::HashMap<String, Value>,
) -> crate::error::Result<Vec<SyncConflict>> {
    let post_events = core
        .query(Query::new().event_type_prefix("prime.node."))
        .await?;

    // Build post-sync latest state per entity
    let mut post_states: std::collections::HashMap<String, Value> = std::collections::HashMap::new();
    // Track how many distinct update payloads each entity has
    let mut update_payloads: std::collections::HashMap<String, Vec<Value>> =
        std::collections::HashMap::new();

    for event in &post_events {
        post_states.insert(event.entity_id.clone(), event.payload.clone());
        if event.event_type == crate::prime::types::event_types::NODE_UPDATED {
            update_payloads
                .entry(event.entity_id.clone())
                .or_default()
                .push(event.payload.clone());
        }
    }

    let mut conflicts = Vec::new();

    for (entity_id, pre_state) in pre_sync_states {
        if let Some(post_state) = post_states.get(entity_id) {
            // If the state changed and there are multiple distinct updates,
            // this indicates concurrent modifications that were merged
            if pre_state != post_state
                && let Some(updates) = update_payloads.get(entity_id)
            {
                // Check for distinct update payloads (not just count)
                let distinct: std::collections::HashSet<String> = updates
                    .iter()
                    .map(|v| serde_json::to_string(v).unwrap_or_default())
                    .collect();
                if distinct.len() > 1 {
                    conflicts.push(SyncConflict {
                        entity_id: entity_id.clone(),
                        conflict_type: ConflictType::NodePropertyConflict,
                        description: format!(
                            "Entity {entity_id} has {} distinct updates — concurrent modification merged",
                            distinct.len()
                        ),
                    });
                }
            }
        }
    }

    Ok(conflicts)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::embedded::{Config, IngestEvent};
    use crate::prime::types::event_types;

    async fn test_core(node_id: u32) -> EmbeddedCore {
        let config = Config::builder().node_id(node_id).build().unwrap();
        EmbeddedCore::open(config).await.unwrap()
    }

    #[tokio::test]
    async fn test_sync_bidirectional() {
        let local = test_core(1).await;
        let remote = test_core(2).await;

        // Add a node on local
        local
            .ingest(IngestEvent {
                entity_id: "node:person:alice",
                event_type: event_types::NODE_CREATED,
                payload: serde_json::json!({"node_type": "person", "properties": {"name": "Alice"}}),
                metadata: None,
                tenant_id: None,
            })
            .await
            .unwrap();

        // Add a different node on remote
        remote
            .ingest(IngestEvent {
                entity_id: "node:person:bob",
                event_type: event_types::NODE_CREATED,
                payload: serde_json::json!({"node_type": "person", "properties": {"name": "Bob"}}),
                metadata: None,
                tenant_id: None,
            })
            .await
            .unwrap();

        let report = sync(&local, &remote).await.unwrap();

        // Both should have pushed 1 event and pulled 1
        assert!(report.pushed > 0, "should have pushed events");
        assert!(report.pulled > 0, "should have pulled events");

        // Verify both sides have both nodes
        let local_events = local
            .query(Query::new().event_type(event_types::NODE_CREATED))
            .await
            .unwrap();
        assert_eq!(local_events.len(), 2);

        let remote_events = remote
            .query(Query::new().event_type(event_types::NODE_CREATED))
            .await
            .unwrap();
        assert_eq!(remote_events.len(), 2);

        local.shutdown().await.unwrap();
        remote.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn test_sync_preview_estimates() {
        let local = test_core(3).await;
        let remote = test_core(4).await;

        // Add 3 events on local, 1 on remote
        for i in 0..3 {
            local
                .ingest(IngestEvent {
                    entity_id: &format!("node:n:l{i}"),
                    event_type: event_types::NODE_CREATED,
                    payload: serde_json::json!({"node_type": "n"}),
                    metadata: None,
                    tenant_id: None,
                })
                .await
                .unwrap();
        }

        remote
            .ingest(IngestEvent {
                entity_id: "node:n:r0",
                event_type: event_types::NODE_CREATED,
                payload: serde_json::json!({"node_type": "n"}),
                metadata: None,
                tenant_id: None,
            })
            .await
            .unwrap();

        let preview = sync_preview(&local, &remote).await.unwrap();

        // Local has 3, remote has 1 → will_push ~2, will_pull ~0
        assert!(preview.will_push > 0);

        local.shutdown().await.unwrap();
        remote.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn test_sync_empty_cores() {
        let local = test_core(5).await;
        let remote = test_core(6).await;

        let report = sync(&local, &remote).await.unwrap();
        assert_eq!(report.pushed, 0);
        assert_eq!(report.pulled, 0);
        assert!(report.conflicts.is_empty());

        local.shutdown().await.unwrap();
        remote.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn test_conflict_type_serialization() {
        let conflict = SyncConflict {
            entity_id: "node:person:alice".to_string(),
            conflict_type: ConflictType::NodePropertyConflict,
            description: "test".to_string(),
        };
        let json = serde_json::to_string(&conflict).unwrap();
        let parsed: SyncConflict = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.conflict_type, ConflictType::NodePropertyConflict);
    }
}
