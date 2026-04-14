mod handlers;
mod state;

use crate::infrastructure::core_task_repo::CoreTaskRepository;
use axum::{
    Router,
    routing::{get, post},
};
use state::AppState;
use std::sync::Arc;

pub async fn run(repo: CoreTaskRepository, port: u16, open_browser: bool) -> anyhow::Result<()> {
    let state = AppState {
        repo: Arc::new(repo),
    };

    let app = Router::new()
        // Pages
        .route("/", get(handlers::index))
        .route("/kanban", get(handlers::kanban_page))
        .route("/graph", get(handlers::graph_page))
        // Static assets
        .route("/style.css", get(handlers::style_css))
        .route("/htmx.min.js", get(handlers::htmx_js))
        // JSON API
        .route("/api/tasks", get(handlers::api_tasks))
        .route("/api/tasks/{id}", get(handlers::api_task_detail))
        .route("/api/tasks/{id}/claim", post(handlers::api_claim))
        .route("/api/tasks/{id}/done", post(handlers::api_done))
        .route("/api/tasks/{id}/approve", post(handlers::api_approve))
        .route("/api/export", get(handlers::api_export))
        // SSE live-reload stream
        .route("/events/stream", get(handlers::events_stream))
        // HTMX partials
        .route("/partials/stats", get(handlers::partial_stats))
        .route("/partials/task-list", get(handlers::partial_task_list))
        .route(
            "/partials/task-detail/{id}",
            get(handlers::partial_task_detail),
        )
        .route("/partials/kanban", get(handlers::partial_kanban))
        .route("/partials/graph", get(handlers::partial_graph))
        .with_state(state);

    let addr = format!("0.0.0.0:{port}");
    let listener = tokio::net::TcpListener::bind(&addr).await?;

    println!("chronis web viewer: http://localhost:{port}");
    println!("Press Ctrl+C to stop");

    if open_browser {
        let _ = std::process::Command::new("open")
            .arg(format!("http://localhost:{port}"))
            .spawn();
    }

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    Ok(())
}

async fn shutdown_signal() {
    tokio::signal::ctrl_c()
        .await
        .expect("failed to install Ctrl+C handler");
    println!("\nShutting down...");
}
