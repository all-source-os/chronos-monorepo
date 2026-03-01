//! TDD RED phase tests for Phase 6: Streaming Token Events + Compaction.
//!
//! These tests define the target API for:
//! - High-frequency token event ingestion (~50 tokens/sec per workflow)
//! - Token compaction into workflow.output.complete
//! - Batch ingestion performance under streaming load
//!
//! Run with: cargo test --features embedded-streaming --test token_streaming

#[cfg(feature = "embedded-streaming")]
mod tests {
    use allsource_core::embedded::{Config, EmbeddedCore, IngestEvent, Query};
    use serde_json::json;

    // =========================================================================
    // High-Frequency Token Ingestion
    // =========================================================================

    #[tokio::test]
    async fn high_frequency_token_ingestion_200_tokens() {
        let core = open_core().await;

        // Simulate token stream: 200 tokens for a single workflow
        // Real-world: ~50 tokens/sec = 3000/min per workflow
        let mut events = Vec::with_capacity(200);
        for i in 0..200 {
            events.push(IngestEvent {
                entity_id: "wf-1",
                event_type: "workflow.token",
                payload: json!({"token": format!("word_{i}"), "index": i, "model": "claude-sonnet-4-20250514"}),
                metadata: None,
                tenant_id: None,
            });
        }

        core.ingest_batch(events).await.unwrap();
        assert_eq!(core.stats().total_events, 200);

        let tokens = core
            .query(Query::new().entity_id("wf-1").event_type("workflow.token"))
            .await
            .unwrap();
        assert_eq!(tokens.len(), 200);
    }

    #[tokio::test]
    async fn multiple_concurrent_token_streams() {
        let core = open_core().await;

        // Two workflows streaming tokens concurrently
        for i in 0..100 {
            core.ingest(IngestEvent {
                entity_id: "wf-1",
                event_type: "workflow.token",
                payload: json!({"token": format!("a_{i}"), "index": i}),
                metadata: None,
                tenant_id: None,
            })
            .await
            .unwrap();

            core.ingest(IngestEvent {
                entity_id: "wf-2",
                event_type: "workflow.token",
                payload: json!({"token": format!("b_{i}"), "index": i}),
                metadata: None,
                tenant_id: None,
            })
            .await
            .unwrap();
        }

        let wf1 = core.query(Query::new().entity_id("wf-1")).await.unwrap();
        let wf2 = core.query(Query::new().entity_id("wf-2")).await.unwrap();

        assert_eq!(wf1.len(), 100);
        assert_eq!(wf2.len(), 100);
    }

    // =========================================================================
    // Token Compaction
    // =========================================================================

    #[tokio::test]
    async fn compact_merges_tokens_into_output_complete() {
        let core = open_core().await;

        // Ingest token stream
        for i in 0..50 {
            core.ingest(IngestEvent {
                entity_id: "wf-1",
                event_type: "workflow.token",
                payload: json!({"token": format!("w{i}"), "index": i}),
                metadata: None,
                tenant_id: None,
            })
            .await
            .unwrap();
        }

        assert_eq!(core.stats().total_events, 50);

        // Trigger compaction for this entity's token stream
        core.compact_tokens("wf-1").await.unwrap();

        // After compaction: token events replaced by single output.complete
        let events = core.query(Query::new().entity_id("wf-1")).await.unwrap();

        let output = events
            .iter()
            .find(|e| e.event_type == "workflow.output.complete");
        assert!(
            output.is_some(),
            "should have a workflow.output.complete event"
        );

        // The merged output should contain all tokens concatenated
        let output_payload = &output.unwrap().payload;
        assert!(output_payload["text"].is_string());
        let text = output_payload["text"].as_str().unwrap();
        assert!(text.contains("w0"));
        assert!(text.contains("w49"));

        // Original token events should be compacted away
        let remaining_tokens: Vec<_> = events
            .iter()
            .filter(|e| e.event_type == "workflow.token")
            .collect();
        assert!(
            remaining_tokens.is_empty(),
            "token events should be compacted"
        );
    }

