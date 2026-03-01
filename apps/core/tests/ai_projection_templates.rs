//! TDD RED phase tests for Phase 7: AI Workflow Projection Templates.
//!
//! These tests define the target projections for common AI agent patterns:
//! - Token usage / cost tracking per entity
//! - MCP tool call audit (success rate, latency percentiles)
//! - Human-in-the-loop approval queue
//! - Agent utilization (active/idle/capacity)
//!
//! Run with: cargo test --features embedded-projections --test ai_projection_templates

#[cfg(feature = "embedded-projections")]
mod tests {
    use allsource_core::embedded::{Config, EmbeddedCore, IngestEvent};
    use serde_json::json;

    // =========================================================================
    // Token Usage / Cost Tracking Projection
    // =========================================================================

    #[tokio::test]
    async fn token_usage_sums_costs_per_entity() {
        let core = open_core().await;

        core.ingest(IngestEvent {
            entity_id: "wf-1",
            event_type: "llm.call.completed",
            payload: json!({
                "model": "claude-sonnet-4-20250514",
                "input_tokens": 1500,
                "output_tokens": 800,
                "cost_usd": 0.0078,
                "latency_ms": 2340
            }),
            metadata: None,
            tenant_id: None,
        })
        .await
        .unwrap();

        core.ingest(IngestEvent {
            entity_id: "wf-1",
            event_type: "llm.call.completed",
            payload: json!({
                "model": "claude-sonnet-4-20250514",
                "input_tokens": 500,
                "output_tokens": 200,
                "cost_usd": 0.002,
                "latency_ms": 1100
            }),
            metadata: None,
            tenant_id: None,
        })
        .await
        .unwrap();

        // Different workflow — should not affect wf-1's totals
        core.ingest(IngestEvent {
            entity_id: "wf-2",
            event_type: "llm.call.completed",
            payload: json!({
                "model": "claude-haiku-4-5-20251001",
                "input_tokens": 1000,
                "output_tokens": 500,
                "cost_usd": 0.001,
                "latency_ms": 500
            }),
            metadata: None,
            tenant_id: None,
        })
        .await
        .unwrap();

        let usage = core.projection("token_usage", "wf-1").unwrap();
        assert_eq!(usage["total_input_tokens"], 2000);
        assert_eq!(usage["total_output_tokens"], 1000);

        // Floating point: compare with tolerance
        let cost = usage["total_cost_usd"].as_f64().unwrap();
        assert!((cost - 0.0098).abs() < 0.0001);

        assert_eq!(usage["calls_count"], 2);

        // wf-2 should have its own totals
        let usage_2 = core.projection("token_usage", "wf-2").unwrap();
        assert_eq!(usage_2["total_input_tokens"], 1000);
        assert_eq!(usage_2["calls_count"], 1);
    }

    #[tokio::test]
    async fn token_usage_tracks_per_model_breakdown() {
        let core = open_core().await;

        core.ingest(IngestEvent {
            entity_id: "wf-1",
            event_type: "llm.call.completed",
            payload: json!({
                "model": "claude-sonnet-4-20250514",
                "input_tokens": 1000, "output_tokens": 500,
                "cost_usd": 0.005, "latency_ms": 2000
            }),
            metadata: None,
            tenant_id: None,
        })
        .await
        .unwrap();

        core.ingest(IngestEvent {
            entity_id: "wf-1",
            event_type: "llm.call.completed",
            payload: json!({
                "model": "claude-haiku-4-5-20251001",
                "input_tokens": 2000, "output_tokens": 1000,
                "cost_usd": 0.001, "latency_ms": 300
            }),
            metadata: None,
            tenant_id: None,
        })
        .await
        .unwrap();

        let usage = core.projection("token_usage", "wf-1").unwrap();

        // Per-model breakdown
        let by_model = &usage["by_model"];
        assert_eq!(by_model["claude-sonnet-4-20250514"]["calls"], 1);
        assert_eq!(by_model["claude-haiku-4-5-20251001"]["calls"], 1);
        assert_eq!(by_model["claude-sonnet-4-20250514"]["input_tokens"], 1000);
        assert_eq!(by_model["claude-haiku-4-5-20251001"]["input_tokens"], 2000);
    }

