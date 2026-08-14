# Replay Studio analysis and SDK design

Date: 2026-08-14
Chronis: `t-bfa32a`

## Product job

Let an operator understand a projection rebuild before starting it, then run and monitor the same tenant-safe workflow from Replay Studio or an AllSource SDK.

## Competitive lessons

- AWS EventBridge makes source, destination, replay window, state, and last replayed time explicit. Replayed events also carry replay identity.
- Kafka Streams requires a dry run before an offset reset and shows affected topics and positions.
- Axon exposes reset position, reset context, segment progress, and catch-up state.
- KurrentDB clients expose checkpoints and distinguish catch-up replay from live delivery.

AllSource should retain its stronger invariant: projection history folds into a shadow generation while the current read-model remains live, then publishes through one atomic pointer swap.

Sources:

- https://docs.aws.amazon.com/eventbridge/latest/userguide/eb-replay-archived-event.html
- https://docs.confluent.io/platform/current/streams/developer-guide/app-reset-tool.html
- https://docs.axoniq.io/axoniq-platform-reference/features/management/
- https://docs.kurrent.io/clients/python/v1.3/subscriptions

## Options considered

1. Client-only analysis. Fast, but every SDK would implement different sampling and safety rules.
2. Query Service analysis with thin SDK clients. Chosen: one tenant-scoped contract, one source of truth, consistent UI and SDK behavior.
3. Full replay orchestration API with callbacks and scheduled jobs. Deferred until preview and monitoring usage proves demand.

## Contract

`POST /api/replay/preview` accepts one enabled `projection_name` and returns:

- projection identity, kind, and current status;
- exact event total when Core provides it;
- bounded sample size and whether distributions are sampled;
- event-type distribution;
- most active sampled entities;
- first and last sampled event timestamps;
- server-asserted tenant, source, publication, and availability checks.

Preview never mutates events or projection state. Start remains `POST /api/replay`; run status remains tenant-scoped.

SDK workflow:

```text
analyzeProjectionReplay(name)
        ↓
startProjectionReplay(name)
        ↓
getProjectionReplay(id) / listProjectionReplays()
        ↓
cancelProjectionReplay(id)
```

## Replay Studio

Page job: answer “what changes if I rebuild this read-model?” before offering start.

Visual direction:

- Palette: existing dashboard canvas, border, foreground, primary blue, success emerald, warning amber.
- Type: existing UI face for operations; monospace for IDs, timestamps, and counts.
- Layout: source → analysis → atomic publish rail above a dense impact console.
- Signature: event-composition rail whose segments come from replay preview data, not decoration.
- Motion: progress only. Respect reduced motion.

Replay start stays disabled until current target has a successful analysis. Impact panel shows event total, sampled entities, event window, event-type composition, and four safety checks. SDK panel turns current analysis into copyable integration code.

## Error handling

- Missing or disabled target: 422 with actionable message.
- Core query failure: preview fails without starting replay.
- Empty history: valid preview, zero events, start disabled.
- Partial sample: labelled “sample”; exact total remains separate.
- Changed target: discard stale analysis.
- Failed/cancelled run: current generation remains active.

## Acceptance

- Preview is tenant-scoped and read-only.
- Replay Studio requires preview before start and labels sampled data.
- Event composition, affected entities, window, and safety checks render on desktop and mobile.
- TypeScript, Rust, Python, and Go SDKs expose typed analyze/start/status/list/cancel methods.
- Query Service, SDK, web tests and production builds pass.
- UI proof records zero console and server errors.
