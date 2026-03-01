//! Tests for the MCP tool event emission helper.
//!
//! Run with: cargo test --features embedded-projections --test mcp_tool_emission

#[cfg(feature = "embedded-projections")]
mod tests {
    use allsource_core::embedded::mcp_events::McpToolTracker;
    use allsource_core::embedded::{Config, EmbeddedCore, Query};
    use serde_json::json;

    #[tokio::test]
    async fn test_emit_tool_result_creates_event() {
        let core = open_core().await;
        let tracker = McpToolTracker::new(&core);

        tracker.emit_tool_result("wf-1", "read_file", 50).await.unwrap();

        let events = core
            .query(Query::new().entity_id("wf-1").event_type("mcp.tool.result"))
            .await
            .unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].payload["tool_name"], "read_file");
        assert_eq!(events[0].payload["duration_ms"], 50);
    }

    #[tokio::test]
    async fn test_emit_tool_error_creates_event() {
        let core = open_core().await;
        let tracker = McpToolTracker::new(&core);

        tracker
            .emit_tool_error("wf-1", "web_search", "timeout")
            .await
            .unwrap();

        let events = core
            .query(Query::new().entity_id("wf-1").event_type("mcp.tool.error"))
            .await
            .unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].payload["tool_name"], "web_search");
        assert_eq!(events[0].payload["error"], "timeout");
    }

    #[tokio::test]
    async fn test_track_success_emits_result_with_duration() {
        let core = open_core().await;
        let tracker = McpToolTracker::new(&core);

        let result = tracker
            .track("wf-2", "read_file", || async {
                Ok(json!({"content": "hello"}))
            })
            .await
            .unwrap();

        assert_eq!(result, json!({"content": "hello"}));

        let events = core
            .query(Query::new().entity_id("wf-2").event_type("mcp.tool.result"))
            .await
            .unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].payload["tool_name"], "read_file");
        assert!(events[0].payload["duration_ms"].as_u64().is_some());
    }

    #[tokio::test]
    async fn test_track_error_emits_error_event() {
        let core = open_core().await;
        let tracker = McpToolTracker::new(&core);

        let result = tracker
            .track("wf-3", "web_search", || async {
                Err(allsource_core::error::AllSourceError::InvalidInput(
                    "network timeout".to_string(),
                ))
            })
            .await;

        assert!(result.is_err());

        let events = core
            .query(Query::new().entity_id("wf-3").event_type("mcp.tool.error"))
            .await
            .unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].payload["tool_name"], "web_search");
        assert!(events[0].payload["error"]
            .as_str()
            .unwrap()
            .contains("network timeout"));
        assert!(events[0].payload["duration_ms"].as_u64().is_some());
    }

    #[tokio::test]
    async fn test_projection_receives_emitted_events() {
        let core = open_core().await;
        let tracker = McpToolTracker::new(&core);

        // Emit some tool events
        tracker.emit_tool_result("wf-e2e", "read_file", 42).await.unwrap();
        tracker.emit_tool_result("wf-e2e", "read_file", 55).await.unwrap();
        tracker
            .emit_tool_error("wf-e2e", "web_search", "failed")
            .await
            .unwrap();

        // Check the tool_call_audit projection has received them
        let state = core.projection("tool_call_audit", "wf-e2e");
        assert!(state.is_some(), "projection should have state");

        let state = state.unwrap();
        // read_file: 2 successes
        let read_file = &state["read_file"];
        assert_eq!(read_file["total_calls"], 2);
        assert_eq!(read_file["successes"], 2);
        assert_eq!(read_file["failures"], 0);

        // web_search: 1 failure
        let web_search = &state["web_search"];
        assert_eq!(web_search["total_calls"], 1);
        assert_eq!(web_search["successes"], 0);
        assert_eq!(web_search["failures"], 1);
    }

    async fn open_core() -> EmbeddedCore {
        EmbeddedCore::open(Config::builder().build().unwrap())
            .await
            .unwrap()
    }
}
