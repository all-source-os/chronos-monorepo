//! TDD RED phase tests for Phase 4: Bidirectional Sync Protocol.
//!
//! These tests define the target API for:
//! - HLC (Hybrid Logical Clock) ordering and causality
//! - Version vector delta computation and merge
//! - Sync between two EmbeddedCore instances
//! - Conflict resolution strategies (LWW, first-write-wins, append-only)
//! - Idempotent sync
//! - Offline queue drain on reconnect
//!
//! Run with: cargo test --features embedded-sync --test bidirectional_sync

#[cfg(feature = "embedded-sync")]
mod tests {
    use allsource_core::cluster::{
        ConflictResolution, CrdtResolver, HlcTimestamp, HybridLogicalClock, ReplicatedEvent,
        VersionVector,
    };
    use allsource_core::embedded::{Config, EmbeddedCore, IngestEvent, Query};
    use serde_json::json;

    // =========================================================================
    // HLC Ordering
    // =========================================================================

    #[test]
    fn hlc_provides_total_order_across_nodes() {
        let clock_a = HybridLogicalClock::new(1);
        let clock_b = HybridLogicalClock::new(2);

        let t1 = clock_a.now();
        let t2 = clock_b.now();

        // Both generated independently — HLC must provide total order via node_id
        assert_ne!(t1, t2);
        // One must be strictly less than the other
        assert!(t1 < t2 || t2 < t1);
    }

    #[test]
    fn hlc_respects_causality_after_receive() {
        let clock_a = HybridLogicalClock::new(1);
        let clock_b = HybridLogicalClock::new(2);

        let t1 = clock_a.now();
        // b learns of a's time
        let _ = clock_b.receive(&t1).unwrap();
        let t2 = clock_b.now();

        // t2 must be causally after t1
        assert!(t2 > t1);
    }

    #[test]
    fn hlc_logical_counter_breaks_same_ms_ties() {
        let clock = HybridLogicalClock::new(1);

        // Two timestamps in rapid succession (likely same physical ms)
        let t1 = clock.now();
        let t2 = clock.now();

        // Must be ordered even if physical ms is the same
        assert!(t2 > t1);
    }

    #[test]
    fn hlc_rejects_excessive_clock_drift() {
        let clock = HybridLogicalClock::with_max_drift(1, 1000); // 1 second max drift

        // Create a remote timestamp far in the future
        let future_ts = HlcTimestamp::new(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64
                + 60_000, // 60 seconds ahead
            0,
            99,
        );

        let result = clock.receive(&future_ts);
        assert!(result.is_err()); // Should reject — drift exceeds 1s
    }

    // =========================================================================
    // Version Vectors
    // =========================================================================

    #[test]
    fn version_vector_tracks_per_node_progress() {
        let mut vv = VersionVector::new();
        let ts1 = HlcTimestamp::new(1000, 0, 1);
        let ts2 = HlcTimestamp::new(2000, 0, 1);

        vv.advance("node-1", ts1);
        assert_eq!(vv.get("node-1").unwrap().physical_ms, 1000);

        vv.advance("node-1", ts2);
        assert_eq!(vv.get("node-1").unwrap().physical_ms, 2000);
    }

    #[test]
    fn version_vector_detects_unseen_events() {
        let mut vv = VersionVector::new();
        let ts_old = HlcTimestamp::new(1000, 0, 1);
        let ts_new = HlcTimestamp::new(2000, 0, 1);

        vv.advance("node-1", ts_old);

        // ts_new is newer than what we've seen — should be "new"
        assert!(vv.is_new("node-1", &ts_new));
        // ts_old is not new — already at or past it
        assert!(!vv.is_new("node-1", &ts_old));
    }

