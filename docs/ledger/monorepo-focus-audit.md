# Monorepo audit — unused code, dormant features, focus

Date: 2026-08-11. Commit: `fee857b` + local perf work.

## On method — why this is only half an autoresearch run

Autoresearch needs one scalar and a propose/score/keep loop. "Unused code" has
one; **"missed opportunities to improve focus" does not**. Forcing a strategic
judgement through a scoring loop produces ritual, not measurement, so this
document is explicit about which claims are counted and which are argued:

- §1 and §2 are **measured** — every number here was produced by a command.
- §3 is **argued** from that evidence. Treat it as a case, not a metric.

No propose/keep/discard loop was run: nothing was removed. This is the analysis
pass that would decide what a removal loop should target.

## 1. Unused code — measured

### Rust: essentially clean

| Signal | Count |
|---|---|
| `#[allow(dead_code)]` across all first-party Rust | **9** (6 core, 2 prime-mcp, 1 elsewhere) |
| `cargo check --workspace` warnings | **0** |

Across ~150K lines of first-party Rust. **Rust is not where the waste is.**

### TypeScript: small, and mostly free

Scanned 348 `.ts`/`.tsx` files across `apps/web`, `apps/admin`, `packages/ui`:

| | Count |
|---|---|
| Exported symbols | 545 |
| Exported but referenced in no other file | 141 |
| — of which **types/interfaces** (erased at compile time, **zero runtime cost**) | 106 |
| — of which **values** (functions/consts/classes — real shipped bytes) | 35 |

The 35 values do **not** all reflect waste. Verified by hand:

- **3 are false positives** — `generateStaticParams`, `generateMetadata`,
  `dynamicParams` are Next.js conventions invoked by the framework, not by
  application code. A name-reference scan cannot see framework contracts.
- **Several are over-exported, not unused** — e.g. `surfaceForHost`,
  `surfaceForCampaignToken`, `discoverySource` have zero external references but
  are used inside their own module. They should be private; they are not dead.
  (These are mine, added earlier today — the detector caught my own sloppiness.)
- **4 are genuinely dead**, confirmed at zero references anywhere:

  | Symbol | File | Lines |
  |---|---|---|
  | `useStreams` | `src/hooks/use-streams.ts` | 57 |
  | `useEventTypes` | `src/hooks/use-event-types.ts` | 57 |
  | `useEventsByEntity` | `src/hooks/use-events.ts` | — |
  | `UsageProgress` | `src/components/dashboard/usage-progress.tsx` | 95 |

**~209 lines of confirmed dead code in a ~370K-line repo.** That is a rounding
error. The honest headline of this audit is: **the codebase is not carrying
meaningful dead code.** Anyone budgeting a cleanup sprint against it would be
optimising the wrong thing.

Caveat on the detector: it matches identifiers textually, so it cannot see
dynamic imports, string-keyed registries, or framework contracts. It over-reports
(as shown) and could under-report re-exported barrels. Every claim above was
hand-verified; the raw 141 was not.

## 2. Unused *deployments* — measured, and this one costs money

Two services are **running in production right now**, health-probed by the
control plane, with **zero commits in 90 days**:

| App | Last commit | Commits/90d | Live check |
|---|---|---|---|
| `apps/auth` | 2026-04-28 | **0** | `https://allsource-auth.fly.dev/health` → **HTTP 200** |
| `apps/registry` | 2026-03-09 | **0** | `https://allsource-registry.fly.dev/health` → **HTTP 200** |

Both have a `fly.toml`. Both are billed Fly machines.

`apps/auth` is the sharper case: the architecture moved on. The Control Plane
owns authentication now, yet the Rust auth service is still deployed, still
health-checked (`heartbeat.go:104`), and still referenced by the Query Service's
JWKS plug (`AUTH_JWKS_URL` → `allsource-auth.internal:3903`). That plug has an
error fallback, so whether it is genuinely load-bearing is **not determinable
from source alone** — it depends on whether any live token is validated through
that path. That question needs production telemetry, not a grep.

`apps/registry` (777 lines) is referenced by nothing but its own manifests and
the C4 architecture diagram (`apps/web/src/data/c4-model.ts`).

**Do not delete either on the strength of this document.** Determine first
whether traffic flows through them. But they are the only findings here with a
recurring monthly cost attached.

## 3. Focus — argued, not measured

The evidence above says the problem is not code you should delete. Looking at
what is *built*, the pattern is different and more interesting.

**Surface area, one maintainer:** 11 apps, 4 SDKs, 17 tooling crates,
3 shared crates, ~370K lines of first-party code, **30 design proposals** in
`docs/proposals/`, 10 runbooks.

**The recurring shape: built to ~90%, then parked awaiting one external step.**
Not speculation — the integration code exists and is idle:

| Feature | Code present | Blocked on |
|---|---|---|
| Proactive-comms engine | 11 Go files reference Resend | Resend not the live sender |
| AI inbox | 6 files reference Nylas | Nylas credentials |
| GEO measurement | full harness, 2 frozen prompt sets, 206 tests | 4 provider API keys; baseline never run |
| Prime Hound | 18 of 22 beads done | hosted infra (billing metering, team graphs) |

Four substantial systems, each finished enough to test and none of them
producing value, each waiting on a credential or an infra decision rather than
on engineering.

**The implication.** The scarce resource in this repo is not engineering
throughput — the code gets written, and it gets written well (Rust clean, tests
green, contracts documented). The scarce resource is **activation**. Every hour
spent building a fifth parked system is worth less than the hour that turns one
of the four already-built ones on.

The cheapest available wins, in order:

1. **Turn one parked system on.** GEO is closest — the harness runs today and
   needs four API keys and ~$20–30 to produce a real baseline.
2. **Decide `apps/auth`'s fate.** Confirm whether the JWKS path is live; if not,
   that is a deployed service and its maintenance surface removed outright.
3. **Same question for `apps/registry`**, which has an even weaker claim on
   existence.
4. **Delete the 4 dead TS symbols and tighten the over-exports** — genuinely
   trivial, listed here only so nobody mistakes it for the main event.

**What this document cannot tell you:** which of the parked systems is worth the
most. That depends on revenue and strategy, not on the repository. The audit can
say they are parked; it cannot rank them.

## Still open

- Whether `apps/auth` and `apps/registry` serve live traffic — needs production
  telemetry, not source analysis.
- The TS detector's 106 "dead types" were not hand-verified individually; they
  carry no runtime cost, so the effort is hard to justify.
- Elixir (`apps/query-service`, `apps/mcp-server-elixir`, ~67K lines) and Go
  (`apps/control-plane`, 57K lines) were **not** scanned for dead code — no
  detector was run against them. Their absence from §1 is a gap in coverage, not
  a clean bill of health.
