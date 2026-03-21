# ADR-016: Eventual Consistency Model for Prime

**Status:** Accepted
**Date:** 2026-03-20
**Deciders:** Architecture Review

## Context

Prime's facade methods follow a check-then-act pattern: they read projection state (DashMap lookups), make a decision, and then write events to the WAL. Because projections are updated asynchronously after events are written, there is a window where the projection state read during the "check" phase may be stale by the time the "act" phase executes.

Specific examples in the current codebase:

### 1. `update_node()` on a concurrently deleted node

```
Thread A: update_node("node:person:1", props)
  → reads NodeStateProjection: node exists, not deleted ✓
Thread B: delete_node("node:person:1")
  → writes NODE_DELETED event, projection marks node as deleted
Thread A: writes NODE_UPDATED event for a now-deleted node
```

The WAL now contains a `NODE_UPDATED` after `NODE_DELETED`. The `NodeStateProjection` processes events in order and handles this gracefully (the update on a deleted node is merged but the `deleted` flag remains true), but the behavior is surprising.

### 2. `add_edge()` between concurrently deleted nodes

```
Thread A: add_edge("node:person:1", "node:person:2", "knows")
  → reads NodeStateProjection: both nodes exist ✓
Thread B: delete_node("node:person:1")
Thread A: writes EDGE_CREATED event referencing a deleted source
```

The edge exists in the WAL and adjacency index, pointing to a deleted node. `neighbors()` for node:person:2 returns an edge to a deleted node.

### 3. `delete_node()` with concurrent edge additions

```
Thread A: delete_node("node:person:1")
  → reads adjacency: outgoing=[e1, e2], incoming=[e3]
  → emits EDGE_DELETED for e1, e2, e3, then NODE_DELETED
Thread B: add_edge("node:person:1", "node:person:3", "knows")
  → writes EDGE_CREATED for e4 (after Thread A read the adjacency list)
```

Edge e4 is now orphaned — it references a deleted node and was not included in Thread A's deletion batch.

### 4. `neighbors_as_of()` O(E) cost

The `neighbors_as_of()` temporal query cannot use the adjacency projection (which reflects current state, not historical state). Instead, it queries all `prime.edge.*` events up to the `as_of` timestamp and replays them to build a point-in-time adjacency list. This is O(E) where E is the total number of edge events in the system, not just the edges connected to the queried node.

## Decision

**Accept eventual consistency rather than introducing per-entity locks or serialized writes.** The reasons:

1. **Prime's use case is agent memory.** Agents typically operate in a single-writer pattern (one agent session writes, many read). True concurrent writes to the same entity are rare in practice.

2. **Per-entity locks add complexity and deadlock risk.** Locking `node:person:1` during `delete_node()` while iterating edges that may touch other locked nodes creates a lock ordering problem. Hierarchical locking (lock all edges, then node) is complex and hurts throughput.

3. **Event sourcing is inherently append-only.** The WAL records all events faithfully. Even "inconsistent" event sequences (UPDATE after DELETE) are valid history — they record what happened. Projections are free to interpret them as they wish.

4. **Projections already handle stale reads gracefully:**
   - `NodeStateProjection`: `NODE_UPDATED` on a deleted node is a no-op (deleted flag stays true).
   - `AdjacencyListProjection`: Edges to deleted nodes are returned but `get_node()` on the peer returns `None`, which callers already handle.
   - `GraphStatsProjection`: Counters may briefly be inconsistent but converge after all events are processed.

**Document the consistency model explicitly** in the `Prime` struct's doc comments and in the API reference, with these guarantees:

- **Durability:** Once `ingest()` or `ingest_batch()` returns `Ok`, the events are durable in the WAL.
- **Projection convergence:** Projections converge to a consistent state after all events are processed. There is no permanent inconsistency.
- **No phantom reads in single-threaded use:** If a single thread calls `add_node()` then `get_node()`, the node is visible (projections are updated synchronously within the ingest path).
- **Eventual consistency under concurrency:** Concurrent writers may observe stale projection state. All events are recorded and projections will converge.

**Document the O(E) cost of `neighbors_as_of()`** and note that a future optimization (temporal adjacency projection that maintains per-timestamp snapshots) can reduce this to O(degree) but is not yet justified by usage patterns.

## Consequences

### Positive

- **Simplicity.** No locking infrastructure, no deadlock risk, no lock contention on the hot path.
- **Throughput.** Concurrent writers proceed without blocking each other. The WAL's single-writer lock (for append serialization) is the only contention point, and it is brief.
- **Auditability.** Every event is recorded, even "stale" ones. An operator can replay the event log and understand exactly what happened, including the race.
- **Projection independence.** Each projection defines its own semantics for out-of-order or stale events. No global consistency protocol is needed.

### Negative

- **Orphaned edges on concurrent delete + add.** An edge can reference a deleted node if the edge creation races with node deletion. Callers must handle `get_node()` returning `None` for edge peers.
- **`neighbors_as_of()` is O(E).** For large graphs with millions of edge events, this query is slow. Acceptable for current usage (small agent graphs, infrequent temporal queries), but will need optimization if usage patterns change.
- **Surprising semantics for database-minded developers.** Developers accustomed to ACID transactions may expect `add_edge()` to fail if the source node is concurrently deleted. Instead, the edge is created and points to a deleted node.

### Risks

- **Stale reads causing user-visible bugs.** If an application relies on `update_node()` failing for deleted nodes as a correctness guarantee (not just a convenience check), the eventual consistency model violates that assumption. Mitigation: document that the check is best-effort and callers must tolerate stale-read races.
- **O(E) temporal queries becoming a bottleneck.** If agent memory graphs grow to millions of events, `neighbors_as_of()` will become impractical. Mitigation: monitor query latency; if P99 exceeds 100ms, introduce a temporal adjacency projection (tracked as a future ADR).
