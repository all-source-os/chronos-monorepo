---
name: chronos-data-flow-embedded
description: Test Chronos MCP server with embedded Core backend (CORE_MODE=embedded). Verifies Rustler NIF prerequisites, event ingest/query through embedded backend, schema operations, stats/health, and data persistence across restarts. Triggers on "test embedded data flow", "test embedded", "embedded health", "test embedded backend", "verify embedded", "test nif".
category: testing
color: cyan
displayName: Chronos Embedded Data Flow Test
---

# Chronos Embedded Data Flow Test

Tests the MCP server running with an embedded Core backend — no separate Core container needed. Core runs in-process via Rustler NIFs (`CORE_MODE=embedded`).

## When invoked:

1. Run the embedded data flow test script:
   ```bash
   bash tooling/embedded-data-flow-test/test-embedded-data-flow.sh
   ```

2. Analyze the output and report:
   - Whether prerequisites are met (NIF compiled, CoreBackend behaviour, CoreEmbedded module)
   - Which tests passed, failed, or were skipped
   - Whether the embedded backend is fully functional or still in development
   - Root cause analysis for any failures

3. If specific sections were requested, use the appropriate flag:
   - `--prereqs` — check prerequisites only (no server start)
   - `--ingest` — event ingestion tests only
   - `--query` — event querying tests only
   - `--schema` — schema operations tests only
   - `--stats` — stats/health tests only
   - `--persist` — persistence test (restart cycle, opt-in)
   - `--json` — append for machine-readable output

4. If the user just wants to check implementation status, run with `--prereqs` only.

## Test Coverage

The script tests these embedded data flow paths:

| Test Area | What's Checked |
|-----------|---------------|
| **Prerequisites** | MCP dir, mix.exs, Elixir runtime, Rustler NIF source/binary, CoreBackend behaviour, CoreEmbedded module, CORE_MODE config |
| **Server Start** | Start MCP server with `CORE_MODE=embedded`, MCP initialize handshake |
| **Event Ingestion** | Ingest single event, ingest batch (3 events) via `ingest_event` tool |
| **Event Querying** | Query by entity_id, query by event_type, query with limit, state reconstruction |
| **Schema Ops** | Register schema, list schemas, get schema by subject |
| **Stats/Health** | get_stats, storage_stats, wal_status, deep health check |
| **Persistence** | Write 5 events, stop server, restart with same data dir, verify events survived |

## Architecture Context

In embedded mode, the data flow is:

```
MCP Tool Call → CoreBackend behaviour → CoreEmbedded (Rustler NIF) → Core Rust Engine
                                                                        ↓
                                                              WAL + Parquet + DashMap
```

vs. remote mode (the Docker stack):

```
MCP Tool Call → CoreBackend behaviour → CoreClient (HTTP) → Core Container (port 3900)
```

Both backends implement the same `CoreBackend` behaviour (27 callbacks). The embedded test verifies the NIF path works identically to the HTTP path.

## Implementation Status

The embedded backend is being built in phases (see `docs/proposals/MCP_EMBEDDED_BACKEND.md`):

| Phase | Component | Status |
|-------|-----------|--------|
| 1 | CoreBackend behaviour | Done |
| 1 | CoreClient implements @behaviour | Done |
| 1 | mcp_tools.ex uses state.backend | Not started |
| 1 | server.ex stores backend in state | Not started |
| 2 | CoreEmbedded NIF module | Not started |
| 2 | native/core_nif/ Rustler crate | Not started |
| 3 | CORE_MODE config switch | Not started |
| 4 | application.ex conditional children | Not started |

Run `--prereqs` to get a live status check of which components exist.

## Common Failure Patterns

- **"NIF not compiled"**: Expected during development. Run `--prereqs` to verify implementation status.
- **"Cannot start MCP server"**: The CoreEmbedded module or CORE_MODE config switch isn't implemented yet.
- **"Events not found after restart"**: WAL/Parquet persistence in the embedded backend isn't wired up — check that `ALLSOURCE_DATA_DIR` is being passed to the Rust engine via NIF.
- **"Timeout on tool call"**: NIF is blocking the BEAM scheduler — ensure `schedule = "DirtyCpu"` is set on all Rustler functions.

## Environment Variables

```bash
# Override defaults
MCP_DIR=/path/to/mcp-server-elixir \
DATA_DIR=/tmp/my-test-data \
bash tooling/embedded-data-flow-test/test-embedded-data-flow.sh
```
