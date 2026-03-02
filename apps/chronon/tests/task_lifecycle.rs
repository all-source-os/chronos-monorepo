use std::sync::Arc;

use allsource_core::embedded::{Config, EmbeddedCore};
use chronon::domain::error::ChronError;
use chronon::domain::repository::TaskRepository;
use chronon::infrastructure::core_task_repo::CoreTaskRepository;
use chronon::infrastructure::projection::TaskProjection;

async fn setup() -> CoreTaskRepository {
    let config = Config::builder()
        .single_tenant(true)
        .build()
        .expect("config");
    let core = EmbeddedCore::open(config).await.expect("core");
    let core = Arc::new(core);
    core.inner()
        .register_projection_with_backfill(Arc::new(TaskProjection::new()))
        .expect("projection");
    CoreTaskRepository::new(core)
}

#[tokio::test]
async fn create_and_list_task() {
    let repo = setup().await;
    repo.create_task("t-0001", "Write tests", "p1", &[]).await.unwrap();

    let tasks = repo.list_tasks(None).unwrap();
    assert_eq!(tasks.len(), 1);
    assert_eq!(tasks[0].id, "t-0001");
    assert_eq!(tasks[0].title, "Write tests");
    assert_eq!(tasks[0].priority.to_string(), "p1");
    assert_eq!(tasks[0].status.to_string(), "open");
}

#[tokio::test]
async fn full_lifecycle_create_claim_done() {
    let repo = setup().await;
    repo.create_task("t-0001", "Build feature", "p0", &[]).await.unwrap();

    repo.claim_task("t-0001", "agent-1").await.unwrap();
    let task = repo.get_task("t-0001").unwrap();
    assert_eq!(task.status.to_string(), "in-progress");
    assert_eq!(task.claimed_by.as_deref(), Some("agent-1"));

    repo.complete_task("t-0001", Some("shipped")).await.unwrap();
    let task = repo.get_task("t-0001").unwrap();
    assert_eq!(task.status.to_string(), "done");
    assert_eq!(task.done_reason.as_deref(), Some("shipped"));
}

#[tokio::test]
async fn claim_non_open_task_fails() {
    let repo = setup().await;
    repo.create_task("t-0001", "Task", "p2", &[]).await.unwrap();
    repo.claim_task("t-0001", "a").await.unwrap();

    let err = repo.claim_task("t-0001", "b").await.unwrap_err();
    assert!(matches!(err, ChronError::InvalidTransition { .. }));
}

#[tokio::test]
async fn complete_already_done_fails() {
    let repo = setup().await;
    repo.create_task("t-0001", "Task", "p2", &[]).await.unwrap();
    repo.complete_task("t-0001", None).await.unwrap();

    let err = repo.complete_task("t-0001", None).await.unwrap_err();
    assert!(matches!(err, ChronError::AlreadyDone(_)));
}

#[tokio::test]
async fn ready_excludes_blocked_tasks() {
    let repo = setup().await;
    repo.create_task("t-0001", "Dep", "p2", &[]).await.unwrap();
    repo.create_task("t-0002", "Blocked", "p2", &["t-0001".to_string()])
        .await
        .unwrap();
    repo.create_task("t-0003", "Free", "p2", &[]).await.unwrap();

    let ready = repo.ready_tasks().unwrap();
    let ids: Vec<&str> = ready.iter().map(|t| t.id.as_str()).collect();
    assert!(ids.contains(&"t-0001"));
    assert!(ids.contains(&"t-0003"));
    assert!(!ids.contains(&"t-0002"));
}

#[tokio::test]
async fn blocker_resolved_unblocks_dependent() {
    let repo = setup().await;
    repo.create_task("t-0001", "Dep", "p2", &[]).await.unwrap();
    repo.create_task("t-0002", "Blocked", "p2", &["t-0001".to_string()])
        .await
        .unwrap();

    // t-0002 should NOT be ready
    let ready = repo.ready_tasks().unwrap();
    assert!(!ready.iter().any(|t| t.id == "t-0002"));

    // Complete the blocker
    repo.complete_task("t-0001", None).await.unwrap();

    // Now t-0002 should be ready
    let ready = repo.ready_tasks().unwrap();
    assert!(ready.iter().any(|t| t.id == "t-0002"));
}

#[tokio::test]
async fn get_task_returns_timeline() {
    let repo = setup().await;
    repo.create_task("t-0001", "Task", "p2", &[]).await.unwrap();
    repo.claim_task("t-0001", "human").await.unwrap();

    let detail = repo.get_task_detail("t-0001").await.unwrap();
    assert_eq!(detail.task.id, "t-0001");
    assert_eq!(detail.timeline.len(), 2);
    assert_eq!(detail.timeline[0].event_type, "task.created");
    assert_eq!(detail.timeline[1].event_type, "workflow.claimed");
}

#[tokio::test]
async fn approve_task() {
    let repo = setup().await;
    repo.create_task("t-0001", "Review", "p1", &[]).await.unwrap();
    repo.approve_task("t-0001").await.unwrap();

    let task = repo.get_task("t-0001").unwrap();
    assert_eq!(task.approved, Some(true));
}
