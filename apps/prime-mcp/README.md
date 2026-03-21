# allsource-prime

Unified agent memory engine — vectors + graph + events in one binary.

MCP server (stdio) and HTTP REST API for AI agents that need persistent, cross-domain memory with temporal reasoning.

## Install

```bash
cargo install allsource-prime
```

## Quick Start — MCP (Claude Desktop)

Add to `~/.claude/claude_desktop_config.json`:

```json
{
  "mcpServers": {
    "prime": {
      "command": "allsource-prime",
      "args": ["--data-dir", "~/.prime/memory"]
    }
  }
}
```

Restart Claude Desktop. You now have 13 tools:

| Tool | Description |
|------|-------------|
| `prime_add_node` | Create a node (person, concept, project...) |
| `prime_add_edge` | Create a relationship between nodes |
| `prime_neighbors` | Find connected nodes (1-hop or multi-hop BFS) |
| `prime_search` | Find nodes by type |
| `prime_shortest_path` | Find shortest path between two nodes |
| `prime_forget` | Soft-delete a node (preserved in history) |
| `prime_history` | Full audit trail for any entity |
| `prime_stats` | Graph statistics |
| `prime_embed` | Store a vector embedding |
| `prime_similar` | Find similar embeddings |
| `prime_recall` | Hybrid recall: vectors + graph + temporal |
| `prime_index` | Compressed knowledge index (for system prompt injection) |
| `prime_context` | Combined retrieval: index + vectors + graph |

## Quick Start — HTTP

```bash
allsource-prime --mode http --port 3905 --data-dir ~/.prime/memory
```

```bash
# Create a node
curl -X POST http://localhost:3905/api/v1/prime/nodes \
  -H 'Content-Type: application/json' \
  -d '{"type": "person", "properties": {"name": "Alice", "role": "engineer"}}'

# Get stats
curl http://localhost:3905/api/v1/prime/stats
```

## Auto-Inject (zer0dex-style)

Pre-inject a compressed knowledge index into every agent conversation:

```bash
allsource-prime --data-dir ~/.prime/memory --auto-inject --auto-inject-max-tokens 1000
```

Exposes `prime://auto-context` as an MCP resource — the agent's system prompt automatically includes a token-efficient summary of everything it knows, organized by domain with cross-references.

## What Makes This Different

| Feature | zer0dex | Mem0 | Letta | **AllSource Prime** |
|---------|---------|------|-------|---------------------|
| Compressed index | Manual markdown | No | No | **Auto-generated** |
| Temporal queries | No | No | No | **Full time-travel** |
| Provenance | No | No | Partial | **Immutable event audit** |
| Cross-domain recall | 80% | ~50% | ~37% | **80%+** |
| Offline/embedded | Yes | No | No | **Yes + sync** |
| Latency | 70ms | Variable | Variable | **12μs** |

## Architecture

```
┌─────────────────────────────────────────────┐
│              AllSource Prime                 │
│                                              │
│  Graph    Vectors    Temporal    Compressed   │
│  Nodes    HNSW       History    Index        │
│  Edges    Embed      Time-travel Cross-refs  │
│           Similar    Diff                    │
│                                              │
│  ┌──────────────────────────────────────┐   │
│  │         AllSource Core Engine         │   │
│  │  WAL + Parquet + DashMap + HLC + CRDT │   │
│  │  469K events/sec │ 12μs queries       │   │
│  └──────────────────────────────────────┘   │
└─────────────────────────────────────────────┘
```

## License

Apache-2.0
