---
title: "allsource-mcp — Local MCP Server for AllSource Debugging"
status: CURRENT
last_updated: 2026-03-13
category: guide
---

# allsource-mcp — Local MCP Server for AllSource Debugging

`allsource-mcp` is a lightweight MCP (Model Context Protocol) server that reads directly from AllSource Core's WAL and Parquet files on disk. No running Core server needed — just point it at a data directory and start querying events through Claude Code, Claude Desktop, or any MCP client.

It embeds `allsource-core` as a Rust library via the [`embedded` feature flag](../adr/001-embedded-core-library.md), giving you the same durability guarantees (WAL + Parquet) and query performance as the full server, without the HTTP overhead.

## Install

```bash
cargo install allsource-mcp
```

Or build from source:

```bash
git clone https://github.com/all-source-os/all-source.git
cd all-source/tooling/allsource-mcp
cargo install --path .
```

Requires **Rust 1.92+**.

### Versioning — this binary lags the platform

`allsource-mcp` versions independently of the AllSource platform, and is currently behind it:
as of 2026-07-25 crates.io has **0.14.8** (published 2026-03-13), which embeds
`allsource-core` **0.20.1**, while the platform is on **0.22.x**.

```bash
allsource-mcp --version                  # what you have
cargo search allsource-mcp               # what is published
cargo install allsource-mcp --force      # upgrade in place
```

