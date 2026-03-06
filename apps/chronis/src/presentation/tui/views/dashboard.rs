use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table, TableState, Wrap},
};

use super::super::app::App;
use crate::domain::repository::TaskRepository;

const NARROW_THRESHOLD: u16 = 100;

pub fn render<R: TaskRepository>(f: &mut Frame, area: Rect, app: &App<R>) {
    if area.width < NARROW_THRESHOLD {
        // Narrow: show detail as overlay when loaded, otherwise full-width table
        if app.detail.is_some() {
            render_detail(f, area, app);
        } else {
            render_task_table(f, area, app);
        }
    } else {
        let chunks = Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(area);
        render_task_table(f, chunks[0], app);
        render_detail(f, chunks[1], app);
    }
}

fn render_task_table<R: TaskRepository>(f: &mut Frame, area: Rect, app: &App<R>) {
    let header = Row::new(vec![
        Cell::from("ID"),
        Cell::from("Type"),
        Cell::from("Title"),
        Cell::from("Pri"),
        Cell::from("Status"),
        Cell::from("Claimed"),
    ])
    .style(
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD),
    );

    let filtered = app.filtered_tasks();

    let filter_label = match app.status_filter {
        Some(crate::domain::task::TaskStatus::Open) => " [filter: open] ",
        Some(crate::domain::task::TaskStatus::InProgress) => " [filter: in-progress] ",
        Some(crate::domain::task::TaskStatus::Done) => " [filter: done] ",
        None => " Tasks ",
    };

    let rows: Vec<Row> = filtered
        .iter()
        .enumerate()
        .map(|(i, task)| {
            let style = if i == app.selected_index {
                Style::default().bg(Color::DarkGray).fg(Color::White)
            } else {
                status_style(task.status)
            };

            let title = if task.title.len() > 30 {
                let end = task.title.floor_char_boundary(27);
                format!("{}...", &task.title[..end])
            } else {
                task.title.clone()
            };

            Row::new(vec![
                Cell::from(task.id.clone()),
                Cell::from(task.task_type.to_string()),
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
            Constraint::Length(7),
            Constraint::Min(20),
            Constraint::Length(4),
            Constraint::Length(12),
            Constraint::Length(10),
        ],
    )
    .header(header)
    .block(
        Block::default()
            .title(filter_label)
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
        let label = Style::default().fg(Color::Yellow);

        let mut lines = vec![
            Line::from(vec![
                Span::styled("ID: ", label),
                Span::raw(&task.id),
            ]),
            Line::from(vec![
                Span::styled("Title: ", label),
                Span::raw(&task.title),
            ]),
            Line::from(vec![
                Span::styled("Type: ", label),
                Span::raw(task.task_type.to_string()),
            ]),
            Line::from(vec![
                Span::styled("Priority: ", label),
                Span::raw(task.priority.to_string()),
            ]),
            Line::from(vec![
                Span::styled("Status: ", label),
                Span::styled(task.status.to_string(), status_style(task.status)),
            ]),
        ];

        if let Some(ref parent) = task.parent {
            lines.push(Line::from(vec![
                Span::styled("Parent: ", label),
                Span::raw(parent),
            ]));
        }

        if let Some(ref claimed) = task.claimed_by {
            lines.push(Line::from(vec![
                Span::styled("Claimed: ", label),
                Span::raw(claimed),
            ]));
        }

        if !task.blocked_by.is_empty() {
            lines.push(Line::from(vec![
                Span::styled("Blocked by: ", label),
                Span::raw(task.blocked_by.join(", ")),
            ]));
        }

        if let Some(ref created) = task.created_at {
            lines.push(Line::from(vec![
                Span::styled("Created: ", label),
                Span::raw(crate::presentation::shared::fmt_timestamp(created)),
            ]));
        }

        // Approval status
        if task.awaiting_approval == Some(true) {
            lines.push(Line::from(vec![
                Span::styled("Approval: ", label),
                Span::styled("awaiting", Style::default().fg(Color::Yellow)),
            ]));
        } else if task.approved == Some(true) {
            let approved_text = if let Some(ref at) = task.approved_at {
                format!("approved at {}", crate::presentation::shared::fmt_timestamp(at))
            } else {
                "approved".to_string()
            };
            lines.push(Line::from(vec![
                Span::styled("Approval: ", label),
                Span::styled(approved_text, Style::default().fg(Color::Green)),
            ]));
        }

        if let Some(ref description) = task.description {
            lines.push(Line::from(""));
            lines.push(Line::styled(
                "Description:",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ));
            render_markdown(description, &mut lines);
        }

        if let Some(ref reason) = task.done_reason {
            lines.push(Line::from(vec![
                Span::styled("Reason: ", label),
                Span::raw(reason),
            ]));
        }

        if let Some(ref done_at) = task.done_at {
            lines.push(Line::from(vec![
                Span::styled("Done at: ", label),
                Span::raw(crate::presentation::shared::fmt_timestamp(done_at)),
            ]));
        }

        if !detail.timeline.is_empty() {
            lines.push(Line::from(""));
            lines.push(Line::styled(
                "Timeline:",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
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

        let paragraph = Paragraph::new(lines).block(block).wrap(Wrap { trim: false });
        f.render_widget(paragraph, area);
    } else {
        let hint = Paragraph::new("Press Enter to load task detail")
            .style(Style::default().fg(Color::DarkGray))
            .block(block);
        f.render_widget(hint, area);
    }
}

fn status_style(status: crate::domain::task::TaskStatus) -> Style {
    crate::presentation::shared::status_style(status)
}

/// Simple markdown-to-ratatui renderer for descriptions.
/// Handles: headers (#), bullet lists (- / *), code blocks (```), bold (**), inline code (`).
fn render_markdown<'a>(text: &'a str, lines: &mut Vec<Line<'a>>) {
    let mut in_code_block = false;

    for raw_line in text.lines() {
        // Code blocks
        if raw_line.trim_start().starts_with("```") {
            in_code_block = !in_code_block;
            continue;
        }

        if in_code_block {
            lines.push(Line::from(vec![
                Span::raw("  "),
                Span::styled(
                    raw_line,
                    Style::default().fg(Color::Green),
                ),
            ]));
            continue;
        }

        let trimmed = raw_line.trim_start();

        // Headers
        if let Some(rest) = trimmed.strip_prefix("### ") {
            lines.push(Line::from(vec![
                Span::raw("  "),
                Span::styled(rest, Style::default().fg(Color::Cyan)),
            ]));
        } else if let Some(rest) = trimmed.strip_prefix("## ") {
            lines.push(Line::from(vec![
                Span::raw("  "),
                Span::styled(
                    rest,
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ),
            ]));
        } else if let Some(rest) = trimmed.strip_prefix("# ") {
            lines.push(Line::from(vec![
                Span::raw("  "),
                Span::styled(
                    rest,
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
                ),
            ]));
        }
        // Bullet lists
        else if trimmed.starts_with("- ") || trimmed.starts_with("* ") {
            let content = &trimmed[2..];
            lines.push(Line::from(vec![
                Span::raw("  "),
                Span::styled("• ", Style::default().fg(Color::Cyan)),
                Span::raw(content),
            ]));
        }
        // Regular text with inline formatting
        else {
            lines.push(Line::from(render_inline_markdown(raw_line)));
        }
    }
}

/// Parse inline markdown: **bold**, `code`
fn render_inline_markdown(text: &str) -> Vec<Span<'_>> {
    let mut spans = vec![Span::raw("  ")];
    let mut rest = text;

    while !rest.is_empty() {
        // Bold **text**
        if let Some(start) = rest.find("**") {
            if start > 0 {
                spans.push(Span::raw(&rest[..start]));
            }
            let after = &rest[start + 2..];
            if let Some(end) = after.find("**") {
                spans.push(Span::styled(
                    &after[..end],
                    Style::default().add_modifier(Modifier::BOLD),
                ));
                rest = &after[end + 2..];
            } else {
                spans.push(Span::raw(&rest[start..]));
                break;
            }
        }
        // Inline code `text`
        else if let Some(start) = rest.find('`') {
            if start > 0 {
                spans.push(Span::raw(&rest[..start]));
            }
            let after = &rest[start + 1..];
            if let Some(end) = after.find('`') {
                spans.push(Span::styled(
                    &after[..end],
                    Style::default().fg(Color::Green),
                ));
                rest = &after[end + 1..];
            } else {
                spans.push(Span::raw(&rest[start..]));
                break;
            }
        } else {
            spans.push(Span::raw(rest));
            break;
        }
    }

    spans
}
