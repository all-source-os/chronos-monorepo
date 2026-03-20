# PRD: Optimistic Concurrency Control + Durable Subscriptions + Server-Side Filtering

**GitHub Issue:** [all-source-os/all-source#85](https://github.com/all-source-os/all-source/issues/85)

## Overview

AllSource Core currently accepts all event writes unconditionally and broadcasts all events to all WebSocket subscribers. This PRD adds three capabilities that transform Core from a pure append-only event store into a coordination-capable event store:

1. **Expected Version on Writes** — reject writes when entity version doesn't match, enabling optimistic concurrency control (queue claiming, leader election, saga state machines)
2. **Durable Subscriptions** — persist consumer cursors so reconnecting clients automatically catch up on missed events
3. **Server-Side Event Type Filtering** — let subscribers declare interest in event type prefixes so Core only sends matching events

These features eliminate the need for PostgreSQL coordination tables (`FOR UPDATE SKIP LOCKED`, advisory locks, mutable state columns) in downstream applications like AlphaSigmaPro.

## Goals

- Enable optimistic concurrency control via `expected_version` field on event ingestion
- Return `409 Conflict` with `current_version` when version check fails
- Expose `entity_version` in query responses so clients know current version
- Persist consumer cursors (last-acknowledged position) durably across Core restarts
- Support both pull (HTTP polling) and push (WebSocket with auto-replay) for durable consumers
- Allow WebSocket subscribers to filter by event type prefix, reducing unnecessary traffic
- Maintain full backwards compatibility — omitting `expected_version` skips the check, existing WebSocket clients receive all events

## Quality Gates

### Epic-Level (run once on epic completion)
- `cargo test --manifest-path apps/core/Cargo.toml` — all unit and integration tests pass
- `cargo clippy --manifest-path apps/core/Cargo.toml -- -D warnings` — no clippy warnings

### Story-Level (checked per story)
- **Backend stories:** Rust integration test covering the story's functionality

## User Stories

### US-001: Track per-entity version in EventStore [Backend]
**Description:** As the Core event store, I need to track a monotonic version counter per `entity_id` so that version checks can be performed on writes.

**Acceptance Criteria:**
- [ ] `EventStore` maintains a per-entity version counter (u64, starting at 0 = no events)
- [ ] Each event appended for an entity increments that entity's version by 1
- [ ] Version is stored in the in-memory `DashMap` and recoverable from WAL on restart
- [ ] Integration test: append 3 events for entity "e1", verify version is 3
- [ ] Integration test: restart EventStore (drop + reload from WAL), verify entity version is still 3

Mark each item [x] as you complete it. Only close when all are checked.

### US-002: Add `expected_version` to event ingestion API [Backend]
**Description:** As an API client, I want to include an `expected_version` field when ingesting an event so that my write is rejected if another writer has modified the entity since I last read it.

**Acceptance Criteria:**
- [ ] `POST /api/v1/events` accepts optional `expected_version` (u64) in request body
- [ ] When `expected_version` is `None`/absent, write proceeds unconditionally (backwards compatible)
- [ ] When `expected_version` matches current entity version, write succeeds and returns `200 OK` with `{"version": N}`
- [ ] When `expected_version` does NOT match, write is rejected with `409 Conflict` and body `{"error": "version_conflict", "current_version": N}`
- [ ] The version check + WAL append is atomic (no TOCTOU race — hold lock during check+write)
- [ ] Integration test: two concurrent writes with same `expected_version` — one succeeds, one gets 409
- [ ] Integration test: write without `expected_version` always succeeds regardless of current version

Mark each item [x] as you complete it. Only close when all are checked.

### US-003: Return `entity_version` in query responses [Backend]
**Description:** As an API client, I want query responses to include the current entity version so I can use it for subsequent `expected_version` writes.

**Acceptance Criteria:**
- [ ] `GET /api/v1/events/query?entity_id=X` response includes `"entity_version": N` field
- [ ] When entity has no events, `entity_version` is `0`
- [ ] When query does not filter by `entity_id` (e.g. query by event_type), `entity_version` is omitted
- [ ] Integration test: ingest 3 events for entity "e1", query by entity_id, verify `entity_version` is 3

Mark each item [x] as you complete it. Only close when all are checked.

### US-004: Consumer registration and storage [Backend]
**Description:** As the Core event store, I need to manage durable consumer registrations with persistent cursor positions so consumers can resume from where they left off.

**Acceptance Criteria:**
- [ ] `POST /api/v1/consumers` creates a consumer with `consumer_id` and optional `event_type_filters` (prefix list)
- [ ] Consumer state stored as events in a reserved stream (e.g. `__consumers` entity) via the same WAL pipeline
- [ ] Consumer state survives Core restart (recovered from WAL)
- [ ] `GET /api/v1/consumers/{consumer_id}` returns consumer metadata and current cursor position
- [ ] Implicit creation: accessing a non-existent `consumer_id` via poll or ack auto-registers it
- [ ] Integration test: register consumer, restart Core, verify consumer still exists with correct cursor

Mark each item [x] as you complete it. Only close when all are checked.

### US-005: Consumer pull-based event polling [Backend]
**Description:** As a consumer, I want to poll for events since my last acknowledged position so I can process them at my own pace.

**Acceptance Criteria:**
- [ ] `GET /api/v1/consumers/{consumer_id}/events?limit=N` returns events after the consumer's last ack position
- [ ] If no events are available, returns empty list with 200
- [ ] Events are filtered by the consumer's `event_type_filters` if configured
- [ ] Response includes a `position` field for each event (used in ack)
- [ ] Integration test: ingest 5 events, register consumer, poll — returns all 5; ack position of event 3; poll again — returns events 4 and 5

Mark each item [x] as you complete it. Only close when all are checked.

### US-006: Consumer acknowledgment [Backend]
**Description:** As a consumer, I want to acknowledge events I've processed so that subsequent polls only return new events.

**Acceptance Criteria:**
- [ ] `POST /api/v1/consumers/{consumer_id}/ack` with `{"position": "..."}` updates the consumer's cursor
- [ ] Ack is durable — survives Core restart
- [ ] Acking a position earlier than current cursor is a no-op (idempotent, no error)
- [ ] Acking a position beyond the latest event returns 400
- [ ] Integration test: ack position, restart Core, poll — returns only events after acked position

Mark each item [x] as you complete it. Only close when all are checked.

### US-007: WebSocket durable subscription with auto-replay [Backend]
**Description:** As a WebSocket client, I want to connect with a `consumer_id` so that missed events are replayed before switching to real-time delivery.

**Acceptance Criteria:**
- [ ] `WS /ws?consumer_id=X` connects and replays all events since consumer's last ack position
- [ ] After replay completes, switches to real-time delivery of new events
- [ ] Events are filtered by the consumer's `event_type_filters` if configured
- [ ] `WS /ws` without `consumer_id` behaves as before (fire-and-forget, all events)
- [ ] Integration test: ingest 3 events, register consumer, ack event 1, connect WebSocket with consumer_id — receives events 2 and 3 as replay, then receives new events in real-time

Mark each item [x] as you complete it. Only close when all are checked.

### US-008: Server-side event type prefix filtering on WebSocket [Backend]
**Description:** As a WebSocket subscriber, I want to declare which event type prefixes I'm interested in so Core only sends me matching events.

**Acceptance Criteria:**
- [ ] After WebSocket connect, client can send `{"type": "subscribe", "filters": ["prefix1.*", "prefix2.*"]}`
- [ ] Core only forwards events whose `event_type` matches at least one filter prefix (e.g. `scheduler.*` matches `scheduler.started`)
- [ ] Prefix matching uses everything before `.*` as the prefix (e.g. `scheduler.*` -> starts with `scheduler.`)
- [ ] Sending a new `subscribe` message replaces previous filters
- [ ] No filter message = receive all events (backwards compatible)
- [ ] Integration test: connect with filter `["scheduler.*"]`, ingest `scheduler.started` and `trade.executed` — only `scheduler.started` is delivered

Mark each item [x] as you complete it. Only close when all are checked.

### US-009: RESP3 server-side filtering [Backend]
**Description:** As a RESP3 subscriber, I want to subscribe with event type prefix filters so I only receive relevant events.

**Acceptance Criteria:**
- [ ] `SUBSCRIBE scheduler.* index.*` subscribes to matching event type prefixes
- [ ] Events not matching any subscribed prefix are not delivered
- [ ] `SUBSCRIBE *` receives all events (default/backwards compatible)
- [ ] Integration test: RESP3 subscribe with `scheduler.*`, ingest mixed events, verify only scheduler events received

Mark each item [x] as you complete it. Only close when all are checked.

## Functional Requirements

- FR-1: The system must accept an optional `expected_version` (u64) on `POST /api/v1/events`
- FR-2: When `expected_version` is provided and does not match current entity version, the system must reject the write with HTTP 409 and body `{"error": "version_conflict", "current_version": N}`
- FR-3: The version check and WAL append must be atomic — no concurrent write can slip between check and append
- FR-4: `GET /api/v1/events/query` must include `entity_version` when filtered by a single `entity_id`
- FR-5: The system must persist consumer registrations and cursor positions durably (survive restart)
- FR-6: `GET /api/v1/consumers/{id}/events` must return only events after the consumer's last acknowledged position
- FR-7: `POST /api/v1/consumers/{id}/ack` must durably advance the consumer's cursor
- FR-8: WebSocket connections with `consumer_id` must replay missed events then switch to real-time
- FR-9: WebSocket `subscribe` messages must filter events by type prefix server-side
- FR-10: RESP3 `SUBSCRIBE` must support event type prefix patterns
- FR-11: All new features must be backwards compatible — existing clients with no `expected_version`, no `consumer_id`, and no filters must behave identically to current behavior

## Non-Goals (Out of Scope)

- **Query Service pass-through** — this PRD covers Core only; Query Service forwarding is a separate effort
- **SDK updates** — Rust/Go/Python/TypeScript SDK changes will be a follow-up PRD
- **Wallet integration** — AlphaSigmaPro migration from Postgres coordination is downstream work
- **Consumer groups / competing consumers** — consumers are independent; load-balanced consumption is future work
- **Exactly-once delivery** — consumers use at-least-once with idempotent processing
- **Event type filtering on HTTP query API** — only on real-time subscriptions (WebSocket/RESP3)
- **Glob patterns beyond prefix** — only `prefix.*` is supported; complex patterns are future work

## Technical Considerations

- **Atomicity of version check:** The `DashMap` entry for an entity must be locked during check + WAL append. Consider using `DashMap::entry()` API or a per-entity lock to prevent TOCTOU races.
- **Version recovery on startup:** When replaying WAL on startup, rebuild per-entity version counters by counting events per entity_id.
- **Consumer cursor storage:** Store as events in a reserved `__system/consumers` namespace using the existing WAL pipeline — avoids adding a new storage mechanism.
- **WebSocket replay ordering:** Replay events must be sent in global order (WAL sequence) filtered by consumer's type filters, followed by a sentinel message indicating replay is complete before switching to real-time.
- **Performance at scale:** Version lookups are O(1) via DashMap. Consumer cursor checks add one DashMap lookup per poll. Prefix matching is O(filters x 1) per event broadcast — negligible at current scale (~1000 events/day).
- **Existing event format:** The `version` field returned on successful write is the new entity version after the append. Existing events in WAL do not need migration — versions are computed on replay.

## Success Metrics

- All integration tests pass for version conflict scenarios (concurrent writers, version mismatch, no-version writes)
- Consumer cursor survives Core restart with zero event loss
- WebSocket replay delivers exactly the missed events on reconnect
- Server-side filtering reduces messages delivered to filtered subscribers to only matching events
- Zero regressions — existing tests continue to pass, existing clients work without changes

## Open Questions

1. **Consumer TTL:** Should inactive consumers be garbage-collected after some period? (Not in scope for v1, but worth considering the schema for future support)
2. **Replay backpressure:** If a consumer has millions of missed events, should replay be paginated or streamed? (At current scale of ~1000 events/day this is not urgent)
3. **Version semantics for batch writes:** If a future batch ingest API is added, does `expected_version` apply to the first event in the batch or each individually? (Not in scope — current API is single-event)
