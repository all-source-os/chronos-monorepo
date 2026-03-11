# AllSource MCP Setup

Configure the AllSource MCP server for local debugging in Claude Code.

Triggers on: "configure allsource mcp", "setup mcp", "allsource mcp config", "add allsource to claude code", "mcp server setup".

---

## Option 1: Rust stdio MCP server (recommended for local debugging)

Reads directly from WAL + Parquet files. No running server needed.

### Build

```bash
cd <chronos-monorepo>
cargo build --release -p allsource-mcp
```

Binary at: `target/release/allsource-mcp`

### Claude Code Configuration

Add to `~/.claude/settings.json` or project `.claude/settings.json`:

```json
{
  "mcpServers": {
    "allsource": {
      "command": "<path-to>/allsource-mcp",
      "args": ["--data-dir", "<path-to-data>"]
    }
  }
}
```

#### Longhand on macOS

```json
{
  "mcpServers": {
    "allsource": {
      "command": "/Users/<you>/Projects/chronos/chronos-monorepo/target/release/allsource-mcp",
      "args": ["--data-dir", "/Users/<you>/Library/Application Support/Longhand/allsource"]
    }
  }
}
```

#### Using environment variable

```json
{
  "mcpServers": {
    "allsource": {
      "command": "/path/to/allsource-mcp",
      "env": {
        "ALLSOURCE_DATA_DIR": "/path/to/data"
      }
    }
  }
}
```

### Available Tools

After configuration, these tools appear in Claude Code:

| Tool | Description |
|------|-------------|
| `query_events` | Query events (entity_id, event_type, time range, limit) |
| `sample_events` | Sample recent events across all entities |
| `quick_stats` | Store summary: counts, types, date range, durability |
| `get_snapshot` | Projection/snapshot state for an entity |
| `event_timeline` | Chronological event list for an entity |
| `explain_entity` | Human-readable entity lifecycle summary |
| `reconstruct_state` | Fold events to rebuild current state |
| `analyze_changes` | Changes within a time window |

### Verification

After adding the config, restart Claude Code and check:
1. Run `/mcp` — allsource should appear in the server list
2. Ask Claude to "use quick_stats" — should return event counts
3. Ask Claude to "query events for entity X" — should return events

---

## Option 2: Elixir MCP server (remote mode)

Connects to a running Core server over HTTP. Use when Core is running in Docker.

### Configuration

```json
{
  "mcpServers": {
    "allsource": {
      "command": "mix",
      "args": ["phx.server"],
      "cwd": "<chronos-monorepo>/apps/mcp-server-elixir",
      "env": {
        "CORE_URL": "http://localhost:3900",
        "CORE_MODE": "remote",
        "PORT": "3904"
      }
    }
  }
}
```

### With Docker stack running

If the Docker compose stack is up (`allsource-core-leader` on port 3280):

```json
{
  "mcpServers": {
    "allsource": {
      "command": "mix",
      "args": ["phx.server"],
      "cwd": "<chronos-monorepo>/apps/mcp-server-elixir",
      "env": {
        "CORE_URL": "http://localhost:3280",
        "CORE_MODE": "remote"
      }
    }
  }
}
```

---

## Option 3: Elixir MCP server (embedded mode)

Runs Core in-process via Rustler NIF. No separate Core needed, but requires Rust + Elixir toolchain.

```json
{
  "mcpServers": {
    "allsource": {
      "command": "mix",
      "args": ["phx.server"],
      "cwd": "<chronos-monorepo>/apps/mcp-server-elixir",
      "env": {
        "CORE_MODE": "embedded",
        "CORE_DATA_DIR": "/path/to/data"
      }
    }
  }
}
```

---

## Troubleshooting

### Server not starting

- **"No such file or directory"**: Check the binary path. Build with `cargo build --release -p allsource-mcp`.
- **"data_dir is required"**: Pass `--data-dir` arg or set `ALLSOURCE_DATA_DIR` env var.
- **Permission denied**: Check file permissions on the data directory.

### No events found

- Verify the data directory has files: `ls <data-dir>/storage/` should show `.parquet` files and/or `ls <data-dir>/wal/` should show `.log` files.
- If empty, the application may not have written events yet, or the path is wrong.
- Check if the data dir has a `__system/` subdirectory — if yes, it's an AllSource data dir.

### Tools not appearing

- Run `/mcp` in Claude Code to check server status.
- Check stderr output: set `RUST_LOG=debug` in the env to see MCP protocol messages.
- Ensure the JSON in settings is valid (no trailing commas).

### Slow startup

- Large Parquet collections take time to scan on first open.
- WAL replay happens at startup — many WAL entries = slower start.
- After first open, subsequent queries are fast (data cached in DashMap).

### Wrong data / stale results

- The MCP server opens the data directory at startup and replays WAL.
- If the application is still running and writing, new events won't appear until MCP server restarts.
- For live debugging, use the Elixir MCP server in remote mode connected to the running Core.
