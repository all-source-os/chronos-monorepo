//! TOON (Token-Oriented Object Notation) encoder for agent-optimized CLI output.
//!
//! Pipe-delimited, no braces/quotes. ~50% fewer tokens than JSON.
//! Format: `[col|col]\nval|val\nval|val`

use crate::domain::{
    repository::{TaskDetail, TimelineEntry},
    task::Task,
};

fn escape(s: &str) -> String {
    s.replace('|', "\\|").replace('\n', "\\n")
}

/// Encode a list of tasks as TOON.
#[cfg_attr(feature = "hotpath", hotpath::measure)]
pub fn tasks(tasks: &[Task]) -> String {
    let mut out = String::from("[id|type|title|pri|status|claimed|blocked_by|parent|archived]\n");
    for t in tasks {
        let claimed = t.claimed_by.as_deref().unwrap_or("");
        let blocked = if t.blocked_by.is_empty() {
            String::new()
        } else {
            t.blocked_by.join(",")
        };
        let parent = t.parent.as_deref().unwrap_or("");
        out.push_str(&format!(
            "{}|{}|{}|{}|{}|{}|{}|{}|{}\n",
            t.id,
            t.task_type,
            escape(&t.title),
            t.priority,
            t.status,
            claimed,
            blocked,
            parent,
            t.archived,
        ));
    }
    out
}

/// Encode a task detail (show) as TOON.
#[cfg_attr(feature = "hotpath", hotpath::measure)]
pub fn task_detail(detail: &TaskDetail) -> String {
    let t = &detail.task;
    let mut out = String::new();

    // Task fields as key:value lines
    out.push_str(&format!("id:{}\n", t.id));
    out.push_str(&format!("type:{}\n", t.task_type));
    out.push_str(&format!("title:{}\n", escape(&t.title)));
    if let Some(ref desc) = t.description {
        out.push_str(&format!("description:{}\n", escape(desc)));
    }
    out.push_str(&format!("priority:{}\n", t.priority));
    out.push_str(&format!("status:{}\n", t.status));
    if let Some(ref parent) = t.parent {
        out.push_str(&format!("parent:{parent}\n"));
    }
    if let Some(ref claimed) = t.claimed_by {
        out.push_str(&format!("claimed_by:{claimed}\n"));
    }
    if !t.blocked_by.is_empty() {
        out.push_str(&format!("blocked_by:{}\n", t.blocked_by.join(",")));
    }
    if let Some(ref reason) = t.done_reason {
        out.push_str(&format!("done_reason:{}\n", escape(reason)));
    }
    out.push_str(&format!("archived:{}\n", t.archived));

    // Children as TOON table
    out.push_str("---\n");
    out
}

/// Encode children list as TOON (compact).
pub fn children(children: &[Task]) -> String {
    if children.is_empty() {
        return String::new();
    }
    let mut out = String::from("[id|status|title]\n");
    for c in children {
        out.push_str(&format!("{}|{}|{}\n", c.id, c.status, escape(&c.title)));
    }
    out
}

/// Encode timeline as TOON.
pub fn timeline(entries: &[TimelineEntry]) -> String {
    if entries.is_empty() {
        return String::new();
    }
    let mut out = String::from("[timestamp|event_type]\n");
    for e in entries {
        out.push_str(&format!("{}|{}\n", e.timestamp, e.event_type));
    }
    out
}

/// Encode a single action result as TOON key:value.
pub fn action(verb: &str, id: &str) -> String {
    format!("ok:{verb}:{id}\n")
}

/// Encode a create result.
pub fn created(task_type: &str, id: &str, title: &str) -> String {
    format!("created:{}:{}:{}\n", task_type, id, escape(title))
}
