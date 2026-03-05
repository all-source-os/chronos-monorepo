use crate::{
    domain::{error::ChronError, repository::TaskRepository, task::TaskType},
    infrastructure::id::generate_task_id,
};

pub struct CreateTaskInput<'a> {
    pub title: &'a str,
    pub priority: &'a str,
    pub blocked_by: &'a [String],
    pub task_type: TaskType,
    pub parent: Option<&'a str>,
    pub description: Option<&'a str>,
}

pub struct CreateTaskOutput {
    pub id: String,
}

pub async fn create_task(
    repo: &impl TaskRepository,
    input: CreateTaskInput<'_>,
) -> Result<CreateTaskOutput, ChronError> {
    let id = generate_task_id();
    repo.create_task(
        &id,
        input.title,
        input.priority,
        input.blocked_by,
        input.task_type,
        input.parent,
        input.description,
    )
    .await?;
    Ok(CreateTaskOutput { id })
}
