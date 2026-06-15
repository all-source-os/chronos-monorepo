# Email event contract (`email.*`)

Source of truth for the durable Core events the AI-inbox connector emits and the
Prime ingester consumes. Bead `t-7b26f9` (P0 of epic `t-dece6c`).
Proposal: [`../../proposals/AI_INBOX_ON_ALLSOURCE.md`](../../proposals/AI_INBOX_ON_ALLSOURCE.md) §4.3, §4.6, §7.

## Why a contract, not a shared type

The emitter is the **Control Plane (Go)**; the consumer is the **Prime ingester (Rust)**.
App isolation forbids a shared crate across that boundary, and Go cannot use a Rust
type anyway. **JSON Schema is the language-neutral contract** both sides validate
against in their own test suites. These schemas are the authority; language types are
derived from them, never the reverse.

## Envelope

Every `email.*` event is POSTed to Core `POST /api/v1/events` through the Control Plane
delegation path. The caller supplies:

| field | who sets it | rule |
|---|---|---|
| `event_type` | connector | const — one of the six below |
| `entity_id` | connector | the dedupe / lifecycle key (see below) |
| `tenant_id` | Control Plane | injected at the gateway; **never client-trusted** |
| `payload` | connector | per-event (see `schema/`) |
| `metadata` | connector | per-event |
| `expected_version` | connector (optional) | optimistic-concurrency dedupe — first-ingest only (§4.6) |

Core assigns `id`, `timestamp`, and the per-entity monotonic `version` **server-side** —
they are NOT part of the ingest body.

### `entity_id`

- `received` / `sent` / `triaged` / `replied` / `archived` → the **provider message id**.
  All lifecycle events for one message share it, so a single ordered read
  (`?entity_id=<id>&order=desc&limit=1`) yields current state.
- `drafted` → the **draft id** (a draft has no provider message id until sent; the later
  `email.sent` links back via `metadata.draft_id`).

### Dedupe (§4.6)

At-least-once delivery is assumed. P0 dedupe = `entity_id` + `expected_version: 0` on the
first `email.received`, so a replayed first event hits `VersionConflict` and is dropped
idempotently. Lifecycle redelivery (`triaged`/`replied`/`archived`) is guarded
connector-side by keying on `message_id` + action until Core's `ExactlyOnceRegistry` is
wired into the live ingest path (P4).

## Events

| event | entity_id | meaning | schema | example |
|---|---|---|---|---|
| `email.received` | message id | inbound message ingested (body inline) | [schema](schema/email.received.schema.json) | [ex](examples/email.received.json) |
| `email.sent` | message id | outbound message sent through the user's mailbox | [schema](schema/email.sent.schema.json) | [ex](examples/email.sent.json) |
| `email.triaged` | message id | classified/labelled by agent or human | [schema](schema/email.triaged.schema.json) | [ex](examples/email.triaged.json) |
| `email.replied` | message id | original message marked replied (links the reply) | [schema](schema/email.replied.schema.json) | [ex](examples/email.replied.json) |
| `email.archived` | message id | message archived | [schema](schema/email.archived.schema.json) | [ex](examples/email.archived.json) |
| `email.drafted` | draft id | reply draft composed (pre-send, confirm-gated) | [schema](schema/email.drafted.schema.json) | [ex](examples/email.drafted.json) |

## Privacy (§7)

- `body` is **inline** and durable in Core (Core has no blob store) but is **never** put
  into the embedding text or shown to the agent by default. Only `subject + snippet` are
  embedded.
- The schemas keep recipient data to resolved name/email pairs. Do **not** extend payloads
  with full raw headers — PII-heavy fields stay out of Core events and out of the vector.

## Provider-neutral

Nothing in these payloads names a provider as a structural requirement — `metadata.provider`
is a free string (`"nylas"` first). The connector's `EmailProvider` trait normalizes every
provider into this one shape; downstream of the Control Plane webhook, Core events and Prime
nodes are provider-neutral.

## Validation

Schemas are **JSON Schema 2020-12**. Every file in `examples/` MUST validate against its
`schema/<event>.schema.json`. Downstream beads validate in-language: the CP emitter
(T2 `t-fa9530`, Go) and the Prime ingester (T4 `t-62c2d5`, Rust) each load these schemas in
their own tests. The fixtures here are the canonical inputs those tests assert against.
