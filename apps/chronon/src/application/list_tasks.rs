use crate::domain::{error::ChronError, repository::TaskRepository, task::Task};

pub fn list_tasks(
    repo: &impl TaskRepository,
    status: Option<&str>,
) -> Result<Vec<Task>, ChronError> {
    repo.list_tasks(status)
}

pub fn ready_tasks(repo: &impl TaskRepository) -> Result<Vec<Task>, ChronError> {
    repo.ready_tasks()
}
