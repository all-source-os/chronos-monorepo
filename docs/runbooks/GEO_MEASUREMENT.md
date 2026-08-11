# Runbook — GEO measurement

**Status:** layers 1 and 2 are live. `apps/web` emits `geo.referral.observed`
for arrivals from AI surfaces, and `geo crawl` ingests verified AI-bot hits
from the edge logs as `geo.crawl.observed`. Layers 3-5 are still stubs.

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
| 1 — Referral attribution | Who arrived here from an AI surface? | `geo.referral.observed` | **live** (prompt 024) |
| 2 — Crawl diagnostics | Which AI crawlers read the site, and did they get a 200? | `geo.crawl.observed` | **live** (prompt 024) |
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

`geo probe` and `geo research` are declared but exit non-zero, each naming the
slice that implements it. That is intentional: the CLI surface is fixed now so
the layer slices can land without re-litigating the shape. `geo crawl` is
implemented — see the layer 2 section below.

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

---

# Layer 1 — Direct attribution (AI referrals)

**Question:** who arrived here from an AI surface, and did they convert?

## Read this before you read the number

**Layer 1 is a floor, not a measurement.** ChatGPT, Claude and most assistant
surfaces strip or omit the `Referer` header on outbound links. The majority of
AI-sourced sessions therefore arrive with an empty referrer and are
indistinguishable from Direct traffic. Published estimates of the undercount
vary, but every one of them is large: assume **the true number is a multiple of
what layer 1 reports**, not a small correction to it.

Practical consequences:

- A small `geo.referral.observed` count means *"the referrer survived rarely"*.
  It never means *"the channel is small"*. Do not size the AI channel from this
  layer alone.
- Movement is more trustworthy than level. A 3× rise week-on-week is real
  signal even though the absolute number is wrong.
- **Layer 2 (crawl) is the leading indicator**, not this one. Cloudflare's June
  2025 figures put OpenAI's crawl-to-referral ratio around 1,700:1 and
  Anthropic's around 73,000:1, versus Google's 14:1 — crawl volume moves months
  before referral volume does.
- **Layer 4 (self-report, prompt 026)** is the only layer that sees through a
  stripped referrer. Until it lands, treat layer 1 as a lower bound and nothing
  more.

`geo report` prints this caveat under the layer 1 table so it cannot be read
without it.

## How it works

```
browser  ──POST /api/geo/referral──▶  Next.js route handler  ──▶  Control Plane
(no key)                              (holds ALLSOURCE_API_KEY)      ──▶ Core
```

| piece | file |
|---|---|
| AI-surface map | `apps/web/src/lib/geo-referrers.ts` (`AI_SURFACES`) |
| envelope + idempotency key | same file (`buildReferralEnvelope`) |
| browser beacon | `apps/web/src/components/geo-referral-tracker.tsx` |
| server forwarder | `apps/web/src/app/api/geo/referral/route.ts` |
| analytics install + ADR | `apps/web/src/app/layout.tsx` |

The browser posts a **raw referrer**; it never says which surface it came from.
The route re-classifies server-side, so this endpoint cannot be used to write
arbitrary rows. **No AllSource key is reachable from the client bundle** — the
key is read from `process.env` inside the route handler at request time.

Two producers write `geo.referral.observed` — `apps/web` (TypeScript) and
`geo-core` (Rust). Their idempotency keys must agree byte-for-byte, or one
session would land in Core as two entities and every layer-1 count would
inflate. A vitest asserts the TypeScript envelope against the committed
contract example that the Rust emitter generates:

```bash
cd apps/web && bun run test   # src/__tests__/geo-referrers.test.ts
```

### Conversions

A conversion (`signup_started`, `api_key_minted`) re-emits the **same natural
key** as the arrival, so Core appends version 2 to the arrival's entity with
`converted: true`. The session is counted once; its latest version is its
current state. Call it from anywhere in the app:

