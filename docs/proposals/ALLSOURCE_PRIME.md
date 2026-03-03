# AllSource Prime

> **Status**: Proposal
> **Author**: Design session 2026-03-02
> **Scope**: Unified agent memory engine — vectors + relationships + events in one embedded binary
> **Depends on**: [Embedded Core](EMBEDDED_CORE_AND_OFFLINE_FIRST.md), [Server-Side Projections](SERVER_SIDE_PROJECTIONS.md), [Core Replication](CORE_REPLICATION_DESIGN.md)
> **Supersedes**: [Knowledge Graph Service](KNOWLEDGE_GRAPH_SERVICE.md) (KG becomes one capability within Prime)

---

## 1. The Problem

AI agents need memory. Today, building agent memory requires gluing together three separate systems:

```
┌─────────────────────────────────────────────────────────────────┐
│                     What agents actually need                    │
├─────────────────┬──────────────────┬────────────────────────────┤
│ Semantic Recall │ Structured       │ Temporal History            │
│ "Find similar"  │ Relationships    │ "What did I know last week?"│
│                 │ "Who knows whom" │ "Who added this and why?"   │
├─────────────────┼──────────────────┼────────────────────────────┤
│ Vector DB       │ Graph DB         │ Event Store                 │
│ (Pinecone,      │ (Neo4j,          │ (custom, or nothing)        │
│  Qdrant,        │  FalkorDB,       │                            │
│  Chroma)        │  Graphiti)       │                            │
└─────────────────┴──────────────────┴────────────────────────────┘
        ↑                  ↑                      ↑
        └──────── 3 databases, 3 APIs, 3 failure modes ──────────┘
```

Every agent memory framework (Mem0, Letta, Cognee, Zep) is a **Python orchestration layer** that glues these three databases together. They add an LLM call on the write path for entity extraction, require API keys, and can't work offline.

The result:

| Pain | Detail |
|------|--------|
| Infrastructure complexity | 3 databases to provision, monitor, backup |
| Write latency | 1-2 seconds (LLM entity extraction + DB writes) |
| No provenance | Memories are mutable — who added this? when? from what source? |
| No time-travel | Can't ask "what did the agent know before Tuesday's update?" |
| Cloud-only | No offline support; no embedded/edge deployment |
| No sync | Multiple agents can't merge independent knowledge |

---

## 2. AllSource Prime

**One embedded engine. Vectors + relationships + events. Full history.**

```
┌─────────────────────────────────────────────────────────────────┐
│                       AllSource Prime                            │
│                                                                  │
│  ┌──────────────┐  ┌──────────────┐  ┌────────────────────────┐ │
│  │   Vectors    │  │   Graph      │  │   Events               │ │
│  │              │  │              │  │                        │ │
│  │  embed()     │  │  add_node()  │  │  Every mutation is     │ │
│  │  similar()   │  │  add_edge()  │  │  an immutable event    │ │
│  │  search()    │  │  neighbors() │  │  with full provenance  │ │
│  │              │  │  traverse()  │  │                        │ │
│  └──────┬───────┘  └──────┬───────┘  └───────────┬────────────┘ │
│         │                 │                      │              │
│         └────────────┬────┘──────────────────────┘              │
│                      │                                          │
│              ┌───────▼───────┐                                  │
│              │  AllSource    │                                   │
│              │  Core Engine  │                                   │
│              │               │                                   │
│              │  WAL + Parquet + DashMap + HLC + CRDT             │
│              │  469K events/sec │ 12μs queries │ durable         │
│              └───────────────┘                                   │
└──────────────────────────────────────────────────────────────────┘
```

Prime is not a wrapper around three databases. It's a **single engine** where:

- **Vectors** are events (stored in WAL, indexed by projection)
- **Graph nodes and edges** are events (stored in WAL, indexed by projections)
- **Everything has history** — because the storage layer IS an event store
- **Everything syncs** — because HLC + CRDT work at the event level, below vectors and graphs

