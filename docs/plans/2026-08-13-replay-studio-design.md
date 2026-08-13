# Replay Studio design

Date: 2026-08-13

## Product job

Help an operator rebuild one tenant-owned read-model from immutable event history without downtime, cross-tenant access, or publishing partial output.

## Diagnosis

Live `/dashboard/tools/replay` crashed while rendering history because Core serializes replay states as lowercase and web expected capitalized values. Rendering dereferenced `STATUS_CONFIG[status].icon` without a fallback.

More important: public Query Service replay routes forwarded to Core's global replay manager. Requests carried no tenant ID, history exposed every replay, and replay processing appended into existing projection state without clearing it. “Rebuild projections” was therefore inaccurate and unsafe.

## Chosen model

Replay belongs to Query Service's existing per-tenant projection engine:

1. Authenticate tenant at controller boundary.
2. Accept one enabled projection target.
3. Capture replay cutoff.
4. Fold tenant events through curated reducer into shadow generation.
5. Buffer live events newer than cutoff while fold runs.
6. Publish completed generation through one active-generation pointer write.
7. Keep existing generation when replay fails or is cancelled.

Replay history stays tenant-scoped. Core's global projection engine is no longer exposed through dashboard endpoints.

## Interface

Page uses one operational story: `Event history → Read-model → Publish`.

- Header states outcome: “Rebuild from truth.”
- Target is an enabled projection selector, not free text.
- Safety contract makes source immutability, tenant isolation, failure isolation, and uninterrupted reads explicit.
- Run history shows target, status, progress, throughput inputs, elapsed time, and recovery actions.
- Unknown backend states render as “Status unavailable” instead of crashing.
- No decorative animation; only active progress spins/transitions.

## Research inputs

- AWS EventBridge Replay separates source, destination, timeframe, status, and source-event preservation: <https://docs.aws.amazon.com/eventbridge/latest/userguide/eb-replay-archived-event.html>
- AWS archives retain source events after replay: <https://docs.aws.amazon.com/eventbridge/latest/userguide/eb-archive.html>
- Axon treats replay/reset as an operational sequence whose progress and read-model availability must be visible: <https://docs.axoniq.io/axoniq-platform-reference/features/management/>

Applied principle: replay is controlled state reconstruction, not generic event re-submission.

## Acceptance

- Lowercase, title-case, and unexpected statuses cannot crash page.
- Tenant cannot list, read, cancel, or delete another tenant's replay.
- Replay cannot target a disabled or unknown projection.
- Rebuild replaces state; it does not double-count existing state.
- Failed rebuild preserves last known-good state.
- Existing read-model stays readable while shadow generation builds.
- Public page explains operation without ambiguous “re-process” copy.
