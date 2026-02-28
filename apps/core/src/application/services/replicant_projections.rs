//! Projections for the Replicant Worker Protocol (Phase 5).
//!
//! Three projections handle cloud worker orchestration:
//! - [`WorkflowStatusProjection`] — folds `workflow.*` events into per-entity status
//! - [`ReplicantRegistryProjection`] — folds `replicant.*` events into worker registry
//! - [`TaskQueueProjection`] — tracks dispatched-but-unclaimed workflows

use crate::{domain::entities::Event, error::Result};
use dashmap::DashMap;
use serde_json::{json, Value};
use std::sync::Arc;

use super::projection::Projection;

// =============================================================================
// Workflow Status Projection
// =============================================================================

/// Folds `workflow.*` events into a per-entity status view.
///
/// State shape:
/// ```json
/// {
///   "status": "pending" | "claimed" | "running" | "awaiting_approval" | "completed" | "failed" | "rejected",
///   "replicant_id": "r-1",
///   "steps_total": 3,
///   "steps_completed": 2,
///   "awaiting_approval": false,
///   "output": null,
///   "error": null
/// }
/// ```
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
        let event_type = event.event_type_str();
        let entity_id = event.entity_id_str().to_string();

        match event_type {
            "workflow.dispatched" => {
                let steps_total = event.payload.get("steps_total")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0);

                self.states.insert(entity_id, json!({
                    "status": "pending",
                    "steps_total": steps_total,
                    "steps_completed": 0,
                    "awaiting_approval": false,
                    "output": null,
                    "error": null,
                    "replicant_id": null,
                }));
            }
            "workflow.claimed" => {
                if let Some(mut entry) = self.states.get_mut(&entity_id) {
                    let state = entry.value_mut();
                    // First-write-wins: only accept claim if still pending
                    if state["status"] == "pending" {
                        let replicant_id = event.payload.get("replicant_id")
                            .cloned()
                            .unwrap_or(Value::Null);
                        state["status"] = json!("claimed");
                        state["replicant_id"] = replicant_id;
                    }
                }
            }
            "workflow.step.started" => {
                if let Some(mut entry) = self.states.get_mut(&entity_id) {
                    let state = entry.value_mut();
                    // Only transition from claimed or running (not from terminal states)
                    let status = state["status"].as_str().unwrap_or("");
                    if status == "claimed" || status == "running" {
                        state["status"] = json!("running");
                    }
                }
            }
            "workflow.step.completed" => {
                if let Some(mut entry) = self.states.get_mut(&entity_id) {
                    let state = entry.value_mut();
                    let status = state["status"].as_str().unwrap_or("");
                    if status == "running" || status == "claimed" {
                        state["status"] = json!("running");
                        let completed = state["steps_completed"].as_u64().unwrap_or(0) + 1;
                        state["steps_completed"] = json!(completed);
                    }
                }
            }
            "workflow.step.failed" => {
                if let Some(mut entry) = self.states.get_mut(&entity_id) {
                    let state = entry.value_mut();
                    let status = state["status"].as_str().unwrap_or("");
                    // Only fail from active states (not from already-completed/rejected)
                    if status == "running" || status == "claimed" || status == "awaiting_approval" {
                        state["status"] = json!("failed");
                        if let Some(error) = event.payload.get("error") {
                            state["error"] = error.clone();
                        }
                    }
                }
            }
            "workflow.approval.requested" => {
                if let Some(mut entry) = self.states.get_mut(&entity_id) {
                    let state = entry.value_mut();
                    let status = state["status"].as_str().unwrap_or("");
                    if status == "running" {
                        state["status"] = json!("awaiting_approval");
                        state["awaiting_approval"] = json!(true);
                    }
                }
            }
            "workflow.approval.granted" => {
                if let Some(mut entry) = self.states.get_mut(&entity_id) {
                    let state = entry.value_mut();
                    let status = state["status"].as_str().unwrap_or("");
                    if status == "awaiting_approval" {
                        state["status"] = json!("running");
                        state["awaiting_approval"] = json!(false);
                    }
                }
            }
            "workflow.approval.rejected" => {
                if let Some(mut entry) = self.states.get_mut(&entity_id) {
                    let state = entry.value_mut();
                    let status = state["status"].as_str().unwrap_or("");
                    if status == "awaiting_approval" {
                        state["status"] = json!("rejected");
                        state["awaiting_approval"] = json!(false);
                    }
                }
            }
            "workflow.output.ready" => {
                if let Some(mut entry) = self.states.get_mut(&entity_id) {
                    let state = entry.value_mut();
                    let status = state["status"].as_str().unwrap_or("");
                    // Allow completion from any active state
                    if status == "running" || status == "claimed" {
                        state["status"] = json!("completed");
                        if let Some(result) = event.payload.get("result") {
                            state["output"] = result.clone();
                        }
                    }
                }
            }
            _ => {}
        }

        Ok(())
    }

    fn get_state(&self, entity_id: &str) -> Option<Value> {
        self.states.get(entity_id).map(|v| v.value().clone())
    }

    fn clear(&self) {
        self.states.clear();
    }
}

