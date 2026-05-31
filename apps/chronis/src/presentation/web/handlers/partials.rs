use axum::{
    extract::{Path, State},
    response::Html,
};

use super::{
    error::AppError,
    html::{
        graph_node_html, html_escape, kanban_card_html, pri_badge, render_markdown_html,
        task_row_html, tree_html, type_badge,
    },
};
use crate::{
    domain::{
        repository::TaskRepository,
        task::{Task, TaskStatus},
    },
    presentation::{shared::TaskTree, web::state::AppState},
};

pub async fn partial_stats(State(state): State<AppState>) -> Result<Html<String>, AppError> {
    let tasks = state.repo.list_tasks(None)?;
    let open = tasks
        .iter()
        .filter(|t| t.status == TaskStatus::Open)
        .count();
    let progress = tasks
        .iter()
        .filter(|t| t.status == TaskStatus::InProgress)
        .count();
    let done = tasks
        .iter()
        .filter(|t| t.status == TaskStatus::Done)
        .count();

    Ok(Html(format!(
        r#"<span class="stat"><span class="stat-dot open"></span>{open} open</span>
<span class="stat"><span class="stat-dot progress"></span>{progress} in-progress</span>
<span class="stat"><span class="stat-dot done"></span>{done} done</span>"#
    )))
}

pub async fn partial_task_list(State(state): State<AppState>) -> Result<Html<String>, AppError> {
    let tasks = state.repo.list_tasks(None)?;
    let mut html = String::new();
    for task in &tasks {
        html.push_str(&task_row_html(task));
        html.push('\n');
    }
    if tasks.is_empty() {
        html.push_str(
            "<tr><td colspan=\"8\" style=\"text-align:center;color:#565f89\">No tasks — run <code>cn task create</code> to get started</td></tr>",
        );
    }
    Ok(Html(html))
}

/// Render a single dependency entry: a clickable row that pivots the detail
/// pane to that task. Shows id + short title + a status-colored badge, and is
/// keyboard-focusable (`tabindex="0"`, Enter via the inline keydown). `known`
/// is `Some(task)` when we resolved the referenced id to a real task; a
/// dangling reference (blocker id with no matching task) still renders, marked
/// "unknown", so a broken edge is visible rather than silently dropped.
fn dep_entry_html(dep_id: &str, known: Option<&Task>) -> String {
    let mut s = String::new();
    s.push_str(
        "<li class=\"dep-item\" tabindex=\"0\" role=\"link\" hx-get=\"/partials/task-detail/",
    );
    s.push_str(dep_id);
    s.push_str("\" hx-target=\"#detail-pane\" hx-swap=\"innerHTML\" ");
    // Enter/Space on the focused item triggers the same HTMX get the click does.
    s.push_str(
        "onkeydown=\"if(event.key==='Enter'||event.key===' '){event.preventDefault();this.click();}\">",
    );
    s.push_str("<span class=\"dep-id\">");
    s.push_str(&html_escape(dep_id));
    s.push_str("</span>");
    match known {
        Some(t) => {
            let status_class = crate::presentation::shared::status_css_class(t.status);
            s.push_str(" <span class=\"dep-title\">");
            s.push_str(&html_escape(&t.title));
            s.push_str("</span> <span class=\"dep-status ");
            s.push_str(status_class);
            s.push_str("\">");
            s.push_str(&t.status.to_string());
            s.push_str("</span>");
        }
        None => {
            s.push_str(" <span class=\"dep-status status-unknown\">unknown</span>");
        }
    }
    s.push_str("</li>");
    s
}

