//! TDD RED phase tests for Phase 5: Replicant Worker Protocol.
//!
//! These tests define the target projections for autonomous cloud worker
//! orchestration via event sourcing — no Temporal needed.
//!
//! Tests cover:
//! - Workflow lifecycle (dispatch → claim → steps → output)
//! - First-write-wins claim guard (prevents double-claim)
//! - Human-in-the-loop approval flow
//! - Replicant registry (registration, heartbeat, stale detection)
//! - Task queue projection (unclaimed workflows)
//!
//! Run with: cargo test --features embedded-replicant --test replicant_protocol

#[cfg(feature = "embedded-replicant")]
mod tests {
    use allsource_core::embedded::{Config, EmbeddedCore, IngestEvent};
    use serde_json::json;

    // =========================================================================
    // Workflow Lifecycle
    // =========================================================================

    #[tokio::test]
    async fn workflow_dispatch_creates_pending_status() {
        let core = open_core().await;

        core.ingest(IngestEvent {
            entity_id: "wf-1",
            event_type: "workflow.dispatched",
            payload: json!({
                "name": "summarize",
                "input": "long text...",
                "steps_total": 3
            }),
            metadata: None,
            tenant_id: None,
        })
        .await
        .unwrap();

        let state = core.projection("workflow_status", "wf-1").unwrap();
        assert_eq!(state["status"], "pending");
        assert_eq!(state["steps_total"], 3);
        assert_eq!(state["steps_completed"], 0);
    }

    #[tokio::test]
    async fn workflow_claim_transitions_to_claimed() {
        let core = open_core().await;

        core.ingest(make_event("wf-1", "workflow.dispatched", json!({})))
            .await
            .unwrap();
        core.ingest(make_event(
            "wf-1",
            "workflow.claimed",
            json!({"replicant_id": "replicant-a"}),
        ))
        .await
        .unwrap();

        let state = core.projection("workflow_status", "wf-1").unwrap();
        assert_eq!(state["status"], "claimed");
        assert_eq!(state["replicant_id"], "replicant-a");
    }

    #[tokio::test]
    async fn workflow_claim_guard_rejects_double_claim() {
        let core = open_core().await;

        core.ingest(make_event("wf-1", "workflow.dispatched", json!({})))
            .await
            .unwrap();
        core.ingest(make_event(
            "wf-1",
            "workflow.claimed",
            json!({"replicant_id": "r-1"}),
        ))
        .await
        .unwrap();

        // Second claim by different replicant
        core.ingest(make_event(
            "wf-1",
            "workflow.claimed",
            json!({"replicant_id": "r-2"}),
        ))
        .await
        .unwrap();

        // First claimer wins — projection ignores the second
        let state = core.projection("workflow_status", "wf-1").unwrap();
        assert_eq!(state["replicant_id"], "r-1");
    }

    #[tokio::test]
    async fn workflow_step_progression() {
        let core = open_core().await;

        core.ingest(make_event(
            "wf-1",
            "workflow.dispatched",
            json!({"steps_total": 3}),
        ))
        .await
        .unwrap();
        core.ingest(make_event(
            "wf-1",
            "workflow.claimed",
            json!({"replicant_id": "r-1"}),
        ))
        .await
        .unwrap();

        core.ingest(make_event(
            "wf-1",
            "workflow.step.completed",
            json!({"step_id": 0, "output": "step 0 done"}),
        ))
        .await
        .unwrap();
        core.ingest(make_event(
            "wf-1",
            "workflow.step.completed",
            json!({"step_id": 1, "output": "step 1 done"}),
        ))
        .await
        .unwrap();

        let state = core.projection("workflow_status", "wf-1").unwrap();
        assert_eq!(state["status"], "running");
        assert_eq!(state["steps_completed"], 2);
    }

