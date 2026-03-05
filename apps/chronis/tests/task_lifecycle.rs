use std::sync::Arc;

use allsource_core::embedded::{Config, EmbeddedCore};
use chronis::{
    domain::{error::ChronError, repository::TaskRepository, task::TaskType},
    infrastructure::{core_task_repo::CoreTaskRepository, projection::TaskProjection},
};

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
    repo.create_task(
        "t-0001",
        "Write tests",
        "p1",
        &[],
        TaskType::Task,
        None,
        None,
    )
    .await
    .unwrap();

    let tasks = repo.list_tasks(None).unwrap();
    assert_eq!(tasks.len(), 1);
    assert_eq!(tasks[0].id, "t-0001");
    assert_eq!(tasks[0].title, "Write tests");
    assert_eq!(tasks[0].priority.to_string(), "p1");
    assert_eq!(tasks[0].status.to_string(), "open");
    assert_eq!(tasks[0].task_type, TaskType::Task);
}

#[tokio::test]
async fn full_lifecycle_create_claim_done() {
    let repo = setup().await;
    repo.create_task(
        "t-0001",
        "Build feature",
        "p0",
        &[],
        TaskType::Feature,
        None,
        None,
    )
    .await
    .unwrap();

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
    repo.create_task("t-0001", "Task", "p2", &[], TaskType::Task, None, None)
        .await
        .unwrap();
    repo.claim_task("t-0001", "a").await.unwrap();

    let err = repo.claim_task("t-0001", "b").await.unwrap_err();
    assert!(matches!(err, ChronError::InvalidTransition { .. }));
}

#[tokio::test]
async fn complete_already_done_fails() {
    let repo = setup().await;
    repo.create_task("t-0001", "Task", "p2", &[], TaskType::Task, None, None)
        .await
        .unwrap();
    repo.complete_task("t-0001", None).await.unwrap();

    let err = repo.complete_task("t-0001", None).await.unwrap_err();
    assert!(matches!(err, ChronError::AlreadyDone(_)));
}

#[tokio::test]
async fn ready_excludes_blocked_tasks() {
    let repo = setup().await;
    repo.create_task("t-0001", "Dep", "p2", &[], TaskType::Task, None, None)
        .await
        .unwrap();
    repo.create_task(
        "t-0002",
        "Blocked",
        "p2",
        &["t-0001".to_string()],
        TaskType::Task,
        None,
        None,
    )
    .await
    .unwrap();
    repo.create_task("t-0003", "Free", "p2", &[], TaskType::Task, None, None)
        .await
        .unwrap();

    let ready = repo.ready_tasks().unwrap();
    let ids: Vec<&str> = ready.iter().map(|t| t.id.as_str()).collect();
    assert!(ids.contains(&"t-0001"));
    assert!(ids.contains(&"t-0003"));
    assert!(!ids.contains(&"t-0002"));
}

#[tokio::test]
async fn blocker_resolved_unblocks_dependent() {
    let repo = setup().await;
    repo.create_task("t-0001", "Dep", "p2", &[], TaskType::Task, None, None)
        .await
        .unwrap();
    repo.create_task(
        "t-0002",
        "Blocked",
        "p2",
        &["t-0001".to_string()],
        TaskType::Task,
        None,
        None,
    )
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
    repo.create_task("t-0001", "Task", "p2", &[], TaskType::Task, None, None)
        .await
        .unwrap();
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
    repo.create_task("t-0001", "Review", "p1", &[], TaskType::Task, None, None)
        .await
        .unwrap();
    repo.approve_task("t-0001").await.unwrap();

    let task = repo.get_task("t-0001").unwrap();
    assert_eq!(task.approved, Some(true));
}

#[tokio::test]
async fn add_dependency_post_creation() {
    let repo = setup().await;
    repo.create_task("t-0001", "Blocker", "p1", &[], TaskType::Task, None, None)
        .await
        .unwrap();
    repo.create_task("t-0002", "Dependent", "p2", &[], TaskType::Task, None, None)
        .await
        .unwrap();

    // t-0002 should be ready initially
    let ready = repo.ready_tasks().unwrap();
    assert!(ready.iter().any(|t| t.id == "t-0002"));

    // Add dependency post-creation
    repo.add_dependency("t-0002", "t-0001").await.unwrap();

    // t-0002 should no longer be ready
    let ready = repo.ready_tasks().unwrap();
    assert!(!ready.iter().any(|t| t.id == "t-0002"));

    // Remove dependency
    repo.remove_dependency("t-0002", "t-0001").await.unwrap();

    // t-0002 should be ready again
    let ready = repo.ready_tasks().unwrap();
    assert!(ready.iter().any(|t| t.id == "t-0002"));
}

#[tokio::test]
async fn epic_with_children() {
    let repo = setup().await;
    repo.create_task(
        "t-epic",
        "Auth System",
        "p0",
        &[],
        TaskType::Epic,
        None,
        None,
    )
    .await
    .unwrap();
    repo.create_task(
        "t-child1",
        "Login flow",
        "p1",
        &[],
        TaskType::Task,
        Some("t-epic"),
        None,
    )
    .await
    .unwrap();
    repo.create_task(
        "t-child2",
        "Signup flow",
        "p2",
        &[],
        TaskType::Task,
        Some("t-epic"),
        None,
    )
    .await
    .unwrap();
    // Non-child task
    repo.create_task(
        "t-other",
        "Unrelated",
        "p3",
        &[],
        TaskType::Task,
        None,
        None,
    )
    .await
    .unwrap();

    let children = repo.children_of("t-epic").unwrap();
    assert_eq!(children.len(), 2);
    let child_ids: Vec<&str> = children.iter().map(|t| t.id.as_str()).collect();
    assert!(child_ids.contains(&"t-child1"));
    assert!(child_ids.contains(&"t-child2"));

    // Verify parent is set on the child
    let child = repo.get_task("t-child1").unwrap();
    assert_eq!(child.parent.as_deref(), Some("t-epic"));
    assert_eq!(child.task_type, TaskType::Task);

    // Verify epic type
    let epic = repo.get_task("t-epic").unwrap();
    assert_eq!(epic.task_type, TaskType::Epic);
}

#[tokio::test]
async fn task_with_description() {
    let repo = setup().await;
    repo.create_task(
        "t-0001",
        "Fix login bug",
        "p1",
        &[],
        TaskType::Bug,
        None,
        Some("Users can't login with special characters in password"),
    )
    .await
    .unwrap();

    let task = repo.get_task("t-0001").unwrap();
    assert_eq!(task.task_type, TaskType::Bug);
    assert_eq!(
        task.description.as_deref(),
        Some("Users can't login with special characters in password")
    );
}
