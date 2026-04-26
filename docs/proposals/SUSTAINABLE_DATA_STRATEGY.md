# Sustainable Data Strategy — proposal

**Status:** draft, 2026-04-26
**Trigger:** incident #160 — Core OOM-killed during WAL replay; api.all-source.xyz returned 502s for ~13h until manual intervention.

The incident was a symptom of a deeper design choice: **Core treats the in-memory map as the canonical query path and materializes all events at boot to fill it.** That makes startup memory and time both linear in total dataset size, which is a property no production database has long-term. This document lays out the path off it.

## Implementation status

| Item | Status |
|---|---|
| **Operational stop-gap** — Core memory 1 GiB → 4 GiB (`shared-cpu-2x`) | ✅ Shipped 2026-04-25 |
| Step 1 — Tenant-partitioned on-disk layout | ⬜ Not started |
| Step 2 — Lazy per-tenant load | ⬜ Not started |
| Step 3 — Memory-budgeted LRU cache | ⬜ Not started |
| Step 4 — Per-tenant snapshots | ⬜ Not started |
| Step 5 — Retention policies (system tenant first) | ⬜ Not started |
| Step 6 — Bounded WAL replay | ⬜ Not started |

The 4 GiB bump is explicitly **not a strategy step**. It buys runway, not survivability — once the dataset grows past the new ceiling we hit the same wall. None of the steps below have been started; this proposal is about what that work looks like.

## The shift in mental model

Today (v0.19.2):
- WAL + Parquet are durable, but DashMap is the *source* served to readers.
- Boot: load every Parquet file → replay WAL → ready. ~870 MB resident at 451k events.
- Memory tracks dataset, not working set.

Target:
- DashMap becomes a **tenant-keyed query cache**, not the source of truth.
- Reads resolve against Parquet (with the cache in front). The cache is hot for live tenants, cold for everyone else.
- Boot: open the WAL, run a structural integrity check, ready. No replay.
- Memory tracks working set.

This is the same pattern Postgres/SQLite use: pages on disk, buffer pool in memory. We just label our "pages" as tenant + time slabs.

## Concrete steps, ordered for ROI

### Step 1 — Tenant-partitioned on-disk layout

Parquet files keyed by `tenant_id/yyyy-mm/`. Today they're a flat pile under `/app/data/storage/`. Once partitioned, every read can prune to one tenant's directory immediately, and a single noisy tenant can't make every query slow. This is the prerequisite for everything else.

### Step 2 — Lazy per-tenant load

Replace boot-time `load_all` with `load_tenant(tenant_id)` triggered on first query for that tenant. Boot becomes O(1) regardless of dataset size. First-query latency for a cold tenant goes up (single-digit seconds for 100k events), subsequent queries hit cache. This is the fix the incident review surfaced: replay scoped by tenant.

### Step 3 — Memory-budgeted LRU cache

Wrap DashMap in an LRU keyed by `(tenant_id, event_type, time_bucket)` with a configurable byte budget (env: `ALLSOURCE_CACHE_BYTES`, default e.g. 2 GiB). Cold tenants evict cleanly, hot tenants stay resident. This is what makes the system fit on a small VM regardless of how big the dataset grows.

### Step 4 — Per-tenant snapshots

Hourly compaction job per tenant emits `snapshot.<tenant>.<from>-<to>.parquet`, replacing the constituent raw files. Bounds first-query latency for cold tenants and bounds disk amplification. (The snapshot machinery already exists; today it's invoked at the global level — needs to be per-tenant.)

### Step 5 — Retention policies

Per-tenant TTL config, applied during the same compaction pass. The CP heartbeat tenant (`system`) is the obvious first user — at 8 services × 6/min × 60 × 24 = ~69k heartbeat events/day, we don't need them past 30 days. Marketing's claim of "every probe is a permanent event" is right that they *can* be permanent; what we want is *configurable*, with sensible defaults.

### Step 6 — Bounded WAL replay

Even with lazy load, WAL replay on dirty shutdown today walks the entire WAL. Compact the WAL after every checkpoint so replay never sees more than one checkpoint interval worth of writes. Caps cold-start time when we *do* need to recover.

## What this buys us, in order

| | Today | After step 2 | After step 3 | After step 5 |
|---|---|---|---|---|
| **Boot time** | O(N events) | O(1) | O(1) | O(1) |
| **Memory** | O(N events) | O(N events) | O(working set) | O(active working set, retention-bounded) |
| **OOM ceiling** | 1 GiB cap = OOM at ~451k events | Same risk | Configurable | Configurable, dataset growth ≠ memory growth |
| **Outage shape** | Total | First-query slow per tenant | Same | Same |
| **Replication feasibility** | Theoretical (CORE_REPLICATION_DESIGN.md) | Same | **Now feasible** — followers lazy-load just what they're asked about | Same |

Step 3 is the inflection: the memory cap stops being a function of total events written and starts being a function of how much you're willing to spend per Core instance. That's the property we need for the marketing claim "Core *is* the database" to hold up under load.

## What's already in flight (adjacent, not part of this strategy)

These shipped alongside the incident response and are documented for completeness:

- **Health endpoint split** — `/livez` (liveness, always 200, used by Fly's machine check) and `/health` (readiness, returns 503 + `status: "unhealthy"` when Core is unreachable). Fixes #160's secondary ask about misleading load-balancer signal.
- **In-memory probe cache in CP** — the status feed reads the heartbeat emitter's in-process cache before falling back to Core. Removes the circular "ask Core whether Core is up" dependency that took the status page down with Core during the incident.
- **`status.all-source.xyz` retired** — single canonical status UI at `www.all-source.xyz/status`; standalone hostname, Vigil app, and Fly cert removed.

These are operational fixes, not data-strategy steps. They are listed here so a future reader doesn't conflate "we already did some work" with "we made progress on the strategy" — we did not.

## Things explicitly out of scope here

- **Adding Postgres for events.** Already a marked anti-pattern in CLAUDE.md; this plan is *how to keep events in Core sustainably*, not how to give up.
- **Replacing DashMap.** It's fine as the cache primitive; the change is what we put *in* it and how we bound it.
- **Multi-region replication.** Tracked separately in `CORE_REPLICATION_DESIGN.md`. Step 3 makes follower bootstrap finally tractable, but the replication protocol design is independent.
