//! Prime integration for semantic task memory.
//!
//! When the `prime` feature is enabled, tasks are indexed as graph nodes
//! with relationships, enabling:
//! - `cn related <id>` — find tasks connected by graph proximity
//! - `cn context` — compressed index of all tasks for agent injection
//! - `cn find "query"` — semantic search (requires `prime-full`)

use std::collections::HashMap;
use std::sync::Arc;

use allsource_core::prime::Prime;
use std::sync::RwLock;
use serde_json::json;

use crate::domain::{error::ChronError, task::Task};

/// Wraps Prime for task-specific graph operations.
///
/// Maintains a mapping from chronis task IDs (e.g. `t-abc`) to
/// Prime entity IDs (e.g. `node:task:<uuid>`).
pub struct TaskMemory {
    prime: Arc<Prime>,
    /// chronis task_id → prime entity_id
    id_map: RwLock<HashMap<String, String>>,
}

impl TaskMemory {
    pub fn new(prime: Arc<Prime>) -> Self {
        Self {
            prime,
            id_map: RwLock::new(HashMap::new()),
        }
    }

    pub fn prime(&self) -> &Prime {
        &self.prime
    }

    /// Get the Prime entity_id for a chronis task ID.
    pub fn entity_id(&self, task_id: &str) -> Option<String> {
        self.id_map.read().unwrap().get(task_id).cloned()
    }

    /// Index a task as a Prime graph node.
    pub async fn index_task(&self, task: &Task) -> Result<(), ChronError> {
        // Skip if already indexed
        if self.id_map.read().unwrap().contains_key(&task.id) {
            return Ok(());
        }

        let props = json!({
            "name": task.title,
            "domain": "chronis",
            "status": format!("{}", task.status),
            "priority": format!("{}", task.priority),
            "task_type": format!("{}", task.task_type),
            "chronis_id": task.id,
        });

        let node_id = self
            .prime
            .add_node("task", props)
            .await
            .map_err(|e| ChronError::Prime(e.to_string()))?;

        #[allow(deprecated)]
        let entity_id = allsource_core::prime::node_entity_id("task", node_id.as_str());
        self.id_map.write().unwrap().insert(task.id.clone(), entity_id);

        Ok(())
    }

    /// Index dependency and parent relationships for a task.
    /// Call after all tasks are indexed so targets exist in the graph.
    pub async fn index_relationships(&self, task: &Task) -> Result<(), ChronError> {
        let Some(source) = self.entity_id(&task.id) else {
            return Ok(());
        };

        // Parent → child_of edge
        if let Some(ref parent_id) = task.parent {
            if let Some(target) = self.entity_id(parent_id) {
                let _ = self.prime.add_edge(&source, &target, "child_of", None).await;
            }
        }

        // Dependencies → depends_on edges
        for dep_id in &task.blocked_by {
            if let Some(target) = self.entity_id(dep_id) {
                let _ = self
                    .prime
                    .add_edge(&source, &target, "depends_on", None)
                    .await;
            }
        }

        Ok(())
    }

    /// Bulk-index all tasks into the Prime graph (nodes first, then edges).
    pub async fn index_all(&self, tasks: &[Task]) -> Result<usize, ChronError> {
        // Phase 1: create all nodes
        for task in tasks {
            self.index_task(task).await?;
        }

        // Phase 2: create edges (all nodes exist now)
        for task in tasks {
            self.index_relationships(task).await?;
        }

        Ok(tasks.len())
    }

    /// Find tasks related to the given task via graph traversal.
    pub fn related(&self, task_id: &str) -> Vec<String> {
        let Some(entity_id) = self.entity_id(task_id) else {
            return vec![];
        };

        self.prime
            .neighbors(&entity_id, None, allsource_core::prime::Direction::Both)
            .iter()
            .filter_map(|n| {
                // Map back from Prime node to chronis task ID
                n.properties
                    .get("chronis_id")
                    .and_then(|v| v.as_str())
                    .map(String::from)
            })
            .collect()
    }

