use super::{error::ChronError, task::Task};

/// Timeline entry for a task event.
pub struct TimelineEntry {
    pub timestamp: String,
    pub event_type: String,
}

/// Task with its event timeline.
pub struct TaskDetail {
    pub task: Task,
    pub timeline: Vec<TimelineEntry>,
}

pub trait TaskRepository: Send + Sync {
    fn get_task(&self, id: &str) -> Result<Task, ChronError>;
    fn list_tasks(&self, status: Option<&str>) -> Result<Vec<Task>, ChronError>;
    fn ready_tasks(&self) -> Result<Vec<Task>, ChronError>;

    fn create_task(
        &self,
        id: &str,
        title: &str,
        priority: &str,
        blocked_by: &[String],
    ) -> impl std::future::Future<Output = Result<(), ChronError>> + Send;

    fn claim_task(
        &self,
        id: &str,
        agent_id: &str,
    ) -> impl std::future::Future<Output = Result<(), ChronError>> + Send;

    fn complete_task(
        &self,
        id: &str,
        reason: Option<&str>,
    ) -> impl std::future::Future<Output = Result<(), ChronError>> + Send;

    fn approve_task(
        &self,
        id: &str,
    ) -> impl std::future::Future<Output = Result<(), ChronError>> + Send;

    fn get_task_detail(
        &self,
        id: &str,
    ) -> impl std::future::Future<Output = Result<TaskDetail, ChronError>> + Send;
}
