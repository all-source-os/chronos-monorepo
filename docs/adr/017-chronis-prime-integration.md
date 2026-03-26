# ADR-017: Chronis Prime Integration

**Status:** Accepted
**Date:** 2026-03-26
**Deciders:** Architecture Review

## Context

Chronis (`cn` CLI) is an event-sourced task manager using AllSource's EmbeddedCore for storage. Tasks are stored as events with a custom `TaskProjection` for state management. Users manage tasks with `cn task create`, `cn claim`, `cn done`, etc.

Current limitations:
- **No semantic search** — `cn list` filters by status/type but can't find tasks by meaning ("payment bug")
- **No relationship discovery** — dependencies are tracked but there's no way to traverse the graph ("what else is affected by this API change?")
- **No agent context** — agents working on tasks have no compressed summary of the task landscape

AllSource Prime provides graph + vector + recall capabilities that directly address these gaps.

## Decision

Integrate Prime into Chronis as **opt-in feature flags**:

- `prime` — graph indexing and relationship discovery (no heavy deps)
- `prime-full` — adds vector search via HNSW (pulls in instant-distance/fastembed)

The base `cn` binary stays lightweight. Users opt in with:
```bash
cargo install chronis --features prime-full
```

### Architecture

```
Chronis CLI
├── EmbeddedCore (existing) — event storage, WAL, projections
├── TaskProjection (existing) — task state from events
└── TaskMemory (new, feature-gated)
    ├── Prime (graph engine)
    │   ├── Nodes: one per task (type, priority, status, domain)
    │   ├── Edges: depends_on, child_of
    │   └── Stats: node/edge counts
    └── Vectors (prime-full only)
        ├── Embed: task title + description → HNSW index
        └── Search: semantic similarity queries
```

### ID Mapping

Chronis uses short IDs (`t-abc`). Prime uses entity IDs (`node:task:<uuid>`). A `HashMap<String, String>` in `TaskMemory` maps between them. The chronis ID is also stored in the node's `chronis_id` property for reverse lookup.

### Indexing Strategy

Tasks are indexed into Prime on-demand (not automatically on every event). The user runs `cn prime index` to bulk-index, or individual tasks are indexed as they're created when the feature is enabled.

## Consequences

### Positive
- Agents can discover related tasks before starting work
- `cn related <id>` shows dependency chains and sibling tasks
- With `prime-full`, `cn find "query"` enables semantic search
- Compressed index gives agents a token-efficient task overview

### Negative
- `prime-full` adds ~30MB to binary size (HNSW + embedding model deps)
- Two state stores (EmbeddedCore events + Prime graph) must stay in sync
- ID mapping adds complexity

### Risks
- Prime graph can drift from TaskProjection if events are replayed without re-indexing
- Mitigation: `cn prime index` rebuilds the full graph from current projection state