```rust
use allsource_prime::{Prime, Node, Edge};

let prime = Prime::open("~/.my-agent/memory").await?;

// Vectors
prime.embed("doc-1", "Event sourcing is an append-only pattern", vector).await?;
let similar = prime.similar("doc-1", 10).await?;

// Graph
let alice = prime.add_node("person", json!({"name": "Alice"})).await?;
let project = prime.add_node("project", json!({"name": "Prime"})).await?;
prime.add_edge(&alice, &project, "works_on", None).await?;
let team = prime.neighbors(&project, Some("works_on"), Direction::Incoming).await?;

// Everything has history
let history = prime.history(&alice).await?;               // full audit trail
let past = prime.neighbors_as_of(&alice, None, last_week).await?;  // time-travel

// Hybrid recall: semantic + structural + temporal
let context = prime.recall("who works on the event store?", top_k=10).await?;
// → combines vector similarity + graph traversal + temporal recency

// Offline sync
prime.sync("https://cloud.example.com").await?;
```

---

## 3. Why "Prime"

The name works on multiple levels:

| Meaning | Connection |
|---------|------------|
| **Prime number** | Fundamental, indivisible building block — can't be decomposed further |
| **Prime** (adjective) | First, foundational, of the highest quality |
| **Prime vector** | Linearly independent — the basis vectors everything else is built from |
| **Optimus Prime** | Transforms between modes (embedded ↔ service ↔ cloud) |
| **Prime directive** | Single purpose: give agents perfect memory |

**AllSource Prime** — the prime memory engine for AI agents.

---

## 4. Architecture

### 4.1 Single-Engine Design

The key insight: vectors, graph nodes, graph edges, and domain events are **all events** at the storage layer. The differentiation happens at the **projection layer** — different projections maintain different indexes over the same unified event stream.

```
┌──────────────────────────────────────────────────────────────────┐
│                          Prime API                                │
│                                                                   │
│  VectorOps      GraphOps      TemporalOps     MemoryOps          │
│  ─────────      ────────      ───────────     ─────────          │
│  embed()        add_node()    history()       recall()           │
│  similar()      add_edge()    as_of()         remember()         │
│  search()       neighbors()   diff()          forget()           │
│                 traverse()    timeline()      compact()          │
│                 shortest()                                       │
└───────────────────────┬──────────────────────────────────────────┘
                        │
┌───────────────────────▼──────────────────────────────────────────┐
│                      Projection Layer                             │
│                                                                   │
│  ┌────────────┐ ┌────────────┐ ┌────────────┐ ┌───────────────┐ │
│  │ VectorIndex│ │ Adjacency  │ │ ReverseIdx │ │ NodeState     │ │
│  │            │ │ List       │ │            │ │               │ │
│  │ HNSW/flat  │ │ node →     │ │ target →   │ │ node →        │ │
│  │ index over │ │ [(rel,     │ │ [(rel,     │ │ {properties,  │ │
│  │ embeddings │ │  target)]  │ │  source)]  │ │  version}     │ │
│  └────────────┘ └────────────┘ └────────────┘ └───────────────┘ │
│  ┌────────────┐ ┌────────────┐ ┌────────────┐ ┌───────────────┐ │
│  │ TypeIndex  │ │ Relevance  │ │ Community  │ │ Contradiction │ │
│  │            │ │ Decay      │ │ Detection  │ │ Detection     │ │
│  │ type →     │ │ score per  │ │ Leiden     │ │ conflicting   │ │
│  │ [node_ids] │ │ node/edge  │ │ clusters   │ │ edges         │ │
│  └────────────┘ └────────────┘ └────────────┘ └───────────────┘ │
└───────────────────────┬──────────────────────────────────────────┘
                        │
┌───────────────────────▼──────────────────────────────────────────┐
│                     AllSource Core Engine                          │
│                                                                   │
│  ┌─────────┐  ┌──────────┐  ┌─────────┐  ┌────────┐ ┌────────┐ │
│  │ DashMap  │  │   WAL    │  │ Parquet │  │  HLC   │ │  CRDT  │ │
│  │ in-mem   │  │ durable  │  │ archive │  │ clock  │ │ merge  │ │
│  │ queries  │  │ CRC32    │  │ Snappy  │  │        │ │        │ │
│  └─────────┘  └──────────┘  └─────────┘  └────────┘ └────────┘ │
│                                                                   │
│  Events: prime.vector.stored, prime.node.created,                │
│          prime.edge.created, prime.node.updated, ...              │
└──────────────────────────────────────────────────────────────────┘
```

