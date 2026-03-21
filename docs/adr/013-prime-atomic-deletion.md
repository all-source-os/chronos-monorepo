# ADR-013: Atomic Node Deletion via Batch Ingestion

**Status:** Accepted
**Date:** 2026-03-20
**Deciders:** Architecture Review

## Context

The current `delete_node()` implementation in `facade.rs` issues sequential `ingest()` calls: one `EDGE_DELETED` event per connected edge, followed by a final `NODE_DELETED` event. Each `ingest()` call independently acquires the WAL write lock, writes to the WAL, and returns.

This creates two problems:

1. **Non-atomic deletion.** If the process crashes after emitting some `EDGE_DELETED` events but before the `NODE_DELETED` event, the system restarts with orphaned edge-deletion events. The node still exists (its `NODE_DELETED` was never written), but some of its edges are gone. The graph is in an inconsistent state that requires manual intervention.

2. **Performance.** For a node with N connected edges, `delete_node()` acquires and releases the WAL lock 2N+1 times (N outgoing + N incoming + 1 node). The `ingest_batch()` API already exists on `EmbeddedCore` and acquires the lock once for an entire batch.

Additionally, the `EDGE_DELETED` event payload currently contains only `{"id": edge_id}`. The adjacency projections must perform O(E) full scans to find and remove the edge because they do not know which source/target bucket the edge belongs to (see ADR-012).

## Decision

1. **Use `ingest_batch()` for `delete_node()`.** Collect all deletion events (edge deletions + node deletion) into a `Vec<IngestEvent>` and submit them as a single batch:

```rust
pub async fn delete_node(&self, entity_id: &str) -> PrimeResult<()> {
    // ... validate node exists ...

    let mut batch = Vec::new();

    // Collect outgoing edge deletions
    for adj in self.adjacency.outgoing(entity_id) {
        batch.push(IngestEvent {
            entity_id: format!("edge:{}", adj.edge_id),
            event_type: event_types::EDGE_DELETED,
            payload: json!({
                "id": adj.edge_id,
                "source": entity_id,
                "target": adj.peer,
            }),
            ..
        });
    }

    // Collect incoming edge deletions
    for adj in self.reverse_index.incoming(entity_id) {
        batch.push(IngestEvent {
            entity_id: format!("edge:{}", adj.edge_id),
            event_type: event_types::EDGE_DELETED,
            payload: json!({
                "id": adj.edge_id,
                "source": adj.peer,
                "target": entity_id,
            }),
            ..
        });
    }

    // Node deletion last
    batch.push(IngestEvent {
        entity_id: entity_id.to_string(),
        event_type: event_types::NODE_DELETED,
        payload: json!({}),
        ..
    });

    self.core.ingest_batch(batch).await?;
    Ok(())
}
```

2. **Enrich `EDGE_DELETED` payloads** with `source` and `target` fields. This enables the adjacency projections to perform O(1) bucket lookup on deletion (see ADR-012) and makes the event self-describing for audit/replay purposes.

3. **Apply the same pattern to `forget()`**, which deletes a vector, node, and edges. All events are batched into a single `ingest_batch()` call.

## Consequences

### Positive

- **Atomic crash recovery.** The WAL writes the entire batch in a single fsync. Either all deletion events are durable or none are. No more orphaned edge-deletion events after a crash.
- **Single lock acquisition.** One `ingest_batch()` call replaces 2N+1 individual `ingest()` calls, eliminating lock contention and reducing syscall overhead.
- **Self-describing events.** Including `source` and `target` in `EDGE_DELETED` payloads makes the event stream independently interpretable without cross-referencing `EDGE_CREATED` events. This is essential for the O(1) adjacency deletion in ADR-012.

### Negative

- **Larger `EDGE_DELETED` events.** Adding `source` and `target` fields increases each edge-deletion event by ~80 bytes. For a node with 100 edges, this adds ~8 KB to the batch — negligible relative to WAL write costs.
- **Batch size for highly-connected nodes.** A node with 10K edges produces a batch of 20K+1 events. The WAL must serialize and fsync this in one operation. In practice, the WAL already handles batches of this size from `ingest_batch()` in other contexts.

### Risks

- **Projection ordering within batch.** Projections process events in batch order. Edge deletions must be processed before the node deletion to avoid a state where the node is marked deleted but its adjacency entries still exist (causing stale neighbor lookups during the batch). Mitigation: edge deletions are always ordered before the node deletion in the batch, and projections process events sequentially.
- **Duplicate edge deletions.** If an edge appears in both outgoing and incoming lists (self-loops, or edge already partially deleted), the batch may contain duplicate `EDGE_DELETED` events for the same edge. Mitigation: deduplicate edge IDs before building the batch using a `HashSet`.
