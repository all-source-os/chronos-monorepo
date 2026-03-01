//! Replicant Worker Protocol projections.
//!
//! Three event-sourced projections that implement autonomous workflow
//! orchestration — no Temporal needed.

use crate::{
    application::services::projection::Projection, domain::entities::Event, error::Result,
};
use dashmap::DashMap;
use serde_json::{Value, json};

// =============================================================================
// Workflow Status Projection
// =============================================================================

/// Tracks workflow lifecycle: dispatched → claimed → running → completed/failed.
///
/// Supports human-in-the-loop approval flow and first-write-wins claim guard.
/// Handles out-of-order events by inserting default state on first encounter.
pub struct WorkflowStatusProjection {
    states: DashMap<String, Value>,
}

impl WorkflowStatusProjection {
    pub fn new() -> Self {
        Self {
            states: DashMap::new(),
        }
    }

    /// Ensure an entry exists for this entity, inserting a default if missing.
    fn ensure_entry(&self, entity_id: &str) -> dashmap::mapref::one::RefMut<'_, String, Value> {
        self.states.entry(entity_id.to_string()).or_insert_with(|| {
            json!({
                "status": "unknown",
                "steps_total": 0,
                "steps_completed": 0,
                "awaiting_approval": false,
            })
        })
    }
}

impl Projection for WorkflowStatusProjection {
    fn name(&self) -> &str {
        "workflow_status"
    }

    fn process(&self, event: &Event) -> Result<()> {
        let entity_id = event.entity_id_str().to_string();
        let event_type = event.event_type_str();
        let payload = &event.payload;

        match event_type {
            "workflow.dispatched" => {
                let steps_total = payload
                    .get("steps_total")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0);
                self.states.insert(
                    entity_id,
                    json!({
                        "status": "pending",
                        "steps_total": steps_total,
                        "steps_completed": 0,
                        "awaiting_approval": false,
                    }),
                );
            }
            "workflow.claimed" => {
                let mut state = self.ensure_entry(&entity_id);
                // First-write-wins: only accept claim if pending or unknown (out-of-order)
                let status = state.get("status").and_then(|s| s.as_str()).unwrap_or("");
                if status == "pending" || status == "unknown" {
                    if let Some(rid) = payload.get("replicant_id") {
                        state["status"] = json!("claimed");
                        state["replicant_id"] = rid.clone();
                    }
                }
            }
            "workflow.step.completed" => {
                let mut state = self.ensure_entry(&entity_id);
                let status = state.get("status").and_then(|s| s.as_str()).unwrap_or("");
                if status == "claimed" || status == "running" || status == "unknown" {
                    let completed = state
                        .get("steps_completed")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(0)
                        + 1;
                    state["status"] = json!("running");
                    state["steps_completed"] = json!(completed);
                }
            }
            "workflow.step.failed" => {
                let mut state = self.ensure_entry(&entity_id);
                state["status"] = json!("failed");
                if let Some(err) = payload.get("error") {
                    state["error"] = err.clone();
                }
            }
            "workflow.output.ready" => {
                let mut state = self.ensure_entry(&entity_id);
                state["status"] = json!("completed");
                if let Some(result) = payload.get("result") {
                    state["output"] = result.clone();
                }
            }
            "workflow.approval.requested" => {
                let mut state = self.ensure_entry(&entity_id);
                state["status"] = json!("awaiting_approval");
                state["awaiting_approval"] = json!(true);
            }
            "workflow.approval.granted" => {
                let mut state = self.ensure_entry(&entity_id);
                state["status"] = json!("running");
                state["awaiting_approval"] = json!(false);
            }
            "workflow.approval.rejected" => {
                let mut state = self.ensure_entry(&entity_id);
                state["status"] = json!("rejected");
                state["awaiting_approval"] = json!(false);
            }
            _ => {}
        }

        Ok(())
    }

    fn get_state(&self, entity_id: &str) -> Option<Value> {
        self.states.get(entity_id).map(|v| v.clone())
    }

    fn clear(&self) {
        self.states.clear();
    }
}

// =============================================================================
// Replicant Registry Projection
// =============================================================================

/// Tracks replicant workers: registration, heartbeats, stale detection.
/// Handles out-of-order events by inserting default state on first encounter.
pub struct ReplicantRegistryProjection {
    states: DashMap<String, Value>,
}

impl ReplicantRegistryProjection {
    pub fn new() -> Self {
        Self {
            states: DashMap::new(),
        }
    }
}

impl Projection for ReplicantRegistryProjection {
    fn name(&self) -> &str {
        "replicant_registry"
    }

    fn process(&self, event: &Event) -> Result<()> {
        let entity_id = event.entity_id_str().to_string();
        let event_type = event.event_type_str();
        let payload = &event.payload;

        match event_type {
            "replicant.registered" => {
                let capabilities = payload.get("capabilities").cloned().unwrap_or(json!([]));
                self.states.insert(
                    entity_id,
                    json!({
                        "status": "active",
                        "capabilities": capabilities,
                    }),
                );
            }
            "replicant.heartbeat" => {
                // Insert default if not registered yet (out-of-order)
                let mut state = self
                    .states
                    .entry(entity_id)
                    .or_insert_with(|| json!({"status": "active", "capabilities": []}));
                state["status"] = json!("active");
                state["last_heartbeat"] = json!(event.timestamp().to_rfc3339());
            }
            "replicant.stale" => {
                let mut state = self
                    .states
                    .entry(entity_id)
                    .or_insert_with(|| json!({"status": "stale", "capabilities": []}));
                state["status"] = json!("stale");
            }
            _ => {}
        }

        Ok(())
    }

    fn get_state(&self, entity_id: &str) -> Option<Value> {
        self.states.get(entity_id).map(|v| v.clone())
    }

    fn clear(&self) {
        self.states.clear();
    }
}

// =============================================================================
// Task Queue Projection
// =============================================================================

/// Tracks unclaimed workflows. Query with entity_id `__all` to get the full queue.
pub struct TaskQueueProjection {
    /// Set of workflow IDs that are pending (dispatched but not claimed/completed).
    pending: DashMap<String, ()>,
}

impl TaskQueueProjection {
    pub fn new() -> Self {
        Self {
            pending: DashMap::new(),
        }
    }
}

impl Projection for TaskQueueProjection {
    fn name(&self) -> &str {
        "task_queue"
    }

    fn process(&self, event: &Event) -> Result<()> {
        let entity_id = event.entity_id_str().to_string();
        let event_type = event.event_type_str();

        match event_type {
            "workflow.dispatched" => {
                self.pending.insert(entity_id, ());
            }
            "workflow.claimed" | "workflow.output.ready" | "workflow.step.failed" => {
                self.pending.remove(&entity_id);
            }
            _ => {}
        }

        Ok(())
    }

    fn get_state(&self, entity_id: &str) -> Option<Value> {
        if entity_id == "__all" {
            let pending: Vec<Value> = self
                .pending
                .iter()
                .map(|entry| json!(entry.key().clone()))
                .collect();
            Some(json!({ "pending": pending }))
        } else if self.pending.contains_key(entity_id) {
            Some(json!({ "status": "pending" }))
        } else {
            None
        }
    }

    fn clear(&self) {
        self.pending.clear();
    }
}
