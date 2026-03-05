use crate::domain::{error::ChronError, repository::TaskRepository};

pub async fn add_dependency(
    repo: &impl TaskRepository,
    task_id: &str,
    blocker_id: &str,
) -> Result<(), ChronError> {
    repo.add_dependency(task_id, blocker_id).await
}
