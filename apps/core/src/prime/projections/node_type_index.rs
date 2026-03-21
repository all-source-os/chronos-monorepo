//! NodeTypeIndex projection — indexes nodes by their `node_type`.
//!
//! Maintains a `DashMap<String, HashSet<String>>` mapping `node_type` to the
//! set of `entity_id`s of that type, enabling efficient "all nodes of type X"
//! queries.

use dashmap::DashMap;
use serde_json::Value;
use std::collections::HashSet;
use std::sync::Arc;

use crate::{
    application::services::projection::Projection,
    domain::entities::Event,
    error::Result,
    prime::types::event_types,
};

/// Projection that indexes nodes by type for O(1) type-scoped queries.
pub struct NodeTypeIndexProjection {
    name: String,
    /// node_type -> set of entity_ids
    index: Arc<DashMap<String, HashSet<String>>>,
}

impl NodeTypeIndexProjection {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            index: Arc::new(DashMap::new()),
        }
    }

    /// Get all entity_ids for a given node type.
    pub fn nodes_by_type(&self, node_type: &str) -> Vec<String> {
        self.index
            .get(node_type)
            .map(|set| set.iter().cloned().collect())
            .unwrap_or_default()
    }

    /// Get all known node types.
    pub fn node_types(&self) -> Vec<String> {
        self.index.iter().map(|entry| entry.key().clone()).collect()
    }

    /// Get all entity IDs across all types. Used by contradiction backfill.
    pub fn all_entity_ids(&self) -> Vec<String> {
        self.index
            .iter()
            .flat_map(|entry| entry.value().iter().cloned().collect::<Vec<_>>())
            .collect()
    }
}

impl Projection for NodeTypeIndexProjection {
    fn name(&self) -> &str {
        &self.name
    }

    fn process(&self, event: &Event) -> Result<()> {
        let event_type = event.event_type_str();
        let entity_id = event.entity_id_str().to_string();
        let payload = &event.payload;

        match event_type {
            event_types::NODE_CREATED => {
                let node_type = payload
                    .get("node_type")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown")
                    .to_string();

                self.index
                    .entry(node_type)
                    .or_default()
                    .insert(entity_id);
            }
            event_types::NODE_DELETED => {
                // Extract type from entity_id format "node:{type}:{id}"
                if let Some(mut set) = entity_id
                    .split(':')
                    .nth(1)
                    .and_then(|node_type| self.index.get_mut(node_type))
                {
                    set.remove(&entity_id);
                }
            }
            _ => {}
        }

        Ok(())
    }

    fn get_state(&self, node_type: &str) -> Option<Value> {
        self.index.get(node_type).map(|set| {
            let ids: Vec<&String> = set.iter().collect();
            serde_json::to_value(ids).unwrap_or(Value::Null)
        })
    }

    fn clear(&self) {
        self.index.clear();
    }

    fn snapshot(&self) -> Option<Value> {
        let entries: Vec<(String, Vec<String>)> = self
            .index
            .iter()
            .map(|entry| {
                let ids: Vec<String> = entry.value().iter().cloned().collect();
                (entry.key().clone(), ids)
            })
            .collect();
        serde_json::to_value(entries).ok()
    }

    fn restore(&self, snapshot: &Value) -> Result<()> {
        let entries: Vec<(String, Vec<String>)> = serde_json::from_value(snapshot.clone())
            .map_err(|e| crate::error::AllSourceError::StorageError(e.to_string()))?;
        self.index.clear();
        for (key, ids) in entries {
            self.index.insert(key, ids.into_iter().collect());
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use uuid::Uuid;

    fn make_event(entity_id: &str, event_type: &str, payload: Value) -> Event {
        Event::reconstruct_from_strings(
            Uuid::new_v4(),
            event_type.to_string(),
            entity_id.to_string(),
            "default".to_string(),
            payload,
            Utc::now(),
            None,
            1,
        )
    }

    #[test]
    fn test_nodes_by_type() {
        let proj = NodeTypeIndexProjection::new("type_index");

        // Add 3 persons
        for id in ["alice", "bob", "carol"] {
            proj.process(&make_event(
                &format!("node:person:{id}"),
                event_types::NODE_CREATED,
                serde_json::json!({"node_type": "person", "properties": {"name": id}}),
            ))
            .unwrap();
        }

        // Add 2 projects
        for id in ["proj-a", "proj-b"] {
            proj.process(&make_event(
                &format!("node:project:{id}"),
                event_types::NODE_CREATED,
                serde_json::json!({"node_type": "project", "properties": {"name": id}}),
            ))
            .unwrap();
        }

        assert_eq!(proj.nodes_by_type("person").len(), 3);
        assert_eq!(proj.nodes_by_type("project").len(), 2);

        let mut types = proj.node_types();
        types.sort();
        assert_eq!(types, vec!["person", "project"]);
    }

    #[test]
    fn test_delete_removes_from_index() {
        let proj = NodeTypeIndexProjection::new("type_index");

        for id in ["alice", "bob", "carol"] {
            proj.process(&make_event(
                &format!("node:person:{id}"),
                event_types::NODE_CREATED,
                serde_json::json!({"node_type": "person", "properties": {"name": id}}),
            ))
            .unwrap();
        }

        assert_eq!(proj.nodes_by_type("person").len(), 3);

        // Delete alice
        proj.process(&make_event(
            "node:person:alice",
            event_types::NODE_DELETED,
            serde_json::json!({}),
        ))
        .unwrap();

        assert_eq!(proj.nodes_by_type("person").len(), 2);
    }

    #[test]
    fn test_snapshot_restore() {
        let proj = NodeTypeIndexProjection::new("type_index");

        proj.process(&make_event(
            "node:person:alice",
            event_types::NODE_CREATED,
            serde_json::json!({"node_type": "person", "properties": {}}),
        ))
        .unwrap();

        let snap = proj.snapshot().unwrap();
        proj.clear();
        assert!(proj.nodes_by_type("person").is_empty());

        proj.restore(&snap).unwrap();
        assert_eq!(proj.nodes_by_type("person").len(), 1);
    }
}