    #[test]
    fn version_vector_merge_is_commutative() {
        let mut a = VersionVector::new();
        let mut b = VersionVector::new();

        a.advance("node-1", HlcTimestamp::new(5000, 0, 1));
        b.advance("node-2", HlcTimestamp::new(3000, 0, 2));

        let mut ab = a.clone();
        ab.merge(&b);

        let mut ba = b.clone();
        ba.merge(&a);

        // merge(a, b) == merge(b, a)
        assert_eq!(ab.get("node-1"), ba.get("node-1"));
        assert_eq!(ab.get("node-2"), ba.get("node-2"));
    }

    #[test]
    fn version_vector_merge_takes_max() {
        let mut a = VersionVector::new();
        let mut b = VersionVector::new();

        a.advance("node-1", HlcTimestamp::new(5000, 0, 1));
        a.advance("node-2", HlcTimestamp::new(1000, 0, 2));

        b.advance("node-1", HlcTimestamp::new(3000, 0, 1));
        b.advance("node-2", HlcTimestamp::new(8000, 0, 2));

        a.merge(&b);

        // Should take max for each node
        assert_eq!(a.get("node-1").unwrap().physical_ms, 5000);
        assert_eq!(a.get("node-2").unwrap().physical_ms, 8000);
    }

    // =========================================================================
    // CRDT Conflict Resolution
    // =========================================================================

    #[test]
    fn crdt_resolver_accepts_new_event() {
        let resolver = CrdtResolver::new();

        let event = ReplicatedEvent {
            event_id: "evt-1".to_string(),
            hlc_timestamp: HlcTimestamp::new(1000, 0, 1),
            origin_region: "node-1".to_string(),
            event_data: json!({"key": "value"}),
        };

        let result = resolver.resolve(&event);
        assert_eq!(result, ConflictResolution::Accept);
    }

    #[test]
    fn crdt_resolver_skips_duplicate_event() {
        let resolver = CrdtResolver::new();

        let event = ReplicatedEvent {
            event_id: "evt-1".to_string(),
            hlc_timestamp: HlcTimestamp::new(1000, 0, 1),
            origin_region: "node-1".to_string(),
            event_data: json!({"key": "value"}),
        };

        // First time: accept
        let _ = resolver.resolve_and_accept(&event);

        // Second time: skip (duplicate)
        let result = resolver.resolve(&event);
        assert_eq!(result, ConflictResolution::Skip);
    }

    // =========================================================================
    // Sync Protocol — Two EmbeddedCore Instances
    // =========================================================================

    #[tokio::test]
    async fn two_instances_sync_after_independent_writes() {
        let core_a = open_core_with_node(1).await;
        let core_b = open_core_with_node(2).await;

        // Independent writes while "disconnected"
        core_a
            .ingest(IngestEvent {
                entity_id: "doc-1",
                event_type: "doc.edited",
                payload: json!({"text": "hello from A"}),
                metadata: None,
                tenant_id: None,
            })
            .await
            .unwrap();

        core_b
            .ingest(IngestEvent {
                entity_id: "doc-2",
                event_type: "doc.edited",
                payload: json!({"text": "hello from B"}),
                metadata: None,
                tenant_id: None,
            })
            .await
            .unwrap();

        // Bidirectional sync
        sync_pair(&core_a, &core_b).await.unwrap();

        // Both should have both events
        let a_events = core_a
            .query(Query::new().event_type_prefix("doc."))
            .await
            .unwrap();
        let b_events = core_b
            .query(Query::new().event_type_prefix("doc."))
            .await
            .unwrap();

        assert_eq!(a_events.len(), 2);
        assert_eq!(b_events.len(), 2);
    }

    #[tokio::test]
    async fn sync_is_idempotent() {
        let core_a = open_core_with_node(1).await;
        let core_b = open_core_with_node(2).await;

        core_a
            .ingest(IngestEvent {
                entity_id: "e1",
                event_type: "test.created",
                payload: json!({}),
                metadata: None,
                tenant_id: None,
            })
            .await
            .unwrap();

        // Sync multiple times
        sync_pair(&core_a, &core_b).await.unwrap();
        sync_pair(&core_a, &core_b).await.unwrap();
        sync_pair(&core_a, &core_b).await.unwrap();

        // Still only 1 event in each — no duplicates
        assert_eq!(core_a.stats().total_events, 1);
        assert_eq!(core_b.stats().total_events, 1);
    }

