use std::{collections::HashSet, fs::OpenOptions, io::Write as _, path::Path, process::Command};

use allsource_core::embedded::{EmbeddedCore, IngestEvent, Query};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::domain::error::ChronError;

/// Wire-format for one JSONL line. Preserves the original creator's UUID and
/// timestamp so every machine refers to the same event by the same identity.
#[derive(Serialize, Deserialize)]
struct SyncEvent {
    id: Uuid,
    event_type: String,
    entity_id: String,
    tenant_id: String,
    payload: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    metadata: Option<serde_json::Value>,
    timestamp: chrono::DateTime<chrono::Utc>,
    version: i64,
}

// ── Persistence helpers for local-only ID sets ──────────────────────────

fn load_id_set(path: &Path) -> Result<HashSet<String>, ChronError> {
    if !path.exists() {
        return Ok(HashSet::new());
    }
    let content = std::fs::read_to_string(path)?;
    serde_json::from_str(&content)
        .map_err(|e| ChronError::Sync(format!("parse {}: {e}", path.display())))
}

fn save_id_set(path: &Path, ids: &HashSet<String>) -> Result<(), ChronError> {
    let content =
        serde_json::to_string(ids).map_err(|e| ChronError::Sync(format!("serialize: {e}")))?;
    std::fs::write(path, content)?;
    Ok(())
}

// ── Git helpers ─────────────────────────────────────────────────────────

fn ensure_git_repo() -> Result<(), ChronError> {
    let output = Command::new("git")
        .args(["rev-parse", "--is-inside-work-tree"])
        .output()
        .map_err(|e| ChronError::Sync(format!("git not found: {e}")))?;
    if !output.status.success() {
        return Err(ChronError::Sync("not inside a git repository".into()));
    }
    Ok(())
}

fn git_pull() -> Result<(), ChronError> {
    let output = Command::new("git").args(["pull", "--rebase"]).output()?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(ChronError::Sync(format!(
            "git pull --rebase failed: {stderr}\nResolve manually, then re-run `cn sync --git`."
        )));
    }
    Ok(())
}

fn git_add(path: &str) -> Result<(), ChronError> {
    let output = Command::new("git").args(["add", path]).output()?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(ChronError::Sync(format!("git add failed: {stderr}")));
    }
    Ok(())
}

fn git_has_staged_changes(path: &str) -> Result<bool, ChronError> {
    let output = Command::new("git")
        .args(["diff", "--cached", "--quiet", path])
        .output()?;
    Ok(!output.status.success())
}

fn git_commit(message: &str) -> Result<(), ChronError> {
    let output = Command::new("git")
        .args(["commit", "-m", message])
        .output()?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(ChronError::Sync(format!("git commit failed: {stderr}")));
    }
    Ok(())
}

fn git_push() {
    match Command::new("git").args(["push"]).output() {
        Ok(o) if !o.status.success() => {
            let stderr = String::from_utf8_lossy(&o.stderr);
            eprintln!("Warning: git push failed: {stderr}");
            eprintln!("Changes are committed locally. Push manually when ready.");
        }
        Err(e) => {
            eprintln!("Warning: git push failed: {e}");
            eprintln!("Changes are committed locally. Push manually when ready.");
        }
        _ => println!("Pushed to remote."),
    }
}

// ── Main sync logic ─────────────────────────────────────────────────────

