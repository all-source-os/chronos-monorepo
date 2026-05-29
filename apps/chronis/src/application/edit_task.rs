use crate::domain::{
    error::ChronError,
    repository::{TaskEdit, TaskRepository},
};

/// Edit an existing task. Rejects an empty edit (`ChronError::NothingToEdit`)
/// so we never emit a no-op `task.updated` event, then delegates to the
/// repository which verifies the task exists before ingesting.
#[cfg_attr(feature = "hotpath", hotpath::measure)]
pub async fn edit_task(
    repo: &impl TaskRepository,
    id: &str,
    edit: TaskEdit,
) -> Result<(), ChronError> {
    if edit.is_empty() {
        return Err(ChronError::NothingToEdit);
    }
    repo.edit_task(id, &edit).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_edit_is_rejected() {
        let edit = TaskEdit::default();
        assert!(edit.is_empty());
    }

    #[test]
    fn any_field_makes_it_non_empty() {
        let edit = TaskEdit {
            title: Some("new".into()),
            ..Default::default()
        };
        assert!(!edit.is_empty());
    }
}
