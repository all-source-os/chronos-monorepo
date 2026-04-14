mod app;
mod event;
mod ui;
mod views;

use std::{sync::Arc, time::Duration};

use crate::infrastructure::{
    backend::{CoreBackend, SubscribeError},
    core_task_repo::CoreTaskRepository,
};
use app::App;
use event::{AppEvent, handle_event};
use tokio::sync::mpsc;

/// Run the TUI. Refresh is driven by `CoreBackend::subscribe()` so both
/// embedded single-process writes and remote writes surface live without
/// polling. A fallback tick every 5s handles the rare case where the
/// subscription drops (e.g. remote backend during reconnect backoff).
pub async fn run(repo: CoreTaskRepository) -> anyhow::Result<()> {
    // Install panic hook that restores terminal before printing panic
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        ratatui::restore();
        original_hook(info);
    }));

    let mut terminal = ratatui::init();
    let backend: Arc<CoreBackend> = Arc::clone(repo.backend_arc());
    let mut app = App::new(repo);
    app.refresh();

    // Channel merges keystrokes, change notifications, and fallback ticks
    // into one ordered stream the main loop awaits.
    let (tx, mut rx) = mpsc::unbounded_channel::<AppEvent>();

    // Keyboard thread — crossterm's reader is blocking, so it lives on
    // std::thread and forwards into the async channel.
    let key_tx = tx.clone();
    std::thread::spawn(move || {
        loop {
            match crossterm::event::poll(Duration::from_millis(250)) {
                Ok(true) => match crossterm::event::read() {
                    Ok(crossterm::event::Event::Key(k)) => {
                        if key_tx.send(AppEvent::Key(k)).is_err() {
                            break;
                        }
                    }
                    Ok(_) => {}
                    Err(_) => break,
                },
                Ok(false) => {}
                Err(_) => break,
            }
        }
    });

    // Backend subscription task — forwards change notifications.
    let change_tx = tx.clone();
    let change_backend = Arc::clone(&backend);
    tokio::spawn(async move {
        let mut sub = change_backend.subscribe();
        loop {
            match sub.recv_change().await {
                Ok(_change) => {
                    if change_tx.send(AppEvent::BackendChanged).is_err() {
                        break;
                    }
                }
                Err(SubscribeError::Lagged(_)) => {
                    let _ = change_tx.send(AppEvent::BackendChanged);
                }
                Err(SubscribeError::Closed) => break,
            }
        }
    });

    // Fallback tick — fires every 5s so a dropped subscription (e.g. remote
    // backend mid-reconnect) still lets the user see fresh state without
    // restarting.
    let tick_tx = tx.clone();
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(Duration::from_secs(5));
        ticker.tick().await; // burn the immediate first tick
        loop {
            ticker.tick().await;
            if tick_tx.send(AppEvent::Tick).is_err() {
                break;
            }
        }
    });

    drop(tx);

    loop {
        terminal.draw(|f| ui::render(f, &app))?;

        let Some(evt) = rx.recv().await else {
            break;
        };
        match evt {
            AppEvent::BackendChanged => app.refresh(),
            other => handle_event(&mut app, other).await,
        }

        if app.should_quit {
            break;
        }
    }

    ratatui::restore();
    Ok(())
}
