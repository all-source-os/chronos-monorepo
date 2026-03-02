use axum::extract::{Path, Query, State};
use axum::http::{header, StatusCode};
use axum::response::{Html, IntoResponse, Json, Response};
use serde::Deserialize;

use crate::domain::error::ChronError;
use crate::domain::repository::TaskRepository;
use crate::domain::task::{Task, TaskStatus};
use super::state::AppState;

// --- Static assets ---

const INDEX_HTML: &str = include_str!("assets/index.html");
const KANBAN_HTML: &str = include_str!("assets/kanban.html");
const STYLE_CSS: &str = include_str!("assets/style.css");

pub async fn index() -> Html<&'static str> {
    Html(INDEX_HTML)
}

pub async fn kanban_page() -> Html<&'static str> {
    Html(KANBAN_HTML)
}

pub async fn style_css() -> impl IntoResponse {
    ([(header::CONTENT_TYPE, "text/css")], STYLE_CSS)
}

pub async fn htmx_js() -> impl IntoResponse {
    (
        [(header::CONTENT_TYPE, "application/javascript")],
        include_str!("assets/htmx.min.js"),
    )
}

// --- JSON API ---

#[derive(Deserialize)]
pub struct TaskFilter {
    pub status: Option<String>,
}

pub async fn api_tasks(
    State(state): State<AppState>,
    Query(filter): Query<TaskFilter>,
) -> Result<Json<Vec<Task>>, AppError> {
    let tasks = state.repo.list_tasks(filter.status.as_deref())?;
    Ok(Json(tasks))
}

pub async fn api_task_detail(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    let detail = state.repo.get_task_detail(&id).await?;
    Ok(Json(serde_json::json!({
        "task": detail.task,
        "timeline": detail.timeline.iter().map(|e| {
            serde_json::json!({
                "timestamp": e.timestamp,
                "event_type": e.event_type,
            })
        }).collect::<Vec<_>>()
    })))
}

pub async fn api_claim(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    let agent = std::env::var("CN_AGENT_ID").unwrap_or_else(|_| "human".to_string());
    state.repo.claim_task(&id, &agent).await?;
    Ok((StatusCode::OK, [("HX-Trigger", "refresh")], "OK"))
}

pub async fn api_done(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    state.repo.complete_task(&id, None).await?;
    Ok((StatusCode::OK, [("HX-Trigger", "refresh")], "OK"))
}

pub async fn api_approve(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    state.repo.approve_task(&id).await?;
    Ok((StatusCode::OK, [("HX-Trigger", "refresh")], "OK"))
}

// --- HTMX Partials ---

fn task_row_html(task: &Task) -> String {
    let status_class = match task.status {
        TaskStatus::Open => "status-open",
        TaskStatus::InProgress => "status-progress",
        TaskStatus::Done => "status-done",
    };
    let claimed = task.claimed_by.as_deref().unwrap_or("-");
    let blocked = if task.blocked_by.is_empty() {
        "-".to_string()
    } else {
        task.blocked_by.len().to_string()
    };
    let id = &task.id;
    let title = html_escape(&task.title);
    let pri = task.priority.to_string();
    let status = task.status.to_string();

    let mut s = String::new();
    s.push_str("<tr class=\"task-row\" hx-get=\"/partials/task-detail/");
    s.push_str(id);
    s.push_str("\" hx-target=\"#detail-pane\" hx-swap=\"innerHTML\">\n");
    s.push_str("  <td>");
    s.push_str(id);
    s.push_str("</td>\n  <td>");
    s.push_str(&title);
    s.push_str("</td>\n  <td>");
    s.push_str(&pri);
    s.push_str("</td>\n  <td><span class=\"");
    s.push_str(status_class);
    s.push_str("\">");
    s.push_str(&status);
    s.push_str("</span></td>\n  <td>");
    s.push_str(claimed);
    s.push_str("</td>\n  <td>");
    s.push_str(&blocked);
    s.push_str("</td>\n  <td>\n");
    s.push_str("    <button class=\"btn btn-sm\" hx-post=\"/api/tasks/");
    s.push_str(id);
    s.push_str("/claim\" hx-swap=\"none\">claim</button>\n");
    s.push_str("    <button class=\"btn btn-sm\" hx-post=\"/api/tasks/");
    s.push_str(id);
    s.push_str("/done\" hx-swap=\"none\">done</button>\n");
    s.push_str("    <button class=\"btn btn-sm\" hx-post=\"/api/tasks/");
    s.push_str(id);
    s.push_str("/approve\" hx-swap=\"none\">approve</button>\n");
    s.push_str("  </td>\n</tr>");
    s
}

pub async fn partial_task_list(
    State(state): State<AppState>,
) -> Result<Html<String>, AppError> {
    let tasks = state.repo.list_tasks(None)?;
    let mut html = String::new();
    for task in &tasks {
        html.push_str(&task_row_html(task));
        html.push('\n');
    }
    if tasks.is_empty() {
        html.push_str("<tr><td colspan=\"7\" style=\"text-align:center;color:#666\">No tasks</td></tr>");
    }
    Ok(Html(html))
}