    #[tokio::test]
    async fn workflow_all_steps_complete_transitions_to_done() {
        let core = open_core().await;

        core.ingest(make_event(
            "wf-1",
            "workflow.dispatched",
            json!({"steps_total": 2}),
        ))
        .await
        .unwrap();
        core.ingest(make_event(
            "wf-1",
            "workflow.claimed",
            json!({"replicant_id": "r-1"}),
        ))
        .await
        .unwrap();
        core.ingest(make_event(
            "wf-1",
            "workflow.step.completed",
            json!({"step_id": 0}),
        ))
        .await
        .unwrap();
        core.ingest(make_event(
            "wf-1",
            "workflow.step.completed",
            json!({"step_id": 1}),
        ))
        .await
        .unwrap();
        core.ingest(make_event(
            "wf-1",
            "workflow.output.ready",
            json!({"result": "final output"}),
        ))
        .await
        .unwrap();

        let state = core.projection("workflow_status", "wf-1").unwrap();
        assert_eq!(state["status"], "completed");
        assert_eq!(state["output"], "final output");
    }

    #[tokio::test]
    async fn workflow_step_failure_transitions_to_failed() {
        let core = open_core().await;

        core.ingest(make_event("wf-1", "workflow.dispatched", json!({})))
            .await
            .unwrap();
        core.ingest(make_event(
            "wf-1",
            "workflow.claimed",
            json!({"replicant_id": "r-1"}),
        ))
        .await
        .unwrap();
        core.ingest(make_event(
            "wf-1",
            "workflow.step.failed",
            json!({"step_id": 0, "error": "OOM", "retryable": false}),
        ))
        .await
        .unwrap();

        let state = core.projection("workflow_status", "wf-1").unwrap();
        assert_eq!(state["status"], "failed");
        assert_eq!(state["error"], "OOM");
    }

    // =========================================================================
    // Human-in-the-Loop Approval
    // =========================================================================

    #[tokio::test]
    async fn approval_request_pauses_workflow() {
        let core = open_core().await;

        core.ingest(make_event("wf-1", "workflow.dispatched", json!({})))
            .await
            .unwrap();
        core.ingest(make_event(
            "wf-1",
            "workflow.claimed",
            json!({"replicant_id": "r-1"}),
        ))
        .await
        .unwrap();
        core.ingest(make_event(
            "wf-1",
            "workflow.approval.requested",
            json!({"reason": "review generated summary"}),
        ))
        .await
        .unwrap();

        let state = core.projection("workflow_status", "wf-1").unwrap();
        assert_eq!(state["status"], "awaiting_approval");
        assert_eq!(state["awaiting_approval"], true);
    }

    #[tokio::test]
    async fn approval_granted_resumes_workflow() {
        let core = open_core().await;

        core.ingest(make_event("wf-1", "workflow.dispatched", json!({})))
            .await
            .unwrap();
        core.ingest(make_event(
            "wf-1",
            "workflow.claimed",
            json!({"replicant_id": "r-1"}),
        ))
        .await
        .unwrap();
        core.ingest(make_event(
            "wf-1",
            "workflow.approval.requested",
            json!({"reason": "confirm deploy"}),
        ))
        .await
        .unwrap();
        core.ingest(make_event("wf-1", "workflow.approval.granted", json!({})))
            .await
            .unwrap();

        let state = core.projection("workflow_status", "wf-1").unwrap();
        assert_eq!(state["status"], "running");
        assert_eq!(state["awaiting_approval"], false);
    }

    #[tokio::test]
    async fn approval_rejected_fails_workflow() {
        let core = open_core().await;

        core.ingest(make_event("wf-1", "workflow.dispatched", json!({})))
            .await
            .unwrap();
        core.ingest(make_event(
            "wf-1",
            "workflow.claimed",
            json!({"replicant_id": "r-1"}),
        ))
        .await
        .unwrap();
        core.ingest(make_event(
            "wf-1",
            "workflow.approval.requested",
            json!({"reason": "risky action"}),
        ))
        .await
        .unwrap();
        core.ingest(make_event(
            "wf-1",
            "workflow.approval.rejected",
            json!({"reason": "not safe"}),
        ))
        .await
        .unwrap();

        let state = core.projection("workflow_status", "wf-1").unwrap();
        assert_eq!(state["status"], "rejected");
    }

