use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
};

use super::super::app::App;
use crate::{
    domain::{repository::TaskRepository, task::TaskStatus},
    presentation::shared::TaskTree,
};

pub fn render<R: TaskRepository>(f: &mut Frame, area: Rect, app: &App<R>) {
    let block = Block::default()
        .title(" Dependency Graph ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let mut lines: Vec<Line> = Vec::new();
    let tree = TaskTree::build(&app.tasks);

    for group in &tree.epics {
        lines.push(task_line(group.epic, 0));
        for child in &group.children {
            lines.push(task_line(child, 1));
            for blocker_id in &child.blocked_by {
                lines.push(Line::from(vec![
                    Span::raw("        "),
                    Span::styled(
                        format!("blocked by {blocker_id}"),
                        Style::default().fg(Color::Red),
                    ),
                ]));
            }
        }
        lines.push(Line::from(""));
    }

    if !tree.standalone.is_empty() && !tree.epics.is_empty() {
        lines.push(Line::styled(
            "Standalone Tasks",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ));
    }

    for task in &tree.standalone {
        lines.push(task_line(task, 0));
        for blocker_id in &task.blocked_by {
            lines.push(Line::from(vec![
                Span::raw("    "),
                Span::styled(
                    format!("blocked by {blocker_id}"),
                    Style::default().fg(Color::Red),
                ),
            ]));
        }
    }

    if lines.is_empty() {
        lines.push(Line::styled(
            "No tasks",
            Style::default().fg(Color::DarkGray),
        ));
    }

    let paragraph = Paragraph::new(lines)
        .block(block)
        .wrap(Wrap { trim: false });
    f.render_widget(paragraph, area);
}

fn task_line(task: &crate::domain::task::Task, depth: usize) -> Line<'_> {
    let indent = "    ".repeat(depth);
    let prefix = if depth > 0 { "├── " } else { "" };

    let status_style = crate::presentation::shared::status_style(task.status);

    let icon = if task.task_type == crate::domain::task::TaskType::Epic {
        "◆ "
    } else if !task.blocked_by.is_empty() && task.status != TaskStatus::Done {
        "◌ "
    } else {
        "● "
    };

    let claimed = task
        .claimed_by
        .as_deref()
        .map(|c| format!(" @{c}"))
        .unwrap_or_default();

    Line::from(vec![
        Span::raw(format!("{indent}{prefix}")),
        Span::styled(icon, status_style),
        Span::styled(&task.id, Style::default().fg(Color::Yellow)),
        Span::raw(" "),
        Span::styled(&task.title, status_style),
        Span::styled(
            format!(" [{}, {}]", task.priority, task.status),
            Style::default().fg(Color::DarkGray),
        ),
        Span::styled(claimed, Style::default().fg(Color::Cyan)),
    ])
}
