use std::path::Path;

use serde::Deserialize;

use crate::domain::{error::ChronError, repository::TaskRepository, task::TaskType};

#[derive(Debug, Deserialize)]
struct BeadsDependency {
    #[serde(rename = "type")]
    dep_type: String,
    depends_on_id: String,
}

#[derive(Debug, Deserialize)]
struct BeadsIssue {
    id: String,
    title: String,
    #[serde(default)]
    description: Option<String>,
    status: String,
    #[serde(default = "default_priority")]
    priority: u8,
    #[serde(default)]
    issue_type: Option<String>,
    #[serde(default)]
    close_reason: Option<String>,
    #[serde(default)]
    dependencies: Vec<BeadsDependency>,
}

fn default_priority() -> u8 {
    2
}

pub struct MigrateResult {
    pub migrated: usize,
    pub skipped: usize,
}

fn beads_id_to_chronis(beads_id: &str) -> String {
    if let Some(suffix) = beads_id.strip_prefix("bd-") {
        format!("t-{suffix}")
    } else {
        format!("t-{beads_id}")
    }
}

fn beads_priority_to_chronis(p: u8) -> &'static str {
    match p {
        0 | 1 => "p0",
        2 => "p1",
        3 => "p2",
        _ => "p3",
    }
}

fn beads_type_to_task_type(issue_type: Option<&str>) -> TaskType {
    match issue_type {
        Some("epic") => TaskType::Epic,
        Some("bug") => TaskType::Bug,
        Some("feature") => TaskType::Feature,
        _ => TaskType::Task,
    }
}

pub async fn migrate_beads(
    repo: &impl TaskRepository,
    beads_dir: &str,
) -> Result<MigrateResult, ChronError> {
    let jsonl_path = Path::new(beads_dir).join("issues.jsonl");
    if !jsonl_path.exists() {
        return Err(ChronError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("No issues.jsonl found at {}", jsonl_path.display()),
        )));
    }

    let content = std::fs::read_to_string(&jsonl_path)?;
    let mut issues: Vec<BeadsIssue> = Vec::new();

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        match serde_json::from_str::<BeadsIssue>(line) {
            Ok(issue) => issues.push(issue),
            Err(e) => {
                eprintln!("Warning: skipping malformed line: {e}");
            }
        }
    }

    let mut migrated = 0;
    let mut skipped = 0;

    // First pass: create all tasks (need them to exist before adding deps)
    for issue in &issues {
        let id = beads_id_to_chronis(&issue.id);
        let priority = beads_priority_to_chronis(issue.priority);
        let task_type = beads_type_to_task_type(issue.issue_type.as_deref());

        // Find parent from dependencies
        let parent = issue
            .dependencies
            .iter()
            .find(|d| d.dep_type == "parent-child")
            .map(|d| beads_id_to_chronis(&d.depends_on_id));

        // Check if task already exists
        if repo.get_task(&id).is_ok() {
            skipped += 1;
            continue;
        }

        repo.create_task(
            &id,
            &issue.title,
            priority,
            &[], // deps added in second pass
            task_type,
            parent.as_deref(),
            issue.description.as_deref(),
        )
        .await?;

        // Apply status transitions
        match issue.status.as_str() {
            "in_progress" | "in-progress" => {
                repo.claim_task(&id, "migrated").await?;
            }
            "closed" => {
                repo.claim_task(&id, "migrated").await?;
                repo.complete_task(&id, issue.close_reason.as_deref())
                    .await?;
            }
            _ => {} // "open" — no transition needed
        }

        migrated += 1;
    }

    // Second pass: add "blocks" dependencies
    for issue in &issues {
        let id = beads_id_to_chronis(&issue.id);
        for dep in &issue.dependencies {
            if dep.dep_type == "blocks" {
                let blocker_id = beads_id_to_chronis(&dep.depends_on_id);
                // Only add if both tasks exist (were migrated)
                if repo.get_task(&id).is_ok() && repo.get_task(&blocker_id).is_ok() {
                    repo.add_dependency(&id, &blocker_id).await?;
                }
            }
        }
    }

    Ok(MigrateResult { migrated, skipped })
}
