# ADR-012: Unified DirectedAdjacencyProjection

**Status:** Accepted
**Date:** 2026-03-20
**Deciders:** Architecture Review

## Context

Prime currently maintains two nearly identical projections for edge adjacency:

- **`AdjacencyListProjection`** — maps `source → Vec<AdjEntry>` for outgoing edges.
- **`ReverseIndexProjection`** — maps `target → Vec<AdjEntry>` for incoming edges.

These two structs share the same `AdjEntry` type, identical snapshot/restore logic, and nearly identical `process()` implementations. The only difference is which payload field (`source` vs `target`) is used as the map key and which is stored in `AdjEntry.peer`.

More critically, both projections perform **O(E) full scan** on edge deletion: they iterate every entry in the `DashMap` and call `retain()` to remove the deleted edge. This is because the `EDGE_DELETED` event payload currently contains only `{"id": edge_id}` — it does not include `source` or `target`, so the projection cannot look up the correct bucket directly.

With a graph of 100K edges, deleting a single edge requires scanning all adjacency lists. This is the dominant cost in `delete_node()`, which cascades across all connected edges.

## Decision

1. **Merge** `AdjacencyListProjection` and `ReverseIndexProjection` into a single generic `DirectedAdjacencyProjection` parameterized by a `Direction` marker:

```rust
pub struct DirectedAdjacencyProjection {
    name: String,
    direction: AdjDirection,
    adj: Arc<DashMap<String, Vec<AdjEntry>>>,
    /// Secondary index: edge_id → key in `adj` map
    edge_index: Arc<DashMap<String, String>>,
}

pub enum AdjDirection {
    /// Key by source, peer is target (forward/outgoing)
    Forward,
    /// Key by target, peer is source (reverse/incoming)
    Reverse,
}
```

2. **Add a secondary index** `edge_index: DashMap<String, String>` mapping `edge_id → key` (the source or target that is the primary key). On `EDGE_CREATED`, insert into both `adj` and `edge_index`. On `EDGE_DELETED`, look up the key in `edge_index` in O(1), then remove the entry from the corresponding `Vec<AdjEntry>` in O(degree) — a dramatic improvement over O(E) full scan.

3. **Preserve backward compatibility** with type aliases:

```rust
pub type AdjacencyListProjection = DirectedAdjacencyProjection;
pub type ReverseIndexProjection = DirectedAdjacencyProjection;
```

And factory functions:

```rust
impl DirectedAdjacencyProjection {
    pub fn forward(name: impl Into<String>) -> Self { /* AdjDirection::Forward */ }
    pub fn reverse(name: impl Into<String>) -> Self { /* AdjDirection::Reverse */ }
}
```

4. **Include `source` and `target` in `EDGE_DELETED` event payloads** (see ADR-013). This allows the secondary index to be populated during backfill from existing events, but the `edge_index` also self-populates from `EDGE_CREATED` events during projection replay.

## Consequences

### Positive

- **O(1) edge deletion in adjacency index.** The `edge_index` lookup eliminates full-scan deletion. For a graph with 100K edges, single-edge deletion drops from ~100K comparisons to ~1 hash lookup + O(degree) vec scan.
- **DRY.** Eliminates ~100 lines of duplicated projection code. One implementation, two instances with different direction parameters.
- **Snapshot consistency.** A single implementation means snapshot/restore logic is tested once and correct in both directions. The previous duplication risked divergence.
- **Backward-compatible public API.** The `outgoing()` and `incoming()` methods are preserved. Type aliases ensure existing code compiles without changes.

### Negative

- **Secondary index memory overhead.** The `edge_index` DashMap adds one entry per live edge (edge_id string + key string). For 100K edges with average 40-byte IDs, this is roughly 8 MB — acceptable.
- **Snapshot format change.** The `edge_index` must be included in snapshots, or rebuilt from `adj` on restore. Either approach requires a snapshot format version bump. Existing snapshots can be loaded by rebuilding the `edge_index` from the `adj` entries.

### Risks

- **Stale `edge_index` entries.** If an `EDGE_CREATED` event is processed but the corresponding `EDGE_DELETED` event is lost (should not happen with WAL durability, but possible in test scenarios), the `edge_index` will have a stale entry. Mitigation: `edge_index` lookups that fail to find the edge in `adj` are treated as no-ops.
- **Type alias confusion.** `AdjacencyListProjection` and `ReverseIndexProjection` becoming aliases for the same struct may confuse readers. Mitigation: clear doc comments on each alias explaining which direction it represents.