    #[tokio::test]
    async fn compact_is_noop_when_no_tokens() {
        let core = open_core().await;

        // Ingest a non-token event
        core.ingest(IngestEvent {
            entity_id: "wf-1",
            event_type: "workflow.dispatched",
            payload: json!({"name": "test"}),
            metadata: None,
            tenant_id: None,
        })
        .await
        .unwrap();

        // Compact should succeed but change nothing
        core.compact_tokens("wf-1").await.unwrap();

        let events = core.query(Query::new().entity_id("wf-1")).await.unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event_type, "workflow.dispatched");
    }

    #[tokio::test]
    async fn compact_preserves_non_token_events() {
        let core = open_core().await;

        // Mix of token and non-token events
        core.ingest(IngestEvent {
            entity_id: "wf-1",
            event_type: "workflow.dispatched",
            payload: json!({"name": "test"}),
            metadata: None,
            tenant_id: None,
        })
        .await
        .unwrap();

        for i in 0..10 {
            core.ingest(IngestEvent {
                entity_id: "wf-1",
                event_type: "workflow.token",
                payload: json!({"token": format!("t{i}"), "index": i}),
                metadata: None,
                tenant_id: None,
            })
            .await
            .unwrap();
        }

        core.ingest(IngestEvent {
            entity_id: "wf-1",
            event_type: "workflow.step.completed",
            payload: json!({"step_id": 0}),
            metadata: None,
            tenant_id: None,
        })
        .await
        .unwrap();

        core.compact_tokens("wf-1").await.unwrap();

        let events = core.query(Query::new().entity_id("wf-1")).await.unwrap();

        // Should have: dispatched + output.complete + step.completed = 3
        assert_eq!(events.len(), 3);
        let types: Vec<&str> = events.iter().map(|e| e.event_type.as_str()).collect();
        assert!(types.contains(&"workflow.dispatched"));
        assert!(types.contains(&"workflow.output.complete"));
        assert!(types.contains(&"workflow.step.completed"));
    }

    // =========================================================================
    // MCP Tool Execution Tracking
    // =========================================================================

    #[tokio::test]
    async fn mcp_tool_call_lifecycle() {
        let core = open_core().await;

        core.ingest(IngestEvent {
            entity_id: "wf-1",
            event_type: "mcp.tool.called",
            payload: json!({
                "tool_name": "read_file",
                "server": "filesystem",
                "arguments": {"path": "/tmp/test.txt"}
            }),
            metadata: None,
            tenant_id: None,
        })
        .await
        .unwrap();

        core.ingest(IngestEvent {
            entity_id: "wf-1",
            event_type: "mcp.tool.result",
            payload: json!({
                "tool_name": "read_file",
                "result": "file contents here",
                "duration_ms": 45
            }),
            metadata: None,
            tenant_id: None,
        })
        .await
        .unwrap();

        let events = core
            .query(
                Query::new()
                    .entity_id("wf-1")
                    .event_type_prefix("mcp.tool."),
            )
            .await
            .unwrap();
        assert_eq!(events.len(), 2);
    }

    // =========================================================================
    // Cost Tracking Events
    // =========================================================================

    #[tokio::test]
    async fn llm_cost_events_ingest_correctly() {
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

        let events = core
            .query(
                Query::new()
                    .entity_id("wf-1")
                    .event_type("llm.call.completed"),
            )
            .await
            .unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].payload["input_tokens"], 1500);
        assert_eq!(events[0].payload["cost_usd"], 0.0078);
    }

    // =========================================================================
    // Cross-tenant compaction guard
    // =========================================================================

    #[tokio::test]
    async fn compact_rejects_cross_tenant_tokens() {
        // Multi-tenant mode
        let core = EmbeddedCore::open(Config::builder().single_tenant(false).build().unwrap())
            .await
            .unwrap();

        // Ingest tokens for same entity_id under two different tenants
        for i in 0..3 {
            core.ingest(IngestEvent {
                entity_id: "wf-shared",
                event_type: "workflow.token",
                payload: json!({"token": format!("a{i}"), "index": i}),
                metadata: None,
                tenant_id: Some("tenant-a"),
            })
            .await
            .unwrap();
        }
        for i in 0..3 {
            core.ingest(IngestEvent {
                entity_id: "wf-shared",
                event_type: "workflow.token",
                payload: json!({"token": format!("b{i}"), "index": i}),
                metadata: None,
                tenant_id: Some("tenant-b"),
            })
            .await
            .unwrap();
        }

        // Compaction should succeed (tokens from both tenants exist but query
        // returns all — the guard should detect mixed tenants and reject)
        let result = core.compact_tokens("wf-shared").await;
        assert!(
            result.is_err(),
            "compact_tokens should reject cross-tenant token events"
        );
        let err_msg = format!("{}", result.unwrap_err());
        assert!(
            err_msg.contains("multiple tenants"),
            "Error should mention multiple tenants, got: {err_msg}"
        );
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
