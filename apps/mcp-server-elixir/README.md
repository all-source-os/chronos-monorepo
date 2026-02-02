---
title: "MCP Server"
status: CURRENT
last_updated: 2026-02-02
category: service
port: 4000
technology: Elixir
---

# AllSource MCP Server (Elixir)

> AI-Native Temporal Event Store Interface via Model Context Protocol

Transform your event store into an AI-queryable knowledge base. Ask questions in natural language, time-travel through data, and get instant insights from your temporal event stream.

## 🌟 Overview

This is the Elixir implementation of the AllSource MCP Server, replacing the TypeScript version to better align with the stack (Go, Rust, Elixir). It provides the same functionality with improved performance, fault tolerance via OTP supervision, and better integration with the existing Elixir query service.

## 🎯 Features

- **11 Core Tools** for event operations
- **TOON Format** by default (~50% fewer tokens than JSON)
- **JSON Fallback** available via `format: "json"` parameter
- **JSON-RPC 2.0** protocol over stdio
- **OTP Supervision** for fault tolerance
- **Tesla HTTP Client** with automatic retries
- **Pattern Matching** for elegant JSON-RPC handling

## 🚀 Quick Start

### 1. Install Dependencies

```bash
cd apps/mcp-server-elixir
mix deps.get
```

### 2. Configure Environment Variables (Optional)

```bash
export ALLSOURCE_CORE_URL="http://localhost:3900"
export ALLSOURCE_CONTROL_URL="http://localhost:3901"
```

### 3. Start AllSource Services

Make sure the Core and Control Plane services are running:

```bash
# Terminal 1 - Core Event Store
cd apps/core
cargo run --release

# Terminal 2 - Control Plane
cd apps/control-plane
go run main.go
```

### 4. Start MCP Server

```bash
cd apps/mcp-server-elixir
mix run --no-halt
```

Or compile and run as a release:

```bash
mix release
_build/dev/rel/mcp_server_elixir/bin/mcp_server_elixir start
```

### 5. Connect Claude Desktop

Update your Claude Desktop configuration:

```json
{
  "mcpServers": {
    "allsource": {
      "command": "mix",
      "args": ["run", "--no-halt"],
      "cwd": "/path/to/chronos-monorepo/apps/mcp-server-elixir"
    }
  }
}
```

Or use the compiled release:

```json
{
  "mcpServers": {
    "allsource": {
      "command": "/path/to/chronos-monorepo/apps/mcp-server-elixir/_build/dev/rel/mcp_server_elixir/bin/mcp_server_elixir",
      "args": ["start"]
    }
  }
}
```

## 📋 Available Tools

The server exposes 11 tools matching the TypeScript implementation:

1. **query_events** - Query events with flexible filters
2. **reconstruct_state** - Time-travel state reconstruction
3. **get_snapshot** - Fast current state retrieval
4. **analyze_changes** - Temporal diff analysis
5. **find_patterns** - Event pattern detection
6. **compare_entities** - Multi-entity comparison
7. **event_timeline** - Chronological timeline
8. **explain_entity** - Comprehensive entity analysis
9. **ingest_event** - Event creation
10. **get_stats** - Store statistics
11. **get_cluster_status** - Cluster health

### Response Format

All tools return responses in **TOON format** by default, which uses approximately **50% fewer tokens** than JSON. This reduces LLM API costs and improves processing speed.

**Format Options:**
- Default: Auto-detects tabular data and uses TOON, falls back to JSON for complex structures
- `format: "toon"` - Force TOON format
- `format: "json"` - Force JSON format

**Example:**
```json
{
  "name": "query_events",
  "arguments": {
    "entity_id": "user-123",
    "format": "toon"  // Optional: "toon" or "json"
  }
}
```

## 🏗️ Architecture

```
┌─────────────────────────────────────────┐
│         Claude Desktop / LLM            │
│                                         │
│  "What changed for user-123 yesterday?" │
└────────────────┬────────────────────────┘
                 │
                 │ JSON-RPC 2.0 over stdio
                 ▼
┌────────────────────────────────────────────┐
│      MCP Server (Elixir)                  │
│                                            │
│  • JSON-RPC Handler                       │
│  • Tool Router                            │
│  • OTP Supervision                       │
└────────────────┬──────────────────────────┘
                 │
      ┌──────────┴──────────┐
      │                     │
      ▼                     ▼
┌──────────────┐   ┌─────────────────┐
│  Core API    │   │  Control Plane  │
│  (Rust)      │   │  (Go)            │
│              │   │                 │
│  :3900       │   │  :3901          │
└──────────────┘   └─────────────────┘
```

## 🔧 Development

### Running Tests

```bash
mix test
```

### Code Formatting

```bash
mix format
```

### Static Analysis

```bash
mix credo
mix dialyzer
```

### Building Release

```bash
MIX_ENV=prod mix release
```

## 📊 Performance

- **Tool latency:** <100ms (local network)
- **Query execution:** <10ms (indexed queries)
- **Time-travel reconstruction:** <50ms (typical entity)
- **Pattern analysis:** <500ms (1000s of events)
- **Token reduction:** ~50% fewer tokens with TOON vs JSON for tabular data

## 🐛 Troubleshooting

### MCP Server Won't Start

```bash
# Check Elixir is installed
elixir --version

# Check AllSource is running
curl http://localhost:3900/health
curl http://localhost:3901/health

# Check for port conflicts
lsof -i :3900
lsof -i :3901
```

### Tool Calls Failing

- Check entity exists before querying
- Use ISO timestamps (YYYY-MM-DDTHH:mm:ssZ)
- Verify AllSource logs for errors
- Test API directly with curl first

### Claude Desktop Not Connecting

- Restart Claude Desktop completely
- Check config path is absolute
- Verify Mix path in config
- Check MCP logs in `~/Library/Logs/Claude/`

## 🚢 Production Considerations

### Security

- Add authentication to API endpoints
- Validate all tool inputs
- Rate limit tool calls
- Audit MCP usage

### Scalability

- MCP server is stateless (scales horizontally)
- AllSource Core handles heavy lifting
- Consider caching frequent queries
- Use read replicas for high traffic

### Monitoring

- Log all tool calls
- Track query latency
- Monitor API errors
- Alert on anomalies

## 📚 Resources

- [Model Context Protocol Spec](https://spec.modelcontextprotocol.io/)
- [Claude Desktop MCP Guide](https://docs.anthropic.com/claude/docs/model-context-protocol)
- [AllSource Documentation](../../README.md)

---

<div align="center">

**AllSource MCP Server (Elixir)** - *Where AI meets temporal data*

Built with ❤️ and Elixir

</div>

