use std::collections::HashMap;

use axum::{
    extract::{Path, Query, State},
    http::{StatusCode, header},
    response::{IntoResponse, Json},
};
use serde::{Deserialize, Serialize};

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

// --- Graph (bubble map) data ---
//
// Emits the dependency graph as `{ nodes, links }` for the force-directed
// bubble view in graph.html. We return the FULL universe (open, in-progress,
// AND done) in one payload so the client can toggle "include done" without a
// re-fetch; the default client view filters to open+in-progress. `subtreeSize`
// is the parent/child descendant count (so an epic's bubble area ∝ how much
// work hangs off it), and `links` are the blocked-by edges with the same
// blocker→blocked direction the text Graph view labels "blocked by".

#[derive(Serialize)]
pub struct GraphNode {
    pub id: String,
    pub title: String,
    pub status: String,
    pub priority: String,
    #[serde(rename = "type")]
    pub task_type: String,
    /// Number of transitive descendants in the parent/child tree (0 for a leaf).
    #[serde(rename = "subtreeSize")]
    pub subtree_size: usize,
}

#[derive(Serialize)]
pub struct GraphLink {
    /// Blocker task id (the upstream dependency).
    pub source: String,
    /// Blocked task id (waits on `source`).
    pub target: String,
}

#[derive(Serialize)]
pub struct GraphData {
    pub nodes: Vec<GraphNode>,
    pub links: Vec<GraphLink>,
}

/// Count transitive descendants of `id` over the parent→children map, with
/// cycle protection (a malformed parent chain must not loop forever).
fn count_descendants(
    id: &str,
    children: &HashMap<&str, Vec<&str>>,
    seen: &mut std::collections::HashSet<String>,
) -> usize {
    let mut total = 0;
    if let Some(kids) = children.get(id) {
        for kid in kids {
            if seen.insert((*kid).to_string()) {
                total += 1 + count_descendants(kid, children, seen);
            }
        }
    }
    total
}

pub async fn api_graph(State(state): State<AppState>) -> Result<Json<GraphData>, AppError> {
    // Full universe (incl. done) so the client can toggle done in/out without a
    // re-fetch and so a done blocker still anchors a real edge.
    let tasks = state.repo.list_tasks_all(None)?;

    let ids: std::collections::HashSet<&str> = tasks.iter().map(|t| t.id.as_str()).collect();

    // parent → [child ids], for the subtree-size roll-up.
    let mut children: HashMap<&str, Vec<&str>> = HashMap::new();
    for t in &tasks {
        if let Some(parent) = t.parent.as_deref() {
            children.entry(parent).or_default().push(t.id.as_str());
        }
    }

    let nodes: Vec<GraphNode> = tasks
        .iter()
        .map(|t| {
            let mut seen = std::collections::HashSet::new();
            GraphNode {
                id: t.id.clone(),
                title: t.title.clone(),
                status: t.status.to_string(),
                priority: t.priority.to_string(),
                task_type: t.task_type.to_string(),
                subtree_size: count_descendants(&t.id, &children, &mut seen),
            }
        })
        .collect();

    // blocked-by edges → links. Skip dangling references (a blocker id with no
    // matching node) so the renderer never draws an edge to a missing bubble.
    let mut links = Vec::new();
    for t in &tasks {
        for blocker in &t.blocked_by {
            if ids.contains(blocker.as_str()) {
                links.push(GraphLink {
                    source: blocker.clone(),
                    target: t.id.clone(),
                });
            }
        }
    }

    Ok(Json(GraphData { nodes, links }))
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
