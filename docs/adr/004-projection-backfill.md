# ADR-004: Projection Backfill on Registration

**Status:** Accepted
**Date:** 2026-03-01
**Related:** ADR-001

## Context

When a projection is registered via `register_projection()`, it only sees events ingested *after* registration. Events already in the store are invisible to the new projection. For embedded use cases (e.g., a Tauri app that adds a new analytics projection in an update), this means the projection starts empty even though historical data exists.

## Decision

Add `EventStore::register_projection_with_backfill()` that replays existing events:

```rust
pub fn register_projection_with_backfill(&self, projection: Arc<dyn Projection>) -> Result<()> {
    // Register first (so new events also flow through)
    { let mut pm = self.projections.write(); pm.register(Arc::clone(&projection)); }

    // Replay historical events under read lock
    let events = self.events.read();
    for event in events.iter() {
        projection.process(event)?;
    }
    Ok(())
}
```

### Design choices

1. **Register before replay**: The projection is registered first so events ingested concurrently are not lost. This means some events may be processed twice (once from concurrent ingest, once from replay). Projections must be idempotent.

2. **Read lock for replay**: Uses a read lock so ingestion is not blocked during backfill. The trade-off is that the read lock is held for the duration of replay (proportional to event count).

3. **Opt-in**: This is a separate method, not a flag on `register_projection()`. The default `register_projection()` remains lightweight for projections that don't need history.

## Consequences

- New projections can see historical data without re-ingesting events
- Backfill duration is O(N) where N is total event count — acceptable for embedded use (typically <1M events)
- Projections used with backfill must be idempotent (duplicate processing is possible during concurrent ingest)
- Read lock held during replay may increase latency for concurrent writers
