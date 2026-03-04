use thiserror::Error;

#[derive(Debug, Error)]
pub enum ChronError {
    #[error("task {0} not found")]
    TaskNotFound(String),

    #[error("task {id} is {current}, can only {action} open tasks")]
    InvalidTransition {
        id: String,
        current: String,
        action: String,
    },

    #[error("task {0} is already done")]
    AlreadyDone(String),

    #[error("no .chronon/ found (searched from {0:?}). Run `cn init` first.")]
    NoWorkspace(std::path::PathBuf),

    #[error(".chronon/ already exists in {0}")]
    WorkspaceExists(String),

    #[error("unknown status: {0}")]
    InvalidStatus(String),

    #[error("unknown priority: {0} (expected p0-p3)")]
    InvalidPriority(String),

    #[error("unknown task type: {0} (expected task, epic, bug, feature)")]
    InvalidTaskType(String),

    #[error(transparent)]
    Core(#[from] CoreError),

    #[error(transparent)]
    Io(#[from] std::io::Error),
}

/// Wrapper around allsource_core errors (which are not `std::error::Error`).
#[derive(Debug, Error)]
#[error("{0}")]
pub struct CoreError(pub String);
