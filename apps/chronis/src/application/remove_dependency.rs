use crate::domain::{error::ChronError, repository::TaskRepository};

pub async fn remove_dependency(
    repo: &impl TaskRepository,
    task_id: &str,
    blocker_id: &str,
) -> Result<(), ChronError> {
    repo.remove_dependency(task_id, blocker_id).await
}
