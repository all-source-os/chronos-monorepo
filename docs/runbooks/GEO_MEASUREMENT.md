# Runbook — GEO measurement

**Status:** foundation only. The contract, the emitter and `geo report` exist;
nothing is instrumented yet, so a live report over any window currently returns
zero. The layer slices are what start filling it.

**Scope:** how AllSource shows up in generative engines (ChatGPT, Claude,
Perplexity, Gemini) — who arrives from them, which of their crawlers read the
site, what they say about us, and whether a change moved any of it.

- Event contract: [`docs/contracts/geo-events/README.md`](../contracts/geo-events/README.md)
- Tooling: [`tooling/geo/`](../../tooling/geo) — `geo-core` (types + emitter), `geo-cli` (the `geo` binary)

## Why this exists

An increasing share of AllSource's buyers never see a blue link. They ask an
assistant what to use for agent memory and act on the answer. `apps/web` has no
analytics, no crawl-log pipeline and no record of what the engines say about
us — so every GEO decision to date has been a guess. This program replaces the
guessing with a durable, queryable timeline in Core.

## The five layers

| layer | question it answers | event | status |
|---|---|---|---|
| 1 — Referral attribution | Who arrived here from an AI surface? | `geo.referral.observed` | not implemented (prompt 026) |
| 2 — Crawl diagnostics | Which AI crawlers read the site, and did they get a 200? | `geo.crawl.observed` | not implemented (prompt 024) |
| 3a — Share of voice | Do the engines name us when asked our category question? | `geo.sov.probed` | not implemented (prompt 025) |
| 3b — Interrogation | When they name us, is what they say true? | `geo.interrogation.probed` | not implemented (prompt 025) |
| 4 — Self-report | What do people say sent them, when the referrer was stripped? | `geo.selfreport.captured` | not implemented (prompt 026) |
| 5 — Experiments | Did a change to a surface move any of the above? | `geo.experiment.started` / `.scored` | not implemented (prompt 027) |

Layers 1 and 2 are observational and cheap. Layer 3 costs LLM calls. Layer 4 is
the only one that survives a stripped referrer. Layer 5 closes the loop.

## Getting an API key

GEO telemetry is written through the **Control Plane gateway**
(`https://api.all-source.xyz`), never to Core directly — Core does not
authenticate public traffic. You need a gateway API key with ingest rights.

Either mint one in the dashboard, or self-serve:

```bash
curl -X POST https://api.all-source.xyz/api/v1/onboard/start \
  -H 'Content-Type: application/json' \
  -d '{"email": "you@example.com", "name": "GEO measurement"}'
```

Then:

```bash
export ALLSOURCE_API_KEY=<key>
export ALLSOURCE_API_URL=https://api.all-source.xyz   # optional; this is the default
```

`ALLSOURCE_API_KEY` is required for anything that touches the gateway. Nothing
in this tooling degrades to a silent no-op when it is missing — a GEO window
with an unnoticed hole in it is worse than a job that fails.

## Building

`tooling/geo` is its own Cargo workspace with its own `Cargo.lock`, excluded
from the root workspace so it can never pull `apps/core` into a tooling build.

```bash
cd tooling/geo
cargo build --release          # binary at tooling/geo/target/release/geo
cargo test
cargo clippy --all-targets -- -D warnings
```

## Running `geo report`

Count `geo.*` events per layer over a window.

```bash
# last 30 days (default), live against the gateway
./tooling/geo/target/release/geo report

# an explicit window
./tooling/geo/target/release/geo report \
  --since 2026-07-01T00:00:00Z --until 2026-08-01T00:00:00Z

# no key, no network: prints the envelopes the emitter would POST,
# then tallies them
./tooling/geo/target/release/geo report --dry-run
```

Flags: `--days` (default 30, ignored when `--since` is given), `--since`,
`--until` (RFC 3339, UTC), `--max-events` (default 100000, guards a runaway
scan), `--page-size` (default 500).

### Reading the table

```
LAYER NAME                   EVENT TYPE                    EVENTS  ENTITIES  PRODUCED BY
1     Referral attribution   geo.referral.observed              1         1  prompt 026
```

**EVENTS vs ENTITIES is the important column pair.** Every observation's
`entity_id` embeds its idempotency key, so re-ingesting the same log window
appends a version to the same entity rather than creating a new one. `ENTITIES`
is therefore replay-safe and `EVENTS` is not — if `EVENTS` runs well ahead of
`ENTITIES`, something re-ingested a window.

An `unrecognised geo.* types` section means a newer producer is writing to the
stream than this binary knows about. Rebuild.

## The other subcommands

`geo crawl`, `geo probe` and `geo research` are declared but exit non-zero,
each naming the slice that implements it. That is intentional: the CLI surface
is fixed now so the layer slices can land without re-litigating the shape.

## Emitting from a later slice

```rust
use geo_core::{EmitMode, GeoEmitter, GeoEvent, CrawlObserved};

let emitter = GeoEmitter::from_env(EmitMode::Live)?;   // or EmitMode::DryRun
emitter.emit(&GeoEvent::Crawl(CrawlObserved { /* ... */ })).await?;
```

The emitter is transport-thin — it knows nothing about layers, bots, prompt
sets or scoring. Keep it that way; that separation is what lets the layer
slices be built independently.

## Troubleshooting

**`ALLSOURCE_API_KEY is not set`** — expected when no key is exported. Export
one, or use `--dry-run`.

**Report shows zeros everywhere** — expected until a layer slice ships. There
is nothing writing `geo.*` events yet.

**`docs/contracts/geo-events/examples/… has drifted from geo-core`** — a
payload type changed without regenerating the committed examples:

```bash
cd tooling/geo && cargo test -p geo-core -- --ignored regenerate_contract_examples
```

Then re-read the diff before committing — it is the contract changing, and the
contract doc's field tables need the same edit.

## Appending to this runbook

Each layer slice should add: how its ingest is triggered and on what schedule,
what it costs, and how to tell a bad run from an empty one.
