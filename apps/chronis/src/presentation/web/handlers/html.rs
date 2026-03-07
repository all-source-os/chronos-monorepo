use crate::domain::task::{Priority, Task, TaskStatus, TaskType};

pub fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

pub fn type_badge(task_type: TaskType) -> &'static str {
    match task_type {
        TaskType::Task => r#"<span class="type-badge type-task">task</span>"#,
        TaskType::Epic => r#"<span class="type-badge type-epic">epic</span>"#,
        TaskType::Bug => r#"<span class="type-badge type-bug">bug</span>"#,
        TaskType::Feature => r#"<span class="type-badge type-feature">feat</span>"#,
    }
}

pub fn pri_badge(task: &Task) -> String {
    let class = match task.priority {
        Priority::P0 => "pri-p0",
        Priority::P1 => "pri-p1",
        Priority::P2 => "pri-p2",
        Priority::P3 => "pri-p3",
    };
    format!(
        r#"<span class="pri-badge {class}">{}</span>"#,
        task.priority
    )
}

pub fn task_row_html(task: &Task) -> String {
    let status_class = crate::presentation::shared::status_css_class(task.status);
    let claimed = task.claimed_by.as_deref().unwrap_or("-");
    let blocked = if task.blocked_by.is_empty() {
        String::from("-")
    } else {
        task.blocked_by.join(", ")
    };
    let id = &task.id;
    let title = html_escape(&task.title);
    let status = task.status.to_string();

    let mut s = String::new();
    s.push_str("<tr class=\"task-row\" data-status=\"");
    s.push_str(&status);
    s.push_str("\" hx-get=\"/partials/task-detail/");
    s.push_str(id);
    s.push_str("\" hx-target=\"#detail-pane\" hx-swap=\"innerHTML\">\n");
    s.push_str("  <td>");
    s.push_str(id);
    s.push_str("</td>\n  <td>");
    s.push_str(type_badge(task.task_type));
    s.push_str("</td>\n  <td>");
    s.push_str(&title);
    s.push_str("</td>\n  <td>");
    s.push_str(&pri_badge(task));
    s.push_str("</td>\n  <td><span class=\"");
    s.push_str(status_class);
    s.push_str("\">");
    s.push_str(&status);
    s.push_str("</span></td>\n  <td>");
    s.push_str(&html_escape(claimed));
    s.push_str("</td>\n  <td>");
    s.push_str(&html_escape(&blocked));
    s.push_str("</td>\n  <td class=\"action-group\">\n");
    s.push_str("    <button class=\"btn btn-sm btn-claim\" hx-post=\"/api/tasks/");
    s.push_str(id);
    s.push_str("/claim\" hx-swap=\"none\">claim</button>\n");
    s.push_str("    <button class=\"btn btn-sm btn-done\" hx-post=\"/api/tasks/");
    s.push_str(id);
    s.push_str("/done\" hx-swap=\"none\">done</button>\n");
    s.push_str("    <button class=\"btn btn-sm btn-approve\" hx-post=\"/api/tasks/");
    s.push_str(id);
    s.push_str("/approve\" hx-swap=\"none\">approve</button>\n");
    s.push_str("  </td>\n</tr>");
    s
}

pub fn kanban_card_html(task: &Task) -> String {
    let id = &task.id;
    let title = html_escape(&task.title);
    let claimed = task.claimed_by.as_deref().unwrap_or("");

    let mut s = String::new();
    s.push_str("<div class=\"kanban-card\" hx-get=\"/partials/task-detail/");
    s.push_str(id);
    s.push_str("\" hx-target=\"#detail-pane\" hx-swap=\"innerHTML\">\n");

    // Header: ID + type + priority
    s.push_str("  <div class=\"card-header\">");
    s.push_str("<span class=\"card-id\">");
    s.push_str(id);
    s.push_str("</span> ");
    s.push_str(type_badge(task.task_type));
    s.push(' ');
    s.push_str(&pri_badge(task));
    s.push_str("</div>\n");

    // Title
    s.push_str("  <div class=\"card-title\">");
    s.push_str(&title);
    s.push_str("</div>\n");

    // Description preview
    if let Some(ref desc) = task.description {
        let preview: String = desc.chars().take(80).collect();
        let preview = html_escape(&preview);
        s.push_str("  <div class=\"card-desc\">");
        s.push_str(&preview);
        if desc.len() > 80 {
            s.push_str("...");
        }
        s.push_str("</div>\n");
    }

    // Meta: claimed + blocked
    s.push_str("  <div class=\"card-meta\">");
    if !claimed.is_empty() {
        s.push_str("<span class=\"card-claimed\">@");
        s.push_str(&html_escape(claimed));
        s.push_str("</span>");
    }
    if !task.blocked_by.is_empty() {
        s.push_str("<span class=\"card-blocked\">blocked by ");
        s.push_str(&html_escape(&task.blocked_by.join(", ")));
        s.push_str("</span>");
    }
    s.push_str("</div>\n");

    // Actions
    s.push_str("  <div class=\"card-actions\">\n");
    s.push_str("    <button class=\"btn btn-sm btn-claim\" hx-post=\"/api/tasks/");
    s.push_str(id);
    s.push_str("/claim\" hx-swap=\"none\">claim</button>\n");
    s.push_str("    <button class=\"btn btn-sm btn-done\" hx-post=\"/api/tasks/");
    s.push_str(id);
    s.push_str("/done\" hx-swap=\"none\">done</button>\n");
    s.push_str("    <button class=\"btn btn-sm btn-approve\" hx-post=\"/api/tasks/");
    s.push_str(id);
    s.push_str("/approve\" hx-swap=\"none\">approve</button>\n");
    s.push_str("  </div>\n</div>");
    s
}

