mod app;
mod event;
mod ui;
mod views;

use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

use crate::infrastructure::core_task_repo::CoreTaskRepository;
use app::App;
use event::{AppEvent, handle_event, poll_event};

pub async fn run(repo: CoreTaskRepository) -> anyhow::Result<()> {
    // Install panic hook that restores terminal before printing panic
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        ratatui::restore();
        original_hook(info);
    }));

    let mut terminal = ratatui::init();
    let mut app = App::new(repo);
    app.refresh();

    // File watcher: set dirty flag when .chronis/ changes
    let dirty = Arc::new(AtomicBool::new(false));
    let _watcher = start_watcher(dirty.clone());

    loop {
        terminal.draw(|f| ui::render(f, &app))?;

        if let Some(evt) = poll_event(Duration::from_millis(250))? {
            match evt {
                AppEvent::Tick => {
                    // Only refresh on file change, not every tick
                    if dirty.swap(false, Ordering::Relaxed) {
                        app.refresh();
                    }
                }
                other => handle_event(&mut app, other).await,
            }
        }

        if app.should_quit {
            break;
        }
    }

    ratatui::restore();
    Ok(())
}

fn start_watcher(dirty: Arc<AtomicBool>) -> Option<notify::RecommendedWatcher> {
    use notify::{RecursiveMode, Watcher};

    // Find .chronis/ directory by walking up from cwd
    let chronis_dir = {
        let mut dir = std::env::current_dir().ok()?;
        loop {
            let candidate = dir.join(".chronis");
            if candidate.is_dir() {
                break Some(candidate);
            }
            if !dir.pop() {
                break None;
            }
        }
    }?;

    let mut watcher = notify::recommended_watcher(move |_res: Result<notify::Event, _>| {
        dirty.store(true, Ordering::Relaxed);
    })
    .ok()?;

    watcher.watch(&chronis_dir, RecursiveMode::Recursive).ok()?;
    Some(watcher)
}
