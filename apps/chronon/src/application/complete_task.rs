use crate::domain::error::ChronError;
use crate::domain::repository::TaskRepository;

pub async fn complete_task(
    repo: &impl TaskRepository,
    id: &str,
    reason: Option<&str>,
) -> Result<(), ChronError> {
    repo.complete_task(id, reason).await
}
