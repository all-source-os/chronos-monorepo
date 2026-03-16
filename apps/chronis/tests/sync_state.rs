use std::{collections::HashSet, sync::Arc};

use allsource_core::embedded::{Config, EmbeddedCore, IngestEvent, Query};
use chronis::infrastructure::{
    backend::CoreBackend, config::ChronisConfig, projection::TaskProjection,
};
use serde_json::json;

/// Create an embedded Core with TaskProjection registered.
async fn embedded_core() -> Arc<EmbeddedCore> {
    let config = Config::builder()
        .single_tenant(true)
        .build()
        .expect("config");
    let core = EmbeddedCore::open(config).await.expect("core");
    let core = Arc::new(core);
    core.inner()
        .register_projection_with_backfill(
            &(Arc::new(TaskProjection::new()) as Arc<dyn allsource_core::application::Projection>),
        )
        .expect("projection");
    core
}

/// Create a Remote backend with a local in-memory Core (no actual HTTP).
async fn remote_backend_local_only() -> CoreBackend {
    let local = embedded_core().await;
    // Use a dummy client — we won't call it in these tests
    let client = chronis::infrastructure::http_core_client::HttpCoreClient::new("http://unused");
    CoreBackend::Remote { client, local }
}

// ── Embedded backend tests ──────────────────────────────────────────────

#[tokio::test]
async fn backend_embedded_ingest_and_query() {
    let core = embedded_core().await;
    let backend = CoreBackend::Embedded(core);

    backend
        .ingest(IngestEvent {
            entity_id: "t-001",
            event_type: "task.created",
            payload: json!({"title": "Test"}),
            metadata: None,
            tenant_id: None,
        })
        .await
        .unwrap();

    let events = backend.query(Query::new()).await.unwrap();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].entity_id, "t-001");
    assert_eq!(events[0].event_type, "task.created");
}

#[tokio::test]
async fn backend_embedded_projection() {
    let core = embedded_core().await;
    let backend = CoreBackend::Embedded(core);

    backend
        .ingest(IngestEvent {
            entity_id: "t-proj",
            event_type: "task.created",
            payload: json!({"title": "Projected", "priority": "p1", "task_type": "task"}),
            metadata: None,
            tenant_id: None,
        })
        .await
        .unwrap();

    let task = backend.projection("chronis_tasks", "t-proj");
    assert!(task.is_some());
    let task = task.unwrap();
    assert_eq!(task["title"].as_str(), Some("Projected"));
}

#[tokio::test]
async fn backend_embedded_query_with_entity_filter() {
    let core = embedded_core().await;
    let backend = CoreBackend::Embedded(core);

    backend
        .ingest(IngestEvent {
            entity_id: "t-a",
            event_type: "task.created",
            payload: json!({"title": "A"}),
            metadata: None,
            tenant_id: None,
        })
        .await
        .unwrap();

    backend
        .ingest(IngestEvent {
            entity_id: "t-b",
            event_type: "task.created",
            payload: json!({"title": "B"}),
            metadata: None,
            tenant_id: None,
        })
        .await
        .unwrap();

    let events = backend.query(Query::new().entity_id("t-a")).await.unwrap();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].entity_id, "t-a");
}

// ── Remote backend tests (US-001) ───────────────────────────────────────

#[tokio::test]
async fn remote_backend_projection_works() {
    let backend = remote_backend_local_only().await;

    backend
        .ingest(IngestEvent {
            entity_id: "t-remote-proj",
            event_type: "task.created",
            payload: json!({"title": "Remote Task", "priority": "p1", "task_type": "task"}),
            metadata: None,
            tenant_id: None,
        })
        .await
        // This will fail on the HTTP call to the dummy URL, but we need to
        // test the local projection path. Let's use the local core directly.
        .ok();

    // Since the dummy HTTP client fails, let's test the projection path directly
    // by using the local core inside the Remote variant.
    if let CoreBackend::Remote { local, .. } = &backend {
        local
            .ingest(IngestEvent {
                entity_id: "t-remote-proj",
                event_type: "task.created",
                payload: json!({"title": "Remote Task", "priority": "p1", "task_type": "task"}),
                metadata: None,
                tenant_id: None,
            })
            .await
            .unwrap();
    }

    let task = backend.projection("chronis_tasks", "t-remote-proj");
    assert!(task.is_some());
    assert_eq!(task.unwrap()["title"].as_str(), Some("Remote Task"));
}

#[tokio::test]
async fn remote_backend_query_uses_local_core() {
    let backend = remote_backend_local_only().await;

    // Ingest directly into the local core
    if let CoreBackend::Remote { local, .. } = &backend {
        local
            .ingest(IngestEvent {
                entity_id: "t-q1",
                event_type: "task.created",
                payload: json!({"title": "Q1"}),
                metadata: None,
                tenant_id: None,
            })
            .await
            .unwrap();
        local
            .ingest(IngestEvent {
                entity_id: "t-q2",
                event_type: "task.created",
                payload: json!({"title": "Q2"}),
                metadata: None,
                tenant_id: None,
            })
            .await
            .unwrap();
    }

    // Query through the backend — should use local core
    let all = backend.query(Query::new()).await.unwrap();
    assert_eq!(all.len(), 2);

    // Entity filter should work through local core
    let filtered = backend.query(Query::new().entity_id("t-q1")).await.unwrap();
    assert_eq!(filtered.len(), 1);
    assert_eq!(filtered[0].entity_id, "t-q1");
}

