# Chronis

**The agent-native task CLI.** Event-sourced, TOON-optimized, built for AI agents and humans alike.

Inspired by [beads_rust](https://github.com/Dicklesworthstone/beads_rust) — the pioneering local-first issue tracker that proved agents can self-manage tasks via CLI. Chronis takes that idea further: every action is an immutable event, state is derived from projections, and output is optimized for LLM token consumption.

Binary: `cn` | Crate: [`chronis`](https://crates.io/crates/chronis) | Storage: `.chronis/` | Powered by [AllSource](https://all-source.xyz)

## Why Chronis?

We built hundreds of tasks with beads_rust — 62+ issues across 280+ agent iterations. It works. But we kept hitting walls:

| Problem with flat task trackers | Chronis solution |
|---|---|
| No event history — "how long was this blocked?" requires grepping git log | **Event sourcing** — every mutation is an immutable event with timestamps |
| Agent output burns tokens — JSON lists, ASCII tables, verbose prose | **TOON output** — `--toon` flag cuts token consumption ~50% on every command |
| No undo — agent closes wrong task, manual re-open is the only fix | **Temporal replay** — reconstruct state at any point from the event stream |
| Cascade operations require scripting — closing an epic means N separate commands | **`--cascade`** — claim or close an epic and all children in one command |
| Completed tasks clutter listings forever | **Archiving** — `cn archive --all-done` hides completed work, `cn unarchive` restores |
| Git sync = `git commit && push`, merge conflicts stall agents | **Append-only JSONL sync** with UUID dedup — no merge conflicts, no duplicates |
| No approval gates or workflow orchestration | **Event-sourced workflows** — claim, complete, approve with first-write-wins semantics |

Credit where it's due: beads_rust by [@Dicklesworthstone](https://github.com/Dicklesworthstone) got the core workflow right — `ready -> claim -> done -> sync`. Chronis preserves that simplicity while adding the observability and durability that production agent systems need.

## Install

```bash
cargo install chronis
```

## Quick Start

```bash
cn init                                          # Create .chronis/ workspace
cn task create "Design auth module" -p p0        # Create a task
cn task create "Write tests" --type=bug          # Create a bug
cn list                                          # List all tasks (human-readable)
cn list --toon                                   # List all tasks (agent-optimized)
cn ready --toon                                  # Show unblocked open tasks
cn claim <id> --toon                             # Claim a task
cn done <id> --toon                              # Complete a task
cn archive --all-done                            # Clean up finished work
cn sync                                          # Sync via git (pull/push)
```

## TOON: Agent-Native Output

This is chronis's signature feature. Pass `--toon` to **any command** for [TOON (Token-Oriented Object Notation)](https://github.com/toon-format/toon) output — the same format used by AllSource's MCP server. No braces, no quotes, no wasted tokens.

### The problem

Traditional CLI tools output for humans. When an LLM agent runs `cn list`, it gets:

```
╭──────────┬──────┬──────────────────────┬─────┬─────────────┬─────────┬─────────╮
│ ID       │ Type │ Title                │ Pri │ Status      │ Claimed │ Blocked │
├──────────┼──────┼──────────────────────┼─────┼─────────────┼─────────┼─────────┤
│ t-abc1   │ task │ Design auth module   │ p0  │ open        │ -       │ -       │
│ t-abc2   │ bug  │ Fix login redirect   │ p1  │ in-progress │ agent-1 │ -       │
│ t-abc3   │ task │ Write integration... │ p2  │ done        │ agent-2 │ -       │
╰──────────┴──────┴──────────────────────┴─────┴─────────────┴─────────┴─────────╯
```

That's **~180 tokens** of box-drawing characters, padding, and repeated dashes that carry zero information for the model.

### The solution

```bash
cn list --toon
```

```
[id|type|title|pri|status|claimed|blocked_by|parent|archived]
t-abc1|task|Design auth module|p0|open||||false
t-abc2|bug|Fix login redirect|p1|in-progress|agent-1|||false
t-abc3|task|Write integration tests|p2|done|agent-2|||false
```

**~60 tokens.** Same information, ~50% fewer tokens. Every field is positional, pipe-delimited, with a header row. LLMs parse this natively.

### TOON across all commands

| Command | Human output | TOON output |
|---------|-------------|-------------|
| `cn list --toon` | ASCII table | `[header]\nrow\nrow` |
| `cn ready --toon` | ASCII table | `[header]\nrow\nrow` |
| `cn show <id> --toon` | Multi-line prose | `key:value` lines + child/timeline tables |
| `cn claim <id> --toon` | "Claimed task t-abc1 (agent: agent-1)" | `ok:claimed:t-abc1` |
| `cn done <id> --toon` | "Completed task t-abc1" | `ok:done:t-abc1` |
| `cn task create --toon` | "Created task t-abc1: Title" | `created:task:t-abc1:Title` |
| `cn archive --toon` | "Archived task t-abc1" | `ok:archived:t-abc1` |

### Agent workflow (4 commands, ~20 tokens of overhead)

```bash
export CN_AGENT_ID=agent-1
cn ready --toon              # → [id|type|title|pri|status|...]\nt-abc1|task|...
cn claim t-abc1 --toon       # → ok:claimed:t-abc1
# ... agent does work ...
cn done t-abc1 --toon        # → ok:done:t-abc1
cn list --toon               # → verify state
```

Compare to JSON-based tools where the same loop burns ~200+ tokens on structural overhead alone.

## Commands

### Task Lifecycle

| Command | Alias | Description |
|---------|-------|-------------|
| `cn init` | | Initialize a `.chronis/` workspace in the current directory |
| `cn task create <title>` | | Create a task (see flags below) |
| `cn list [--status=open]` | `cn ls` | List tasks, optionally filtered by status |
| `cn ready` | `cn r` | Show tasks that are open and unblocked |
| `cn show <id>` | `cn s` | Task details, children, and event timeline |
| `cn claim <id>` | `cn c` | Claim a task (uses `CN_AGENT_ID` env var, defaults to "human") |
| `cn done <id> [--reason=...]` | `cn d` | Mark a task as done |
| `cn approve <id>` | | Approve a task |
| `cn archive <ids...>` | | Archive tasks (hide from default listings) |
| `cn archive --all-done` | | Archive all completed tasks |
| `cn archive --done-before=30` | | Archive tasks done 30+ days ago |
| `cn unarchive <ids...>` | | Restore archived tasks |

All commands accept `--toon` for agent-optimized output.

### Task Creation Flags

```bash
cn task create "Title" \
  -p p1 \                    # Priority: p0 (critical), p1, p2 (default), p3
  --type=epic \              # Type: task (default), epic, bug, feature
  --parent=<id> \            # Parent task ID (for hierarchy under epics)
  --blocked-by=<id1>,<id2> \ # Tasks that block this one
  -d "Description text"      # Description
```

### Bulk Actions (Cascade)

Cascade operations apply to a task and all its children — useful for closing out an entire epic:

```bash
cn claim <epic-id> --cascade             # Claim epic + all children
cn done <epic-id> --cascade              # Mark epic + all children as done
cn done <epic-id> --cascade --reason="Sprint complete"
```

Cascade walks the parent-child tree depth-first, processing children before the parent. Tasks already in the target state are skipped.

### Archiving

Archive tasks to declutter your default listings. Archived tasks are hidden from `cn list` and `cn ready` but preserved in the event stream — nothing is deleted.

```bash
cn archive t-abc1 t-abc2                 # Archive specific tasks
cn archive --all-done                    # Archive all completed tasks
cn archive --done-before 30              # Archive tasks done 30+ days ago
cn unarchive t-abc1                      # Restore an archived task

cn list                                  # Excludes archived (default)
cn list --archived                       # Show only archived tasks
cn list --all                            # Show everything including archived
```

### Dependencies

```bash
cn dep add <task-id> <blocker-id>      # Add a blocker
cn dep remove <task-id> <blocker-id>   # Remove a blocker
```

### Git Sync

Sync chronis state across machines via git. Events are exported to an append-only JSONL file that git can merge naturally.

```bash
cn sync     # Pull remote events, export local events, commit, push
```

**How it works:**

1. `git pull --rebase` — fetch remote changes
2. Import new events from `.chronis/sync/events.jsonl` into local Core
3. Append new local events to the JSONL file
4. `git commit` + `git push`

Deduplication is handled via UUID tracking — each event is written once by its creating machine and never duplicated.

**Multi-machine workflow:**

```bash
# Machine A                    # Machine B
cn task create "Auth" -p p0    cn task create "Docs" -p p2
cn sync                        cn sync          # pulls A's tasks, pushes B's
cn sync                        # pulls B's task
```

### Visualization

```bash
cn tui                        # Interactive ratatui TUI (j/k nav, Tab view, c/d/a actions)
cn serve [--port=3905]        # Embedded web viewer (Axum + HTMX)
cn serve --open               # Auto-open browser
```

### Migration from beads_rust

One command migrates your existing beads issues to chronis — preserving IDs, status, priorities, dependencies, and parent-child relationships:

```bash
cn migrate-beads              # Import issues from .beads/ directory
cn migrate-beads --beads-dir=/path/to/.beads
```

Both `.beads/` and `.chronis/` can coexist during transition. Nothing is deleted.

## Chronis vs beads_rust

| Feature | beads_rust (`bd`) | Chronis (`cn`) |
|---------|-------------------|----------------|
| Storage model | SQLite + JSONL snapshot | Event-sourced (WAL + Parquet) |
| History | Current state only | Full temporal event stream |
| Agent output | JSON / human text | **TOON** (~50% fewer tokens) |
| Cascade operations | Manual scripting | `--cascade` flag |
| Archiving | Not available | `cn archive --all-done` |
| Approval workflows | Not available | `cn approve` with event trail |
| Git sync | `git commit && push` | Append-only JSONL with UUID dedup |
| Undo | Manual re-open | Replay from event stream |
| TUI | Separate binary (beads_viewer) | Built-in (`cn tui`) |
| Web UI | Separate project | Built-in (`cn serve`) |
| Queryable timeline | No | `cn show <id>` with full event history |
| Data durability | SQLite | CRC32 WAL + Snappy Parquet |

Both tools share the same core workflow: `ready -> claim -> done -> sync`. If you're coming from beads_rust, the transition is straightforward — and `cn migrate-beads` handles the data.

## Workflow

```
cn ready  -->  cn claim <id>  -->  (do work)  -->  cn done <id>  -->  cn archive --all-done  -->  cn sync
```

For agent orchestration, set `CN_AGENT_ID` to identify which agent claims tasks:

```bash
export CN_AGENT_ID=agent-1
cn claim <id>     # Records "agent-1" as the claimer
```

## Architecture

Chronis wraps [AllSource](https://all-source.xyz)'s embedded library. Every mutation (create, claim, done, approve, dependency change) emits an event into the WAL. A `TaskProjection` folds these events into queryable task state stored in a DashMap (~12us reads).

```
cn CLI (clap)
    |
    v
TaskRepository trait
    |
    v
EmbeddedCore (allsource-core)
    |
    +--> WAL (CRC32, fsync) --> Parquet (Snappy)
    +--> DashMap (in-memory reads)
    +--> TaskProjection (event folding)
```

Data lives in `.chronis/` at the project root:

```
.chronis/
  wal/            # Write-ahead log segments
  storage/        # Columnar event storage (Parquet)
  sync/           # Git sync exchange (events.jsonl)
  config.toml     # Workspace config
  .gitignore      # Excludes binary data, tracks sync files
```

## Event Types

All state is derived from these events:

| Event | Emitted by |
|-------|-----------|
| `task.created` | `cn task create` |
| `task.updated` | (future: `cn task update`) |
| `task.dependency.added` | `cn dep add` or `--blocked-by` flag |
| `task.dependency.removed` | `cn dep remove` |
| `workflow.claimed` | `cn claim` (first-write-wins) |
| `workflow.step.completed` | `cn done` |
| `workflow.approval.granted` | `cn approve` |
| `task.archived` | `cn archive` |
| `task.unarchived` | `cn unarchive` |

## Quality Gates

```bash
cargo fmt --check             # Formatting
cargo clippy -- -D warnings   # Lints (zero warnings)
cargo test                    # Integration tests
```

## Acknowledgments

Chronis exists because [beads_rust](https://github.com/Dicklesworthstone/beads_rust) proved that agents can self-manage tasks through a simple CLI. The `ready -> claim -> done -> sync` workflow that beads_rust pioneered is the foundation chronis builds on. We're grateful to [@Dicklesworthstone](https://github.com/Dicklesworthstone) and the beads community for blazing that trail.
