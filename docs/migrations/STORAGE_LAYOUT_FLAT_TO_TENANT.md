# Migration: flat → tenant-partitioned Parquet layout

**Status:** ready, 2026-04-26
**Tooling:** `apps/core/src/bin/allsource-migrate-storage.rs`
**Scope:** AllSource Core only (`fly app: allsource-core`)
**Audience:** the operator running the migration

## Why this exists

Step 2 of the [sustainable data strategy](../proposals/SUSTAINABLE_DATA_STRATEGY.md) changed the on-disk layout from a flat pile

```
<storage-dir>/events-<ts>-<uuid>.parquet
```

to a tenant-partitioned tree

```
<storage-dir>/<tenant>/<yyyy-mm>/events-<ts>-<uuid>.parquet
```

Existing data on the production volume (~451k events as of 2026-04-25) was written under the old shape. The recursive walker in commit #1 keeps the old files loadable, so the system runs fine without migration — but every Core boot re-reads them in the original flat shape, and Steps 2/3 (lazy per-tenant load, memory-budgeted cache) cannot prune by tenant on data that lives in a tenantless directory. Running this migration is the one-time bridge.

## Prerequisites

1. Core build that includes `allsource-migrate-storage` (commit `<hash>` or later).
2. Operator access to the Core machine's filesystem (Fly SSH, or running locally against a snapshot).
3. **A confirmed durable backup of the storage volume.** Migration deletes flat files after writing the new ones; if it crashes mid-run, there is no rollback button. Take the snapshot first.
4. Core stopped. The tool does not lock; concurrent writes during migration would cause duplicate events on next load.

## What the tool does

- Walks the storage directory's top level only — only legacy flat-layout files appear there. Already-partitioned subtrees (`<storage-dir>/alice/...`) are ignored.
- For each flat file, loads the events, regroups them by `(tenant_id, yyyy-mm-of-event-timestamp)`. Pre-#2 events all carry `tenant_id = "default"` (the schema didn't track tenant), so they all migrate under `default/<yyyy-mm>/`.
- For each group, writes a fresh Parquet file under the corresponding partition directory. Schema, compression, and file naming match the live writer.
- Once the new file is closed, deletes the flat file.

The tool is **idempotent on a clean run**: re-running after completion sees no flat files at the root and reports a no-op. After a crashed run there may be both a flat file and a partial tree file containing some of its events — re-run before bringing Core back up so the leftover flat file is re-migrated and removed.

## Procedure

### Step 1 — confirm volume backup

Whatever your snapshot mechanism is. The tool writes new files before deleting old ones, so disk space requirement is roughly **2× the size of flat-layout data** during the run.

### Step 2 — stop Core

```bash
fly machine stop -a allsource-core
fly status -a allsource-core   # state should be "stopped"
```

### Step 3 — dry run

Preview the plan with no disk side effects:

```bash
fly ssh console -a allsource-core --command \
  '/app/allsource-migrate-storage --storage-dir /app/data/storage --dry-run'
```

Expected output shape:

```
[dry-run] scanning /app/data/storage ...
dry_run            : true
flat_files_seen    : <N>
flat_files_removed : 0
partitions_written : 0
events_migrated    : <total>
```

If `flat_files_seen = 0`, there is nothing to do; bring Core back up.

### Step 4 — apply

```bash
fly ssh console -a allsource-core --command \
  '/app/allsource-migrate-storage --storage-dir /app/data/storage'
```

Expected output:

```
migrating /app/data/storage (Core MUST be stopped before running)
dry_run            : false
flat_files_seen    : <N>
flat_files_removed : <N>
partitions_written : >= 1
events_migrated    : <total>
```

Sanity check on the FS:

```bash
fly ssh console -a allsource-core --command 'ls /app/data/storage/'
# Expected: only directories (default/, possibly other tenants), no events-*.parquet
fly ssh console -a allsource-core --command 'find /app/data/storage -type f -name "*.parquet" | head'
# Expected: paths under <tenant>/<yyyy-mm>/
```

### Step 5 — restart Core

```bash
fly machine start -a allsource-core
```

WAL recovery + Parquet load should report the same total event count as before migration. If it doesn't, **stop Core** and investigate before serving traffic — duplicates or losses at this stage compound.

### Step 6 — sanity query

Through the gateway, with a known `ask_*` API key:

```bash
curl -s "https://api.all-source.xyz/api/v1/events/query?limit=1" \
  -H "Authorization: Bearer ask_..."
```

Expected: 200 OK, an event from the existing dataset.

## Rollback

There is no in-place rollback. If something goes wrong:

1. Stop Core immediately if it isn't already stopped.
2. Restore the volume snapshot from Step 1. The fly CLI can restore from a known-good snapshot via `fly volumes restore`.
3. Bring Core back up on the restored volume; it will load via the flat layout (recursive walker handles both shapes).
4. Triage what went wrong before re-attempting.

## What this migration is NOT

- It does **not** fix the deeper issue (Core loads everything at boot). Steps 2–6 of the data strategy do that. This is just the prerequisite that makes those steps' tenant-pruning queries work.
- It does **not** infer original tenant identity for pre-#2 events. They stayed `default`-tagged on disk because that's what they actually were when written. If you need richer tenant attribution for historical data, that's a separate replay-from-WAL exercise.