    /// Get graph stats for tasks.
    pub fn stats(&self) -> (usize, usize) {
        let s = self.prime.stats();
        (s.total_nodes, s.total_edges)
    }
}

/// Semantic search (requires `prime-full` feature).
#[cfg(feature = "prime-full")]
impl TaskMemory {
    /// Embed a task's title + description for semantic search.
    pub async fn embed_task(
        &self,
        task: &Task,
        vector: Vec<f32>,
    ) -> Result<(), ChronError> {
        let Some(entity_id) = self.entity_id(&task.id) else {
            return Err(ChronError::Prime("task not indexed yet".into()));
        };

        let text = if let Some(ref desc) = task.description {
            format!("{}: {}", task.title, desc)
        } else {
            task.title.clone()
        };

        self.prime
            .embed(&entity_id, Some(&text), vector)
            .await
            .map_err(|e| ChronError::Prime(e.to_string()))
    }

    /// Semantic search for tasks matching a query vector.
    pub fn search(&self, query_vector: &[f32], top_k: usize) -> Vec<SearchResult> {
        self.prime
            .vector_search(query_vector, top_k)
            .into_iter()
            .map(|r| SearchResult {
                task_id: r
                    .id
                    .strip_prefix("vec:node:task:")
                    .unwrap_or(&r.id)
                    .to_string(),
                score: r.score,
                text: r.text,
            })
            .collect()
    }
}

/// Result from a semantic task search.
#[cfg(feature = "prime-full")]
#[derive(Debug, Clone)]
pub struct SearchResult {
    pub task_id: String,
    pub score: f64,
    pub text: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::task::{Priority, TaskStatus, TaskType};

    fn test_task(id: &str, title: &str, blocked_by: Vec<String>) -> Task {
        Task {
            id: id.into(),
            title: title.into(),
            status: TaskStatus::Open,
            priority: Priority::P1,
            task_type: TaskType::Task,
            parent: None,
            blocked_by,
            description: None,
            claimed_by: None,
            created_at: Some(chrono::Utc::now()),
            done_reason: None,
            done_at: None,
            awaiting_approval: None,
            approved: None,
            approved_at: None,
            archived: false,
        }
    }

    #[tokio::test]
    async fn test_index_and_find_related() {
        let prime = Arc::new(Prime::open_in_memory().await.unwrap());
        let memory = TaskMemory::new(prime);

        let task_a = test_task("t-a", "Build auth", vec![]);
        let task_b = test_task("t-b", "Build API", vec!["t-a".into()]);

        // Index all tasks (nodes + edges)
        memory.index_all(&[task_a, task_b]).await.unwrap();

        // Task B depends_on Task A — they should be related
        let related = memory.related("t-b");
        assert!(
            related.contains(&"t-a".to_string()),
            "t-b should be related to t-a via depends_on, got: {related:?}"
        );

        let (nodes, edges) = memory.stats();
        assert_eq!(nodes, 2);
        assert!(edges >= 1, "expected at least 1 edge, got {edges}");
    }

    #[tokio::test]
    async fn test_index_idempotent() {
        let prime = Arc::new(Prime::open_in_memory().await.unwrap());
        let memory = TaskMemory::new(prime);

        let task = test_task("t-idem", "Test", vec![]);

        memory.index_task(&task).await.unwrap();
        memory.index_task(&task).await.unwrap();

        let (nodes, _) = memory.stats();
        assert_eq!(nodes, 1, "duplicate index should not create second node");
    }

    #[tokio::test]
    async fn test_parent_child_relationship() {
        let prime = Arc::new(Prime::open_in_memory().await.unwrap());
        let memory = TaskMemory::new(prime);

        let mut child = test_task("t-child", "Subtask", vec![]);
        child.parent = Some("t-parent".into());
        let parent = test_task("t-parent", "Epic", vec![]);

        memory.index_all(&[parent, child]).await.unwrap();

        let related = memory.related("t-child");
        assert!(
            related.contains(&"t-parent".to_string()),
            "child should be related to parent via child_of"
        );
    }
}
