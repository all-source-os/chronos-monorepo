use axum::{
    extract::{Path, Query, State},
    http::{StatusCode, header},
    response::{IntoResponse, Json},
};
use serde::Deserialize;

use super::error::AppError;
use crate::{
    domain::{repository::TaskRepository, task::Task},
    presentation::web::state::AppState,
};

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
    Ok(Json(detail))
}

pub async fn api_claim(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    let agent = crate::infrastructure::agent_id();
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

pub async fn api_export(State(state): State<AppState>) -> Result<impl IntoResponse, AppError> {
    let tasks = state.repo.list_tasks(None)?;
    let md = crate::presentation::shared::export_markdown(&tasks);

    Ok((
        [
            (header::CONTENT_TYPE, "text/markdown; charset=utf-8"),
            (
                header::CONTENT_DISPOSITION,
                "attachment; filename=\"chronis_export.md\"",
            ),
        ],
        md,
    ))
}