    #[tokio::test]
    async fn sync_conflict_last_write_wins() {
        let core_a = open_core_with_node(1).await;
        let core_b = open_core_with_node(2).await;

        // Both write to same entity independently
        core_a
            .ingest(IngestEvent {
                entity_id: "config-1",
                event_type: "config.updated",
                payload: json!({"theme": "dark"}),
                metadata: None,
                tenant_id: None,
            })
            .await
            .unwrap();

        // B's event has higher node_id (2 > 1), so at same millisecond
        // B wins the LWW tie-break deterministically.
        core_b
            .ingest(IngestEvent {
                entity_id: "config-1",
                event_type: "config.updated",
                payload: json!({"theme": "light"}),
                metadata: None,
                tenant_id: None,
            })
            .await
            .unwrap();

        sync_pair(&core_a, &core_b).await.unwrap();

        // LWW: B's event wins (later HLC) — projection state converges
        let a_state = core_a.projection("entity_snapshots", "config-1");
        let b_state = core_b.projection("entity_snapshots", "config-1");
        assert_eq!(a_state, b_state);
        assert_eq!(a_state.unwrap()["theme"], "light");
    }

    #[tokio::test]
    async fn offline_queue_drains_on_reconnect() {
        let core_local = open_core_with_node(1).await;
        let core_cloud = open_core_with_node(2).await;

        // Ingest 10 events while "offline" (no sync)
        for i in 0..10 {
            core_local
                .ingest(IngestEvent {
                    entity_id: &format!("item-{i}"),
                    event_type: "item.created",
                    payload: json!({"i": i}),
                    metadata: None,
                    tenant_id: None,
                })
                .await
                .unwrap();
        }

        assert_eq!(core_local.stats().total_events, 10);
        assert_eq!(core_cloud.stats().total_events, 0);

        // "Reconnect" — sync drains the queue
        sync_pair(&core_local, &core_cloud).await.unwrap();

        assert_eq!(core_cloud.stats().total_events, 10);
    }

    #[tokio::test]
    async fn sync_preserves_event_ordering() {
        let core_a = open_core_with_node(1).await;
        let core_b = open_core_with_node(2).await;

        // Sequential events on A
        for i in 0..5 {
            core_a
                .ingest(IngestEvent {
                    entity_id: "seq-entity",
                    event_type: "step.completed",
                    payload: json!({"step": i}),
                    metadata: None,
                    tenant_id: None,
                })
                .await
                .unwrap();
        }

        sync_pair(&core_a, &core_b).await.unwrap();

        let b_events = core_b
            .query(Query::new().entity_id("seq-entity"))
            .await
            .unwrap();
        assert_eq!(b_events.len(), 5);

        // Verify HLC-based causal ordering preserved
        for i in 1..b_events.len() {
            assert!(b_events[i].timestamp >= b_events[i - 1].timestamp);
        }
    }

    // =========================================================================
    // Helpers
    // =========================================================================

    /// Open an in-memory EmbeddedCore with a specific node_id for sync.
    async fn open_core_with_node(node_id: u32) -> EmbeddedCore {
        EmbeddedCore::open(
            Config::builder()
                .node_id(node_id)
                .build()
                .unwrap(),
        )
        .await
        .unwrap()
    }

    /// Bidirectional sync between two cores: a→b then b→a.
    async fn sync_pair(
        a: &EmbeddedCore,
        b: &EmbeddedCore,
    ) -> allsource_core::error::Result<()> {
        a.sync_to(b).await?;
        b.sync_to(a).await?;
        Ok(())
    }
}