    // =========================================================================
    // Replicant Registry
    // =========================================================================

    #[tokio::test]
    async fn replicant_registers_with_capabilities() {
        let core = open_core().await;

        core.ingest(IngestEvent {
            entity_id: "r-1",
            event_type: "replicant.registered",
            payload: json!({
                "capabilities": ["summarize", "translate", "code_review"]
            }),
            metadata: None,
            tenant_id: None,
        })
        .await
        .unwrap();

        let state = core.projection("replicant_registry", "r-1").unwrap();
        assert_eq!(state["status"], "active");
        assert_eq!(state["capabilities"].as_array().unwrap().len(), 3);
    }

    #[tokio::test]
    async fn replicant_heartbeat_updates_last_seen() {
        let core = open_core().await;

        core.ingest(make_event(
            "r-1",
            "replicant.registered",
            json!({"capabilities": ["summarize"]}),
        ))
        .await
        .unwrap();
        core.ingest(make_event("r-1", "replicant.heartbeat", json!({})))
            .await
            .unwrap();

        let state = core.projection("replicant_registry", "r-1").unwrap();
        assert_eq!(state["status"], "active");
        assert!(state["last_heartbeat"].is_string()); // ISO 8601 timestamp
    }

    #[tokio::test]
    async fn replicant_stale_marks_as_stale() {
        let core = open_core().await;

        core.ingest(make_event(
            "r-1",
            "replicant.registered",
            json!({"capabilities": []}),
        ))
        .await
        .unwrap();
        core.ingest(make_event("r-1", "replicant.stale", json!({})))
            .await
            .unwrap();

        let state = core.projection("replicant_registry", "r-1").unwrap();
        assert_eq!(state["status"], "stale");
    }

    // =========================================================================
    // Task Queue Projection
    // =========================================================================

    #[tokio::test]
    async fn task_queue_lists_unclaimed_workflows() {
        let core = open_core().await;

        core.ingest(make_event("wf-1", "workflow.dispatched", json!({})))
            .await
            .unwrap();
        core.ingest(make_event("wf-2", "workflow.dispatched", json!({})))
            .await
            .unwrap();
        core.ingest(make_event("wf-3", "workflow.dispatched", json!({})))
            .await
            .unwrap();

        // Claim wf-1 only
        core.ingest(make_event(
            "wf-1",
            "workflow.claimed",
            json!({"replicant_id": "r-1"}),
        ))
        .await
        .unwrap();

        // Task queue should show wf-2 and wf-3 (unclaimed)
        let queue = core.projection("task_queue", "__all").unwrap();
        let pending = queue["pending"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap())
            .collect::<Vec<_>>();

        assert_eq!(pending.len(), 2);
        assert!(pending.contains(&"wf-2"));
        assert!(pending.contains(&"wf-3"));
        assert!(!pending.contains(&"wf-1"));
    }

    #[tokio::test]
    async fn task_queue_removes_completed_workflows() {
        let core = open_core().await;

        core.ingest(make_event("wf-1", "workflow.dispatched", json!({})))
            .await
            .unwrap();
        core.ingest(make_event(
            "wf-1",
            "workflow.claimed",
            json!({"replicant_id": "r-1"}),
        ))
        .await
        .unwrap();
        core.ingest(make_event(
            "wf-1",
            "workflow.output.ready",
            json!({"result": "done"}),
        ))
        .await
        .unwrap();

        let queue = core.projection("task_queue", "__all").unwrap();
        let pending = queue["pending"].as_array().unwrap();
        assert!(pending.is_empty());
    }

    // =========================================================================
    // Helpers
    // =========================================================================

    async fn open_core() -> EmbeddedCore {
        EmbeddedCore::open(Config::builder().build().unwrap())
            .await
            .unwrap()
    }

    fn make_event<'a>(
        entity_id: &'a str,
        event_type: &'a str,
        payload: serde_json::Value,
    ) -> IngestEvent<'a> {
        IngestEvent {
            entity_id,
            event_type,
            payload,
            metadata: None,
            tenant_id: None,
        }
    }
}