```ts
import { reportGeoConversion } from "@/components/geo-referral-tracker";
reportGeoConversion("signup_started");   // no-op for non-AI sessions
```

It is already wired into both signup paths in `apps/web/src/app/(auth)/signup/page.tsx`.

## Analytics choice (ADR summary)

**Vercel Web Analytics** (`@vercel/analytics`) for site-wide traffic, plus the
first-party beacon above for the GEO event stream. Full reasoning — and what
was rejected (PostHog, Plausible/Fathom, GA4) and why — is in the ADR comment
at the top of `apps/web/src/app/layout.tsx`. The short version: Vercel Web
Analytics is cookieless (no consent-banner rewrite) and same-origin (no CSP
widening), but it does **not** expose the raw referrer and user agent in our own
pipeline and cannot join an arrival to a conversion in AllSource — which is
exactly what layer 1 needs.

## Environment variables you must set BY HAND

These are set in the **Vercel dashboard** (Project Settings → Environment
Variables) for the `allsource-web` Vercel project. There is no `fly.toml` for
`apps/web` and there must never be one — the frontend ships via
`git push origin main` (Vercel auto-build) or `vercel --prod`.

| variable | scope | required | notes |
|---|---|---|---|
| `ALLSOURCE_API_KEY` | **server only** | yes | Gateway API key with ingest rights. Never prefix with `NEXT_PUBLIC_` — that would publish it in the browser bundle. |
| `ALLSOURCE_API_URL` | server only | no | Defaults to `https://api.all-source.xyz`. |

Vercel Web Analytics needs **no** environment variable — enable it in Project
Settings → Analytics.

Same names as `tooling/geo` on purpose: one env-var scheme for the whole GEO
programme, not two.

### Verifying no key leaked into the bundle

```bash
cd apps/web
rm -rf .next
ALLSOURCE_API_KEY='as_CANARY_MUST_NOT_SHIP' bun run build
grep -r 'as_CANARY_MUST_NOT_SHIP' .next/          # must print nothing
```

