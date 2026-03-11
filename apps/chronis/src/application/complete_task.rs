use crate::domain::{error::ChronError, repository::TaskRepository};

#[cfg_attr(feature = "hotpath", hotpath::measure)]
pub async fn complete_task(
    repo: &impl TaskRepository,
    id: &str,
    reason: Option<&str>,
) -> Result<(), ChronError> {
    repo.complete_task(id, reason).await
}