/// Sync local chronis events with a git remote.
///
/// The JSONL file is **append-only**: each event is written once by its
/// creating machine and never rewritten. Two local-only ID sets prevent
/// duplicates:
///
/// - `.remote_ids`  — JSONL UUIDs already imported into local Core
/// - `.local_ids`   — Core UUIDs already appended to the JSONL
///
/// Flow: pull → import new remote events → append new local events → push.
pub async fn sync_git(core: &EmbeddedCore, workspace_root: &Path) -> Result<(), ChronError> {
    let sync_dir = workspace_root.join(".chronis/sync");
    let events_path = sync_dir.join("events.jsonl");
    let remote_ids_path = sync_dir.join(".remote_ids");
    let local_ids_path = sync_dir.join(".local_ids");

    std::fs::create_dir_all(&sync_dir)?;
    ensure_git_repo()?;

    // Phase 1: Pull remote changes (rebase to allow concurrent appends)
    println!("Pulling remote changes...");
    git_pull()?;

    // Phase 2: Snapshot Core state before import
    let pre_import = core
        .query(Query::new())
        .await
        .map_err(|e| ChronError::Sync(format!("query: {e}")))?;
    let pre_import_ids: HashSet<String> = pre_import.iter().map(|e| e.id.to_string()).collect();

    // Phase 3: Import new remote events from JSONL
    let mut remote_ids = load_id_set(&remote_ids_path)?;
    let mut import_count = 0u64;

    if events_path.exists() {
        let content = std::fs::read_to_string(&events_path)?;
        for line in content.lines() {
            if line.trim().is_empty() {
                continue;
            }
            let ev: SyncEvent = serde_json::from_str(line)
                .map_err(|e| ChronError::Sync(format!("parse JSONL: {e}")))?;

            let jsonl_id = ev.id.to_string();
            if remote_ids.contains(&jsonl_id) {
                continue;
            }

            core.ingest(IngestEvent {
                entity_id: &ev.entity_id,
                event_type: &ev.event_type,
                payload: ev.payload,
                metadata: ev.metadata,
                tenant_id: Some(&ev.tenant_id),
            })
            .await
            .map_err(|e| ChronError::Sync(format!("ingest: {e}")))?;

            remote_ids.insert(jsonl_id);
            import_count += 1;
        }
        if import_count > 0 {
            println!("Imported {import_count} remote events.");
        }
    }

    // Phase 4: Mark imported events' Core UUIDs so we don't re-export them
    let mut local_ids = load_id_set(&local_ids_path)?;

    if import_count > 0 {
        let post_import = core
            .query(Query::new())
            .await
            .map_err(|e| ChronError::Sync(format!("query: {e}")))?;
        for ev in &post_import {
            let id = ev.id.to_string();
            if !pre_import_ids.contains(&id) {
                // This Core event was created by import — don't re-export it
                local_ids.insert(id);
            }
        }
    }

    // Phase 5: Append new local events to JSONL
    let all_events = if import_count > 0 {
        let mut evs = core
            .query(Query::new())
            .await
            .map_err(|e| ChronError::Sync(format!("query: {e}")))?;
        evs.sort_by_key(|e| e.timestamp);
        evs
    } else {
        let mut evs = pre_import;
        evs.sort_by_key(|e| e.timestamp);
        evs
    };

    let mut new_lines = String::new();
    let mut export_count = 0u64;

    for ev in &all_events {
        let core_id = ev.id.to_string();
        if local_ids.contains(&core_id) {
            continue;
        }

        let sync_ev = SyncEvent {
            id: ev.id,
            event_type: ev.event_type.clone(),
            entity_id: ev.entity_id.clone(),
            tenant_id: ev.tenant_id.clone(),
            payload: ev.payload.clone(),
            metadata: ev.metadata.clone(),
            timestamp: ev.timestamp,
            version: ev.version,
        };
        let line = serde_json::to_string(&sync_ev)
            .map_err(|e| ChronError::Sync(format!("serialize: {e}")))?;
        new_lines.push_str(&line);
        new_lines.push('\n');

        // Mark in both sets: local_ids prevents re-export, remote_ids
        // prevents re-import when this JSONL comes back via another machine.
        remote_ids.insert(ev.id.to_string());
        local_ids.insert(core_id);
        export_count += 1;
    }

    if !new_lines.is_empty() {
        let mut file = OpenOptions::new()
            .append(true)
            .create(true)
            .open(&events_path)?;
        file.write_all(new_lines.as_bytes())?;
    }

    // Phase 6: Persist state and git commit/push
    save_id_set(&remote_ids_path, &remote_ids)?;
    save_id_set(&local_ids_path, &local_ids)?;

    // Add entire .chronis/ — the inner .gitignore excludes wal/, parquet/,
    // and local state files, so only config + sync exchange are tracked.
    git_add(".chronis/")?;
    if git_has_staged_changes(".chronis/")? {
        let msg = if import_count > 0 && export_count > 0 {
            format!("chronis: sync +{export_count} local, +{import_count} remote")
        } else if export_count > 0 {
            format!("chronis: sync +{export_count} events")
        } else {
            "chronis: sync".to_string()
        };
        git_commit(&msg)?;
        println!("Committed sync changes.");
        git_push();
    } else {
        println!("Nothing to sync — already up to date.");
    }

    Ok(())
}
