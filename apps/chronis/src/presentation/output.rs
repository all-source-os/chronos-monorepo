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
            format!("{}...", &task.title[..task.title.floor_char_boundary(37)])
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

/// Render tasks as a JSON array (`--format json`) for downstream `jq`/tooling.
pub fn print_task_json(tasks: &[Task]) {
    // Tasks derive Serialize; pretty-print so the output is diff/readable.
    match serde_json::to_string_pretty(tasks) {
        Ok(s) => println!("{s}"),
        Err(e) => eprintln!("error: failed to serialize tasks to JSON: {e}"),
    }
}

/// Render tasks as tab-separated values with a header row (`--format tsv`).
pub fn print_task_tsv(tasks: &[Task]) {
    println!("id\ttype\ttitle\tpriority\tstatus\tclaimed_by\tblocked_by\tparent\tarchived");
    for t in tasks {
        // Tabs/newlines in a title would break the row; replace with spaces.
        let title = t.title.replace(['\t', '\n'], " ");
        println!(
            "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
            t.id,
            t.task_type,
            title,
            t.priority,
            t.status,
            t.claimed_by.as_deref().unwrap_or(""),
            t.blocked_by.join(","),
            t.parent.as_deref().unwrap_or(""),
            t.archived,
        );
    }
}
