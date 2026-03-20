# Chronis — Event-Sourced Task CLI for Autonomous Agents

> **Status**: Proposal
> **Author**: Design session 2026-03-02
> **Scope**: New CLI binary (`cn`) wrapping Embedded Core for task orchestration, replacing beads-rust in ralph-tui workflows
> **Depends on**: Embedded Core Library (Phase 1–8, complete)

---

## Name

**Chronis** — the smallest indivisible quantum of time. In physics, a chronis is the hypothetical atom of duration — time cannot be subdivided further. In the AllSource universe, every event is a chronis: an atomic, immutable moment in the timeline from which all state is derived.

The name echoes **Chronos** (the monorepo) and **Vector Prime** (the Transformer who guarded the timestream), placing AllSource firmly in the lineage of time-keepers.

Binary name: `cn` (two letters, fast to type, mnemonic for ChroNoN).

---

## 1. Problem

### What beads-rust does well

beads-rust (`bd`) is a lightweight issue tracker that drives ralph-tui agent orchestration. Over 11 sessions it shipped 9 releases (v0.9.0 → v0.10.7) completing 62+ tasks across ~280 iterations. The workflow is proven:

```
bd ready → agent claims task → does work → bd close → bd sync (git push)
```

### Where it falls short

| Gap | Impact |
|-----|--------|
| **Flat data model** | Tasks have status + priority, but no event history. "How long was this blocked?" requires grepping git log. |
| **No agent observability** | Token usage, tool call success rates, agent utilization — all invisible. Operators discover runaway costs after the fact. |
| **Git-coupled sync** | `bd sync` = `git commit && git push`. Merge conflicts on `.beads/issues.jsonl` stall agents. Offline agents can't sync until they have network. |
| **No queryable history** | "Show me all tasks that took > 30 minutes" is impossible. There's no temporal dimension to the data. |
| **No replay/audit** | Can't reconstruct past state. If an agent closes a task incorrectly, there's no undo — only manual re-open. |
| **No workflow orchestration** | No concept of approval gates, step tracking, or multi-agent coordination beyond simple dependency edges. |

### The opportunity

AllSource Embedded Core already solves the hard infrastructure problems (WAL durability, CRDT sync, AI projections, TOON output). What's missing is a **CLI that exposes this power with beads-level simplicity**.

---

## 2. Solution

Chronis is a single Rust binary (~30MB) that wraps Embedded Core. It stores events in `.chronis/` at the project root — no Docker, no server, no configuration required.

### Design principles

1. **Zero-config start** — `cn init` and go. Single-tenant, WAL-enabled, sensible defaults.
2. **Event-sourced, not CRUD** — every mutation emits an event. State is always derivable.
3. **beads-compatible workflow** — `cn ready`, `cn claim`, `cn done` map 1:1 to the ralph-tui loop.
4. **Agent-first, human-readable** — default output is tables for humans, `--toon` flag for LLM-optimized output.
5. **Sync without git** — CRDT-based bidirectional sync over HTTP/WebSocket. Git integration optional.

---

## 3. Command Reference

### Core workflow (beads parity)

```bash
cn init                                  # Create .chronis/ with embedded Core
cn task create "Fix auth bug" -p p1      # Emit task.created event
cn task create "Add tests" --blocked-by=<id>  # With dependency
cn ready                                 # List unblocked, unclaimed tasks
cn claim <id>                            # Emit workflow.claimed (first-write-wins)
cn done <id>                             # Emit workflow.step.completed
cn done <id> --reason="Shipped in v0.12" # With completion note
cn list                                  # All tasks, grouped by status
cn list --status=open                    # Filter
cn show <id>                             # Full task detail + event timeline
```

### Beyond beads

```bash
# History & replay
cn timeline <id>                         # Full event history for a task
cn replay --to="2026-02-15T10:00:00Z"    # Rebuild state at any point in time
cn diff <id> --from=3h                   # What changed in the last 3 hours

# Agent observability
cn stats                                 # Token usage, agent utilization, cost
cn stats --model=claude-sonnet-4-20250514        # Filter by model
cn audit tools                           # Tool call success rates, p95 latency
cn audit tools --tool=read_file          # Single tool detail
cn approvals                             # Pending human-in-the-loop items
cn approve <id>                          # Emit workflow.approval.granted

# Multi-agent coordination
cn agent register --capabilities=summarize,translate
cn agent heartbeat                       # Liveness ping (called by agent loop)
cn agent list                            # Active agents, utilization

# Sync
cn sync --peer=ssh://host/project        # CRDT bidirectional sync
cn sync --peer=https://cloud.allsource.io/ws  # Sync to AllSource Cloud
cn sync --git                            # Legacy: commit + push (beads compat)

# Export
cn export --format=toon                  # LLM-optimized output
cn export --format=json                  # Standard JSON
cn export --format=csv                   # For spreadsheets / BI tools

# Migration
cn migrate-beads                         # Import .beads/ → .chronis/ events
```

