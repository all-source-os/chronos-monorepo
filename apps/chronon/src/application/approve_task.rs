use crate::domain::error::ChronError;
use crate::domain::repository::TaskRepository;

pub async fn approve_task(repo: &impl TaskRepository, id: &str) -> Result<(), ChronError> {
    repo.approve_task(id).await
}
