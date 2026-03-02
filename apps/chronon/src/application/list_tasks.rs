use crate::domain::error::ChronError;
use crate::domain::repository::TaskRepository;
use crate::domain::task::Task;

pub fn list_tasks(
    repo: &impl TaskRepository,
    status: Option<&str>,
) -> Result<Vec<Task>, ChronError> {
    repo.list_tasks(status)
}

pub fn ready_tasks(repo: &impl TaskRepository) -> Result<Vec<Task>, ChronError> {
    repo.ready_tasks()
}