pub async fn partial_task_detail(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Html<String>, AppError> {
    let detail = state.repo.get_task_detail(&id).await?;
    let task = &detail.task;

    let mut html = String::new();
    html.push_str("<h3>");
    html.push_str(&task.id);
    html.push_str("</h3>\n<dl>\n");
    html.push_str("  <dt>Title</dt><dd>");
    html.push_str(&html_escape(&task.title));
    html.push_str("</dd>\n  <dt>Priority</dt><dd>");
    html.push_str(&task.priority.to_string());
    html.push_str("</dd>\n  <dt>Status</dt><dd>");
    html.push_str(&task.status.to_string());
    html.push_str("</dd>\n  <dt>Claimed</dt><dd>");
    html.push_str(task.claimed_by.as_deref().unwrap_or("-"));
    html.push_str("</dd>\n</dl>");

    if !task.blocked_by.is_empty() {
        html.push_str("<dl><dt>Blocked by</dt><dd>");
        html.push_str(&task.blocked_by.join(", "));
        html.push_str("</dd></dl>");
    }

    if let Some(ref reason) = task.done_reason {
        html.push_str("<dl><dt>Reason</dt><dd>");
        html.push_str(&html_escape(reason));
        html.push_str("</dd></dl>");
    }

    if !detail.timeline.is_empty() {
        html.push_str("<h4>Timeline</h4><ul class=\"timeline\">");
        for entry in &detail.timeline {
            html.push_str("<li><span class=\"ts\">");
            html.push_str(&html_escape(&entry.timestamp));
            html.push_str("</span> ");
            html.push_str(&html_escape(&entry.event_type));
            html.push_str("</li>");
        }
        html.push_str("</ul>");
    }

    Ok(Html(html))
}

fn kanban_card_html(task: &Task) -> String {
    let id = &task.id;
    let pri = task.priority.to_string();
    let title = html_escape(&task.title);
    let claimed = task.claimed_by.as_deref().unwrap_or("");

    let mut s = String::new();
    s.push_str("<div class=\"kanban-card\" hx-get=\"/partials/task-detail/");
    s.push_str(id);
    s.push_str("\" hx-target=\"#detail-pane\" hx-swap=\"innerHTML\">\n");
    s.push_str("  <div class=\"card-id\">");
    s.push_str(id);
    s.push_str(" [");
    s.push_str(&pri);
    s.push_str("]</div>\n");
    s.push_str("  <div class=\"card-title\">");
    s.push_str(&title);
    s.push_str("</div>\n");
    if !claimed.is_empty() {
        s.push_str("  <div class=\"card-claimed\">@");
        s.push_str(&html_escape(claimed));
        s.push_str("</div>\n");
    }
    s.push_str("  <div class=\"card-actions\">\n");
    s.push_str("    <button class=\"btn btn-sm\" hx-post=\"/api/tasks/");
    s.push_str(id);
    s.push_str("/claim\" hx-swap=\"none\">claim</button>\n");
    s.push_str("    <button class=\"btn btn-sm\" hx-post=\"/api/tasks/");
    s.push_str(id);
    s.push_str("/done\" hx-swap=\"none\">done</button>\n");
    s.push_str("  </div>\n</div>");
    s
}

pub async fn partial_kanban(
    State(state): State<AppState>,
) -> Result<Html<String>, AppError> {
    let tasks = state.repo.list_tasks(None)?;

    let mut html = String::from("<div class=\"kanban-board\">");

    for (status, label) in [
        (TaskStatus::Open, "Open"),
        (TaskStatus::InProgress, "In Progress"),
        (TaskStatus::Done, "Done"),
    ] {
        let col_tasks: Vec<&Task> = tasks.iter().filter(|t| t.status == status).collect();
        html.push_str("<div class=\"kanban-col\"><h3>");
        html.push_str(label);
        html.push_str(" (");
        html.push_str(&col_tasks.len().to_string());
        html.push_str(")</h3>");
        for task in col_tasks {
            html.push_str(&kanban_card_html(task));
            html.push('\n');
        }
        html.push_str("</div>");
    }
    html.push_str("</div>");
    Ok(Html(html))
}

// --- Error handling ---

pub struct AppError(ChronError);

impl From<ChronError> for AppError {
    fn from(e: ChronError) -> Self {
        Self(e)
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, msg) = match &self.0 {
            ChronError::TaskNotFound(_) => (StatusCode::NOT_FOUND, self.0.to_string()),
            ChronError::InvalidTransition { .. } | ChronError::AlreadyDone(_) => {
                (StatusCode::CONFLICT, self.0.to_string())
            }
            _ => (StatusCode::INTERNAL_SERVER_ERROR, self.0.to_string()),
        };
        (status, msg).into_response()
    }
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