### Short aliases

| Full command | Alias | Rationale |
|-------------|-------|-----------|
| `cn ready` | `cn r` | Most-used command in agent loops |
| `cn claim <id>` | `cn c <id>` | Second most-used |
| `cn done <id>` | `cn d <id>` | Third most-used |
| `cn list` | `cn ls` | Familiar |
| `cn show <id>` | `cn s <id>` | Quick inspect |
| `cn stats` | `cn st` | Dashboard glance |

---

## 4. Architecture

```
┌─────────────────────────────────────────────────────┐
│                  cn CLI (clap)                       │
│                                                     │
│  Commands ──► Event emission ──► Query/Projection   │
│                                                     │
│  Output formatters:                                 │
│    • Table (human, default)                         │
│    • TOON  (LLM-optimized, --toon)                  │
│    • JSON  (machine, --json)                        │
│    • CSV   (export only)                            │
└───────────────────────┬─────────────────────────────┘
                        │
              EmbeddedCore::open()
                        │
┌───────────────────────┴─────────────────────────────┐
│              allsource_core (embedded)               │
│                                                     │
│  EventStore ─── DashMap (11.9μs reads)              │
│       │                                              │
│       ├── WAL (CRC32, fsync, crash recovery)        │
│       └── Parquet (Snappy compression)              │
│                                                     │
│  Projections:                                       │
│    • TaskQueueProjection     (ready/claimed/done)   │
│    • WorkflowStatusProjection (step tracking)       │
│    • TokenUsageProjection     (cost tracking)       │
│    • ToolCallAuditProjection  (tool success rates)  │
│    • AgentUtilizationProjection (fleet health)      │
│    • HumanInLoopQueueProjection (approvals)         │
│                                                     │
│  Sync:                                              │
│    • HLC timestamps (causal ordering)               │
│    • CRDT resolver (dedup, conflict resolution)     │
│    • HTTP/WS transport (remote peers)               │
└─────────────────────────────────────────────────────┘

Storage layout:
  .chronis/
    ├── wal/           # Write-ahead log segments
    ├── parquet/        # Columnar event storage
    ├── config.toml     # Optional overrides
    └── .lock           # Process lock file
```

---

## 5. Event Schema

All mutations emit events. The task model is built entirely from projections over these events.

### Task lifecycle events

```json
{"event_type": "task.created",    "entity_id": "t-a1b2", "payload": {"title": "Fix auth bug", "priority": "p1", "type": "task"}}
{"event_type": "task.updated",    "entity_id": "t-a1b2", "payload": {"field": "priority", "from": "p1", "to": "p0"}}
{"event_type": "task.dependency.added", "entity_id": "t-a1b2", "payload": {"depends_on": "t-c3d4"}}
{"event_type": "task.dependency.removed", "entity_id": "t-a1b2", "payload": {"depends_on": "t-c3d4"}}
```

### Workflow events (reuse existing Replicant protocol)

```json
{"event_type": "workflow.claimed",        "entity_id": "t-a1b2", "payload": {"agent_id": "r-1"}}
{"event_type": "workflow.step.started",   "entity_id": "t-a1b2", "payload": {"step": "implementation"}}
{"event_type": "workflow.step.completed", "entity_id": "t-a1b2", "payload": {"step": "implementation", "reason": "Shipped"}}
{"event_type": "workflow.approval.requested", "entity_id": "t-a1b2", "payload": {"reason": "Review output"}}
{"event_type": "workflow.approval.granted",   "entity_id": "t-a1b2", "payload": {"approved_by": "human"}}
```

### Agent events (reuse existing Replicant protocol)

