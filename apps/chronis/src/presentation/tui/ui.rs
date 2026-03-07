use ratatui::{
    Frame,
    layout::{Constraint, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
};

use super::{
    app::{App, InputMode, View},
    views::{dashboard, graph, kanban},
};
use crate::domain::{repository::TaskRepository, task::TaskStatus};

pub fn render<R: TaskRepository>(f: &mut Frame, app: &App<R>) {
    let is_searching = app.input_mode == InputMode::Search || !app.search_query.is_empty();

    let chunks = Layout::vertical([
        Constraint::Length(1),                                // title bar
        Constraint::Length(if is_searching { 1 } else { 0 }), // search bar
        Constraint::Min(0),                                   // main view
        Constraint::Length(1),                                // status bar
    ])
    .split(f.area());

    // Title bar
    let view_name = match app.view {
        View::Dashboard => "Dashboard",
        View::Kanban => "Kanban",
        View::Graph => "Graph",
    };
    let open = app
        .tasks
        .iter()
        .filter(|t| t.status == TaskStatus::Open)
        .count();
    let progress = app
        .tasks
        .iter()
        .filter(|t| t.status == TaskStatus::InProgress)
        .count();
    let done = app
        .tasks
        .iter()
        .filter(|t| t.status == TaskStatus::Done)
        .count();

    let title = Line::from(vec![
        Span::styled(
            " chronis",
            Style::default()
                .fg(Color::Black)
                .bg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" "),
        Span::styled(view_name, Style::default().fg(Color::Yellow)),
        Span::raw("  "),
        Span::styled(format!("{open} open"), Style::default().fg(Color::Green)),
        Span::styled(" | ", Style::default().fg(Color::DarkGray)),
        Span::styled(
            format!("{progress} in-progress"),
            Style::default().fg(Color::Yellow),
        ),
        Span::styled(" | ", Style::default().fg(Color::DarkGray)),
        Span::styled(format!("{done} done"), Style::default().fg(Color::DarkGray)),
    ]);
    f.render_widget(Paragraph::new(title), chunks[0]);

    // Search bar
    if is_searching {
        let match_count = app.filtered_tasks().len();
        let search_line = Line::from(vec![
            Span::styled(
                " /",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(&app.search_query),
            if app.input_mode == InputMode::Search {
                Span::styled(
                    "_",
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(Modifier::SLOW_BLINK),
                )
            } else {
                Span::raw("")
            },
            Span::styled(
                format!("  ({match_count} matches)"),
                Style::default().fg(Color::DarkGray),
            ),
        ]);
        f.render_widget(Paragraph::new(search_line), chunks[1]);
    }

    // Main view
    match app.view {
        View::Dashboard => dashboard::render(f, chunks[2], app),
        View::Kanban => kanban::render(f, chunks[2], app),
        View::Graph => graph::render(f, chunks[2], app),
    }

    // Status bar
    let status = if app.input_mode == InputMode::Search {
        Line::from(vec![
            Span::styled(" Esc", Style::default().fg(Color::Yellow)),
            Span::raw(" cancel  "),
            Span::styled("Enter", Style::default().fg(Color::Yellow)),
            Span::raw(" select  "),
            Span::styled("Up/Down", Style::default().fg(Color::Yellow)),
            Span::raw(" navigate"),
        ])
    } else if let Some(ref msg) = app.status_message {
        Line::from(vec![
            Span::raw(" "),
            Span::styled(msg.as_str(), Style::default().fg(Color::Green)),
        ])
    } else {
        Line::from(vec![
            Span::styled(" q", Style::default().fg(Color::Yellow)),
            Span::raw(" quit  "),
            Span::styled("Tab", Style::default().fg(Color::Yellow)),
            Span::raw(" view  "),
            Span::styled("j/k", Style::default().fg(Color::Yellow)),
            Span::raw(" nav  "),
            Span::styled("/", Style::default().fg(Color::Yellow)),
            Span::raw(" search  "),
            Span::styled("Enter", Style::default().fg(Color::Yellow)),
            Span::raw(" detail  "),
            Span::styled("c", Style::default().fg(Color::Yellow)),
            Span::raw(" claim  "),
            Span::styled("d", Style::default().fg(Color::Yellow)),
            Span::raw(" done  "),
            Span::styled("a", Style::default().fg(Color::Yellow)),
            Span::raw(" approve  "),
            Span::styled("1/2/3", Style::default().fg(Color::Yellow)),
            Span::raw(" filter  "),
            Span::styled("0", Style::default().fg(Color::Yellow)),
            Span::raw(" all"),
        ])
    };
    f.render_widget(Paragraph::new(status), chunks[3]);
}
