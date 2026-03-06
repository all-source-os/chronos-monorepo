# Chronis

Event-sourced task CLI powered by [AllSource](https://all-source.xyz), the embedded event database. Every action is an immutable event — state is derived from projections over the event stream.

Binary: `cn` | Crate: `chronis` | Storage: `.chronis/` | Author: [Decebal Dobrica](https://decebaldobrica.com)

## Install

```bash
cargo install chronis
```

## Quick Start

```bash
cn init                                          # Create .chronis/ workspace
cn task create "Design auth module" -p p0        # Create a task
cn task create "Write tests" --type=bug          # Create a bug
cn list                                          # List all tasks
cn ready                                         # Show unblocked open tasks
cn claim <id>                                    # Claim a task
cn done <id> --reason="Shipped"                  # Complete a task
cn show <id>                                     # Task detail + event timeline
```

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

### Task Creation Flags

```bash
cn task create "Title" \
  -p p1 \                    # Priority: p0 (critical), p1, p2 (default), p3
  --type=epic \              # Type: task (default), epic, bug, feature
  --parent=<id> \            # Parent task ID (for hierarchy under epics)
  --blocked-by=<id1>,<id2> \ # Tasks that block this one
  -d "Description text"      # Description
```

### Dependencies

```bash
cn dep add <task-id> <blocker-id>      # Add a blocker
cn dep remove <task-id> <blocker-id>   # Remove a blocker
```

### Visualization

```bash
cn tui                        # Interactive ratatui TUI (j/k nav, Tab view, c/d/a actions)
cn serve [--port=3905]        # Embedded web viewer (Axum + HTMX)
cn serve --open               # Auto-open browser
```

### Sync & Migration

```bash
cn sync --git                 # Stage .chronis/, commit, push
cn migrate-beads              # Import issues from .beads/ directory
cn migrate-beads --beads-dir=/path/to/.beads
```

## Workflow

```
cn ready  -->  cn claim <id>  -->  (do work)  -->  cn done <id>  -->  cn sync --git
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
  wal/          # Write-ahead log segments
  parquet/      # Columnar event storage
  config.toml   # Workspace config
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

## Quality Gates

```bash
cargo fmt --check             # Formatting
cargo clippy -- -D warnings   # Lints (zero warnings)
cargo test                    # 11 integration tests
```
