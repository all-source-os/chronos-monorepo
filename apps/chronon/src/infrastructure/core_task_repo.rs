use std::collections::HashSet;
use std::sync::Arc;

use allsource_core::embedded::{EmbeddedCore, IngestEvent, Query};
use serde_json::json;

use crate::domain::error::{ChronError, CoreError};
use crate::domain::repository::{TaskDetail, TaskRepository, TimelineEntry};
use crate::domain::task::{Task, TaskStatus};

pub struct CoreTaskRepository {
    core: Arc<EmbeddedCore>,
}

impl CoreTaskRepository {
    pub fn new(core: Arc<EmbeddedCore>) -> Self {
        Self { core }
    }

    fn value_to_task(&self, value: &serde_json::Value) -> Result<Task, ChronError> {
        serde_json::from_value::<Task>(value.clone())
            .map_err(|e| CoreError(format!("failed to deserialize task: {e}")).into())
    }

    fn all_tasks_raw(&self) -> Vec<serde_json::Value> {
        let all = self
            .core
            .projection("chronon_tasks", "__all")
            .unwrap_or_else(|| json!({ "tasks": [] }));
        all["tasks"].as_array().cloned().unwrap_or_default()
    }
}

impl TaskRepository for CoreTaskRepository {
    fn get_task(&self, id: &str) -> Result<Task, ChronError> {
        let value = self
            .core
            .projection("chronon_tasks", id)
            .ok_or_else(|| ChronError::TaskNotFound(id.to_string()))?;
        self.value_to_task(&value)
    }

    fn list_tasks(&self, status: Option<&str>) -> Result<Vec<Task>, ChronError> {
        let raw = self.all_tasks_raw();
        let filtered: Vec<_> = if let Some(status) = status {
            raw.into_iter()
                .filter(|t| t["status"].as_str().unwrap_or("") == status)
                .collect()
        } else {
            raw
        };
        filtered.iter().map(|v| self.value_to_task(v)).collect()
    }

    fn ready_tasks(&self) -> Result<Vec<Task>, ChronError> {
        let raw = self.all_tasks_raw();

        let done_ids: HashSet<String> = raw
            .iter()
            .filter(|t| t["status"].as_str() == Some("done"))
            .filter_map(|t| t["id"].as_str().map(String::from))
            .collect();

        let ready: Vec<_> = raw
            .into_iter()
            .filter(|t| t["status"].as_str() == Some("open"))
            .filter(|t| {
                let blocked_by = t["blocked_by"].as_array();
                match blocked_by {
                    None => true,
                    Some(deps) => deps
                        .iter()
                        .all(|d| d.as_str().is_some_and(|id| done_ids.contains(id))),
                }
            })
            .collect();

        ready.iter().map(|v| self.value_to_task(v)).collect()
    }

    async fn create_task(
        &self,
        id: &str,
        title: &str,
        priority: &str,
        blocked_by: &[String],
    ) -> Result<(), ChronError> {
        self.core
            .ingest(IngestEvent {
                entity_id: id,
                event_type: "task.created",
                payload: json!({
                    "title": title,
                    "priority": priority,
                }),
                metadata: None,
                tenant_id: None,
            })
            .await
            .map_err(|e| CoreError(e.to_string()))?;

        for dep in blocked_by {
            self.core
                .ingest(IngestEvent {
                    entity_id: id,
                    event_type: "task.dependency.added",
                    payload: json!({ "depends_on": dep }),
                    metadata: None,
                    tenant_id: None,
                })
                .await
                .map_err(|e| CoreError(e.to_string()))?;
        }

        Ok(())
    }

    async fn claim_task(&self, id: &str, agent_id: &str) -> Result<(), ChronError> {
        // Validate state
        let task = self.get_task(id)?;
        if task.status != TaskStatus::Open {
            return Err(ChronError::InvalidTransition {
                id: id.to_string(),
                current: task.status.to_string(),
                action: "claim".to_string(),
            });
        }

        self.core
            .ingest(IngestEvent {
                entity_id: id,
                event_type: "workflow.claimed",
                payload: json!({ "agent_id": agent_id }),
                metadata: None,
                tenant_id: None,
            })
            .await
            .map_err(|e| CoreError(e.to_string()))?;

        Ok(())
    }

    async fn complete_task(&self, id: &str, reason: Option<&str>) -> Result<(), ChronError> {
        let task = self.get_task(id)?;
        if task.status == TaskStatus::Done {
            return Err(ChronError::AlreadyDone(id.to_string()));
        }

        let mut payload = json!({});
        if let Some(reason) = reason {
            payload["reason"] = json!(reason);
        }

        self.core
            .ingest(IngestEvent {
                entity_id: id,
                event_type: "workflow.step.completed",
                payload,
                metadata: None,
                tenant_id: None,
            })
            .await
            .map_err(|e| CoreError(e.to_string()))?;

        Ok(())
    }

    async fn approve_task(&self, id: &str) -> Result<(), ChronError> {
        let task = self.get_task(id)?;
        if task.status == TaskStatus::Done {
            return Err(ChronError::AlreadyDone(id.to_string()));
        }

        self.core
            .ingest(IngestEvent {
                entity_id: id,
                event_type: "workflow.approval.granted",
                payload: json!({}),
                metadata: None,
                tenant_id: None,
            })
            .await
            .map_err(|e| CoreError(e.to_string()))?;

        Ok(())
    }

    async fn get_task_detail(&self, id: &str) -> Result<TaskDetail, ChronError> {
        let task = self.get_task(id)?;

        let events = self
            .core
            .query(Query::new().entity_id(id))
            .await
            .map_err(|e| CoreError(e.to_string()))?;

        let timeline = events
            .iter()
            .map(|ev| TimelineEntry {
                timestamp: ev.timestamp.format("%Y-%m-%d %H:%M:%S").to_string(),
                event_type: ev.event_type.clone(),
            })
            .collect();

        Ok(TaskDetail { task, timeline })
    }
}
