use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, List, ListItem, ListState};

use crate::domain::repository::TaskRepository;
use crate::domain::task::TaskStatus;
use super::super::app::{App, KanbanColumn};

pub fn render<R: TaskRepository>(f: &mut Frame, area: Rect, app: &App<R>) {
    let columns = Layout::horizontal([
        Constraint::Percentage(33),
        Constraint::Percentage(34),
        Constraint::Percentage(33),
    ])
    .split(area);

    render_column(f, columns[0], app, TaskStatus::Open, KanbanColumn::Open, "Open");
    render_column(f, columns[1], app, TaskStatus::InProgress, KanbanColumn::InProgress, "In Progress");
    render_column(f, columns[2], app, TaskStatus::Done, KanbanColumn::Done, "Done");
}

fn render_column<R: TaskRepository>(
    f: &mut Frame,
    area: Rect,
    app: &App<R>,
    status: TaskStatus,
    column: KanbanColumn,
    title: &str,
) {
    let is_active = app.kanban_column == column;
    let border_color = if is_active { Color::Cyan } else { Color::DarkGray };

    let tasks = app.tasks_by_status(status);
    let count = tasks.len();

    let items: Vec<ListItem> = tasks
        .iter()
        .map(|task| {
            let title_text = if task.title.len() > 25 {
                format!("{}...", &task.title[..22])
            } else {
                task.title.clone()
            };

            let claimed = task
                .claimed_by
                .as_deref()
                .map(|c| format!(" @{c}"))
                .unwrap_or_default();

            ListItem::new(vec![
                Line::styled(
                    format!("{} [{}]", task.id, task.priority),
                    Style::default().fg(Color::Yellow),
                ),
                Line::raw(format!("  {title_text}{claimed}")),
            ])
        })
        .collect();

    let block = Block::default()
        .title(format!(" {title} ({count}) "))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border_color));

    let list = List::new(items)
        .block(block)
        .highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("> ");

    let idx = app.kanban_indices[column as usize];
    let mut state = ListState::default();
    if is_active && count > 0 {
        state.select(Some(idx.min(count.saturating_sub(1))));
    }

    f.render_stateful_widget(list, area, &mut state);
}
