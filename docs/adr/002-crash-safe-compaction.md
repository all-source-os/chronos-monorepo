# ADR-002: Crash-Safe Token Compaction

**Status:** Accepted
**Date:** 2026-03-01
**Related:** ADR-001

## Context

Token compaction (`compact_entity_tokens`) merges multiple token-type events for an entity into a single merged event. The original implementation had two crash-safety issues:

1. **WAL append outside write lock**: The WAL write happened after the write lock was released. A crash between lock release and WAL write would lose the merged event, leaving the store with neither the originals (already removed from the vec) nor the merged result.

2. **Cross-tenant merge**: If the same `entity_id` existed across multiple tenants (e.g., `"user-123"` in both tenant A and tenant B), compaction would merge events from different tenants into one, corrupting data.

## Decision

### Fix 1: WAL inside write lock

Move the WAL append inside the write lock scope:

```rust
// Phase 3: Acquire write lock for swap + WAL + index rebuild
let mut events = self.events.write();
events.retain(|e| !(matches));
events.push(merged_event.clone());

// WAL append INSIDE write lock — crash-safe
if let Some(ref wal) = self.wal {
    wal.append(merged_event)?;
}
```

If WAL append fails, the error propagates and the entire operation is rolled back (the lock is poisoned). If the process crashes after WAL write but before lock release, WAL replay will reconstruct the correct state.

Trade-off: Write lock is held slightly longer (includes WAL I/O). Acceptable because compaction is infrequent and not on the hot path.

### Fix 2: Tenant validation guard

Before merging, validate all token events belong to the same tenant:

```rust
let tenant_id = all_events[0].tenant_id_str().to_string();
if all_events.iter().any(|e| e.tenant_id_str() != tenant_id) {
    return Err(AllSourceError::InvalidInput(
        format!("compact_tokens: entity '{}' has token events across multiple tenants", entity_id)
    ));
}
```

This is a hard error, not a silent skip, because cross-tenant entity_id collision indicates a data modeling problem the caller should fix.

## Consequences

- Compaction is now crash-safe: no data loss window between vec mutation and WAL write
- Multi-tenant systems cannot accidentally merge events across tenant boundaries
- Write lock duration increases by WAL I/O latency (~100μs typical) during compaction
- O(N) index rebuild still happens inside the write lock (documented as known limitation, acceptable for compaction's batch nature)