    #[tokio::test]
    async fn token_usage_returns_none_for_unknown_entity() {
        let core = open_core().await;
        let usage = core.projection("token_usage", "nonexistent");
        assert!(usage.is_none());
    }

    // =========================================================================
    // MCP Tool Call Audit Projection
    // =========================================================================

    #[tokio::test]
    async fn tool_call_audit_tracks_success_and_failure() {
        let core = open_core().await;

        // Two successful calls
        core.ingest(IngestEvent {
            entity_id: "wf-1",
            event_type: "mcp.tool.result",
            payload: json!({"tool_name": "read_file", "duration_ms": 50}),
            metadata: None,
            tenant_id: None,
        })
        .await
        .unwrap();

        core.ingest(IngestEvent {
            entity_id: "wf-1",
            event_type: "mcp.tool.result",
            payload: json!({"tool_name": "read_file", "duration_ms": 120}),
            metadata: None,
            tenant_id: None,
        })
        .await
        .unwrap();

        // One failure
        core.ingest(IngestEvent {
            entity_id: "wf-1",
            event_type: "mcp.tool.error",
            payload: json!({
                "tool_name": "read_file",
                "error": "file not found",
                "retryable": true
            }),
            metadata: None,
            tenant_id: None,
        })
        .await
        .unwrap();

        let audit = core.projection("tool_call_audit", "wf-1").unwrap();
        let read_file = &audit["read_file"];

        assert_eq!(read_file["total_calls"], 3);
        assert_eq!(read_file["successes"], 2);
        assert_eq!(read_file["failures"], 1);

        // Success rate: 2/3 ≈ 0.667
        let rate = read_file["success_rate"].as_f64().unwrap();
        assert!(rate > 0.6 && rate < 0.7);
    }

    #[tokio::test]
    async fn tool_call_audit_tracks_latency_stats() {
        let core = open_core().await;

        let durations = [10, 20, 30, 50, 100, 150, 200, 500, 1000, 2000];
        for d in &durations {
            core.ingest(IngestEvent {
                entity_id: "wf-1",
                event_type: "mcp.tool.result",
                payload: json!({"tool_name": "web_search", "duration_ms": d}),
                metadata: None,
                tenant_id: None,
            })
            .await
            .unwrap();
        }

        let audit = core.projection("tool_call_audit", "wf-1").unwrap();
        let ws = &audit["web_search"];

        assert_eq!(ws["total_calls"], 10);

        // P50 should be around 75ms (median of 10, 20, 30, 50, 100, 150, 200, 500, 1000, 2000)
        let p50 = ws["p50_ms"].as_f64().unwrap();
        assert!(p50 > 50.0 && p50 < 200.0);

        // P95 should be high
        let p95 = ws["p95_ms"].as_f64().unwrap();
        assert!(p95 >= 1000.0);
    }

    #[tokio::test]
    async fn tool_call_audit_multiple_tools() {
        let core = open_core().await;

        core.ingest(IngestEvent {
            entity_id: "wf-1",
            event_type: "mcp.tool.result",
            payload: json!({"tool_name": "read_file", "duration_ms": 50}),
            metadata: None,
            tenant_id: None,
        })
        .await
        .unwrap();

        core.ingest(IngestEvent {
            entity_id: "wf-1",
            event_type: "mcp.tool.result",
            payload: json!({"tool_name": "web_search", "duration_ms": 500}),
            metadata: None,
            tenant_id: None,
        })
        .await
        .unwrap();

        core.ingest(IngestEvent {
            entity_id: "wf-1",
            event_type: "mcp.tool.result",
            payload: json!({"tool_name": "execute_code", "duration_ms": 2000}),
            metadata: None,
            tenant_id: None,
        })
        .await
        .unwrap();

        let audit = core.projection("tool_call_audit", "wf-1").unwrap();

        // Should have entries for all three tools
        assert!(audit["read_file"].is_object());
        assert!(audit["web_search"].is_object());
        assert!(audit["execute_code"].is_object());
    }

