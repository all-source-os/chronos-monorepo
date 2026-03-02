use crate::domain::error::ChronError;
use crate::domain::repository::TaskRepository;

pub async fn claim_task(
    repo: &impl TaskRepository,
    id: &str,
    agent_id: &str,
) -> Result<(), ChronError> {
    repo.claim_task(id, agent_id).await
}
