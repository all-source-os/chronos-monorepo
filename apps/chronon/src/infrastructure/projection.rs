use allsource_core::{
    application::services::projection::Projection, domain::entities::Event, error::Result,
};
use dashmap::DashMap;
use serde_json::{Value, json};

/// Projection that aggregates task lifecycle events into queryable task state.
///
/// Handles: task.created, task.updated, task.dependency.added, task.dependency.removed,
/// workflow.claimed (first-write-wins), workflow.step.completed,
/// workflow.approval.requested, workflow.approval.granted
pub struct TaskProjection {
    states: DashMap<String, Value>,
}

impl Default for TaskProjection {
    fn default() -> Self {
        Self::new()
    }
}

impl TaskProjection {
    pub fn new() -> Self {
        Self {
            states: DashMap::new(),
        }
    }
}

impl Projection for TaskProjection {
    fn name(&self) -> &str {
        "chronon_tasks"
    }

    fn process(&self, event: &Event) -> Result<()> {
        let entity_id = event.entity_id_str().to_string();
        let event_type = event.event_type_str();
        let payload = &event.payload;

        match event_type {
            "task.created" => {
                let title = payload
                    .get("title")
                    .and_then(|v| v.as_str())
                    .unwrap_or("untitled")
                    .to_string();
                let priority = payload
                    .get("priority")
                    .and_then(|v| v.as_str())
                    .unwrap_or("p2")
                    .to_string();
                let task_type = payload
                    .get("task_type")
                    .and_then(|v| v.as_str())
                    .unwrap_or("task")
                    .to_string();
                let parent = payload.get("parent").and_then(|v| v.as_str()).map(String::from);
                let description = payload
                    .get("description")
                    .and_then(|v| v.as_str())
                    .map(String::from);
                let mut state = json!({
                    "title": title,
                    "priority": priority,
                    "status": "open",
                    "task_type": task_type,
                    "claimed_by": null,
                    "blocked_by": [],
                    "created_at": event.timestamp().to_rfc3339(),
                });
                if let Some(p) = parent {
                    state["parent"] = json!(p);
                }
                if let Some(d) = description {
                    state["description"] = json!(d);
                }
                self.states.insert(entity_id, state);
            }
            "task.updated" => {
                if let Some(mut state) = self.states.get_mut(&entity_id) {
                    if let Some(title) = payload.get("title") {
                        state["title"] = title.clone();
                    }
                    if let Some(priority) = payload.get("priority") {
                        state["priority"] = priority.clone();
                    }
                }
            }
            "task.dependency.added" => {
                if let Some(mut state) = self.states.get_mut(&entity_id) {
                    let dep_id = payload.get("depends_on").cloned().unwrap_or(json!(null));
                    if let Some(arr) = state["blocked_by"].as_array_mut() {
                        if !arr.contains(&dep_id) {
                            arr.push(dep_id);
                        }
                    }
                }
            }
            "task.dependency.removed" => {
                if let Some(mut state) = self.states.get_mut(&entity_id) {
                    if let Some(dep_id) = payload.get("depends_on") {
                        if let Some(arr) = state["blocked_by"].as_array_mut() {
                            arr.retain(|v| v != dep_id);
                        }
                    }
                }
            }
            "workflow.claimed" => {
                if let Some(mut state) = self.states.get_mut(&entity_id) {
                    // First-write-wins: only accept claim if still open
                    let status = state.get("status").and_then(|s| s.as_str()).unwrap_or("");
                    if status == "open" {
                        state["status"] = json!("in-progress");
                        if let Some(agent) = payload.get("agent_id") {
                            state["claimed_by"] = agent.clone();
                        }
                    }
                }
            }
            "workflow.step.completed" => {
                if let Some(mut state) = self.states.get_mut(&entity_id) {
                    state["status"] = json!("done");
                    if let Some(reason) = payload.get("reason") {
                        state["done_reason"] = reason.clone();
                    }
                    state["done_at"] = json!(event.timestamp().to_rfc3339());
                }
            }
            "workflow.approval.requested" => {
                if let Some(mut state) = self.states.get_mut(&entity_id) {
                    state["awaiting_approval"] = json!(true);
                }
            }
            "workflow.approval.granted" => {
                if let Some(mut state) = self.states.get_mut(&entity_id) {
                    state["awaiting_approval"] = json!(false);
                    state["approved"] = json!(true);
                    state["approved_at"] = json!(event.timestamp().to_rfc3339());
                }
            }
            _ => {}
        }

        Ok(())
    }

    fn get_state(&self, entity_id: &str) -> Option<Value> {
        if entity_id == "__all" {
            let tasks: Vec<Value> = self
                .states
                .iter()
                .map(|entry| {
                    let mut task = entry.value().clone();
                    task["id"] = json!(entry.key().clone());
                    task
                })
                .collect();
            Some(json!({ "tasks": tasks }))
        } else {
            self.states.get(entity_id).map(|v| {
                let mut task = v.clone();
                task["id"] = json!(entity_id);
                task
            })
        }
    }

    fn clear(&self) {
        self.states.clear();
    }
}