### 4.2 Event Namespace

All Prime events live under the `prime.` namespace:

| Event Type | Entity ID | Payload |
|------------|-----------|---------|
| `prime.vector.stored` | `vec:{id}` | `{text?, dimensions, metadata}` |
| `prime.vector.deleted` | `vec:{id}` | `{reason?}` |
| `prime.node.created` | `node:{type}:{id}` | `{type, properties, labels}` |
| `prime.node.updated` | `node:{type}:{id}` | `{properties}` |
| `prime.node.deleted` | `node:{type}:{id}` | `{reason?}` |
| `prime.edge.created` | `edge:{id}` | `{source, target, relation, properties, weight?}` |
| `prime.edge.deleted` | `edge:{id}` | `{reason?}` |
| `prime.memory.recalled` | `recall:{id}` | `{query, results, context}` |
| `prime.memory.compacted` | `node:{type}:{id}` | `{merged_from, new_state}` |

Vectors are stored as event metadata (the embedding) + event payload (the source text, dimensions, any labels). The `VectorIndexProjection` maintains the HNSW/flat index in memory, rebuilt from events on startup.

### 4.3 Deployment Modes

```
Mode A: Embedded Library
┌──────────────────────────┐
│  Your Agent / App        │
│  ┌────────────────────┐  │
│  │   Prime (in-proc)  │  │
│  │   WAL + Parquet    │  │
│  └────────────────────┘  │
└──────────────────────────┘
  cargo add allsource-core --features prime

Mode B: MCP Server (stdio)
┌──────────────┐    stdio    ┌──────────────────┐
│  AI Agent    │◄───────────►│  allsource-prime │
│  (Claude,    │   MCP       │  (single binary) │
│   custom)    │   tools     │  WAL + Parquet   │
└──────────────┘             └──────────────────┘
  brew install allsource-prime

Mode C: HTTP Service
┌──────────────┐    HTTP     ┌──────────────────┐
│  Any client  │◄───────────►│  Prime Service   │
│  (web, SDK,  │  REST API   │  port 3905       │
│   agent)     │             │  WAL + Parquet   │
└──────────────┘             └──────────────────┘
  docker run ghcr.io/all-source-os/prime

Mode D: Behind Query Service (SaaS)
  Clients → Query Service (auth, billing) → Prime → Core cluster
```

---

## 5. Competitive Analysis

### 5.1 The Three Silos

Today's landscape is siloed. No single product covers all three:

**Vector databases** (semantic recall):

