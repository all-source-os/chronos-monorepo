use chrono::{DateTime, Utc};
use ratatui::style::{Color, Style};

use crate::domain::task::{Task, TaskStatus, TaskType};

/// Format a UTC timestamp for display.
pub fn fmt_timestamp(dt: &DateTime<Utc>) -> String {
    dt.format("%Y-%m-%d %H:%M:%S").to_string()
}

/// Canonical status → color mapping shared by all presentation layers.
pub fn status_color(status: TaskStatus) -> Color {
    match status {
        TaskStatus::Open => Color::Green,
        TaskStatus::InProgress => Color::Yellow,
        TaskStatus::Done => Color::DarkGray,
    }
}

/// Convenience: returns a `Style` with the status foreground color.
pub fn status_style(status: TaskStatus) -> Style {
    Style::default().fg(status_color(status))
}

/// CSS class name for a task status (used by web handlers).
pub fn status_css_class(status: TaskStatus) -> &'static str {
    match status {
        TaskStatus::Open => "status-open",
        TaskStatus::InProgress => "status-progress",
        TaskStatus::Done => "status-done",
    }
}

/// Generate markdown export for a list of tasks.
pub fn export_markdown(tasks: &[Task]) -> String {
    let mut md = String::from("# Chronis Task Export\n\n");

    for task in tasks {
        md.push_str(&format!("## {} — {}\n\n", task.id, task.title));
        md.push_str("| Field | Value |\n|-------|-------|\n");
        md.push_str(&format!("| Type | {} |\n", task.task_type));
        md.push_str(&format!("| Priority | {} |\n", task.priority));
        md.push_str(&format!("| Status | {} |\n", task.status));
        if let Some(ref parent) = task.parent {
            md.push_str(&format!("| Parent | {parent} |\n"));
        }
        if let Some(ref claimed) = task.claimed_by {
            md.push_str(&format!("| Claimed | {claimed} |\n"));
        }
        if !task.blocked_by.is_empty() {
            md.push_str(&format!(
                "| Blocked by | {} |\n",
                task.blocked_by.join(", ")
            ));
        }
        if let Some(ref created) = task.created_at {
            md.push_str(&format!("| Created | {} |\n", fmt_timestamp(created)));
        }
        md.push('\n');
        if let Some(ref desc) = task.description {
            md.push_str(desc);
            md.push_str("\n\n");
        }
        md.push_str("---\n\n");
    }

    md
}

/// A grouped view of tasks: epics with children, then standalone tasks.
pub struct TaskTree<'a> {
    pub epics: Vec<EpicGroup<'a>>,
    pub standalone: Vec<&'a Task>,
}

pub struct EpicGroup<'a> {
    pub epic: &'a Task,
    pub children: Vec<&'a Task>,
}

impl<'a> TaskTree<'a> {
    pub fn build(tasks: &'a [Task]) -> Self {
        let epics: Vec<&Task> = tasks
            .iter()
            .filter(|t| t.task_type == TaskType::Epic)
            .collect();

        let standalone: Vec<&Task> = tasks
            .iter()
            .filter(|t| t.task_type != TaskType::Epic && t.parent.is_none())
            .collect();

        let epic_groups = epics
            .into_iter()
            .map(|epic| {
                let children = tasks
                    .iter()
                    .filter(|t| t.parent.as_deref() == Some(&epic.id))
                    .collect();
                EpicGroup { epic, children }
            })
            .collect();

        Self {
            epics: epic_groups,
            standalone,
        }
    }
}