pub async fn partial_task_detail(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Html<String>, AppError> {
    let detail = state.repo.get_task_detail(&id).await?;
    let task = &detail.task;

    // Two-way dependency edges.
    //
    // The repository/domain only stores the FORWARD edge per task
    // (`task.blocked_by` = upstream blockers). There is no stored reverse index
    // for the inverse "blocks" (downstream) relation, so we derive it with a
    // single bounded scan over the full task universe, collecting every task
    // whose `blocked_by` contains this id. For a local single-workspace viewer
    // the universe is small, so this O(n) pass is fine and — importantly — it
    // reproduces exactly the same edge set the Graph view renders from
    // `blocked_by`, keeping the two views consistent. We also use the same scan
    // to resolve each upstream blocker id to its task (title + status) so both
    // directions render with the same id/title/status format. `list_tasks_all`
    // is used (not the active-only `list_tasks`) so a done blocker/blockee is
    // still shown — a dependency doesn't disappear just because it closed.
    let universe = state.repo.list_tasks_all(None)?;
    let by_id: std::collections::HashMap<&str, &Task> =
        universe.iter().map(|t| (t.id.as_str(), t)).collect();
    let blocks: Vec<&Task> = universe
        .iter()
        .filter(|t| t.blocked_by.iter().any(|b| b == &task.id))
        .collect();

    let mut html = String::new();
    // Header row: title (h3) + expand/collapse control. The control's behaviour
    // is wired by the shared detail-pane.js (delegated from <body>, survives
    // HTMX swaps); we only emit the button + correct initial aria-expanded.
    html.push_str("<div class=\"detail-header\"><h3>");
    html.push_str(&html_escape(&task.id));
    html.push_str("</h3>");
    html.push_str(
        "<button type=\"button\" class=\"detail-expand-btn\" aria-expanded=\"false\" \
         aria-label=\"Expand detail to full screen\" title=\"Expand (full screen)\" \
         data-detail-expand>⛶</button>",
    );
    html.push_str("</div>\n<dl>\n");

    // Title
    html.push_str("  <dt>Title</dt><dd>");
    html.push_str(&html_escape(&task.title));
    html.push_str("</dd>\n");

    // Type
    html.push_str("  <dt>Type</dt><dd>");
    html.push_str(type_badge(task.task_type));
    html.push_str("</dd>\n");

    // Priority
    html.push_str("  <dt>Priority</dt><dd>");
    html.push_str(&pri_badge(task));
    html.push_str("</dd>\n");

    // Status
    let status_class = crate::presentation::shared::status_css_class(task.status);
    html.push_str("  <dt>Status</dt><dd><span class=\"");
    html.push_str(status_class);
    html.push_str("\">");
    html.push_str(&task.status.to_string());
    html.push_str("</span></dd>\n");

    // Parent
    if let Some(ref parent) = task.parent {
        html.push_str("  <dt>Parent</dt><dd>");
        html.push_str(&html_escape(parent));
        html.push_str("</dd>\n");
    }

    // Claimed
    html.push_str("  <dt>Claimed</dt><dd>");
    html.push_str(&html_escape(task.claimed_by.as_deref().unwrap_or("-")));
    html.push_str("</dd>\n");

    // Created at
    if let Some(ref created) = task.created_at {
        html.push_str("  <dt>Created</dt><dd>");
        html.push_str(&html_escape(&crate::presentation::shared::fmt_timestamp(
            created,
        )));
        html.push_str("</dd>\n");
    }

    html.push_str("</dl>");

    // Dependencies (two-way): upstream "Blocked by" + downstream "Blocks".
    html.push_str("<h4>Dependencies</h4>\n<div class=\"dep-section\">\n");

    // Upstream — what this task waits on.
    html.push_str("<div class=\"dep-group\"><div class=\"dep-group-title\">Blocked by</div>");
    if task.blocked_by.is_empty() {
        html.push_str("<p class=\"dep-empty\">Nothing upstream</p>");
    } else {
        html.push_str("<ul class=\"dep-list\">");
        for blocker_id in &task.blocked_by {
            html.push_str(&dep_entry_html(
                blocker_id,
                by_id.get(blocker_id.as_str()).copied(),
            ));
        }
        html.push_str("</ul>");
    }
    html.push_str("</div>");

    // Downstream — what waits on this task (reverse edges computed above).
    html.push_str("<div class=\"dep-group\"><div class=\"dep-group-title\">Blocks</div>");
    if blocks.is_empty() {
        html.push_str("<p class=\"dep-empty\">Nothing downstream</p>");
    } else {
        html.push_str("<ul class=\"dep-list\">");
        for t in &blocks {
            html.push_str(&dep_entry_html(&t.id, Some(t)));
        }
        html.push_str("</ul>");
    }
    html.push_str("</div>\n</div>\n");

    // Approval status
    if task.awaiting_approval == Some(true) {
        html.push_str(r#"<dl><dt>Approval</dt><dd><span class="approval-badge approval-awaiting">awaiting</span></dd></dl>"#);
    } else if task.approved == Some(true) {
        let text = if let Some(ref at) = task.approved_at {
            format!(
                "approved at {}",
                crate::presentation::shared::fmt_timestamp(at)
            )
        } else {
            String::from("approved")
        };
        html.push_str("<dl><dt>Approval</dt><dd><span class=\"approval-badge approval-approved\">");
        html.push_str(&html_escape(&text));
        html.push_str("</span></dd></dl>");
    }

    // Done info
    if let Some(ref reason) = task.done_reason {
        html.push_str("<dl><dt>Reason</dt><dd>");
        html.push_str(&html_escape(reason));
        html.push_str("</dd></dl>");
    }
    if let Some(ref done_at) = task.done_at {
        html.push_str("<dl><dt>Done at</dt><dd>");
        html.push_str(&html_escape(&crate::presentation::shared::fmt_timestamp(
            done_at,
        )));
        html.push_str("</dd></dl>");
    }

    // Actions
    html.push_str("<div class=\"action-group\" style=\"margin-top: 12px;\">\n");
    html.push_str("  <button class=\"btn btn-claim\" hx-post=\"/api/tasks/");
    html.push_str(&task.id);
    html.push_str("/claim\" hx-swap=\"none\">Claim</button>\n");
    html.push_str("  <button class=\"btn btn-done\" hx-post=\"/api/tasks/");
    html.push_str(&task.id);
    html.push_str("/done\" hx-swap=\"none\">Done</button>\n");
    html.push_str("  <button class=\"btn btn-approve\" hx-post=\"/api/tasks/");
    html.push_str(&task.id);
    html.push_str("/approve\" hx-swap=\"none\">Approve</button>\n");
    html.push_str("</div>\n");

    // Description
    if let Some(ref desc) = task.description {
        html.push_str("<h4>Description</h4>\n");
        html.push_str("<div class=\"detail-description\">");
        html.push_str(&render_markdown_html(desc));
        html.push_str("</div>");
    }

    // Timeline
    if !detail.timeline.is_empty() {
        html.push_str("<h4>Timeline</h4><ul class=\"timeline\">");
        for entry in &detail.timeline {
            html.push_str("<li><span class=\"ts\">");
            html.push_str(&html_escape(&entry.timestamp));
            html.push_str("</span> <span class=\"evt\">");
            html.push_str(&html_escape(&entry.event_type));
            html.push_str("</span></li>");
        }
        html.push_str("</ul>");
    }

    Ok(Html(html))
}

pub async fn partial_tree(State(state): State<AppState>) -> Result<Html<String>, AppError> {
    // The tree needs every task (incl. done children, for roll-up counts), so
    // pull the full set rather than the default active-only listing.
    let tasks = state.repo.list_tasks_all(None)?;
    Ok(Html(tree_html(&tasks)))
}

pub async fn partial_kanban(State(state): State<AppState>) -> Result<Html<String>, AppError> {
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

pub async fn partial_graph(State(state): State<AppState>) -> Result<Html<String>, AppError> {
    let tasks = state.repo.list_tasks(None)?;
    let tree = TaskTree::build(&tasks);
    let mut html = String::new();

    for group in &tree.epics {
        html.push_str(&graph_node_html(group.epic, 0));
        for child in &group.children {
            html.push_str(&graph_node_html(child, 1));
            for blocker in &child.blocked_by {
                html.push_str("<div class=\"graph-blocker\">blocked by ");
                html.push_str(&html_escape(blocker));
                html.push_str("</div>\n");
            }
        }
    }

    if !tree.standalone.is_empty() && !tree.epics.is_empty() {
        html.push_str("<div class=\"graph-section-title\">Standalone Tasks</div>\n");
    }

    for task in &tree.standalone {
        html.push_str(&graph_node_html(task, 0));
        for blocker in &task.blocked_by {
            html.push_str("<div class=\"graph-blocker\">blocked by ");
            html.push_str(&html_escape(blocker));
            html.push_str("</div>\n");
        }
    }

    if html.is_empty() {
        html.push_str("<p class=\"empty-state\">No tasks</p>");
    }

    Ok(Html(html))
}
