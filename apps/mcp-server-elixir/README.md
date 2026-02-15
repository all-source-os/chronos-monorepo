---
title: "MCP Server"
status: CURRENT
version: 0.10.1
last_updated: 2026-02-15
category: service
port: 4000
technology: Elixir
---

# AllSource MCP Server

> Turn your event store into an AI-queryable knowledge base. 61 tools for natural language event exploration, time-travel analysis, and real-time insights — all via Model Context Protocol.

[![CI](https://github.com/all-source-os/all-source/actions/workflows/ci.yml/badge.svg)](https://github.com/all-source-os/all-source/actions/workflows/ci.yml)
[![Elixir](https://img.shields.io/badge/elixir-1.17%2B-purple.svg)](https://elixir-lang.org/)
[![Tools](https://img.shields.io/badge/MCP%20tools-61-orange.svg)]()
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](../../LICENSE)

**Current Version**: v0.10.1

## What Can It Do?

Connect Claude Desktop (or any MCP client) to your AllSource event store and ask questions like:

- *"What changed for user-123 yesterday?"*
- *"Show me the signup-to-purchase funnel for the last 30 days"*
- *"Which entities have anomalous activity patterns?"*
- *"Forecast event volume for next week"*
- *"Compare user-123 and user-456 side by side"*

The MCP server translates natural language into precise event store queries, returns results in a token-efficient format, and maintains conversation context across multi-turn interactions.

## Quick Start

### 1. Start AllSource Services

```bash
# Core Event Store
cd apps/core && cargo run --release

# Control Plane (optional, needed for tenant management)
cd apps/control-plane && go run .
```

### 2. Start MCP Server

```bash
cd apps/mcp-server-elixir
mix deps.get
mix run --no-halt
```

### 3. Connect Claude Desktop

Add to your Claude Desktop configuration (`~/Library/Application Support/Claude/claude_desktop_config.json`):

```json
{
  "mcpServers": {
    "allsource": {
      "command": "mix",
      "args": ["run", "--no-halt"],
      "cwd": "/path/to/all-source/apps/mcp-server-elixir"
    }
  }
}
```

Or with a compiled release:

```json
{
  "mcpServers": {
    "allsource": {
      "command": "/path/to/apps/mcp-server-elixir/_build/dev/rel/mcp_server_elixir/bin/mcp_server_elixir",
      "args": ["start"]
    }
  }
}
```

## 61 Tools Across 11 Categories

Tools follow a guided workflow from discovery to deep analysis. Each phase builds on the previous one.

### Phase 1: Discover — *What data exists? Start here.*
| Tool | Description |
|------|-------------|
| `quick_stats` | Rapid approximate statistics for fast orientation |
| `sample_events` | Stratified sampling for data exploration |
| `get_stats` | Detailed store statistics |
| `get_cluster_status` | Cluster health and node status |
| `list_schemas` | Available event schemas |

### Phase 2: Search — *Find relevant data*
| Tool | Description |
|------|-------------|
| `query_events` | Flexible event queries with filters, time ranges, pagination |
| `semantic_search_events` | Vector similarity search across event payloads |
| `hybrid_search` | Combined vector + keyword search |
| `get_query_advice` | AI-guided query recommendations for your use case |

### Phase 3: Drill Down — *Deep analysis*
| Tool | Description |
|------|-------------|
| `get_snapshot` | Fast current state retrieval |
| `reconstruct_state` | Time-travel state reconstruction at any timestamp |
| `analyze_changes` | Temporal diff between two points in time |
| `event_timeline` | Chronological event timeline for an entity |
| `explain_entity` | Comprehensive entity analysis with context |
| `find_patterns` | Event pattern detection and anomalies |
| `compare_entities` | Side-by-side multi-entity comparison |

### Phase 4: Context — *Multi-turn conversations*
| Tool | Description |
|------|-------------|
| `start_session` | Begin a conversation session with context |
| `refine_query` | Iteratively refine queries based on results |
| `get_session_context` | Retrieve accumulated session context |

### Phase 5: Mutate — *Write operations*
| Tool | Description |
|------|-------------|
| `ingest_event` | Create new events |
| `delete_events` | Soft delete with audit trail (GDPR/CCPA compliant) |
| `archive_events` | Move to cold storage with retention policies |
| `import_events` | Bulk import with validation and deduplication |
| `clone_entity` | Deep copy entity with all events |
| `merge_entities` | Combine event streams from multiple entities |
| `split_entity` | Partition entity event stream by criteria |

### Phase 6: Event Lifecycle — *Always available*
| Tool | Description |
|------|-------------|
| `restore_events` | Restore deleted or archived events |
| `export_events` | Export to JSON, JSONL, CSV, or Parquet |

### Phase 7: Operations — *System inspection & admin*

**Read (always available):**

| Tool | Description |
|------|-------------|
| `storage_stats` | Storage utilization and file details |
| `partition_info` | Partition distribution and balance |
| `wal_status` | Write-Ahead Log health and segment info |
| `backup_list` | Available backups |
| `health_deep` | Deep health check across all subsystems |
| `performance_report` | Performance metrics and bottleneck detection |
| `audit_log` | Audit trail inspection |

**Write (gated by read-only mode):**

| Tool | Description |
|------|-------------|
| `compact_storage` | Trigger storage compaction |
| `backup_create` | Create point-in-time backup |
| `backup_restore` | Restore from backup |

### Phase 8: Tenants — *Multi-tenancy management*
| Tool | Description |
|------|-------------|
| `tenant_create` | Create new tenant with quotas |
| `tenant_update` | Update tenant configuration |
| `tenant_usage` | Usage statistics and billing data |
| `tenant_quotas` | Manage quotas and rate limits |
| `tenant_suspend` | Suspend/reactivate tenant |
| `tenant_export` | Export all tenant data |

### Phase 9: Schema & Validation — *Event type governance*
| Tool | Description |
|------|-------------|
| `register_schema` | Register event schema with versioning |
| `validate_schema` | Validate events against schema |
| `migrate_schema` | Migrate to new schema version |
| `infer_schema` | Auto-infer schema from existing events |
| `schema_diff` | Compare schema versions for breaking changes |

### Phase 10: Analytics — *Business intelligence*
| Tool | Description |
|------|-------------|
| `cohort_analysis` | Analyze user cohorts over time |
| `correlation_analysis` | Find correlations between event types |
| `forecast_events` | Forecast future event volumes |
| `segment_analysis` | User segment analysis |
| `path_analysis` | User journey and conversion path analysis |
| `attribution_analysis` | Attribution modeling for conversions |
| `churn_prediction` | Predict churn risk |
| `ltv_calculation` | Calculate customer lifetime value |

### Phase 11: Developer Experience — *Productivity & testing*
| Tool | Description |
|------|-------------|
| `generate_client` | Generate client code for your language |
| `mock_events` | Generate realistic mock events for testing |
| `debug_query` | Debug query performance with explain plans |
| `benchmark_query` | Benchmark query execution |

## Token-Efficient Responses

All tools return responses in **TOON format** by default — approximately **50% fewer tokens** than JSON for tabular data. This directly reduces LLM API costs and speeds up processing.

| Format | Tokens (typical) | Use Case |
|--------|:----------------:|----------|
| TOON (default) | ~500 | Tabular data, lists, summaries |
| JSON | ~1,000 | Complex nested structures |

Override per-request with `format: "json"` or `format: "toon"`.

## Access Control

Tools are gated by configuration to prevent unintended modifications:

| Gate | Tools Affected | Default |
|------|---------------|---------|
| **Read-only mode** | All write operations (ingest, delete, archive, import, clone, merge, split, compact, backup create/restore) | Off |
| **Control Plane** | All tenant management tools | Enabled when Control Plane URL is configured |

## Architecture

```
┌─────────────────────────────────────────┐
│         Claude Desktop / LLM            │
│                                         │
│  "What changed for user-123 yesterday?" │
└────────────────┬────────────────────────┘
                 │ JSON-RPC 2.0 over stdio
                 ▼
┌────────────────────────────────────────────┐
│      MCP Server (Elixir/OTP)              │
│                                            │
│  • JSON-RPC Handler (pattern matching)    │
│  • Tool Router (61 tools)                 │
│  • Session Manager (multi-turn context)   │
│  • TOON Formatter (token optimization)    │
│  • OTP Supervision (fault tolerance)      │
└────────────────┬──────────────────────────┘
                 │ Tesla HTTP Client
      ┌──────────┴──────────┐
      ▼                     ▼
┌──────────────┐   ┌─────────────────┐
│  Core API    │   │  Control Plane  │
│  (Rust)      │   │  (Go)           │
│  :3900       │   │  :3901          │
└──────────────┘   └─────────────────┘
```

## Performance

| Metric | Value |
|--------|-------|
| Tool call latency | <100ms (local network) |
| Query execution | <10ms (indexed queries) |
| Time-travel reconstruction | <50ms (typical entity) |
| Pattern analysis | <500ms (thousands of events) |
| Token reduction | ~50% fewer tokens vs JSON |

## Development

```bash
# Install dependencies
mix deps.get

# Run tests
mix test

# Code formatting
mix format

# Static analysis
mix credo
mix dialyzer

# Build release
MIX_ENV=prod mix release
```

## Configuration

| Environment Variable | Default | Description |
|---------------------|---------|-------------|
| `ALLSOURCE_CORE_URL` | `http://localhost:3900` | Core event store URL |
| `ALLSOURCE_CONTROL_URL` | `http://localhost:3901` | Control Plane URL (enables tenant tools) |
| `MCP_READ_ONLY` | `false` | Disable all write operations |

## Troubleshooting

### MCP Server Won't Start
```bash
elixir --version           # Check Elixir is installed
curl localhost:3900/health # Check Core is running
curl localhost:3901/health # Check Control Plane is running
```

### Claude Desktop Not Connecting
- Restart Claude Desktop completely
- Check config path is absolute
- Verify MCP logs in `~/Library/Logs/Claude/`

### Tool Calls Failing
- Use ISO timestamps (`YYYY-MM-DDTHH:mm:ssZ`)
- Check entity exists before querying
- Test the underlying API directly with curl

## Resources

- [Model Context Protocol Spec](https://spec.modelcontextprotocol.io/)
- [AllSource Documentation](../../README.md)
- [Core API Reference](../core/README.md)

## License

[MIT](../../LICENSE)

---

<div align="center">

**AllSource MCP Server** — 61 AI-native tools for your event store

Query, analyze, and manage temporal data through natural language

[GitHub](https://github.com/all-source-os/all-source) | [AllSource Core](../core/README.md)

</div>