```json
{"event_type": "replicant.registered", "entity_id": "r-1", "payload": {"capabilities": ["summarize"]}}
{"event_type": "replicant.heartbeat",  "entity_id": "r-1", "payload": {}}
```

### AI observability events (reuse existing AI projections)

```json
{"event_type": "llm.call.completed", "entity_id": "t-a1b2", "payload": {"model": "claude-sonnet-4-20250514", "input_tokens": 1500, "output_tokens": 500, "cost_usd": 0.0045}}
{"event_type": "mcp.tool.result",    "entity_id": "t-a1b2", "payload": {"tool": "read_file", "duration_ms": 42, "success": true}}
```

**Key insight**: No new event types needed. Chronis reuses the schemas already defined in Embedded Core phases 5–7.

---

## 6. TaskProjection — The Core Derivation

The `cn list` / `cn ready` / `cn show` views are all derived from a single `TaskProjection` that folds task + workflow events:

```rust
struct TaskState {
    id: String,
    title: String,
    priority: Priority,
    status: TaskStatus,          // Open, InProgress, Done
    claimed_by: Option<String>,  // agent ID
    blocked_by: Vec<String>,     // task IDs with status != Done
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    events_count: usize,         // for timeline depth
}

enum TaskStatus {
    Open,        // task.created, no workflow.claimed
    InProgress,  // workflow.claimed received
    Done,        // workflow.step.completed received
}
```

`cn ready` = tasks where `status == Open && blocked_by.is_empty()`.

This is a standard AllSource projection — computed incrementally as events arrive, stored in DashMap, queryable in 11.9μs.

---

## 7. Migration from beads-rust

```bash
cn migrate-beads [--beads-dir=.beads]
```

Reads `.beads/issues.jsonl` and emits equivalent events:

| beads field | Chronis event |
|-------------|---------------|
| `id: "bd-ksh"` | `entity_id: "t-ksh"` (prefix swap) |
| `status: "open"` | `task.created` event |
| `status: "in_progress"` | `task.created` + `workflow.claimed` events |
| `status: "closed"` | `task.created` + `workflow.step.completed` events |
| `priority: 2` | `payload.priority: "p2"` |
| `dependencies.blocked_by` | `task.dependency.added` events |
| `created_at` | Event timestamp preserved |

After migration, the `.beads/` directory is left untouched (no destructive action). Both systems can coexist during transition.

---

## 8. ralph-tui Integration

ralph-tui currently calls `bd` commands via shell. The switch requires:

### Phase 1: Drop-in replacement

ralph-tui agent loop changes from:

```bash
# Before (beads)
bd ready                    → pick task
bd update <id> --status=in_progress → claim
# ... do work ...
bd close <id>               → complete
bd sync                     → git push
```

To:

```bash
# After (Chronis)
cn ready                    → pick task (same output format)
cn claim <id>               → claim (event-sourced)
# ... do work ...
cn done <id>                → complete (event-sourced)
cn sync --git               → git push (compat mode)
```

### Phase 2: Agent instrumentation

Once the basic loop works, agents emit observability events:

```bash
# Agent startup
cn agent register --capabilities=code,test,review

# After each LLM call (in agent wrapper)
cn emit llm.call.completed --entity=<task-id> \
  --payload='{"model":"claude-sonnet-4-20250514","input_tokens":1500,"output_tokens":500}'

# After each tool call
cn emit mcp.tool.result --entity=<task-id> \
  --payload='{"tool":"read_file","duration_ms":42,"success":true}'

# Heartbeat (in agent loop)
cn agent heartbeat
```

### Phase 3: Operator dashboard

```bash
# Operator checks fleet health
cn stats
┌──────────────────────────────────────────────────┐
│ Chronis — Session Stats                          │
├────────────────────┬─────────────────────────────┤
│ Tasks completed    │ 12 / 25                     │
│ Active agents      │ 3 / 5                       │
│ Total tokens       │ 1.2M (input) + 340K (out)   │
│ Estimated cost     │ $4.82                        │
│ Top tool (by calls)│ read_file (342 calls, 98.5%) │
│ Blocked tasks      │ 2 (awaiting t-f8g9)         │
│ Pending approvals  │ 1 (t-a1b2: "review output") │
└────────────────────┴─────────────────────────────┘
```

---

## 9. Implementation Phases

### Phase 1: CLI Skeleton + Task CRUD (P0)

