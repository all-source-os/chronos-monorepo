use crate::domain::error::ChronError;
use crate::domain::repository::TaskRepository;
use crate::infrastructure::id::generate_task_id;

pub struct CreateTaskInput<'a> {
    pub title: &'a str,
    pub priority: &'a str,
    pub blocked_by: &'a [String],
}

pub struct CreateTaskOutput {
    pub id: String,
}

pub async fn create_task(
    repo: &impl TaskRepository,
    input: CreateTaskInput<'_>,
) -> Result<CreateTaskOutput, ChronError> {
    let id = generate_task_id();
    repo.create_task(&id, input.title, input.priority, input.blocked_by)
        .await?;
    Ok(CreateTaskOutput { id })
}
