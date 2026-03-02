use crate::domain::error::ChronError;
use crate::domain::repository::{TaskDetail, TaskRepository};

pub async fn get_task(repo: &impl TaskRepository, id: &str) -> Result<TaskDetail, ChronError> {
    repo.get_task_detail(id).await
}
