use crate::domain::{error::ChronError, repository::TaskRepository};

pub async fn archive_task(repo: &impl TaskRepository, id: &str) -> Result<(), ChronError> {
    repo.archive_task(id).await
}

pub async fn unarchive_task(repo: &impl TaskRepository, id: &str) -> Result<(), ChronError> {
    repo.unarchive_task(id).await
}
