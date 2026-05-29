mod api;
mod error;
pub mod html;
mod partials;
mod sse;

use axum::{
    http::header,
    response::{Html, IntoResponse},
};

// Re-export all handlers for use in router
pub use api::{api_approve, api_claim, api_done, api_export, api_task_detail, api_tasks};
pub use partials::{
    partial_graph, partial_kanban, partial_stats, partial_task_detail, partial_task_list,
    partial_tree,
};
pub use sse::events_stream;

// --- Static assets ---

const INDEX_HTML: &str = include_str!("../assets/index.html");
const KANBAN_HTML: &str = include_str!("../assets/kanban.html");
const GRAPH_HTML: &str = include_str!("../assets/graph.html");
const TREE_HTML: &str = include_str!("../assets/tree.html");
const STYLE_CSS: &str = include_str!("../assets/style.css");

pub async fn index() -> Html<&'static str> {
    Html(INDEX_HTML)
}

pub async fn kanban_page() -> Html<&'static str> {
    Html(KANBAN_HTML)
}

pub async fn graph_page() -> Html<&'static str> {
    Html(GRAPH_HTML)
}

pub async fn tree_page() -> Html<&'static str> {
    Html(TREE_HTML)
}

pub async fn style_css() -> impl IntoResponse {
    ([(header::CONTENT_TYPE, "text/css")], STYLE_CSS)
}

pub async fn htmx_js() -> impl IntoResponse {
    (
        [(header::CONTENT_TYPE, "application/javascript")],
        include_str!("../assets/htmx.min.js"),
    )
}
