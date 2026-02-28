//! Replicant Worker Protocol projections.
//!
//! Three event-sourced projections that implement autonomous workflow
//! orchestration — no Temporal needed.

use crate::{application::services::projection::Projection, domain::entities::Event, error::Result};
use dashmap::DashMap;
use serde_json::{json, Value};
use std::sync::Arc;

// =============================================================================
// Workflow Status Projection
// =============================================================================

/// Tracks workflow lifecycle: dispatched → claimed → running → completed/failed.
///
/// Supports human-in-the-loop approval flow and first-write-wins claim guard.
pub struct WorkflowStatusProjection {
    states: Arc<DashMap<String, Value>>,
}

impl WorkflowStatusProjection {
    pub fn new() -> Self {
        Self {
            states: Arc::new(DashMap::new()),
        }
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
                let steps_total = payload.get("steps_total").and_then(|v| v.as_u64()).unwrap_or(0);
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
                self.states.entry(entity_id).and_modify(|state| {
                    // First-write-wins: only accept claim if still pending
                    if state.get("status").and_then(|s| s.as_str()) == Some("pending") {
                        if let Some(rid) = payload.get("replicant_id") {
                            state["status"] = json!("claimed");
                            state["replicant_id"] = rid.clone();
                        }
                    }
                });
            }
            "workflow.step.completed" => {
                self.states.entry(entity_id).and_modify(|state| {
                    let status = state.get("status").and_then(|s| s.as_str()).unwrap_or("");
                    if status == "claimed" || status == "running" {
                        let completed = state
                            .get("steps_completed")
                            .and_then(|v| v.as_u64())
                            .unwrap_or(0)
                            + 1;
                        state["status"] = json!("running");
                        state["steps_completed"] = json!(completed);
                    }
                });
            }
            "workflow.step.failed" => {
                self.states.entry(entity_id).and_modify(|state| {
                    state["status"] = json!("failed");
                    if let Some(err) = payload.get("error") {
                        state["error"] = err.clone();
                    }
                });
            }
            "workflow.output.ready" => {
                self.states.entry(entity_id).and_modify(|state| {
                    state["status"] = json!("completed");
                    if let Some(result) = payload.get("result") {
                        state["output"] = result.clone();
                    }
                });
            }
            "workflow.approval.requested" => {
                self.states.entry(entity_id).and_modify(|state| {
                    state["status"] = json!("awaiting_approval");
                    state["awaiting_approval"] = json!(true);
                });
            }
            "workflow.approval.granted" => {
                self.states.entry(entity_id).and_modify(|state| {
                    state["status"] = json!("running");
                    state["awaiting_approval"] = json!(false);
                });
            }
            "workflow.approval.rejected" => {
                self.states.entry(entity_id).and_modify(|state| {
                    state["status"] = json!("rejected");
                    state["awaiting_approval"] = json!(false);
                });
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
pub struct ReplicantRegistryProjection {
    states: Arc<DashMap<String, Value>>,
}

impl ReplicantRegistryProjection {
    pub fn new() -> Self {
        Self {
            states: Arc::new(DashMap::new()),
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
                let capabilities = payload
                    .get("capabilities")
                    .cloned()
                    .unwrap_or(json!([]));
                self.states.insert(
                    entity_id,
                    json!({
                        "status": "active",
                        "capabilities": capabilities,
                    }),
                );
            }
            "replicant.heartbeat" => {
                self.states.entry(entity_id).and_modify(|state| {
                    state["status"] = json!("active");
                    state["last_heartbeat"] = json!(event.timestamp().to_rfc3339());
                });
            }
            "replicant.stale" => {
                self.states.entry(entity_id).and_modify(|state| {
                    state["status"] = json!("stale");
                });
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
    pending: Arc<DashMap<String, ()>>,
}

impl TaskQueueProjection {
    pub fn new() -> Self {
        Self {
            pending: Arc::new(DashMap::new()),
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
        } else {
            if self.pending.contains_key(entity_id) {
                Some(json!({ "status": "pending" }))
            } else {
                None
            }
        }
    }

    fn clear(&self) {
        self.pending.clear();
    }
}