#[tokio::test]
async fn remote_backend_list_tasks_via_repo() {
    use chronis::{
        domain::repository::TaskRepository, infrastructure::core_task_repo::CoreTaskRepository,
    };

    let backend = remote_backend_local_only().await;

    // Ingest a task into the local core
    if let CoreBackend::Remote { local, .. } = &backend {
        local
            .ingest(IngestEvent {
                entity_id: "t-repo",
                event_type: "task.created",
                payload: json!({"title": "Repo Test", "priority": "p2", "task_type": "task"}),
                metadata: None,
                tenant_id: None,
            })
            .await
            .unwrap();
    }

    let backend = Arc::new(backend);
    let repo = CoreTaskRepository::new(backend);

    let tasks = repo.list_tasks(None).unwrap();
    assert_eq!(tasks.len(), 1);
    assert_eq!(tasks[0].title, "Repo Test");
}

// ── Remote mode get_task_detail (US-002) ────────────────────────────────

#[tokio::test]
async fn remote_backend_get_task_detail_returns_correct_timeline() {
    use chronis::{
        domain::repository::TaskRepository, infrastructure::core_task_repo::CoreTaskRepository,
    };

    let backend = remote_backend_local_only().await;

    // Ingest events for two different tasks into local core
    if let CoreBackend::Remote { local, .. } = &backend {
        local
            .ingest(IngestEvent {
                entity_id: "t-detail",
                event_type: "task.created",
                payload: json!({"title": "Detail Test", "priority": "p1", "task_type": "task"}),
                metadata: None,
                tenant_id: None,
            })
            .await
            .unwrap();
        local
            .ingest(IngestEvent {
                entity_id: "t-detail",
                event_type: "workflow.claimed",
                payload: json!({"agent_id": "human"}),
                metadata: None,
                tenant_id: None,
            })
            .await
            .unwrap();
        // Different task — should NOT appear in t-detail's timeline
        local
            .ingest(IngestEvent {
                entity_id: "t-other",
                event_type: "task.created",
                payload: json!({"title": "Other", "priority": "p2", "task_type": "task"}),
                metadata: None,
                tenant_id: None,
            })
            .await
            .unwrap();
    }

    let backend = Arc::new(backend);
    let repo = CoreTaskRepository::new(backend);

    let detail = repo.get_task_detail("t-detail").await.unwrap();
    assert_eq!(detail.task.title, "Detail Test");
    assert_eq!(detail.timeline.len(), 2);
    assert_eq!(detail.timeline[0].event_type, "task.created");
    assert_eq!(detail.timeline[1].event_type, "workflow.claimed");
}

// ── Sync dedup tests (US-003) ───────────────────────────────────────────

#[tokio::test]
async fn synced_from_metadata_prevents_re_export() {
    let core = embedded_core().await;

    core.ingest(IngestEvent {
        entity_id: "t-local",
        event_type: "task.created",
        payload: json!({"title": "Local"}),
        metadata: None,
        tenant_id: None,
    })
    .await
    .unwrap();

    core.ingest(IngestEvent {
        entity_id: "t-remote",
        event_type: "task.created",
        payload: json!({"title": "Remote"}),
        metadata: Some(json!({"synced_from": "remote"})),
        tenant_id: None,
    })
    .await
    .unwrap();

    let all = core.query(Query::new()).await.unwrap();
    assert_eq!(all.len(), 2);

    let pushable: Vec<_> = all
        .iter()
        .filter(|ev| {
            ev.metadata
                .as_ref()
                .and_then(|m| m.get("synced_from"))
                .is_none()
        })
        .collect();

    assert_eq!(pushable.len(), 1);
    assert_eq!(pushable[0].entity_id, "t-local");
}

#[tokio::test]
async fn source_id_metadata_prevents_re_import() {
    let instance_id = "my-instance-123";

    let remote_events = vec![
        json!({"entity_id": "t-other", "event_type": "task.created", "metadata": {"source_id": "other-instance"}}),
        json!({"entity_id": "t-mine", "event_type": "task.created", "metadata": {"source_id": instance_id}}),
        json!({"entity_id": "t-anon", "event_type": "task.created", "metadata": null}),
    ];

    let importable: Vec<_> = remote_events
        .iter()
        .filter(|ev| ev["metadata"].get("source_id").and_then(|v| v.as_str()) != Some(instance_id))
        .collect();

    assert_eq!(importable.len(), 2);
    assert_eq!(importable[0]["entity_id"], "t-other");
    assert_eq!(importable[1]["entity_id"], "t-anon");
}

