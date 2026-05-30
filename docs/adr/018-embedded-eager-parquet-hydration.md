# ADR 018: Eager Parquet Hydration on Embedded Boot

- **Status:** Accepted
- **Date:** 2026-05-30
- **Release:** chronis v0.7.2 (consumer fix); allsource-core unreleased (source fix)
- **Supersedes / relates to:** [ADR 004 (Projection Backfill on Registration)](004-projection-backfill.md), issue #160 (cold-tier lazy Parquet hydration), issue #101 (Parquet restore silent failure)

## Context

### The incident

chronis 0.7.1 (which bumped the embedded `allsource-core` pin 0.19 → 0.21) made `cn list` / `cn ready` / `cn show` return **"No tasks found"** on workspaces that had data. To the user it looked like the release had **nuked their store** — Longhand (~740 task-events), the chronos-monorepo store (353), and longhand-secondary (23) all read empty after the upgrade.

**No event was ever lost.** Every event remained durable on disk in Parquet (Longhand: 2142 flat files / 2436 events, intact). The failure was entirely in the **boot read path**.

### Root cause

`EmbeddedCore::open` builds the `EventStore` (which recovers the WAL) and registers projections, but never hydrates the Parquet archive into the in-memory event pile. That was survivable in 0.19 because the old boot path eagerly loaded Parquet. Issue #160 (cold-tier / cross-region) changed the **multi-tenant server** to keep Parquet **cold** and hydrate each tenant lazily *on first query* — the server can't fit every tenant in memory.

Embedded single-store consumers are the opposite case: their **projections are the read surface** and never trigger the server's lazy-query hydration path. So after #160, an embedded boot populated its pile only from the WAL. When the WAL was short, rotated, relocated, or — as in this incident — created fresh and empty at a new path because 0.21 also changed the on-disk layout (flat `storage/events-*.parquet` → partitioned `storage/<tenant>/<yyyy-mm>/`, and a new WAL path), the projection booted with **zero events** despite a full Parquet archive sitting next to it.

`EventStore::hydrate_all_from_storage()` already existed for exactly this case, and its doc comment already named it: *"Embedded single-store consumers like Prime ... their projections are the queryable surface ... must be backfilled from the complete history."* Prime called it. The chronis embedded boot did not. Nothing enforced that an embedded consumer must hydrate.

## Decision

**Embedded boot must eagerly reconstruct the in-memory pile from the full durable Parquet archive before registering projections.** Concretely:

1. `EmbeddedCore::open` calls `EventStore::hydrate_all_from_storage()` immediately after constructing the store and before any projection registration. It **fails loud** on a storage read error rather than silently presenting an empty store.
2. `hydrate_all_from_storage` reads **both** on-disk layouts — legacy flat `storage/*.parquet` and the partitioned `storage/<tenant>/<yyyy-mm>/` tree — via a recursive scan, so an upgrade needs **no data migration** for reads to work.
3. Dedup in `append_loaded_event` keeps hydration safe alongside WAL recovery (events present in both are not double-counted), so ordering with WAL recovery is not load-bearing.

chronis 0.7.2 ships the same call at the consumer (`workspace.rs`) so users were unblocked without waiting for a core release; the core change makes it structural so no future embedded consumer (Prime, others) can reintroduce the bug.

## Consequences

**Positive**
- A store with durable Parquet but an empty/short/relocated WAL reads its full history. The durability promise holds end-to-end, not just at the storage layer.
- Layout-agnostic reads: flat and partitioned Parquet both load, so a version bump that changes layout no longer hides old data.
- Fail-loud on read error surfaces corruption instead of masquerading as an empty store (counters the #101 silent-failure class).

**Negative / trade-offs**
- Embedded boot does O(all events) work up front instead of lazily. Acceptable: embedded consumers are single-store and want hot data; their whole value is the projection being immediately queryable. (The server keeps its lazy path — this ADR is embedded-only.)
- Larger embedded stores pay a longer cold-start. If this becomes a problem, the mitigation is a projection snapshot/checkpoint, not skipping hydration.

## Alternatives considered

1. **Migrate-on-boot (flat → partitioned), then rely on the existing read path.** Rejected as the primary fix: it mutates the user's store on upgrade and still wouldn't help if the reader doesn't replay Parquet. The recursive back-compat reader makes migration optional (a separate `allsource-migrate-storage` tool remains for operators who want to consolidate layout).
2. **Make the server's lazy hydration also fire for embedded.** Rejected: embedded has no query-driven trigger by design; bolting one on is more surface than hydrating once at boot.
3. **Leave it to each consumer (status quo).** Rejected: this incident is exactly what "leave it to each consumer" produces.

## Verification

- Real affected store (Longhand): fixed `cn` reads 740 tasks from both the un-migrated flat store and a partition-migrated copy; broken 0.7.1 read 0.
- Regression test `apps/core/tests/embedded_cold_parquet_boot.rs`: ingest 5 → checkpoint to Parquet → zero the WAL segments → reopen → assert all 5 read and queryable. Was 0 before the fix.

## Open follow-up

This ADR fixes the **read** path. It does **not** fix the underlying layout instability that orphaned data on upgrade (Parquet path change + WAL path change between 0.19 and 0.21). Un-checkpointed events that lived only in an orphaned old-path WAL are not recoverable when no surviving WAL segment holds them. Tracked p0 in bead `t-3bd50e`: version-stable storage/WAL path derivation, boot-time replay of legacy-path WAL segments, and a mandatory back-compat reader or migration for **every** durable artifact (not just Parquet) whenever an on-disk layout changes.

## Postmortem note

0.7.1 was yanked from crates.io. No customer data was lost. The durability layer (WAL + Parquet) did its job; the regression was a read-path omission introduced when a server-oriented performance change (#160) was not mirrored by an embedded-oriented hydration guard. The lesson encoded here: **for an embedded event store, "the projection is empty" and "there is no data" are different statements, and the boot path must never conflate them.**