pub fn graph_node_html(task: &Task, depth: usize) -> String {
    let node_class = match task.status {
        TaskStatus::Open => "node-open",
        TaskStatus::InProgress => "node-progress",
        TaskStatus::Done => "node-done",
    };

    let icon = if task.task_type == TaskType::Epic {
        "&#9670;" // diamond
    } else if !task.blocked_by.is_empty() && task.status != TaskStatus::Done {
        "&#9676;" // dotted circle
    } else {
        "&#9679;" // filled circle
    };

    let claimed = task
        .claimed_by
        .as_deref()
        .map(|c| format!(" <span class=\"card-claimed\">@{}</span>", html_escape(c)))
        .unwrap_or_default();

    let connector = if depth > 0 {
        "&#9500;&#9472;&#9472; "
    } else {
        ""
    };
    let id = &task.id;
    let title = html_escape(&task.title);
    let pri = task.priority;
    let status = task.status;

    let mut s = String::new();
    s.push_str("<div class=\"graph-node depth-");
    s.push_str(&depth.to_string());
    s.push(' ');
    s.push_str(node_class);
    s.push_str("\" hx-get=\"/partials/task-detail/");
    s.push_str(id);
    s.push_str("\" hx-target=\"#detail-pane\" hx-swap=\"innerHTML\" style=\"cursor:pointer\">\n");
    s.push_str("  <span class=\"graph-connector\">");
    s.push_str(connector);
    s.push_str("</span><span class=\"graph-icon\">");
    s.push_str(icon);
    s.push_str("</span>\n  <span class=\"graph-id\">");
    s.push_str(id);
    s.push_str("</span>\n  <span class=\"graph-title\">");
    s.push_str(&title);
    s.push_str("</span>\n  <span style=\"color:#565f89;font-size:12px\">[");
    s.push_str(&pri.to_string());
    s.push_str(", ");
    s.push_str(&status.to_string());
    s.push_str("]</span>");
    s.push_str(&claimed);
    s.push_str("\n</div>\n");
    s
}

/// Render markdown text to HTML.
pub fn render_markdown_html(text: &str) -> String {
    let mut html = String::new();
    let mut in_code_block = false;

    for line in text.lines() {
        let trimmed = line.trim_start();

        if trimmed.starts_with("```") {
            if in_code_block {
                html.push_str("</code></pre>\n");
            } else {
                html.push_str("<pre><code>");
            }
            in_code_block = !in_code_block;
            continue;
        }

        if in_code_block {
            html.push_str(&html_escape(line));
            html.push('\n');
            continue;
        }

        if let Some(rest) = trimmed.strip_prefix("### ") {
            html.push_str("<h3>");
            html.push_str(&inline_md(&html_escape(rest)));
            html.push_str("</h3>\n");
        } else if let Some(rest) = trimmed.strip_prefix("## ") {
            html.push_str("<h2>");
            html.push_str(&inline_md(&html_escape(rest)));
            html.push_str("</h2>\n");
        } else if let Some(rest) = trimmed.strip_prefix("# ") {
            html.push_str("<h1>");
            html.push_str(&inline_md(&html_escape(rest)));
            html.push_str("</h1>\n");
        } else if trimmed.starts_with("- ") || trimmed.starts_with("* ") {
            html.push_str("<li>");
            html.push_str(&inline_md(&html_escape(&trimmed[2..])));
            html.push_str("</li>\n");
        } else if trimmed.is_empty() {
            html.push_str("<br>\n");
        } else {
            html.push_str("<p>");
            html.push_str(&inline_md(&html_escape(line)));
            html.push_str("</p>\n");
        }
    }

    if in_code_block {
        html.push_str("</code></pre>\n");
    }

    html
}

fn inline_md(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 32);
    let mut rest = text;

    while !rest.is_empty() {
        if let Some(start) = rest.find("**") {
            out.push_str(&rest[..start]);
            let after = &rest[start + 2..];
            if let Some(end) = after.find("**") {
                out.push_str("<strong>");
                out.push_str(&after[..end]);
                out.push_str("</strong>");
                rest = &after[end + 2..];
            } else {
                out.push_str(&rest[start..]);
                break;
            }
        } else if let Some(start) = rest.find('`') {
            out.push_str(&rest[..start]);
            let after = &rest[start + 1..];
            if let Some(end) = after.find('`') {
                out.push_str("<code>");
                out.push_str(&after[..end]);
                out.push_str("</code>");
                rest = &after[end + 1..];
            } else {
                out.push_str(&rest[start..]);
                break;
            }
        } else {
            out.push_str(rest);
            break;
        }
    }

    out
}