// =============================================================================
// Replicant Registry Projection
// =============================================================================

/// Folds `replicant.*` events into a per-replicant status view.
///
/// State shape:
/// ```json
/// {
///   "status": "active" | "stale",
///   "capabilities": ["summarize", "translate"],
///   "last_heartbeat": "2024-01-01T00:00:00Z"
/// }
/// ```
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
        let event_type = event.event_type_str();
        let entity_id = event.entity_id_str().to_string();

        match event_type {
            "replicant.registered" => {
                let capabilities = event.payload.get("capabilities")
                    .cloned()
                    .unwrap_or(json!([]));

                self.states.insert(entity_id, json!({
                    "status": "active",
                    "capabilities": capabilities,
                    "last_heartbeat": event.timestamp.to_rfc3339(),
                }));
            }
            "replicant.heartbeat" => {
                if let Some(mut entry) = self.states.get_mut(&entity_id) {
                    let state = entry.value_mut();
                    state["status"] = json!("active");
                    state["last_heartbeat"] = json!(event.timestamp.to_rfc3339());
                }
            }
            "replicant.stale" => {
                if let Some(mut entry) = self.states.get_mut(&entity_id) {
                    let state = entry.value_mut();
                    state["status"] = json!("stale");
                }
            }
            _ => {}
        }

        Ok(())
    }

    fn get_state(&self, entity_id: &str) -> Option<Value> {
        self.states.get(entity_id).map(|v| v.value().clone())
    }

    fn clear(&self) {
        self.states.clear();
    }
}

// =============================================================================
// Task Queue Projection
// =============================================================================

/// Tracks dispatched-but-unclaimed workflows.
///
/// Use `get_state("__all")` to return the full pending queue (sorted
/// alphabetically for deterministic output). Use a specific entity ID
/// to check if a single workflow is queued.
///
/// State shape for `__all`:
/// ```json
/// { "pending": ["wf-2", "wf-3"] }
/// ```
pub struct TaskQueueProjection {
    /// Set of workflow IDs that are dispatched but not yet claimed or completed.
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
        let event_type = event.event_type_str();
        let entity_id = event.entity_id_str().to_string();

        match event_type {
            "workflow.dispatched" => {
                self.pending.insert(entity_id, ());
            }
            "workflow.claimed" | "workflow.output.ready" | "workflow.step.failed"
            | "workflow.approval.rejected" => {
                self.pending.remove(&entity_id);
            }
            _ => {}
        }

        Ok(())
    }

    /// Get task queue state.
    ///
    /// Use `entity_id = "__all"` to get the full pending queue (sorted for
    /// deterministic output). Use a specific entity ID to check if it's queued.
    fn get_state(&self, entity_id: &str) -> Option<Value> {
        if entity_id == "__all" {
            let mut pending: Vec<String> = self.pending
                .iter()
                .map(|entry| entry.key().clone())
                .collect();
            pending.sort();
            let pending_values: Vec<Value> = pending.into_iter().map(|s| json!(s)).collect();
            Some(json!({ "pending": pending_values }))
        } else {
            // Check if a specific workflow is in the queue
            if self.pending.contains_key(entity_id) {
                Some(json!({ "status": "queued" }))
            } else {
                None
            }
        }
    }

    fn clear(&self) {
        self.pending.clear();
    }
}
