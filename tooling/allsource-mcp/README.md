# allsource-mcp

Lightweight MCP server for local AllSource debugging. Reads directly from WAL + Parquet files on disk — no running Core server needed.

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

## Claude Code Configuration

Add to `~/.claude/settings.json` (or project `.claude/settings.json`):

```json
{
  "mcpServers": {
    "allsource": {
      "command": "allsource-mcp",
      "args": ["--data-dir", "/path/to/allsource/data"],
      "env": {}
    }
  }
}
```

For Longhand on macOS:

```json
{
  "mcpServers": {
    "allsource": {
      "command": "allsource-mcp",
      "args": ["--data-dir", "~/Library/Application Support/Longhand/allsource"],
      "env": {}
    }
  }
}
```

Or use the environment variable instead of `--data-dir`:

```json
{
  "mcpServers": {
    "allsource": {
      "command": "allsource-mcp",
      "env": {
        "ALLSOURCE_DATA_DIR": "~/Library/Application Support/Longhand/allsource"
      }
    }
  }
}
```

## Available Tools

| Tool | Description |
|------|-------------|
| `query_events` | Query events with filters (entity_id, event_type, time range, limit) |
| `sample_events` | Sample recent events across all entities |
| `quick_stats` | Event store summary: counts, date range, event type distribution |
| `get_snapshot` | Get latest projection/snapshot state for an entity |
| `event_timeline` | Chronological event timeline for an entity |
| `explain_entity` | Human-readable lifecycle summary of an entity |
| `reconstruct_state` | Fold all events to reconstruct current entity state |
| `analyze_changes` | Analyze changes within a time window |

## Example Session

```
> Use query_events to find all workflow_run events for entity workflow:abc-123

Found 5 events:
1. workflow_run.started (2024-01-15T10:00:00Z)
2. workflow_run.step_completed (2024-01-15T10:00:05Z)
3. workflow_run.step_completed (2024-01-15T10:00:12Z)
4. workflow_run.step_completed (2024-01-15T10:00:18Z)
5. workflow_run.completed (2024-01-15T10:00:20Z)

> Use explain_entity to summarize this workflow

Entity workflow:abc-123 has 5 events spanning 20 seconds.
Lifecycle: started → 3 steps completed → completed
```
