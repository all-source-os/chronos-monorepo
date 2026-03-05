use ratatui::{
    Frame,
    layout::{Constraint, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
};

use super::{
    app::{App, View},
    views::{dashboard, kanban},
};
use crate::domain::repository::TaskRepository;

pub fn render<R: TaskRepository>(f: &mut Frame, app: &App<R>) {
    let chunks = Layout::vertical([
        Constraint::Length(1), // title bar
        Constraint::Min(0),    // main view
        Constraint::Length(1), // status bar
    ])
    .split(f.area());

    // Title bar
    let view_name = match app.view {
        View::Dashboard => "Dashboard",
        View::Kanban => "Kanban",
    };
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
        Span::styled(
            format!("{} tasks", app.tasks.len()),
            Style::default().fg(Color::DarkGray),
        ),
    ]);
    f.render_widget(Paragraph::new(title), chunks[0]);

    // Main view
    match app.view {
        View::Dashboard => dashboard::render(f, chunks[1], app),
        View::Kanban => kanban::render(f, chunks[1], app),
    }

    // Status bar
    let status = if let Some(ref msg) = app.status_message {
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
            Span::styled("Enter", Style::default().fg(Color::Yellow)),
            Span::raw(" detail  "),
            Span::styled("c", Style::default().fg(Color::Yellow)),
            Span::raw(" claim  "),
            Span::styled("d", Style::default().fg(Color::Yellow)),
            Span::raw(" done  "),
            Span::styled("a", Style::default().fg(Color::Yellow)),
            Span::raw(" approve"),
        ])
    };
    f.render_widget(Paragraph::new(status), chunks[2]);
}
