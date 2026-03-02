use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Cell, Paragraph, Row, Table, TableState};

use crate::domain::repository::TaskRepository;
use super::super::app::App;

pub fn render<R: TaskRepository>(f: &mut Frame, area: Rect, app: &App<R>) {
    let chunks = Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    render_task_table(f, chunks[0], app);
    render_detail(f, chunks[1], app);
}

fn render_task_table<R: TaskRepository>(f: &mut Frame, area: Rect, app: &App<R>) {
    let header = Row::new(vec![
        Cell::from("ID"),
        Cell::from("Title"),
        Cell::from("Pri"),
        Cell::from("Status"),
        Cell::from("Claimed"),
    ])
    .style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD));

    let rows: Vec<Row> = app
        .tasks
        .iter()
        .enumerate()
        .map(|(i, task)| {
            let style = if i == app.selected_index {
                Style::default().bg(Color::DarkGray).fg(Color::White)
            } else {
                status_style(task.status)
            };

            let title = if task.title.len() > 30 {
                format!("{}...", &task.title[..27])
            } else {
                task.title.clone()
            };

            Row::new(vec![
                Cell::from(task.id.clone()),
                Cell::from(title),
                Cell::from(task.priority.to_string()),
                Cell::from(task.status.to_string()),
                Cell::from(task.claimed_by.clone().unwrap_or_else(|| "-".into())),
            ])
            .style(style)
        })
        .collect();

    let table = Table::new(
        rows,
        [
            Constraint::Length(8),
            Constraint::Min(20),
            Constraint::Length(4),
            Constraint::Length(12),
            Constraint::Length(10),
        ],
    )
    .header(header)
    .block(
        Block::default()
            .title(" Tasks ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan)),
    )
    .row_highlight_style(Style::default().bg(Color::DarkGray));

    let mut state = TableState::default();
    state.select(Some(app.selected_index));
    f.render_stateful_widget(table, area, &mut state);
}

fn render_detail<R: TaskRepository>(f: &mut Frame, area: Rect, app: &App<R>) {
    let block = Block::default()
        .title(" Detail ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    if let Some(ref detail) = app.detail {
        let task = &detail.task;
        let mut lines = vec![
            Line::from(vec![
                Span::styled("ID: ", Style::default().fg(Color::Yellow)),
                Span::raw(&task.id),
            ]),
            Line::from(vec![
                Span::styled("Title: ", Style::default().fg(Color::Yellow)),
                Span::raw(&task.title),
            ]),
            Line::from(vec![
                Span::styled("Priority: ", Style::default().fg(Color::Yellow)),
                Span::raw(task.priority.to_string()),
            ]),
            Line::from(vec![
                Span::styled("Status: ", Style::default().fg(Color::Yellow)),
                Span::styled(task.status.to_string(), status_style(task.status)),
            ]),
        ];

        if let Some(ref claimed) = task.claimed_by {
            lines.push(Line::from(vec![
                Span::styled("Claimed: ", Style::default().fg(Color::Yellow)),
                Span::raw(claimed),
            ]));
        }

        if !task.blocked_by.is_empty() {
            lines.push(Line::from(vec![
                Span::styled("Blocked by: ", Style::default().fg(Color::Yellow)),
                Span::raw(task.blocked_by.join(", ")),
            ]));
        }

        if let Some(ref reason) = task.done_reason {
            lines.push(Line::from(vec![
                Span::styled("Reason: ", Style::default().fg(Color::Yellow)),
                Span::raw(reason),
            ]));
        }

        if !detail.timeline.is_empty() {
            lines.push(Line::from(""));
            lines.push(Line::styled(
                "Timeline:",
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
            ));
            for entry in &detail.timeline {
                lines.push(Line::from(vec![
                    Span::styled(
                        format!("  {} ", entry.timestamp),
                        Style::default().fg(Color::DarkGray),
                    ),
                    Span::raw(&entry.event_type),
                ]));
            }
        }

        let paragraph = Paragraph::new(lines).block(block);
        f.render_widget(paragraph, area);
    } else {
        let hint = Paragraph::new("Press Enter to load task detail")
            .style(Style::default().fg(Color::DarkGray))
            .block(block);
        f.render_widget(hint, area);
    }
}

fn status_style(status: crate::domain::task::TaskStatus) -> Style {
    use crate::domain::task::TaskStatus;
    match status {
        TaskStatus::Open => Style::default().fg(Color::Green),
        TaskStatus::InProgress => Style::default().fg(Color::Yellow),
        TaskStatus::Done => Style::default().fg(Color::DarkGray),
    }
}