    // =========================================================================
    // Human-in-the-Loop Queue Projection
    // =========================================================================

    #[tokio::test]
    async fn hitl_queue_tracks_pending_approvals() {
        let core = open_core().await;

        core.ingest(IngestEvent {
            entity_id: "wf-1",
            event_type: "workflow.approval.requested",
            payload: json!({"reason": "review generated summary"}),
            metadata: None,
            tenant_id: None,
        })
        .await
        .unwrap();

        core.ingest(IngestEvent {
            entity_id: "wf-2",
            event_type: "workflow.approval.requested",
            payload: json!({"reason": "confirm deployment"}),
            metadata: None,
            tenant_id: None,
        })
        .await
        .unwrap();

        // Approve wf-1
        core.ingest(IngestEvent {
            entity_id: "wf-1",
            event_type: "workflow.approval.granted",
            payload: json!({}),
            metadata: None,
            tenant_id: None,
        })
        .await
        .unwrap();

        let queue = core.projection("human_in_loop_queue", "__all").unwrap();
        let pending = queue["pending_approvals"].as_array().unwrap();

        // Only wf-2 should remain pending
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0]["entity_id"], "wf-2");
        assert_eq!(pending[0]["reason"], "confirm deployment");
    }

    #[tokio::test]
    async fn hitl_queue_sorted_by_age() {
        let core = open_core().await;

        // Request approvals in order
        core.ingest(IngestEvent {
            entity_id: "wf-oldest",
            event_type: "workflow.approval.requested",
            payload: json!({"reason": "first"}),
            metadata: None,
            tenant_id: None,
        })
        .await
        .unwrap();

        core.ingest(IngestEvent {
            entity_id: "wf-newer",
            event_type: "workflow.approval.requested",
            payload: json!({"reason": "second"}),
            metadata: None,
            tenant_id: None,
        })
        .await
        .unwrap();

        core.ingest(IngestEvent {
            entity_id: "wf-newest",
            event_type: "workflow.approval.requested",
            payload: json!({"reason": "third"}),
            metadata: None,
            tenant_id: None,
        })
        .await
        .unwrap();

        let queue = core.projection("human_in_loop_queue", "__all").unwrap();
        let pending = queue["pending_approvals"].as_array().unwrap();

        assert_eq!(pending.len(), 3);
        // Oldest first
        assert_eq!(pending[0]["entity_id"], "wf-oldest");
        assert_eq!(pending[1]["entity_id"], "wf-newer");
        assert_eq!(pending[2]["entity_id"], "wf-newest");
    }

    #[tokio::test]
    async fn hitl_queue_rejection_removes_from_pending() {
        let core = open_core().await;

        core.ingest(IngestEvent {
            entity_id: "wf-1",
            event_type: "workflow.approval.requested",
            payload: json!({"reason": "risky"}),
            metadata: None,
            tenant_id: None,
        })
        .await
        .unwrap();

        core.ingest(IngestEvent {
            entity_id: "wf-1",
            event_type: "workflow.approval.rejected",
            payload: json!({"reason": "too dangerous"}),
            metadata: None,
            tenant_id: None,
        })
        .await
        .unwrap();

        let queue = core.projection("human_in_loop_queue", "__all").unwrap();
        let pending = queue["pending_approvals"].as_array().unwrap();
        assert!(pending.is_empty());
    }

    // =========================================================================
    // Agent Utilization Projection
    // =========================================================================

    #[tokio::test]
    async fn agent_utilization_tracks_capacity() {
        let core = open_core().await;

        // Register two replicants
        core.ingest(IngestEvent {
            entity_id: "r-1",
            event_type: "replicant.registered",
            payload: json!({"capabilities": ["summarize"]}),
            metadata: None,
            tenant_id: None,
        })
        .await
        .unwrap();

        core.ingest(IngestEvent {
            entity_id: "r-2",
            event_type: "replicant.registered",
            payload: json!({"capabilities": ["translate"]}),
            metadata: None,
            tenant_id: None,
        })
        .await
        .unwrap();

        // r-1 claims a workflow
        core.ingest(IngestEvent {
            entity_id: "wf-1",
            event_type: "workflow.dispatched",
            payload: json!({}),
            metadata: None,
            tenant_id: None,
        })
        .await
        .unwrap();
        core.ingest(IngestEvent {
            entity_id: "wf-1",
            event_type: "workflow.claimed",
            payload: json!({"replicant_id": "r-1"}),
            metadata: None,
            tenant_id: None,
        })
        .await
        .unwrap();

        let util = core.projection("agent_utilization", "__all").unwrap();
        assert_eq!(util["total_capacity"], 2);
        assert_eq!(util["active"], 1); // r-1 is working
        assert_eq!(util["idle"], 1); // r-2 is idle
    }

    #[tokio::test]
    async fn agent_utilization_updates_on_workflow_completion() {
        let core = open_core().await;

        // Register replicant
        core.ingest(IngestEvent {
            entity_id: "r-1",
            event_type: "replicant.registered",
            payload: json!({"capabilities": []}),
            metadata: None,
            tenant_id: None,
        })
        .await
        .unwrap();

        // Claim and complete workflow
        core.ingest(IngestEvent {
            entity_id: "wf-1",
            event_type: "workflow.dispatched",
            payload: json!({}),
            metadata: None,
            tenant_id: None,
        })
        .await
        .unwrap();
        core.ingest(IngestEvent {
            entity_id: "wf-1",
            event_type: "workflow.claimed",
            payload: json!({"replicant_id": "r-1"}),
            metadata: None,
            tenant_id: None,
        })
        .await
        .unwrap();
        core.ingest(IngestEvent {
            entity_id: "wf-1",
            event_type: "workflow.output.ready",
            payload: json!({"result": "done"}),
            metadata: None,
            tenant_id: None,
        })
        .await
        .unwrap();

        let util = core.projection("agent_utilization", "__all").unwrap();
        assert_eq!(util["active"], 0);
        assert_eq!(util["idle"], 1); // r-1 is free again
    }

    #[tokio::test]
    async fn agent_utilization_excludes_stale_replicants() {
        let core = open_core().await;

        core.ingest(IngestEvent {
            entity_id: "r-1",
            event_type: "replicant.registered",
            payload: json!({"capabilities": []}),
            metadata: None,
            tenant_id: None,
        })
        .await
        .unwrap();

        core.ingest(IngestEvent {
            entity_id: "r-2",
            event_type: "replicant.registered",
            payload: json!({"capabilities": []}),
            metadata: None,
            tenant_id: None,
        })
        .await
        .unwrap();

        // r-2 goes stale
        core.ingest(IngestEvent {
            entity_id: "r-2",
            event_type: "replicant.stale",
            payload: json!({}),
            metadata: None,
            tenant_id: None,
        })
        .await
        .unwrap();

        let util = core.projection("agent_utilization", "__all").unwrap();
        assert_eq!(util["total_capacity"], 1); // only r-1 is alive
        assert_eq!(util["idle"], 1);
    }

    // =========================================================================
    // Helper
    // =========================================================================

    async fn open_core() -> EmbeddedCore {
        EmbeddedCore::open(Config::builder().build().unwrap())
            .await
            .unwrap()
    }
}