(The string `ALLSOURCE_API_KEY` *does* appear in one client chunk — that is the
pre-existing onboarding UI rendering a copy-paste install command containing the
*user's own* key, not ours.)

## Troubleshooting

**Route returns 503 `GEO telemetry is not configured`** — `ALLSOURCE_API_KEY`
is unset in the Vercel environment. The route logs `[geo] ALLSOURCE_API_KEY is
not set` and refuses; it deliberately does not pretend to succeed.

**Route returns 204** — the arrival was not from a known AI surface. Normal:
the beacon fires on the first page view of every session.

**Arrivals but no conversions** — `reportGeoConversion` reads `sessionStorage`,
which is per-tab. A signup completed in a new tab loses the link. That is a
known undercount on top of the referrer undercount.

---

# Layer 2 — Crawl log diagnostics

**Question:** which AI bots read the site, how often, and were they really who
they said they were?

## The three categories, and why they are never blended

| category | what a rise means | examples |
|---|---|---|
| **Training crawlers** | infrastructure readiness — we are reachable by the corpus | `GPTBot`, `ClaudeBot`, `CCBot` |
| **Search indexers** | citation eligibility — we are in the answer-time index | `OAI-SearchBot`, `Claude-SearchBot`, `PerplexityBot` |
| **User-triggered fetchers** | real-time human demand — someone is asking about us *now* | `ChatGPT-User`, `Claude-User`, `Perplexity-User` |

Their volumes differ by orders of magnitude and they answer different
questions. `geo report` prints them separately and **has no all-bots total by
design**. If you find yourself adding one, stop.

The machine-readable taxonomy is `tooling/geo/geo-core/src/bots.rs`
(`BOTS`, versioned by `TAXONOMY_VERSION`), with each vendor's own documentation
URL cited next to the entry. `geo.crawl.observed` stores the `category` and the
`taxonomy_version` on the event, so a historical count keeps the categorisation
it was written with even after the taxonomy moves a bot.

## Verification is not optional

A `User-Agent` is a string the client chose. Scrapers routinely wear `GPTBot`
to get past a permissive robots.txt, so an unverified count is not a
measurement. Every hit is checked against the vendor's **own published IP
ranges**:

| range list | verifies |
|---|---|
| `https://openai.com/gptbot.json` | `GPTBot` |
| `https://openai.com/searchbot.json` | `OAI-SearchBot` |
| `https://openai.com/chatgpt-user.json` | `ChatGPT-User` |
| `https://claude.com/crawling/bots.json` | `ClaudeBot`, `anthropic-ai`, `Claude-SearchBot`, `Claude-User` |
| `https://www.perplexity.ai/perplexitybot.json` | `PerplexityBot` |
| `https://www.perplexity.ai/perplexity-user.json` | `Perplexity-User` |
| `https://developers.google.com/static/crawling/ipranges/special-crawlers.json` | `Google-CloudVertexBot` |

All seven publish the same JSON shape, so one parser covers them.

Some vendors publish nothing (Common Crawl, ByteDance, Meta, DuckDuckGo). Their
hits are honestly reported as **unverifiable**, never as verified, and are only
emitted with `--include-unverified` (stamped `verified: false`, and excluded
from every count in the report).

There are four distinct outcomes, and collapsing them into a boolean would hide
the one that matters:

| verdict | meaning | emitted? |
|---|---|---|
| `verified` | IP is in the vendor's published range | yes |
| `rejected` | vendor publishes ranges, IP is not in them — **someone is spoofing** | **never** |
| `unverifiable (vendor publishes no ranges)` | claim is uncheckable | only with `--include-unverified` |
| `unverifiable (range list unavailable)` | *our* fetch failed / `--offline` | only with `--include-unverified` |

A fetch failure is never reported as a spoof. That distinction is unit-tested,
because a bug there would make "our network broke" look like "the whole
internet is impersonating GPTBot".

## Running the ingest

```bash
cd tooling/geo && cargo build --release
GEO=./tooling/geo/target/release/geo

# dry run over a log-drain dump — categorised counts, no writes, no key needed
$GEO crawl --since 7d --dry-run --file /path/to/drain.ndjson

# live: write verified hits to the gateway
export ALLSOURCE_API_KEY=<key>
$GEO crawl --since 7d --file /path/to/drain.ndjson

# fully offline and deterministic, against the committed fixture + range snapshot
$GEO crawl --since 2026-07-06T00:00:00Z --until 2026-08-11T00:00:00Z --dry-run \
  --file tooling/geo/geo-cli/tests/fixtures/vercel-log-drain.ndjson \
  --ranges-dir tooling/geo/geo-cli/tests/fixtures/ranges --offline
```

Flags: `--since` (duration `7d`/`12h`/`4w` or RFC 3339), `--until`, `--days`
(default 7), `--file` (repeatable), `--vercel-project`, `--fly-app`,
`--format auto|json|clf`, `--aggregate hit|hourly|daily`, `--ranges-dir`,
`--offline`, `--include-unverified`, `--dry-run`.

### Where the logs come from

- **`--file`** is the workhorse and needs no credential. A **Vercel log drain**
  writes exactly the NDJSON this parses. This is the only source that gives a
  durable multi-week series, so stand the drain up.
- **`--vercel-project`** shells out to `vercel logs <project> --json`. Retrieval
  only — every decision about the bytes is made in Rust. `vercel logs` serves a
  short recent window, not history.
- **`--fly-app`** shells out to `fly logs -a <app> --no-tail --json`, for
  `api.all-source.xyz` (Control Plane). Its lines carry the access line inside
  `message`, which the parser falls through to Combined Log Format for.

Both vendor CLIs need you to be logged in (`vercel login`, `fly auth login`).
If a CLI is missing or unauthenticated the run fails loudly with the install
hint — it does not report zero.

### Aggregation: which level, and why

**Default is `--aggregate hit`: one event per bot hit.** Core is the database
(WAL + Parquet + DashMap, 469K events/sec); it is not the thing under strain,
and per-hit fidelity is what makes "which paths does `Claude-User` actually
fetch" answerable later. The machine noise argument does not apply either,
because `geo.*` lives in its own `geo:` entity namespace and no human reads the
raw stream — they read `geo report`.

Use `--aggregate hourly` or `--aggregate daily` for a wide backfill where you
want a trend and not a million rows. Aggregation is **declared in the payload**
(`aggregation`, `hits`, `window_end`), so a reader can never mistake one row
for one hit, and the bucketed rows' natural key includes the level and window —
so re-running at `--aggregate hit` over the same raw logs recovers per-hit
fidelity without colliding with them.

### Idempotency

Re-running the same window is safe. Each event's `entity_id` embeds its
idempotency key, so a replay appends a version to the same Core entity instead
of creating a second one — and `geo report` folds crawl rows **by entity, last
version wins**, so the hit counts do not double either.

Proven locally: three identical ingests of the same window produced 687 stored
events across **229 distinct entities**, and the reported hit counts
(185 / 27 / 17) were identical to a single ingest.

The natural key includes `request_id`, so two genuinely distinct hits in the
same second on the same path stay two entities rather than collapsing into one.

## Reading `geo report`

```bash
$GEO report --since 30d
```

Per category, per week: hit count, distinct paths, top paths, and a **rolling
4-week median**.

**A median, not a mean, on purpose.** A single overnight crawl burst — one
vendor re-reading the whole site in an hour — moves a 4-week mean by a quarter
and leaves the median untouched. In the committed fixture, week 4 is a 69-hit
burst against a ~28-hit baseline; the mean would read 38, the median reads 29.5.

The `event inventory` table at the top counts **stored rows, not bot hits**.
`EVENTS` vs `ENTITIES` diverging there means a window was re-ingested — which is
fine, and is exactly why the hit counts are folded by entity.

## Fixture and tests

`tooling/geo/geo-cli/tests/fixtures/` holds a five-week Vercel log-drain dump
(256 lines, including humans, two unreadable lines and a spoofed `GPTBot` from
`203.0.113.77`) plus a snapshot of all seven **real** vendor range lists. That
makes the whole pipeline testable offline and deterministically against the
vendors' actual published prefixes.

```bash
cd tooling/geo
cargo test
cargo clippy --all-targets -- -D warnings

# regenerate the fixture after changing the generator
cargo test -p geo-cli -- --ignored regenerate_crawl_fixture
```

## robots.txt

`apps/web/public/robots.txt` now names all three categories explicitly and
**allows all three**, with one paragraph of reasoning per category. Before this
slice the file said `User-agent: * / Allow: /` and named no AI bot: every AI
crawler was allowed *by accident*.

**If you change a decision, read the STRUCTURE NOTE at the top of the file
first.** A crawler obeys exactly one group — the most specific one naming it —
and a named group does **not** inherit the `User-agent: *` rules. The
closed-area `Disallow` list is therefore repeated verbatim in all four groups.
Add a path to one and you must add it to all four, or you will quietly open
`/dashboard/` to a named bot.

## Troubleshooting

**"0 verified" with a non-zero identified count** — the run prints a loud
warning for exactly this, because a verification bug looks identical to "no AI
traffic". Check the `vendor IP range lists` block in the output: if every list
says `UNAVAILABLE`, it is a fetch problem, not a traffic problem.

**"refusing to report a crawl number built from a fraction of the log"** — over
half the lines were unreadable. Check `--format` against the sample lines the
run prints.

**A vendor list looks stale** — the run prints each list's own `creationTime`.
Vendors update these irregularly; a months-old `creationTime` is normal, but a
sudden jump in `rejected` right after a vendor rotates ranges is not a spoofing
wave — refresh `--ranges-dir`.