| Product | Embeddable | Event-sourced | Graph | Offline sync | Write latency |
|---------|------------|---------------|-------|-------------|---------------|
| [Pinecone](https://www.pinecone.io/) | No (cloud) | No | No | No | ~20-50ms |
| [Qdrant](https://qdrant.tech/) | No (server) | No | No | No | ~10-20ms |
| [Weaviate](https://weaviate.io/) | No (server) | No | No | No | ~20-50ms |
| [Chroma](https://www.trychroma.com/) | Yes (Python) | No | No | No | <10ms |
| [Milvus](https://milvus.io/) | No (server) | No | No | No | <10ms |
| [LanceDB](https://lancedb.com/) | Yes (Rust) | Versioned¹ | No | No | <5ms |

¹ LanceDB has automatic versioning per write, but not true event sourcing — no entity-level audit trail, no time-travel queries by entity.

**Agent memory frameworks** (orchestration):

| Product | Embeddable | Event-sourced | Graph | Vectors | Offline sync | LLM on write |
|---------|------------|---------------|-------|---------|-------------|-------------|
| [Mem0](https://github.com/mem0ai/mem0) | No | No | Partial² | Yes | No | Yes |
| [Zep/Graphiti](https://github.com/getzep/graphiti) | No | Partial³ | Yes | Yes | No | **Yes (required)** |
| [Letta](https://github.com/letta-ai/letta) | No | No | Bolted-on | Yes | No | Yes |
| [Cognee](https://github.com/topoteretes/cognee) | Partial | No | Yes | Yes | No | Yes |

² Mem0g adds graph as secondary index, not source of truth.
³ Graphiti tracks episodes but mutates the graph in place.

**Graph databases** (structured relationships):

| Product | Embeddable | Event-sourced | Vectors | Offline sync | Write latency |
|---------|------------|---------------|---------|-------------|---------------|
| [Neo4j](https://neo4j.com/) | No (JVM) | No | Plugin | No | ~1-5ms |
| [FalkorDB](https://www.falkordb.com/) | No (Redis) | No | No | No | <1ms |
| TypeDB | No (JVM) | No | No | No | ~5-10ms |

### 5.2 The Unified Matrix

```
                Event-    Temporal   Embedded  Offline   No LLM    Vectors  Graph  Write
                Sourced   Queries    (no infra) Sync     on Write          Edges  Latency
                ────────  ────────   ────────  ────────  ────────  ───────  ─────  ───────
Pinecone        ✗         ✗          ✗         ✗         ✓         ✓        ✗      ~30ms
Qdrant          ✗         ✗          ✗         ✗         ✓         ✓        ✗      ~15ms
LanceDB         partial   ✗          ✓         ✗         ✓         ✓        ✗      <5ms
Chroma          ✗         ✗          ✓ (Py)    ✗         ✓         ✓        ✗      <10ms
Neo4j           ✗         ✗          ✗         ✗         ✓         plugin   ✓      ~3ms
Mem0            ✗         ✗          ✗         ✗         ✗         ✓        ✓      ~200ms
Graphiti        partial   ✓          ✗         ✗         ✗         ✓        ✓      ~1.5s
Letta           ✗         partial    ✗         ✗         ✗         ✓        ✗      ~500ms
─────────────────────────────────────────────────────────────────────────────────────────
Prime           ✓         ✓          ✓         ✓(CRDT)   ✓         ✓        ✓      <50μs
```

**Nobody occupies the center of the Venn diagram.** Every product is strong in one silo and absent from the others. Prime is the unified engine.

### 5.3 Why This Matters for Agents

An agent doing research needs all three in a single operation:

```
Agent reads a paper about CRDTs:

  1. VECTOR:  embed("doc-42", paper_abstract, embedding)
             → for later "find papers like this one"

  2. GRAPH:   add_node("paper", {title, authors, year})
              add_edge(paper → concept:crdt, "discusses")
              add_edge(paper → author:shapiro, "authored_by")
              → for structured traversal "who else writes about CRDTs?"

  3. EVENTS:  all of the above are immutable events
              → "when did the agent first learn about CRDTs?"
              → "show me everything the agent learned last Tuesday"
              → "what was the agent's knowledge state before it read this paper?"

With existing tools: 3 API calls to 3 databases, 3 failure modes, no transactional guarantees.
With Prime: one engine, one API, one data directory. All three are the same event stream.
```

### 5.4 Closest Competitor: LanceDB

[LanceDB](https://lancedb.com/) is the closest in spirit — embeddable, Rust core, columnar storage. The [existing comparison](../roadmaps/CHRONOS_VS_LANCEDB_COMPARISON.md) identified them as complementary. Prime changes that framing:

| Aspect | LanceDB | Prime |
|--------|---------|-------|
| Core strength | Vector search + multimodal | Unified vectors + graph + events |
| Storage format | Lance (columnar, versioned) | WAL + Parquet (event-sourced) |
| Graph support | None | Native (projections) |
| Time-travel | Table-level versioning | Entity-level audit trail |
| Offline sync | None | CRDT bidirectional |
| Write model | Mutable tables with versions | Append-only event log |
| Query model | SQL + vector search | Projection lookups + traversal + vector |

LanceDB is a better **vector database**. Prime is a better **agent memory engine**. If all you need is vector search, use LanceDB. If your agent needs to remember, relate, and recall with provenance — that's Prime.

---

## 6. API Design

### 6.1 Core API (Embedded Rust)

```rust
use allsource_prime::{Prime, Direction};

// Open — one line, one data directory
let prime = Prime::open("~/.agent/memory").await?;

// ─── Vectors ─────────────────────────────────────────────────

// Store embedding with source text
prime.embed("doc-42", "CRDTs enable conflict-free replication", embedding_vec).await?;

// Find similar
let results = prime.similar("doc-42", 10).await?;
// → [{id: "doc-17", score: 0.92, text: "..."}, ...]

// Direct vector search (no reference document needed)
let results = prime.vector_search(&query_embedding, 10).await?;

// ─── Graph ───────────────────────────────────────────────────

// Nodes
let alice = prime.add_node("person", json!({"name": "Alice", "role": "engineer"})).await?;
let crdt = prime.add_node("concept", json!({"name": "CRDT", "domain": "distributed-systems"})).await?;
let paper = prime.add_node("paper", json!({"title": "A comprehensive study of CRDTs"})).await?;

// Edges (directed, typed, with optional properties and weight)
prime.add_edge(&paper, &crdt, "discusses", Some(json!({"relevance": 0.95}))).await?;
prime.add_edge(&alice, &crdt, "expert_in", None).await?;

// Traversal
let neighbors = prime.neighbors(&alice, None, Direction::Outgoing).await?;
let path = prime.shortest_path(&alice, &paper, None).await?;
let subgraph = prime.subgraph(&crdt, 2).await?; // 2-hop ego network

// ─── Temporal ────────────────────────────────────────────────

// Full audit trail for any entity
let history = prime.history(&alice).await?;
// → [Created(t1, {name: "Alice"}), Updated(t2, {role: "engineer"}), ...]

// Time-travel: what did the graph look like last week?
let past_neighbors = prime.neighbors_as_of(&alice, None, last_week).await?;

// Diff: what changed between two points?
let diff = prime.diff(last_month, now).await?;
// → {nodes_added: 47, edges_added: 183, vectors_stored: 92}

// ─── Hybrid Recall ───────────────────────────────────────────

// The killer feature: combines all three
let context = prime.recall(RecallQuery {
    text: "distributed systems conflict resolution",  // semantic
    node_type: Some("concept"),                        // structural filter
    depth: 2,                                          // graph traversal
    top_k: 10,
    recency_weight: 0.3,                               // temporal boost
}).await?;
// → Ranked results combining vector similarity + graph proximity + recency

// ─── Sync ────────────────────────────────────────────────────

let stats = prime.sync("https://cloud.example.com").await?;
// → SyncStats { pushed: 142, pulled: 37, conflicts: 2 }

// ─── Lifecycle ───────────────────────────────────────────────

prime.compact("node:person:alice").await?;  // merge redundant data
prime.shutdown().await?;
```

### 6.2 MCP Tools

The primary agent integration path. Agents get these tools via MCP:

```
prime_embed         — Store a vector embedding with text and metadata
prime_similar       — Find semantically similar items
prime_add_node      — Create a graph node
prime_add_edge      — Create a relationship between nodes
prime_neighbors     — Query connected nodes (with depth, direction, relation filter)
prime_search        — Search nodes by type, label, or properties
prime_recall        — Hybrid query: semantic + structural + temporal
prime_history       — Full audit trail for any entity
prime_shortest_path — Find path between two nodes
prime_forget        — Soft-delete a node/edge (event-sourced, reversible)
```

**Agent MCP config** (zero infrastructure):
```json
{
  "mcpServers": {
    "memory": {
      "command": "allsource-prime",
      "args": ["--data-dir", "~/.agent/memory"]
    }
  }
}
```

That's it. No Docker, no API keys, no external databases. The agent has persistent, structured, searchable memory.

**Example agent session**:

```
User: "Research CRDTs for me"

Agent:
  → prime_add_node(type="concept", properties={"name": "CRDT", "full_name": "Conflict-free Replicated Data Type"})
  ← node:concept:crdt-a1b2

  → prime_embed(id="crdt-overview", text="CRDTs are data structures that can be replicated across...", vector=[...])
  ← stored

  → prime_add_node(type="paper", properties={"title": "A comprehensive study of CRDTs", "year": 2011, "authors": ["Shapiro et al."]})
  ← node:paper:paper-c3d4

  → prime_add_edge(source="node:paper:paper-c3d4", target="node:concept:crdt-a1b2", relation="defines")
  ← edge:e5f6

--- next day ---

User: "What do I know about distributed systems?"

Agent:
  → prime_recall(text="distributed systems", depth=2, top_k=10)
  ← {
      vectors: [{id: "crdt-overview", score: 0.89, text: "CRDTs are data structures..."}],
      nodes: [{id: "node:concept:crdt-a1b2", type: "concept", name: "CRDT", depth: 0},
              {id: "node:paper:paper-c3d4", type: "paper", title: "A comprehensive study...", depth: 1}],
      edges: [{source: "paper-c3d4", target: "crdt-a1b2", relation: "defines"}]
    }

Agent: "Based on my research, you know about CRDTs (Conflict-free Replicated Data Types).
        I found a key paper by Shapiro et al. (2011) that defines the concept.
        I learned about this yesterday during our research session."
        ^                                          ^
        structured recall (graph)                  temporal provenance (events)
```

### 6.3 HTTP API

```
POST   /api/v1/prime/vectors              Store embedding
POST   /api/v1/prime/vectors/search       Vector similarity search
DELETE /api/v1/prime/vectors/:id           Remove embedding

POST   /api/v1/prime/nodes                Create node
GET    /api/v1/prime/nodes/:id            Get node
PATCH  /api/v1/prime/nodes/:id            Update node properties
DELETE /api/v1/prime/nodes/:id            Soft-delete node

POST   /api/v1/prime/edges                Create edge
GET    /api/v1/prime/edges/:id            Get edge
DELETE /api/v1/prime/edges/:id            Soft-delete edge

GET    /api/v1/prime/nodes/:id/neighbors  Traverse neighbors
POST   /api/v1/prime/shortest-path        Find shortest path
GET    /api/v1/prime/nodes/:id/subgraph   Extract ego network

GET    /api/v1/prime/nodes/:id/history    Audit trail
GET    /api/v1/prime/diff                 Graph diff between timestamps

POST   /api/v1/prime/recall               Hybrid recall (semantic + graph + temporal)

POST   /api/v1/prime/sync/pull            Sync pull (delta exchange)
POST   /api/v1/prime/sync/push            Sync push (send events)

GET    /api/v1/prime/stats                Memory statistics
GET    /health                            Health check
```

---

## 7. Implementation

### 7.1 Crate Structure

Prime is a feature flag on `allsource-core`, not a separate crate. Same pattern as `embedded`, `embedded-sync`, etc.

```toml
# Cargo.toml
[features]
prime = ["embedded"]
prime-vectors = ["prime", "vector-search"]   # adds HNSW index
prime-full = ["prime", "prime-vectors"]       # everything
```

Source tree:

```
apps/core/src/
  prime/                    ← new module, gated by "prime" feature
    mod.rs                  ← Prime facade
    types.rs                ← Node, Edge, NodeId, VectorEntry, RecallQuery
    projections/
      mod.rs
      adjacency.rs          ← AdjacencyListProjection
      reverse_index.rs      ← ReverseIndexProjection
      node_state.rs         ← NodeStateProjection
      node_type_index.rs    ← NodeTypeIndexProjection
      vector_index.rs       ← VectorIndexProjection (HNSW)
      relevance.rs          ← RelevanceDecayProjection
      contradiction.rs      ← ContradictionDetectionProjection
      stats.rs              ← GraphStatsProjection
    traversal.rs            ← BFS, Dijkstra, subgraph extraction
    temporal.rs             ← history, as_of, diff
    recall.rs               ← hybrid recall (vector + graph + temporal)
    sync.rs                 ← graph-aware sync wrapper
  embedded/                 ← existing (Prime wraps this)
```

### 7.2 Prime Facade

```rust
pub struct Prime {
    core: EmbeddedCore,
    // Projections registered in open()
}

impl Prime {
    pub async fn open(path: impl AsRef<Path>) -> Result<Self> {
        let config = Config::builder()
            .data_dir(path)
            .merge_strategy("prime.node.created", MergeStrategy::FirstWriteWins)
            .merge_strategy("prime.node.updated", MergeStrategy::LastWriteWins)
            .merge_strategy("prime.edge.created", MergeStrategy::AppendOnly)
            .merge_strategy("prime.edge.deleted", MergeStrategy::LastWriteWins)
            .merge_strategy("prime.vector.stored", MergeStrategy::LastWriteWins)
            .build()?;

        let core = EmbeddedCore::open(config).await?;

        // Register graph + vector projections
        // (via core.inner() → ProjectionManager)

        Ok(Self { core })
    }

    // ... all API methods delegate to core.ingest() for writes
    // ... and projections for reads
}
```

### 7.3 MCP Binary

Thin binary that wraps Prime + exposes MCP tools over stdio:

```
apps/core/src/bin/allsource-prime.rs
  OR
apps/prime-mcp/                      ← small separate crate if needed
```

```rust
fn main() {
    let args = Args::parse();
    let prime = Prime::open(&args.data_dir).await?;

    McpServer::stdio()
        .tool("prime_embed", |p| prime.embed(...))
        .tool("prime_similar", |p| prime.similar(...))
        .tool("prime_add_node", |p| prime.add_node(...))
        .tool("prime_add_edge", |p| prime.add_edge(...))
        .tool("prime_neighbors", |p| prime.neighbors(...))
        .tool("prime_recall", |p| prime.recall(...))
        .tool("prime_history", |p| prime.history(...))
        .tool("prime_search", |p| prime.search(...))
        .tool("prime_forget", |p| prime.forget(...))
        .tool("prime_shortest_path", |p| prime.shortest_path(...))
        .serve()
        .await;
}
```

---

## 8. Roadmap

### M1: Graph Primitives + Traversal (v0.13.0)

| # | Feature | Detail |
|---|---------|--------|
| 1 | `Prime` facade | Wraps EmbeddedCore, registers projections |
| 2 | Node CRUD | `add_node`, `get_node`, `update_node`, `delete_node` |
| 3 | Edge CRUD | `add_edge`, `get_edge`, `delete_edge` |
| 4 | AdjacencyListProjection | O(1) outgoing neighbor lookup |
| 5 | ReverseIndexProjection | O(1) incoming neighbor lookup |
| 6 | NodeStateProjection | Current merged state per node |
| 7 | `neighbors()` | 1-hop with direction + relation filter |
| 8 | `neighbors_within(depth)` | BFS up to N hops |
| 9 | `shortest_path()` | BFS (unweighted), Dijkstra (weighted) |
| 10 | `subgraph(center, depth)` | Ego network extraction |

**Exit criteria**: 40+ tests, graph operations under 50μs.

### M2: Vectors + Hybrid Recall (v0.14.0)

| # | Feature | Detail |
|---|---------|--------|
| 1 | `embed()` | Store vector with optional source text |
| 2 | `similar()` | Cosine similarity search |
| 3 | `vector_search()` | Direct vector query |
| 4 | VectorIndexProjection | HNSW index maintained as projection |
| 5 | `recall()` | Hybrid: vector similarity + graph neighbors + recency |

**Exit criteria**: Vector search under 5ms for 100K vectors. Hybrid recall returns ranked results combining all three signals.

### M3: Temporal Queries (v0.14.0)

| # | Feature | Detail |
|---|---------|--------|
| 1 | `history(entity)` | Full audit trail |
| 2 | `neighbors_as_of(timestamp)` | Time-travel graph state |
| 3 | `diff(t1, t2)` | What changed between two times |
| 4 | `timeline(entity)` | Chronological event stream |

### M4: MCP Server + HTTP API (v0.15.0)

| # | Feature | Detail |
|---|---------|--------|
| 1 | `allsource-prime` binary | MCP server over stdio |
| 2 | 10 MCP tools | embed, similar, add_node, add_edge, neighbors, recall, history, search, forget, shortest_path |
| 3 | HTTP REST API | All operations over HTTP (port 3905) |
| 4 | Docker image | `ghcr.io/all-source-os/prime` |

**Exit criteria**: Claude Code can use Prime as an MCP server. Agent builds + queries a knowledge graph across sessions.

### M5: Agent Memory Features (v0.16.0)

| # | Feature | Detail |
|---|---------|--------|
| 1 | Contradiction detection | Detects conflicting edges, emits invalidation events |
| 2 | Relevance decay | Configurable scoring based on access patterns + recency |
| 3 | Memory compaction | Agent-triggered merge of redundant nodes |
| 4 | Conversation scoping | Associate mutations with conversation_id |
| 5 | Schema enforcement | JSON Schema validation per node/edge type |

### M6: Offline Sync + Batch Ops (v0.17.0)

| # | Feature | Detail |
|---|---------|--------|
| 1 | `sync(remote)` | High-level CRDT sync with graph-aware conflict reporting |
| 2 | Sync preview | Show what would change before applying |
| 3 | Batch import/export | Atomic bulk operations, GraphML/JSON export |
| 4 | Community detection | Leiden clustering as incremental projection |

---

## 9. Positioning

### One-liner

> **AllSource Prime**: The unified memory engine for AI agents — vectors, relationships, and events in one embedded binary.

### Elevator pitch

> Every agent memory system today glues together a vector DB, a graph DB, and maybe an event log — three databases, three APIs, three failure modes. Prime is one Rust engine that does all three. Vectors for semantic recall. A graph for structured relationships. An immutable event log for full provenance. Time-travel to see your agent's knowledge at any point. Runs embedded with zero infrastructure. Syncs offline-first with CRDT conflict resolution. 50-microsecond writes, not 2-second LLM round-trips.

### Comparison pitch

> "Chroma for vectors. Neo4j for graphs. Neither for history. Prime for all three."

### Developer pitch

> ```
> brew install allsource-prime
> # Add to your agent's MCP config. Done.
> # No Docker. No API keys. No Pinecone bill. No Neo4j instance.
> # Your agent remembers everything. Forever. With receipts.
> ```

---

## 10. Decision Record

| Decision | Rationale |
|----------|-----------|
| Feature flag in `allsource-core`, not separate crate | Needs direct access to EventStore, Projection trait, ProjectionManager. Same pattern as embedded modules. |
| `prime.` event namespace | Clear separation from user events and the existing `graph.` proposal. Single namespace for the unified product. |
| Projections for ALL indexes (vector, graph, stats) | Unified update mechanism. One event ingestion path updates all views. No separate indexing pipeline. |
| No LLM on write path | Agents are the callers — they decide what to remember. Prime stores exactly what's sent. Fast, deterministic, cheap. |
| MCP as primary agent interface | Matches how agents integrate with tools today. Zero-config: one binary, one data directory. |
| Hybrid recall as a first-class operation | This is the killer feature. No competitor combines vector similarity + graph traversal + temporal recency in one query. |
| Graph superseded by Prime | The KG proposal was a graph-only product. Prime subsumes it — graph is one capability, not the whole product. |
| Name "Prime" | Mathematically resonant (prime vectors, prime numbers). Communicates "foundational, first, irreducible." Works as a standalone brand: AllSource Prime. |

---

## 11. What's NOT Included

- **Custom query language** — Rust builder API + MCP tools + REST cover the use cases
- **Multi-modal embeddings** (images, audio) — store any vector, but no built-in encoder. LanceDB is better here.
- **LLM-powered entity extraction** — explicitly avoided. The agent decides what to store.
- **Graph visualization** — export to tools that do this well (Gephi, D3, etc.)
- **Training/fine-tuning** — Prime is a memory engine, not an ML framework
- **Multi-leader sync** — single-leader CRDT first; multi-leader adds complexity
