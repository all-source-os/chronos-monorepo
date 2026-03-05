use crate::infrastructure::core_task_repo::CoreTaskRepository;
use std::sync::Arc;

#[derive(Clone)]
pub struct AppState {
    pub repo: Arc<CoreTaskRepository>,
}