**Scope**: `cn init`, `cn task create`, `cn list`, `cn show`, `cn ready`
**Effort**: Small — clap CLI wrapping EmbeddedCore::open + ingest + query
**Output**: Human-readable tables via `comfy-table` or `tabled`
**Tests**: Integration tests — init, create, list, filter, show

### Phase 2: Workflow Commands (P0)

**Scope**: `cn claim`, `cn done`, `cn approve`, TaskProjection
**Effort**: Small — emit workflow events, fold in projection
**Tests**: Claim idempotency (first-write-wins), dependency blocking, approval flow

### Phase 3: History & Observability (P1)

**Scope**: `cn timeline`, `cn stats`, `cn audit`, `cn replay`
**Effort**: Medium — wire existing AI projections to CLI output formatters
**Tests**: Token aggregation accuracy, timeline ordering, replay correctness

### Phase 4: Agent Registration & Coordination (P1)

**Scope**: `cn agent register`, `cn agent heartbeat`, `cn agent list`, `cn emit`
**Effort**: Small — emit replicant events, wire ReplicantRegistryProjection
**Tests**: Heartbeat staleness detection, capability filtering

### Phase 5: Sync & Export (P2)

**Scope**: `cn sync --peer`, `cn sync --git`, `cn export`
**Effort**: Medium — wire sync_transport.rs to CLI, add git wrapper
**Tests**: Bidirectional sync convergence, conflict resolution, TOON output validity

### Phase 6: Migration & Compatibility (P2)

**Scope**: `cn migrate-beads`, beads output format compat
**Effort**: Small — parse issues.jsonl, emit equivalent events
**Tests**: Round-trip fidelity, timestamp preservation

---

## 10. Binary & Distribution

```toml
# apps/chronis/Cargo.toml (new crate in apps/)
[package]
name = "chronis"
version = "0.1.0"

[[bin]]
name = "cn"
path = "src/main.rs"

[dependencies]
allsource-core = { path = "../core", default-features = false, features = ["embedded", "embedded-replicant", "embedded-projections"] }
clap = { version = "4", features = ["derive"] }
tabled = "0.15"
serde_json = "1"
tokio = { version = "1", features = ["rt-multi-thread", "macros"] }
chrono = "0.4"
```

**Build**: `cargo build --release -p chronis` → `~30MB` binary (no server deps).

**Distribution**:
- GitHub Releases (prebuilt binaries for linux-amd64, linux-arm64, darwin-arm64, darwin-amd64)
- `cargo install chronis` (from crates.io, after Rust SDK publishing rules)
- Homebrew tap (future)

---

## 11. Decision Record

| Decision | Chosen | Alternatives considered | Rationale |
|----------|--------|------------------------|-----------|
| Name | Chronis (`cn`) | AllSpark, Forge, Arc, Nexus, Axiom, Vectis | Chronis = smallest quantum of time; echoes Chronos monorepo + Vector Prime lineage; `cn` is short and unambiguous |
| Storage dir | `.chronis/` | `.allsource/`, `.cn/` | Distinct from `.allsource/` (cloud config); `.cn/` too terse; `.chronis/` self-documenting |
| Output default | Human tables | JSON | Agents can use `--toon`; humans need readability by default |
| Task IDs | `t-` + 4 char hash | UUID, sequential int | Short enough to type; `t-` prefix avoids collisions with agent IDs (`r-`) |
| Sync default | CRDT over HTTP | Git | Git sync available as `--git` flag; CRDT is the primary path |
| Event schema | Reuse Embedded Core schemas | New schema | Zero new event types needed; all projections already exist |
| Crate location | `apps/chronis/` | `sdks/`, `tooling/` | It's a deployable binary, belongs in `apps/` per monorepo rules |
| Migration | Non-destructive | Replace .beads/ in-place | Users can run both systems during transition |

---

## 12. What This Unlocks

Beyond replacing beads-rust, Chronis becomes the **local event store for any CLI workflow**:

- **CI pipelines**: `cn emit build.completed` → track build times, failure rates across runs
- **Dev journals**: `cn emit note.created --payload='{"text":"found the bug"}'` → searchable dev log
- **MCP tool auditing**: MCP servers emit tool events → Chronis aggregates success rates locally
- **Multi-repo orchestration**: `cn sync` between repos → unified view of work across projects

Every event is a chronis. Every chronis is immutable. The timeline remembers everything.
