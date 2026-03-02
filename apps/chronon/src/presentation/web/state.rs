use std::sync::Arc;
use crate::infrastructure::core_task_repo::CoreTaskRepository;

#[derive(Clone)]
pub struct AppState {
    pub repo: Arc<CoreTaskRepository>,
}