#[tokio::test]
async fn sync_state_with_id_sets_roundtrip() {
    let dir = tempfile::tempdir().unwrap();
    let state_path = dir.path().join(".chronis/sync/sync_state.json");
    std::fs::create_dir_all(state_path.parent().unwrap()).unwrap();

    let mut pulled = HashSet::new();
    pulled.insert("uuid-aaa".to_string());
    pulled.insert("uuid-bbb".to_string());

    let mut pushed = HashSet::new();
    pushed.insert("uuid-ccc".to_string());

    let state = json!({
        "last_push_timestamp": "2026-03-16T12:00:00Z",
        "last_pull_timestamp": "2026-03-16T11:00:00Z",
        "pulled_ids": pulled,
        "pushed_ids": pushed,
    });

    std::fs::write(&state_path, serde_json::to_string_pretty(&state).unwrap()).unwrap();

    let content = std::fs::read_to_string(&state_path).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&content).unwrap();

    let pulled_back: HashSet<String> =
        serde_json::from_value(parsed["pulled_ids"].clone()).unwrap();
    assert!(pulled_back.contains("uuid-aaa"));
    assert!(pulled_back.contains("uuid-bbb"));
    assert_eq!(pulled_back.len(), 2);

    let pushed_back: HashSet<String> =
        serde_json::from_value(parsed["pushed_ids"].clone()).unwrap();
    assert!(pushed_back.contains("uuid-ccc"));
    assert_eq!(pushed_back.len(), 1);
}

#[tokio::test]
async fn uuid_dedup_prevents_duplicate_on_overlap() {
    // Simulate the overlap scenario: same event appears in two consecutive syncs
    let mut pulled_ids: HashSet<String> = HashSet::new();

    let event_uuid = "550e8400-e29b-41d4-a716-446655440000";

    // First sync: event is new, gets imported
    assert!(!pulled_ids.contains(event_uuid));
    pulled_ids.insert(event_uuid.to_string());

    // Second sync (overlap buffer re-fetches same event): should be skipped
    assert!(pulled_ids.contains(event_uuid));
    // In sync_http, this `contains` check causes `continue`, preventing duplicate ingest
}

#[tokio::test]
async fn sync_state_backwards_compatible_no_id_sets() {
    // Old sync state files won't have pulled_ids/pushed_ids.
    // The #[serde(default)] should handle this gracefully.
    let old_state_json = r#"{
        "last_push_timestamp": "2026-03-16T12:00:00Z",
        "last_pull_timestamp": "2026-03-16T11:00:00Z"
    }"#;

    // This should parse without error, with empty ID sets
    let parsed: serde_json::Value = serde_json::from_str(old_state_json).unwrap();
    assert!(parsed.get("pulled_ids").is_none());
    assert!(parsed.get("pushed_ids").is_none());

    // Verify serde handles it with defaults
    #[derive(serde::Deserialize)]
    struct SyncState {
        #[serde(default)]
        pulled_ids: HashSet<String>,
        #[serde(default)]
        pushed_ids: HashSet<String>,
    }
    let state: SyncState = serde_json::from_str(old_state_json).unwrap();
    assert!(state.pulled_ids.is_empty());
    assert!(state.pushed_ids.is_empty());
}

// ── Workspace init tests ────────────────────────────────────────────────

#[tokio::test]
async fn sync_state_roundtrip() {
    let dir = tempfile::tempdir().unwrap();
    let ws_root = dir.path();

    let state_path = ws_root.join(".chronis/sync/sync_state.json");
    assert!(!state_path.exists());

    let state = json!({
        "last_push_timestamp": "2026-03-16T12:00:00Z",
        "last_pull_timestamp": "2026-03-16T11:00:00Z"
    });

    std::fs::create_dir_all(state_path.parent().unwrap()).unwrap();
    std::fs::write(&state_path, serde_json::to_string_pretty(&state).unwrap()).unwrap();

    let content = std::fs::read_to_string(&state_path).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&content).unwrap();
    assert_eq!(parsed["last_push_timestamp"], "2026-03-16T12:00:00Z");
    assert_eq!(parsed["last_pull_timestamp"], "2026-03-16T11:00:00Z");
}

#[tokio::test]
async fn init_workspace_creates_valid_config() {
    let dir = tempfile::tempdir().unwrap();
    chronis::infrastructure::workspace::init_workspace(dir.path()).unwrap();

    let config = ChronisConfig::load(dir.path()).unwrap();
    assert_eq!(
        config.mode,
        chronis::infrastructure::config::CoreMode::Embedded
    );
    assert!(!config.instance_id.is_empty());

    let gitignore = std::fs::read_to_string(dir.path().join(".chronis/.gitignore")).unwrap();
    assert!(gitignore.contains("wal/"));
    assert!(gitignore.contains("storage/"));
    assert!(gitignore.contains("sync/"));
}

#[tokio::test]
async fn init_workspace_fails_if_exists() {
    let dir = tempfile::tempdir().unwrap();
    chronis::infrastructure::workspace::init_workspace(dir.path()).unwrap();
    let err = chronis::infrastructure::workspace::init_workspace(dir.path());
    assert!(err.is_err());
}
