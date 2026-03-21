# ADR-015: Generation Counter for VectorIndexProjection

**Status:** Accepted
**Date:** 2026-03-20
**Deciders:** Architecture Review

## Context

`VectorIndexProjection` uses an `AtomicBool` named `dirty` to track whether the HNSW index needs rebuilding. When a `VECTOR_STORED` or `VECTOR_DELETED` event arrives, `dirty` is set to `true`. When `search()` is called, `ensure_index()` checks `dirty`, rebuilds the index from the `vectors` DashMap (excluding entries in the `deleted` DashMap), and sets `dirty` back to `false`.

This design has three bugs:

### 1. TOCTOU race on dirty flag

```
Thread A: ensure_index() reads dirty=true, begins rebuilding
Thread B: process(VECTOR_STORED) inserts new vector, sets dirty=true
Thread A: finishes rebuild (missing Thread B's vector), sets dirty=false
Thread C: search() sees dirty=false, uses stale index — misses the new vector
```

The new vector is in the `vectors` DashMap but not in the HNSW index, and `dirty` is `false` so no rebuild is triggered. The vector becomes invisible to search until the next mutation.

### 2. Separate `deleted` DashMap

Deletions insert into a separate `deleted: DashMap<String, ()>` rather than removing from `vectors`. This creates:

- **Memory leak:** Deleted vectors remain in `vectors` forever, consuming memory for their `Vec<f32>` embeddings.
- **Snapshot/restore asymmetry:** `snapshot()` filters out deleted entries and serializes only live vectors. `restore()` populates `vectors` from the snapshot and clears `deleted`. But if events are replayed after restore (backfill), `VECTOR_DELETED` events insert into `deleted` for vectors that are no longer in `vectors` (they were filtered out during snapshot). The `len()` method returns `vectors.len() - deleted.len()`, which underflows if `deleted` contains entries not in `vectors`.

### 3. Rebuild cost opacity

The `AtomicBool` provides no way to know *how many* mutations occurred since the last rebuild. Logging and metrics cannot report "index rebuilt after N mutations" — only "index was dirty."

## Decision

1. **Replace `AtomicBool dirty` with `AtomicU64 generation`.** Each mutation (VECTOR_STORED, VECTOR_DELETED) increments `generation` via `fetch_add(1, Ordering::Release)`. The built index records the generation at which it was constructed.

```rust
pub struct VectorIndexProjection {
    name: String,
    vectors: Arc<DashMap<String, VectorRecord>>,
    index: Arc<RwLock<Option<BuiltIndex>>>,
    generation: Arc<AtomicU64>,
    built_generation: Arc<AtomicU64>,
    config: VectorIndexConfig,
}
```

`ensure_index()` compares `generation` to `built_generation`. If they differ, it rebuilds. After rebuilding, it sets `built_generation` to the `generation` value read *before* the rebuild started. If new mutations arrived during the rebuild, `generation > built_generation` and the next search will trigger another rebuild — no mutations are lost.

2. **Remove the `deleted` DashMap.** Deletions simply `remove()` the entry from `vectors` and bump the generation counter. This eliminates the memory leak and the snapshot/restore asymmetry.

```rust
// In process():
vec_events::VECTOR_DELETED => {
    self.vectors.remove(&entity_id);
    self.generation.fetch_add(1, Ordering::Release);
}
```

3. **Simplify `len()` and `snapshot()`.** `len()` returns `self.vectors.len()` directly. `snapshot()` serializes all entries in `vectors` (no filtering needed). `restore()` inserts all entries and bumps the generation.

## Consequences

### Positive

- **Fixes the TOCTOU race.** The generation counter is read before rebuild and compared after. Concurrent mutations increment the counter, ensuring the next search detects staleness even if the rebuild was in flight.
- **Eliminates the memory leak.** Deleted vectors are removed from `vectors` immediately, freeing their embedding allocations.
- **Eliminates snapshot/restore asymmetry.** `vectors` contains exactly the live set. No separate `deleted` map to synchronize.
- **Observable mutations.** The generation counter can be exposed via metrics (`prime_vector_index_generation` gauge) and logged on rebuild: "rebuilt HNSW index, generation 42→57, 15 mutations since last rebuild."
- **Correct `len()`.** No more `vectors.len() - deleted.len()` arithmetic that can underflow.

### Negative

- **Double rebuild on concurrent mutation.** If a mutation arrives during rebuild, the next search triggers another rebuild even though only one vector changed. This is the same behavior as the `AtomicBool` approach but is now correct (no missed vectors) rather than silently stale.
- **`AtomicU64` wrapping.** The generation counter wraps at 2^64 (~18 quintillion). At 1 million mutations per second, this would take 584,542 years. Not a practical concern.

### Risks

- **Ordering semantics.** The generation counter uses `Release` on increment and `Acquire` on read to ensure the vector data written by `process()` is visible to the thread performing the rebuild. Incorrect ordering would reintroduce the TOCTOU race. Mitigation: the `Release`/`Acquire` pair is a well-established pattern; unit tests with `loom` or stress tests under MIRI can validate correctness.
- **Backfill performance.** During WAL replay, every event increments the generation counter. If N vectors are replayed, generation reaches N before the first search. The first search rebuilds once (not N times), so there is no performance regression.
