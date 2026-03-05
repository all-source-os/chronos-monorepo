mod app;
mod event;
mod ui;
mod views;

use std::time::Duration;

use crate::infrastructure::core_task_repo::CoreTaskRepository;
use app::App;
use event::{handle_event, poll_event};

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

    loop {
        terminal.draw(|f| ui::render(f, &app))?;

        if let Some(evt) = poll_event(Duration::from_secs(1))? {
            handle_event(&mut app, evt).await;
        }

        if app.should_quit {
            break;
        }
    }

    ratatui::restore();
    Ok(())
}
