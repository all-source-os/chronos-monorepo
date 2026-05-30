---
status: draft
audience: build-in-public / engineering
topic: chronis 0.7.1 cold-Parquet-boot incident + fix
---

# X Thread — The release that *looked* like it nuked a user's data

> Draft. Honest postmortem, build-in-public tone. Review before posting. Pair tweet 1 with a screenshot of `cn list → No tasks found`, and tweet 7 with the `740 tasks` recovery.

## Tweet 1/11 — Hook

Yesterday I shipped a release of our event-sourced task CLI that made a user's store read **completely empty**. 327+ tasks, gone. "No tasks found."

Except nothing was lost. Every event was safe on disk the whole time.

Here's the bug, the fix, and why it could happen at all 🧵

## Tweet 2/11

The tool (chronis / `cn`) is built on our embedded event store: Write-Ahead Log for durability + Parquet for the cold archive + an in-memory projection you actually query.

Append-only. Event-sourced. "Your data survives restarts" is the entire pitch.

## Tweet 3/11

User upgrades to the new release. Runs `cn list`.

`No tasks found.`

`cn ready` — nothing. `cn show <id>` — "task not found" for IDs they were using an hour ago.

From the outside: the release nuked the store. Understandably, they were furious.

## Tweet 4/11

First rule of a data-loss report: **don't trust the read path, check the bytes.**

`.chronis/storage/` → 2142 Parquet files, 8.7MB, timestamps spanning weeks. `strings` on the newest one showed the actual task titles.

The data was *right there*. Durable. Untouched.

## Tweet 5/11

So this was never data loss. It was a **read** bug.

The in-memory projection — the thing `cn list` reads — booted empty, because the boot path filled it only from the WAL. And the WAL had been reset to 0 bytes during the upgrade.

Parquet, the source of truth, was never loaded.

## Tweet 6/11

Why would boot ignore the archive?

Months ago we made the *multi-tenant server* keep Parquet **cold** and hydrate each tenant lazily, on first query. A server can't hold every tenant in memory. Correct call — for the server.

The embedded store is the opposite case. We missed it.

## Tweet 7/11

In embedded mode the projection *is* the query surface. There's no lazy-query trigger to wake the cold archive. So after that change, embedded boot = "replay the WAL and stop."

Short WAL, rotated WAL, or (this release) a freshly-created empty WAL → projection boots empty. Full archive ignored.

## Tweet 8/11

The painful part: the function that fixes it already existed.

`hydrate_all_from_storage()` — with a doc comment literally describing this case: *"embedded consumers' projections are the queryable surface and must be backfilled from the complete history."*

One consumer called it. The new one didn't.

## Tweet 9/11

The fix: hydrate Parquet on boot, *before* registering projections, reading **both** the old flat layout and the new partitioned one — so an upgrade needs zero migration. And fail **loud** on a read error instead of quietly showing an empty store.

Result: 740 tasks back. 0 → whole.

## Tweet 10/11

Hardening so it can't recur:
- Regression test: write events → checkpoint to Parquet → **zero the WAL** → reopen → assert everything reads. (Was 0 before.)
- Moved the guard into the core boot path so no future embedded consumer can forget it.
- Yanked the bad release.

## Tweet 11/11

The lesson I'm keeping:

For an embedded event store, **"the projection is empty" and "there is no data" are different sentences.** The boot path must never conflate them.

Durability held. The read path lied. Full postmortem in our ADRs.

Build in public, bugs included.

---

## Notes for the poster
- Optional shorter cut: tweets 1, 3, 4, 5, 8, 9, 11.
- Do NOT name the client repo if posting publicly — "a user's store" is enough.
- Link target for tweet 11: ADR 018 (`docs/adr/018-embedded-eager-parquet-hydration.md`) once docs are published.
- Tone check: owns the mistake, shows the durability layer worked, no blame, no spin.
