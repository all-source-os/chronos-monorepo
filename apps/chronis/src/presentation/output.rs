use tabled::{Table, Tabled, settings::Style};

use crate::domain::task::Task;

#[derive(Tabled)]
pub struct TaskRow {
    #[tabled(rename = "ID")]
    pub id: String,
    #[tabled(rename = "Type")]
    pub task_type: String,
    #[tabled(rename = "Title")]
    pub title: String,
    #[tabled(rename = "Pri")]
    pub priority: String,
    #[tabled(rename = "Status")]
    pub status: String,
    #[tabled(rename = "Claimed")]
    pub claimed: String,
    #[tabled(rename = "Blocked")]
    pub blocked: String,
}

impl From<&Task> for TaskRow {
    fn from(task: &Task) -> Self {
        let title = if task.title.len() > 40 {
            format!("{}...", &task.title[..37])
        } else {
            task.title.clone()
        };

        let blocked = if task.blocked_by.is_empty() {
            "-".to_string()
        } else {
            task.blocked_by.len().to_string()
        };

        Self {
            id: task.id.clone(),
            task_type: task.task_type.to_string(),
            title,
            priority: task.priority.to_string(),
            status: task.status.to_string(),
            claimed: task.claimed_by.clone().unwrap_or_else(|| "-".to_string()),
            blocked,
        }
    }
}

pub fn print_task_table(tasks: &[Task]) {
    if tasks.is_empty() {
        println!("No tasks found.");
        return;
    }
    let rows: Vec<TaskRow> = tasks.iter().map(TaskRow::from).collect();
    let mut table = Table::new(rows);
    table.with(Style::rounded());
    println!("{table}");
}
