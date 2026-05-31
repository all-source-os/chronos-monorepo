# mammoth hosted tier — pricing & abuse-control options

> **Status:** Decision input for chronis bead `t-a238`. This is an **options
> menu, not a decision** — the pricing call is the owner's. Drafted 2026-05-31.
> Source positioning: `docs/proposals/ALLSOURCE_AGENT_MEMORY_ECOSYSTEM.md`.

## The one hard constraint (already decided)

Local-only is **free forever** and stays the default. The hosted tier
(cross-machine / team sync via `--sync-to` + `--api-key`) is the *only* thing
being priced. The proposal's rule: **metered/paid from day one — never an
unbounded free hosted tier.** A stars-driven launch could dump thousands of free
users onto Core overnight; an open free sync tier is the failure mode, not a
growth lever. So every option below meters or caps the hosted tier.

## What's meterable (the billing surface)

The hosted tier ships `prime.*` events to a tenant's Core. The natural meters,
in order of how legible they are to a user:

| meter | pros | cons |
|---|---|---|
| **Stored memories** (node count) | intuitive ("how much it remembers"), maps to value | needs a count cap + eviction story |
| **Synced events / month** (writes) | matches Core's actual cost driver; event-sourced billing already exists (`t-0e20`, x402 `t-2560`) | less intuitive than "memories" |
| **Seats** (team members on a tenant) | simplest team monetization; predictable revenue | doesn't cap per-user backend cost |
| **Machines / devices** | mirrors the core value ("memory across machines") | easy to circumvent; weak meter |

Recommendation: **price on a primary meter the user understands (stored memories
or seats) and *cap* on the cost driver (synced events/month) for abuse control.**
Reuse the existing `VARIANT_MAP` tier resolution (`t-32b7`) and the
event-sourced billing path rather than building new metering.

---

## Three tier-structure options

### Option A — "Free local, flat team" (simplest)

| tier | price | what |
|---|---|---|
| **Local** | $0 | on-disk, unlimited, no account (the default) |
| **Sync** | flat **$X/user/mo** | cross-machine + team sync, soft cap on synced events/mo |

- **Pros:** dead simple to explain and bill; one paid SKU; predictable revenue.
- **Cons:** flat fee is a wall for solo devs who just want laptop↔desktop; no
  free taste of sync (hurts the caveman-style curve).
- **Best if:** the goal is revenue clarity over funnel width.

### Option B — "Free sync taste, then metered" (recommended for adoption)

| tier | price | what |
|---|---|---|
| **Local** | $0 | on-disk, unlimited (default) |
| **Sync Free** | $0 | cross-machine for **1 user**, capped (e.g. ≤N stored memories / ≤M synced events/mo) |
| **Sync Pro** | **$X/mo** | higher caps, 1 user |
| **Team** | **$Y/user/mo** | shared tenant memory, seats, admin |

- **Pros:** preserves a frictionless cross-machine taste (the magic moment crosses
  machines for free), then converts on caps; widest funnel; matches how the
  proposal frames hosted as a *pull*. The free-sync cap is itself the abuse limit.
- **Cons:** a metered free tier still has per-user cost — the caps must be tight
  enough that abuse can't be free. More SKUs to maintain.
- **Best if:** the goal is the caveman-style adoption curve. **Recommended.**

### Option C — "Usage-based / agent-native" (x402)

| tier | price | what |
|---|---|---|
| **Local** | $0 | default |
| **Sync** | **pay-as-you-go** per synced event / per recall, via x402 | no subscription; agents pay per use |

- **Pros:** aligns cost exactly to usage; leans on the existing x402 agent-payments
  rails (`t-2560`); novel + on-brand for an agent-native product; zero "is it
  worth the subscription" friction.
- **Cons:** unpredictable bills scare humans; metering per-recall adds latency/
  complexity; harder to message on a marketplace card than "$X/mo".
- **Best if:** the buyer is an *agent/automation* budget, not a human seat. Could
  layer **under** Option B (subscription for humans, x402 overage for agents).

---

## Abuse controls (independent of tier choice — ship all)

Per the proposal's MCP-fragmentation / backend-cost risks. These are the gateway
guardrails, not the pricing:

1. **Hard cap on the free sync tier** — N stored memories AND M synced events/mo,
   enforced at the Control Plane (auth already terminates there). Past the cap:
   stop syncing, keep local working, prompt upgrade. Never silently drop data.
2. **Per-tenant rate limit** on the sync ingest path (reuse the existing abuse
   epic `t-2b42` rate-limit machinery: Fly concurrency `hard_limit`, IP limits).
3. **Per-key event-size + frequency ceiling** — Core already enforces a 256KB
   max event payload (`t-364e`); add a per-API-key events/sec ceiling.
4. **Email-verification gate** before any hosted sync (mirrors `t-6769`,
   "email verification gate at 10K events") — kills throwaway-account abuse.
5. **Metering is event-sourced** — bill from the same Core events that the
   billing epic (`t-0e20`) already projects; no separate meter to drift or game.

---

## Recommendation (still the owner's call)

**Option B + all abuse controls**, primary meter = stored memories, hard cap =
synced events/mo. It keeps the frictionless cross-machine taste that drives the
launch curve, converts on a meter users understand, and the free-tier cap doubles
as the abuse limit. Layer Option C (x402 overage) later if agent-budget demand
shows up — the rails already exist.

Pick a structure (A/B/C), then the only remaining inputs are the **numbers**
(price points + cap thresholds), which need a unit-cost-per-synced-user estimate
from Core's Fly bill — that's the one piece of data not in this repo.

## Not in scope

This doc decides nothing. It does not set prices, change `VARIANT_MAP`, or touch
billing code. Implementing the chosen tier (caps, gateway limits, the
`/memory-status` upgrade nudge wiring) is follow-up work once the structure +
numbers are chosen.