If you need a Core feature newer than 0.20.1, build from source against the monorepo
(`cargo install --path .` above) or use the Docker connector
(`ghcr.io/all-source-os/allsource-mcp-server`), which tracks platform releases. See
[/docs/mcp](https://www.all-source.xyz/docs/mcp) for how the four MCP servers differ.

## How It Works

```
Claude Code / Claude Desktop
        |
    stdio JSON-RPC
        |
  allsource-mcp (Rust binary)
        |
  EmbeddedCore (in-process)
        |
  ┌─────┴─────┐
  WAL       Parquet
  (append)  (columnar)
```

`allsource-mcp` opens the data directory with `EmbeddedCore::open()` — the same facade that powers [Chronis](../../apps/chronis/), [Longhand](https://github.com/technical-leaders/longhand), and any Rust application embedding AllSource. It exposes 8 read-only MCP tools over stdio, so LLMs can query your event store in natural language without a running server process.

Because it reads directly from the durable storage layer (WAL + Parquet), it works on:
- **Live data directories** while Core is running (read-only, no conflicts)
- **Backup copies** of data directories (cold analysis)
- **Local dev data** from embedded apps like Chronis

## Configuration

### Claude Code

Add to `~/.claude/settings.json` or your project's `.claude/settings.json`:

```json
{
  "mcpServers": {
    "allsource": {
      "command": "allsource-mcp",
      "args": ["--data-dir", "/path/to/allsource/data"]
    }
  }
}
```

### Claude Desktop

Add to `~/Library/Application Support/Claude/claude_desktop_config.json` (macOS):

```json
{
  "mcpServers": {
    "allsource": {
      "command": "allsource-mcp",
      "args": ["--data-dir", "/path/to/allsource/data"]
    }
  }
}
```

### Environment variable alternative

Instead of `--data-dir`, you can set `ALLSOURCE_DATA_DIR`:

```json
{
  "mcpServers": {
    "allsource": {
      "command": "allsource-mcp",
      "env": {
        "ALLSOURCE_DATA_DIR": "/path/to/allsource/data"
      }
    }
  }
}
```

### Common data directory locations

| Application | Data directory |
|---|---|
| Longhand (macOS) | `~/Library/Application Support/Longhand/allsource` |
| Chronis | `.chronis/` in your project root |
| Core (Docker default) | `/data` inside the container (mount to host) |
| Core (local dev) | `./data/` in the Core working directory |

## Available Tools

`allsource-mcp` exposes 8 read-only tools. LLMs call these automatically based on your natural language questions.

### `query_events`

Query events with filters: `entity_id`, `event_type` (prefix match), time range (`since`/`until`), and `limit`.

**Ask:** "Show me all events for order-123"
**Ask:** "Find workflow_run events from the last hour"

### `sample_events`

Return a sample of recent events across all entities. Useful for discovering what data exists when you don't know the entity IDs yet.

**Ask:** "What kind of data is in here?"
**Ask:** "Show me some recent events"

### `quick_stats`

Summary of the event store: total events, unique entity count, event type distribution, date range, and durability status (WAL/Parquet health).

**Ask:** "Give me an overview of this event store"
**Ask:** "How many events are stored?"

### `get_snapshot`

Get the latest projection state for an entity. Falls back to `reconstruct_state` if no projection exists.

**Ask:** "What's the current state of user-456?"

### `event_timeline`

Chronological timeline with timestamps, event types, and payload summaries. Perfect for understanding entity lifecycles.

**Ask:** "Show me the timeline for workflow:abc-123"

### `explain_entity`

Human-readable lifecycle summary: creation date, last activity, event type distribution, and lifecycle phases.

**Ask:** "Explain everything about order-789"

### `reconstruct_state`

Fold all events to rebuild current state (last-write-wins per payload field). The event-sourced equivalent of a SQL `SELECT *`.

**Ask:** "Reconstruct the full state of user-123"

### `analyze_changes`

Analyze what changed for an entity within a time window. Shows each event's changed fields and payloads.

**Ask:** "What changed for user-123 between Monday and Friday?"

## Example Session

```
You: What's in this event store?

Claude: [calls quick_stats]
  Found 1,247 events across 89 entities.
  Top types: workflow_run.started (312), workflow_run.completed (298),
  task.created (187), task.claimed (142)...

You: Show me the lifecycle of workflow:abc-123

Claude: [calls explain_entity]
  Entity workflow:abc-123 has 5 events spanning 20 seconds.
  Lifecycle: started -> 3 steps completed -> completed

You: What changed in the last step?

Claude: [calls analyze_changes with since/until from the timeline]
  1 change: step_completed at 10:00:18Z
  Changed fields: step_name ("validate"), status ("passed"), duration_ms (312)
```

## allsource-mcp vs. Elixir MCP Server

There are two MCP servers in the AllSource ecosystem. They serve different purposes:

| | `allsource-mcp` (this guide) | Elixir MCP Server |
|---|---|---|
| **Location** | `tooling/allsource-mcp` | `apps/mcp-server-elixir` |
| **Language** | Rust | Elixir |
| **Transport** | stdio (local process) | stdio (local process) |
| **Backend** | Embedded Core (reads files directly) | HTTP to running Core server |
| **Tools** | 8 (read-only debugging) | 61 (full CRUD, analytics, schema, tenants) |
| **Use case** | Local debugging, offline analysis, dev workflows | Production AI interface, Claude Desktop with full stack |
| **Dependencies** | None (single binary) | Running Core + Control Plane servers |
| **Install** | `cargo install allsource-mcp` | `mix deps.get` in monorepo |

**When to use `allsource-mcp`:** You want to point an LLM at AllSource data files and ask questions — no server infrastructure needed. Ideal for debugging, local dev, and CI pipelines.

**When to use the Elixir MCP Server:** You're running the full AllSource stack and want the complete 61-tool suite including writes, schema management, analytics, and multi-tenant operations.

## Debugging

Enable trace logging to stderr (stdout is reserved for MCP JSON-RPC):

```bash
RUST_LOG=debug allsource-mcp --data-dir /path/to/data
```

Common issues:

| Symptom | Cause | Fix |
|---|---|---|
| "server not starting" in Claude | Binary not on `$PATH` | Run `which allsource-mcp` — if missing, check `~/.cargo/bin` is in your `PATH` |
| "No events found" | Wrong data directory | Verify the directory contains `wal/` and/or `storage/` subdirectories |
| Stale data | Binary built from old source | `cargo install allsource-mcp` to update |

## Relationship to Embedded Core

`allsource-mcp` is one of several applications built on the [Embedded Core library API](../proposals/EMBEDDED_CORE_AND_OFFLINE_FIRST.md):

- **[Chronis CLI](../../apps/chronis/)** — event-sourced task management for AI agents
- **[Longhand](https://github.com/technical-leaders/longhand)** — offline-first Tauri desktop app
- **allsource-mcp** (this) — MCP server for LLM debugging
- **Custom applications** — any Rust app via `allsource-core` with `features = ["embedded"]`

The Embedded Core API is documented in [ADR-001](../adr/001-embedded-core-library.md). All embedded applications share the same durability guarantees: WAL with CRC32 checksums, Parquet columnar persistence, and crash recovery.

## Further Reading

- [Embedded Core proposal (8 phases)](../proposals/EMBEDDED_CORE_AND_OFFLINE_FIRST.md) — full design and implementation history
- [ADR-001: Embedded Core Library API](../adr/001-embedded-core-library.md) — architecture decision record
- [Elixir MCP Server setup](mcp-server/CLAUDE_DESKTOP_SETUP.md) — for the full 61-tool production MCP server
- [allsource-mcp README](../../tooling/allsource-mcp/README.md) — quick reference
