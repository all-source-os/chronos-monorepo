# ADR-003: Batch Ingestion with Single Lock Acquisition

**Status:** Accepted
**Date:** 2026-03-01
**Related:** ADR-001

## Context

The original `EmbeddedCore::ingest_batch()` looped over events and called `store.ingest()` for each one. Each `ingest()` call acquired and released the write lock independently, causing:

1. **Lock contention**: N lock acquisitions for N events
2. **No atomicity**: Concurrent readers could observe partial batches
3. **Overhead**: Lock acquisition/release per event (~microseconds each, but adds up at scale)

## Decision

Add `EventStore::ingest_batch(batch: Vec<Event>)` that acquires the write lock once:

```rust
pub fn ingest_batch(&self, batch: Vec<Event>) -> Result<()> {
    if batch.is_empty() { return Ok(()); }

    // Validate all events upfront (before lock)
    for event in &batch { self.validate_event(event)?; }

    // WAL append (sequential, outside write lock — WAL has its own sync)
    if let Some(ref wal) = self.wal {
        for event in &batch { wal.append(event.clone())?; }
    }

    // Single write lock for all mutations
    let mut events = self.events.write();
    let projections = self.projections.read();
    for event in batch {
        // index, project, store, geo-index, schema-analyze
        events.push(event);
    }
    // ...
}
```

`EmbeddedCore::ingest_batch()` delegates to this instead of looping `store.ingest()`.

### Semantics

- **Not atomic in the database sense**: If the process crashes mid-batch, WAL replay will recover the events that were written. Some events may appear and others may not.
- **Atomic from a reader's perspective**: Since the write lock is held for the entire batch, concurrent readers see either none or all of the batch events.

## Consequences

- Batch of 100 events: 1 lock acquisition instead of 100
- Readers cannot observe partial batches during ingestion
- Validation failures are all-or-nothing (fail before any write)
- WAL writes are still sequential per-event (WAL doesn't support multi-event atomic append)
