use crate::domain::{
    repository::{TaskDetail, TaskRepository},
    task::{Task, TaskStatus},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum View {
    Dashboard,
    Kanban,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KanbanColumn {
    Open,
    InProgress,
    Done,
}

impl KanbanColumn {
    pub fn next(self) -> Self {
        match self {
            Self::Open => Self::InProgress,
            Self::InProgress => Self::Done,
            Self::Done => Self::Done,
        }
    }

    pub fn prev(self) -> Self {
        match self {
            Self::Open => Self::Open,
            Self::InProgress => Self::Open,
            Self::Done => Self::InProgress,
        }
    }
}

pub struct App<R: TaskRepository> {
    pub repo: R,
    pub tasks: Vec<Task>,
    pub selected_index: usize,
    pub view: View,
    pub kanban_column: KanbanColumn,
    pub kanban_indices: [usize; 3], // per-column selection
    pub detail: Option<TaskDetail>,
    pub status_message: Option<String>,
    pub should_quit: bool,
}

impl<R: TaskRepository> App<R> {
    pub fn new(repo: R) -> Self {
        Self {
            repo,
            tasks: Vec::new(),
            selected_index: 0,
            view: View::Dashboard,
            kanban_column: KanbanColumn::Open,
            kanban_indices: [0; 3],
            detail: None,
            status_message: None,
            should_quit: false,
        }
    }

    pub fn refresh(&mut self) {
        match self.repo.list_tasks(None) {
            Ok(tasks) => {
                self.tasks = tasks;
                // Clamp selection
                if !self.tasks.is_empty() && self.selected_index >= self.tasks.len() {
                    self.selected_index = self.tasks.len() - 1;
                }
            }
            Err(e) => {
                self.status_message = Some(format!("Error: {e}"));
            }
        }
    }

    pub fn selected_task(&self) -> Option<&Task> {
        self.tasks.get(self.selected_index)
    }

    pub fn tasks_by_status(&self, status: TaskStatus) -> Vec<&Task> {
        self.tasks.iter().filter(|t| t.status == status).collect()
    }

    pub fn select_next(&mut self) {
        if !self.tasks.is_empty() {
            self.selected_index = (self.selected_index + 1).min(self.tasks.len() - 1);
        }
    }

    pub fn select_prev(&mut self) {
        self.selected_index = self.selected_index.saturating_sub(1);
    }

    pub fn toggle_view(&mut self) {
        self.view = match self.view {
            View::Dashboard => View::Kanban,
            View::Kanban => View::Dashboard,
        };
        self.detail = None;
    }

    /// Get the currently focused task (works for both views)
    pub fn focused_task_id(&self) -> Option<String> {
        match self.view {
            View::Dashboard => self.selected_task().map(|t| t.id.clone()),
            View::Kanban => {
                let status = match self.kanban_column {
                    KanbanColumn::Open => TaskStatus::Open,
                    KanbanColumn::InProgress => TaskStatus::InProgress,
                    KanbanColumn::Done => TaskStatus::Done,
                };
                let col_tasks = self.tasks_by_status(status);
                let idx = self.kanban_indices[self.kanban_column as usize];
                col_tasks.get(idx).map(|t| t.id.clone())
            }
        }
    }

    pub fn kanban_select_next(&mut self) {
        let status = match self.kanban_column {
            KanbanColumn::Open => TaskStatus::Open,
            KanbanColumn::InProgress => TaskStatus::InProgress,
            KanbanColumn::Done => TaskStatus::Done,
        };
        let count = self.tasks_by_status(status).len();
        let idx = &mut self.kanban_indices[self.kanban_column as usize];
        if count > 0 {
            *idx = (*idx + 1).min(count - 1);
        }
    }

    pub fn kanban_select_prev(&mut self) {
        let idx = &mut self.kanban_indices[self.kanban_column as usize];
        *idx = idx.saturating_sub(1);
    }

    pub async fn load_detail(&mut self) {
        if let Some(id) = self.focused_task_id() {
            match self.repo.get_task_detail(&id).await {
                Ok(detail) => self.detail = Some(detail),
                Err(e) => self.status_message = Some(format!("Error: {e}")),
            }
        }
    }

    pub async fn claim_focused(&mut self) {
        if let Some(id) = self.focused_task_id() {
            let agent = std::env::var("CN_AGENT_ID").unwrap_or_else(|_| "human".to_string());
            match self.repo.claim_task(&id, &agent).await {
                Ok(()) => {
                    self.status_message = Some(format!("Claimed {id}"));
                    self.refresh();
                }
                Err(e) => self.status_message = Some(format!("Error: {e}")),
            }
        }
    }

    pub async fn complete_focused(&mut self) {
        if let Some(id) = self.focused_task_id() {
            match self.repo.complete_task(&id, None).await {
                Ok(()) => {
                    self.status_message = Some(format!("Completed {id}"));
                    self.refresh();
                }
                Err(e) => self.status_message = Some(format!("Error: {e}")),
            }
        }
    }

    pub async fn approve_focused(&mut self) {
        if let Some(id) = self.focused_task_id() {
            match self.repo.approve_task(&id).await {
                Ok(()) => {
                    self.status_message = Some(format!("Approved {id}"));
                    self.refresh();
                }
                Err(e) => self.status_message = Some(format!("Error: {e}")),
            }
        }
    }
}
