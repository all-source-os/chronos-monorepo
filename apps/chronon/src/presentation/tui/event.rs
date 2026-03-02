use std::time::Duration;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use crate::domain::repository::TaskRepository;
use super::app::{App, View};

pub enum AppEvent {
    Tick,
    Key(KeyEvent),
}

pub fn poll_event(timeout: Duration) -> std::io::Result<Option<AppEvent>> {
    if event::poll(timeout)? {
        if let Event::Key(key) = event::read()? {
            return Ok(Some(AppEvent::Key(key)));
        }
    }
    Ok(Some(AppEvent::Tick))
}

pub async fn handle_event<R: TaskRepository>(app: &mut App<R>, evt: AppEvent) {
    match evt {
        AppEvent::Tick => {
            app.refresh();
        }
        AppEvent::Key(key) => {
            // Ctrl+C always quits
            if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
                app.should_quit = true;
                return;
            }

            match key.code {
                KeyCode::Char('q') => app.should_quit = true,
                KeyCode::Tab => app.toggle_view(),
                KeyCode::Char('c') => app.claim_focused().await,
                KeyCode::Char('d') => app.complete_focused().await,
                KeyCode::Char('a') => app.approve_focused().await,
                KeyCode::Enter => app.load_detail().await,

                // Navigation depends on view
                KeyCode::Char('j') | KeyCode::Down => match app.view {
                    View::Dashboard => app.select_next(),
                    View::Kanban => app.kanban_select_next(),
                },
                KeyCode::Char('k') | KeyCode::Up => match app.view {
                    View::Dashboard => app.select_prev(),
                    View::Kanban => app.kanban_select_prev(),
                },
                KeyCode::Char('h') | KeyCode::Left => {
                    if app.view == View::Kanban {
                        app.kanban_column = app.kanban_column.prev();
                    }
                }
                KeyCode::Char('l') | KeyCode::Right => {
                    if app.view == View::Kanban {
                        app.kanban_column = app.kanban_column.next();
                    }
                }
                KeyCode::Char('r') => {
                    app.refresh();
                    app.status_message = Some("Refreshed".to_string());
                }
                _ => {}
            }
        }
    }
}
