#!/bin/sh
# Optional one-shot migration step. When ALLSOURCE_MIGRATE_FLAT_TO_TENANT=true,
# rewrite legacy flat-layout Parquet files into the tenant-partitioned tree
# (Step 1 of the sustainable data strategy) before Core starts.
#
# The migrator is idempotent: on a tree that's already partitioned, it
# scans the storage root, finds no flat files, and exits 0 immediately.
# That means the env var can stay set permanently — every boot pays a
# directory listing's worth of cost for a one-time correctness benefit.
#
# `set -e` is load-bearing: if the migrator fails for any reason, we MUST
# NOT continue to exec Core, because Core might then partially-load the
# storage tree on top of half-migrated data. Fly will restart the
# container; the operator should check logs and decide whether to roll
# the env var back to false or fix the underlying issue.
set -e

if [ "$ALLSOURCE_MIGRATE_FLAT_TO_TENANT" = "true" ]; then
    echo "[entrypoint] running flat→tenant Parquet migration"
    /app/allsource-migrate-storage --storage-dir "${ALLSOURCE_DATA_DIR:-/app/data}/storage"
    echo "[entrypoint] migration complete"
fi

exec /app/allsource-core
