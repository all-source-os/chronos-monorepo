# AllSource Knowledge Graph Service

> **Status**: Superseded by [AllSource Prime](ALLSOURCE_PRIME.md)
> **Author**: Design session 2026-03-01
> **Scope**: New service — event-sourced knowledge graph built on AllSource Core
> **Depends on**: [Embedded Core](EMBEDDED_CORE_AND_OFFLINE_FIRST.md), [Server-Side Projections](SERVER_SIDE_PROJECTIONS.md), [Core Replication](CORE_REPLICATION_DESIGN.md)
> **Note**: The knowledge graph capability described here is subsumed by AllSource Prime, which unifies vectors + graph + events into a single engine. This document remains as reference for the graph-specific design decisions.

---

## 1. Motivation

Knowledge graphs are the connective tissue behind RAG pipelines, agent memory, entity resolution, compliance audit trails, and recommendation engines. Today, teams choose between:

| Option | Trade-off |
|--------|-----------|
| Neo4j / Dgraph | Powerful traversal, but no temporal history, no event sourcing, expensive at scale |
| PostgreSQL + ltree / recursive CTEs | Bolted-on graph queries, poor traversal performance beyond 2-3 hops |
| Custom in-memory graph | Fast, but no durability, no sync, no audit trail |
| Vector DBs (Pinecone, Weaviate) | Semantic similarity only — no structured relationships |

None of them give you **temporal graphs** (what did the graph look like last Tuesday?), **event-sourced audit trails** (who added this edge and why?), **offline-first sync** (build knowledge locally, merge to cloud), or **sub-millisecond projection queries** backed by durable storage.

AllSource Core already has every primitive needed:

```
Events         → graph mutations (node created, edge added, property updated)
Projections    → materialized graph views (adjacency lists, reachability, PageRank)
HLC + CRDT     → distributed graph construction with automatic conflict resolution
WAL + Parquet  → full durability with columnar analytics
EmbeddedCore   → in-process graph for desktop apps and AI agents
Schemas        → enforce node/edge structure
Snapshots      → fast recovery of large subgraphs
```

**What's missing is the graph-aware layer on top** — a service that speaks nodes, edges, and traversals instead of raw events.

---

## 2. Design Goals

1. **Graph-native API** — create/query nodes and edges without thinking about events
2. **Temporal by default** — every query can include `as_of` for time-travel
3. **Event-sourced provenance** — every node and edge has a full audit trail
4. **Embeddable** — runs as a Rust library (no server) for desktop apps and AI agents
5. **Syncable** — offline-first graph construction with CRDT merge to cloud
6. **Schema-enforced** — node types and edge types are validated on ingest
7. **AI-native** — MCP tools for agent-driven graph construction, vector embeddings for semantic edges
8. **Zero new storage engines** — AllSource Core IS the database; this service is a domain layer

---

## 3. Architecture

### 3.1 Deployment Modes

```
Mode A: Embedded Library (AI agents, desktop apps, CLI tools)
┌──────────────────────────────────────┐
│  Your Application                    │
│  ┌────────────────────────────────┐  │
│  │  KnowledgeGraph (facade)       │  │
│  │  ┌──────────┐ ┌─────────────┐ │  │
│  │  │ GraphOps │ │ Projections │ │  │
│  │  └────┬─────┘ └──────┬──────┘ │  │
│  │       └──────┬───────┘        │  │
│  │        EmbeddedCore           │  │
│  │   ┌───────┬────────┬───────┐  │  │
│  │   │DashMap│  WAL   │Parquet│  │  │
│  │   └───────┴────────┴───────┘  │  │
│  └────────────────────────────────┘  │
└──────────────────────────────────────┘

Mode B: Standalone Service (HTTP API, multi-tenant)
┌──────────────────────────────────────┐
│  KG Service (Axum, port 3905)        │
│  ┌──────────┐ ┌──────────────────┐   │
│  │ Graph API│ │ Graph Projections│   │
│  └────┬─────┘ └────────┬────────┘   │
│       └───────┬────────┘             │
│         AllSource Core               │
│   ┌───────┬────────┬───────┐         │
│   │DashMap│  WAL   │Parquet│         │
│   └───────┴────────┴───────┘         │
└──────────────────────────────────────┘
        │
        │ HTTP sync (pull/push)
        ▼
┌──────────────────────────────────────┐
│  AllSource Cloud (existing Core)     │
└──────────────────────────────────────┘

Mode C: Behind Query Service (managed, multi-tenant SaaS)
Clients → Query Service (auth, billing) → KG Service → Core
```

### 3.2 Component Diagram

```
┌─────────────────────────────────────────────────────────────┐
│                    KnowledgeGraph Facade                     │
├──────────┬──────────┬────────────┬──────────┬───────────────┤
│ NodeOps  │ EdgeOps  │ TraversalQ │ TemporalQ│ SemanticSearch│
│          │          │            │          │               │
│ add_node │ add_edge │ neighbors  │ as_of    │ similar_to    │
│ get_node │ get_edge │ shortest   │ history  │ embed_node    │
│ update   │ remove   │ subgraph   │ diff     │ vector_search │
│ delete   │ list     │ connected  │ timeline │               │
└────┬─────┴────┬─────┴─────┬──────┴────┬─────┴───────┬───────┘
     │          │            │           │             │
┌────▼──────────▼────────────▼───────────▼─────────────▼───────┐
│                     Graph Projections                         │
├────────────────┬──────────────┬───────────────┬──────────────┤
│ AdjacencyList  │ ReverseIndex │ NodeTypeIndex │ PathCache    │
│                │              │               │              │
│ entity →       │ target →     │ "person" →    │ A→B →        │
│ [(pred,target)]│ [(pred,src)] │ [entity_ids]  │ [A,C,D,B]   │
└────────────────┴──────────────┴───────────────┴──────────────┘
     │                    │                │
┌────▼────────────────────▼────────────────▼───────────────────┐
│                     EmbeddedCore / EventStore                 │
│  Events: node.created, node.updated, edge.created, etc.      │
│  WAL + Parquet + DashMap + HLC + CRDT                        │
└──────────────────────────────────────────────────────────────┘
```

---

## 4. Data Model

### 4.1 Event Conventions

All graph mutations are modeled as AllSource events. The KG service translates graph operations into these events and reads graph state from projections.

**Namespace**: `graph.` prefix for all KG events.

| Event Type | Entity ID | Payload | Meaning |
|------------|-----------|---------|---------|
| `graph.node.created` | `node:{type}:{id}` | `{type, properties, labels}` | New node |
| `graph.node.updated` | `node:{type}:{id}` | `{properties}` | Merge properties |
| `graph.node.deleted` | `node:{type}:{id}` | `{reason?}` | Soft-delete node |
| `graph.edge.created` | `edge:{id}` | `{source, target, relation, properties, weight?}` | New directed edge |
| `graph.edge.updated` | `edge:{id}` | `{properties}` | Update edge properties |
| `graph.edge.deleted` | `edge:{id}` | `{reason?}` | Soft-delete edge |
| `graph.schema.registered` | `schema:{type}` | `{node_schema?, edge_schema?}` | Register type schema |
| `graph.batch.imported` | `batch:{id}` | `{node_count, edge_count, source}` | Batch import metadata |

**Entity ID format**: `node:{type}:{id}` or `edge:{uuid}`. The type prefix enables efficient queries by node type via `event_type_prefix`.

### 4.2 Node

```rust
pub struct Node {
    pub id: NodeId,                        // "node:person:alice-123"
    pub node_type: String,                 // "person"
    pub labels: Vec<String>,               // ["employee", "engineer"]
    pub properties: serde_json::Value,     // {"name": "Alice", "age": 30}
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub version: u64,                      // event count for this node
}
```

### 4.3 Edge

```rust
pub struct Edge {
    pub id: EdgeId,                        // "edge:rel-uuid"
    pub source: NodeId,                    // "node:person:alice"
    pub target: NodeId,                    // "node:person:bob"
    pub relation: String,                  // "knows"
    pub properties: serde_json::Value,     // {"since": "2020-01-01", "strength": 0.9}
    pub weight: Option<f64>,               // optional numeric weight for traversals
    pub directed: bool,                    // default true
    pub created_at: DateTime<Utc>,
}
```

### 4.4 CRDT Merge Strategies for Graph Events

| Event Pattern | Strategy | Rationale |
|--------------|----------|-----------|
| `graph.node.created` | `FirstWriteWins` | Same node created on two offline devices — keep first, skip duplicate |
| `graph.node.updated` | `LastWriteWins` | Property updates — latest wins |
| `graph.node.deleted` | `LastWriteWins` | Delete should win over concurrent update |
| `graph.edge.created` | `AppendOnly` | Edges are additive — "Alice knows Bob" from two sources is still valid |
| `graph.edge.deleted` | `LastWriteWins` | Explicit deletion wins |

These are configured via the `MergeStrategy` system from v0.12.0:

```rust
Config::builder()
    .merge_strategy("graph.node.created", MergeStrategy::FirstWriteWins)
    .merge_strategy("graph.node.updated", MergeStrategy::LastWriteWins)
    .merge_strategy("graph.node.deleted", MergeStrategy::LastWriteWins)
    .merge_strategy("graph.edge.created", MergeStrategy::AppendOnly)
    .merge_strategy("graph.edge.deleted", MergeStrategy::LastWriteWins)
    .build()?
```

---

## 5. Graph Projections

The KG service registers custom projections that maintain materialized graph views. These update in real-time as events arrive — no separate indexing step.

### 5.1 AdjacencyListProjection

Maintains outgoing edges per node. O(1) lookup for "who does Alice know?"

```rust
pub struct AdjacencyListProjection {
    // node_id → Vec<(relation, target_id, edge_id, weight)>
    adjacency: DashMap<String, Vec<AdjacencyEntry>>,
}

impl Projection for AdjacencyListProjection {
    fn name(&self) -> &str { "graph_adjacency" }

    fn process(&self, event: &Event) -> Result<()> {
        match event.event_type.as_str() {
            "graph.edge.created" => {
                let source = event.payload["source"].as_str()?;
                let target = event.payload["target"].as_str()?;
                let relation = event.payload["relation"].as_str()?;
                self.adjacency.entry(source.to_string())
                    .or_default()
                    .push(AdjacencyEntry { relation, target, edge_id, weight });
            }
            "graph.edge.deleted" => { /* remove from adjacency list */ }
            _ => {}
        }
        Ok(())
    }

    fn get_state(&self, entity_id: &str) -> Option<Value> {
        self.adjacency.get(entity_id).map(|edges| json!({
            "outgoing": edges.value()
        }))
    }
}
```

### 5.2 ReverseIndexProjection

Maintains incoming edges per node. O(1) lookup for "who knows Alice?"

```rust
// target_id → Vec<(relation, source_id, edge_id)>
```

### 5.3 NodeTypeIndexProjection

Maintains a set of node IDs per type. O(1) lookup for "all person nodes."

```rust
// node_type → HashSet<NodeId>
```

### 5.4 NodeStateProjection

Maintains current merged state of each node (folded from create + update events).

```rust
// node_id → { type, labels, properties, version, deleted }
```

### 5.5 GraphStatsProjection

Maintains aggregate counts: total nodes, total edges, nodes per type, edges per relation type.

```rust
// "stats" → { node_count, edge_count, types: { person: 42, company: 7 }, relations: { knows: 128 } }
```

---

## 6. API Design

### 6.1 Embedded Rust API

```rust
use allsource_core::kg::{KnowledgeGraph, Node, Edge, TraversalQuery};

// Open with graph-optimized config
let kg = KnowledgeGraph::open(Config::builder()
    .data_dir("~/.myapp/knowledge")
    .merge_strategy("graph.node.created", MergeStrategy::FirstWriteWins)
    .merge_strategy("graph.node.updated", MergeStrategy::LastWriteWins)
    .merge_strategy("graph.edge.created", MergeStrategy::AppendOnly)
    .build()?
).await?;

// --- Node operations ---

let alice = kg.add_node("person", json!({
    "name": "Alice Chen",
    "role": "engineer",
    "team": "platform"
})).await?;
// Returns: NodeId("node:person:a1b2c3")

let bob = kg.add_node("person", json!({
    "name": "Bob Park",
    "role": "designer"
})).await?;

// Update node properties (merge, not replace)
kg.update_node(&alice, json!({"level": "senior"})).await?;

// Get current node state (from projection, ~12μs)
let node = kg.get_node(&alice).await?;
assert_eq!(node.properties["level"], "senior");

// --- Edge operations ---

kg.add_edge(&alice, &bob, "works_with", json!({
    "project": "knowledge-graph",
    "since": "2026-01"
})).await?;

kg.add_edge(&alice, &bob, "mentors", json!({
    "topic": "rust"
})).await?;

// --- Traversal queries ---

// Direct neighbors
let neighbors = kg.neighbors(&alice, None).await?;
// [{ node: bob, relation: "works_with" }, { node: bob, relation: "mentors" }]

// Filtered by relation type
let mentees = kg.neighbors(&alice, Some("mentors")).await?;

// Incoming edges (reverse traversal)
let mentors_of_bob = kg.incoming(&bob, Some("mentors")).await?;

// Multi-hop: 2-degree connections
let two_hops = kg.neighbors_within(&alice, 2, None).await?;

// Shortest path
let path = kg.shortest_path(&alice, &target, None).await?;
// Some(Path { nodes: [alice, charlie, target], edges: [...], hops: 2 })

// Subgraph extraction
let subgraph = kg.subgraph(&alice, 3).await?;  // 3-hop ego graph
// Subgraph { nodes: [...], edges: [...] }

// --- Temporal queries ---

// Graph state at a past time
let past_neighbors = kg.neighbors_as_of(&alice, None, "2026-01-15T00:00:00Z").await?;

// Node history (full audit trail)
let history = kg.node_history(&alice).await?;
// [{ event: "created", timestamp: ..., properties: {...} },
//  { event: "updated", timestamp: ..., changes: {"level": "senior"} }]

// Diff between two points in time
let diff = kg.diff("2026-01-01T00:00:00Z", "2026-03-01T00:00:00Z").await?;
// GraphDiff { nodes_added: 47, nodes_removed: 2, edges_added: 183, ... }

// --- Schema enforcement ---

kg.register_node_schema("person", json_schema!({
    "required": ["name"],
    "properties": {
        "name": { "type": "string" },
        "role": { "type": "string" },
        "level": { "type": "string", "enum": ["junior", "mid", "senior", "staff"] }
    }
})).await?;

// This would fail validation:
// kg.add_node("person", json!({"role": "engineer"})).await?;
// Error: missing required property "name"

// --- Sync (offline → cloud) ---

let cloud = SyncClient::new("https://cloud.allsource.xyz", kg.node_id());
let stats = cloud.sync(kg.core()).await?;
// SyncStats { pushed: 42 nodes + 128 edges, pulled: 7, conflicts: 0 }
```

### 6.2 HTTP API (Standalone Service)

When running as a service on port 3905:

```
POST   /api/v1/graph/nodes                    Create node
GET    /api/v1/graph/nodes/:id                Get node (current state)
PATCH  /api/v1/graph/nodes/:id                Update node properties
DELETE /api/v1/graph/nodes/:id                Soft-delete node

POST   /api/v1/graph/edges                    Create edge
GET    /api/v1/graph/edges/:id                Get edge
DELETE /api/v1/graph/edges/:id                Soft-delete edge

GET    /api/v1/graph/nodes/:id/neighbors      Outgoing neighbors
GET    /api/v1/graph/nodes/:id/incoming        Incoming neighbors
GET    /api/v1/graph/nodes/:id/subgraph?depth=N   Ego subgraph
POST   /api/v1/graph/shortest-path            Shortest path between two nodes

GET    /api/v1/graph/nodes/:id/history         Audit trail
GET    /api/v1/graph/query?as_of=<ISO8601>     Time-travel query

POST   /api/v1/graph/batch                    Bulk import nodes + edges
GET    /api/v1/graph/stats                    Graph statistics

POST   /api/v1/graph/schemas                  Register node/edge type schema
GET    /api/v1/graph/schemas                  List schemas
```

**Example: Create node**

```bash
curl -X POST http://localhost:3905/api/v1/graph/nodes \
  -H "Content-Type: application/json" \
  -d '{
    "type": "concept",
    "properties": {
      "name": "Event Sourcing",
      "description": "Append-only log of state changes",
      "domain": "software-architecture"
    },
    "labels": ["pattern", "architecture"]
  }'
```

```json
{
  "id": "node:concept:evt-src-a1b2",
  "type": "concept",
  "properties": { "name": "Event Sourcing", ... },
  "created_at": "2026-03-01T10:00:00Z"
}
```

**Example: Traverse neighbors**

```bash
curl "http://localhost:3905/api/v1/graph/nodes/node:concept:evt-src-a1b2/neighbors?relation=related_to&depth=2"
```

```json
{
  "nodes": [
    { "id": "node:concept:cqrs", "type": "concept", "relation": "related_to", "depth": 1 },
    { "id": "node:concept:saga", "type": "concept", "relation": "related_to", "depth": 2 }
  ],
  "edges": [ ... ],
  "total_nodes": 2,
  "traversal_depth": 2
}
```

### 6.3 MCP Tools (AI Agent Interface)

The KG service exposes MCP tools so AI agents can build and query knowledge graphs natively:

```json
[
  {
    "name": "kg_add_node",
    "description": "Add a node to the knowledge graph",
    "inputSchema": {
      "type": "object",
      "properties": {
        "type": { "type": "string", "description": "Node type (e.g., 'concept', 'person', 'document')" },
        "properties": { "type": "object", "description": "Node properties" },
        "labels": { "type": "array", "items": { "type": "string" } }
      },
      "required": ["type", "properties"]
    }
  },
  {
    "name": "kg_add_edge",
    "description": "Create a relationship between two nodes",
    "inputSchema": {
      "type": "object",
      "properties": {
        "source": { "type": "string" },
        "target": { "type": "string" },
        "relation": { "type": "string", "description": "Relationship type (e.g., 'mentions', 'depends_on', 'authored_by')" },
        "properties": { "type": "object" },
        "weight": { "type": "number" }
      },
      "required": ["source", "target", "relation"]
    }
  },
  {
    "name": "kg_query_neighbors",
    "description": "Find nodes connected to a given node",
    "inputSchema": {
      "type": "object",
      "properties": {
        "node_id": { "type": "string" },
        "relation": { "type": "string", "description": "Filter by relation type" },
        "depth": { "type": "integer", "default": 1, "maximum": 5 },
        "direction": { "type": "string", "enum": ["outgoing", "incoming", "both"] }
      },
      "required": ["node_id"]
    }
  },
  {
    "name": "kg_search_nodes",
    "description": "Search nodes by type, label, or property values",
    "inputSchema": {
      "type": "object",
      "properties": {
        "type": { "type": "string" },
        "label": { "type": "string" },
        "property_filter": { "type": "object" },
        "limit": { "type": "integer", "default": 20 }
      }
    }
  },
  {
    "name": "kg_shortest_path",
    "description": "Find the shortest path between two nodes"
  },
  {
    "name": "kg_node_history",
    "description": "Get the full audit trail for a node"
  }
]
```

**Agent workflow example — research assistant building a knowledge graph:**

```
Agent reads paper → kg_add_node(type="paper", properties={title, authors, year})
                  → kg_add_node(type="concept", properties={name: "CRDT"})
                  → kg_add_edge(source=paper, target=concept, relation="discusses")
                  → kg_add_edge(source=paper, target=other_paper, relation="cites")

Agent answers question → kg_query_neighbors(node="concept:crdt", depth=2)
                       → Uses subgraph context for grounded response
```

---

## 7. Use Cases

### UC-1: AI Agent Memory — Persistent Relationship Context

**Actor**: AI agent (Claude Code, custom agent framework)
**Problem**: Agents lose context between sessions. Vector search finds similar text but not structured relationships ("Alice manages Bob who works on Project X").

**Solution**: Agent builds a knowledge graph during each session. On next session, queries the graph for structured context.

```
Session 1:
  Agent learns "Alice manages the platform team"
  → kg_add_node(person, "Alice"), kg_add_node(team, "Platform")
  → kg_add_edge(Alice, Platform, "manages")

Session 2:
  User asks "Who should I talk to about the platform?"
  → kg_query_neighbors("node:team:platform", relation="manages", direction="incoming")
  → Returns Alice with full context
```

**Why AllSource vs. alternatives**:
- Vector DB would need "who manages platform" to be semantically close to the stored text — brittle
- Relational DB has no built-in traversal, no temporal history
- AllSource KG: structured traversal + audit trail + offline sync

### UC-2: Compliance Knowledge Graph — Regulatory Entity Tracking

**Actor**: Compliance team at a financial institution
**Problem**: Track relationships between entities (companies, individuals, regulations, obligations) with full audit trail. Regulators ask "when did you first know about this relationship?"

**Solution**: Event-sourced graph where every node and edge has provenance.

```
kg.add_node("company", json!({"name": "Acme Corp", "jurisdiction": "US"}))
kg.add_node("regulation", json!({"name": "SOX Section 404", "authority": "SEC"}))
kg.add_edge(acme, sox, "subject_to", json!({"effective_date": "2026-01-01", "assessor": "jane@compliance.com"}))

// Auditor asks: "When was Acme first linked to SOX compliance?"
let history = kg.node_history(&acme).await?;
// Full provenance: who created the edge, when, from what source
```

**Temporal queries**:
```
// "What did the compliance graph look like before the merger?"
let pre_merger = kg.neighbors_as_of(&acme, None, "2025-06-01T00:00:00Z").await?;
```

### UC-3: Codebase Knowledge Graph — Architecture Understanding

**Actor**: Developer tools, code analysis pipelines
**Problem**: Understanding large codebases requires knowing relationships between modules, functions, types, and dependencies. Static analysis tools produce snapshots but lose history.

**Solution**: Build a knowledge graph from code analysis, updated on each commit.

```
CI pipeline on each commit:
  → Parse AST
  → kg_add_node(type="module", properties={path: "src/store.rs", language: "rust"})
  → kg_add_node(type="function", properties={name: "ingest", module: "store"})
  → kg_add_edge(function, module, "defined_in")
  → kg_add_edge(function_a, function_b, "calls")
  → kg_add_edge(module_a, module_b, "depends_on")

Developer asks: "What would break if I change EventStore::query?"
  → kg_query_neighbors("node:function:EventStore::query", relation="calls", direction="incoming", depth=3)
  → Returns full call chain with 3 levels of callers

Architect asks: "How has the module dependency graph changed this quarter?"
  → kg.diff("2026-01-01", "2026-03-01")
  → Shows 12 new modules, 47 new edges, 3 circular dependencies introduced
```

### UC-4: Research Literature Graph — Paper Connections

**Actor**: Research teams, academics, R&D departments
**Problem**: Understanding the landscape of a research field means tracking papers, authors, concepts, citations, and how they evolve over time.

**Solution**: Agents process papers and build a citation + concept graph.

```
Agent processes PDF:
  → kg_add_node(type="paper", properties={title, doi, year, abstract})
  → kg_add_node(type="author", properties={name, affiliation})
  → kg_add_edge(author, paper, "authored")
  → kg_add_edge(paper, cited_paper, "cites")
  → kg_add_node(type="concept", properties={name: "knowledge graph"})
  → kg_add_edge(paper, concept, "discusses", {relevance: 0.9})

Researcher queries:
  "What concepts bridge these two research areas?"
  → kg.shortest_path("node:concept:event-sourcing", "node:concept:knowledge-graphs")
  → Path through shared papers and intermediate concepts

  "Who are the key authors in CRDT research?"
  → kg.neighbors("node:concept:crdt", relation="discusses", direction="incoming")  // papers
  → kg.neighbors(each_paper, relation="authored", direction="incoming")             // authors
  → Rank by edge count
```

### UC-5: Distributed Team Knowledge — Offline-First Collaboration

**Actor**: Field teams, distributed offices, air-gapped environments
**Problem**: Multiple teams build knowledge graphs independently (e.g., field intelligence, customer relationships, asset inventories). Need to merge without conflicts when reconnected.

**Solution**: Each team runs an embedded KG instance. CRDT sync merges graphs automatically.

```
Field Team A (offline for 2 weeks):
  → Builds 500 nodes, 1200 edges about local assets

Field Team B (different region, also offline):
  → Builds 300 nodes, 800 edges

Reconnection:
  let stats = cloud_sync.sync(team_a.core()).await?;
  // SyncStats { pushed: 1700 events, pulled: 1100, conflicts: 3 }
  // Conflicts auto-resolved:
  //   - Same node created by both → FirstWriteWins (keep first, skip dupe)
  //   - Same node updated by both → LastWriteWins (latest properties win)
  //   - Same edge from both → AppendOnly (both kept, deduplicated by content)
```

### UC-6: Product Recommendation Engine — User-Item-Feature Graph

**Actor**: E-commerce platform, content recommendation system
**Problem**: Recommendations need structured relationships (user → purchased → product → has_feature → category) not just collaborative filtering vectors.

**Solution**: Model users, products, features, and interactions as a knowledge graph.

```
// User behavior events become graph edges
kg.add_edge(user, product, "viewed", json!({"duration_sec": 45}))
kg.add_edge(user, product, "purchased", json!({"price": 29.99}))
kg.add_edge(product, feature, "has_feature", json!({"value": "waterproof"}))

// Recommendation query: "Products similar to what Alice bought"
let purchased = kg.neighbors(&alice, Some("purchased")).await?;
for product in purchased {
    let features = kg.neighbors(&product.id, Some("has_feature")).await?;
    let similar = kg.neighbors_with_filter(&features, "has_feature", direction="incoming", exclude=purchased).await?;
}

// Temporal: "What was trending last week?"
let last_week_edges = kg.query_edges_between(
    "2026-02-22T00:00:00Z",
    "2026-03-01T00:00:00Z",
    Some("purchased")
).await?;
```

---

## 8. Implementation Phases

> **Full roadmap with competitive feature tagging**: [KNOWLEDGE_GRAPH_ROADMAP.md](../roadmaps/KNOWLEDGE_GRAPH_ROADMAP.md)
>
> The phases below cover the core proposal. The roadmap extends these with additional milestones for contradiction detection, community detection, agent memory primitives, and offline sync — features identified through competitive analysis.

### Phase 1: Core Graph Primitives — P0

**Scope**: Node/edge CRUD, basic projections, embedded API only.

| Deliverable | Detail |
|-------------|--------|
| `KnowledgeGraph` facade | Wraps `EmbeddedCore`, registers graph projections |
| `NodeOps` | `add_node`, `get_node`, `update_node`, `delete_node` |
| `EdgeOps` | `add_edge`, `get_edge`, `delete_edge`, `list_edges` |
| `AdjacencyListProjection` | Outgoing edge index |
| `ReverseIndexProjection` | Incoming edge index |
| `NodeStateProjection` | Current merged node state |
| `NodeTypeIndexProjection` | Nodes grouped by type |
| Event conventions | `graph.node.*`, `graph.edge.*` namespaces |
| Tests | CRUD, projection consistency, concurrent access |

**New files**:
```
apps/core/src/kg/mod.rs
apps/core/src/kg/graph.rs           — KnowledgeGraph facade
apps/core/src/kg/types.rs           — Node, Edge, NodeId, EdgeId
apps/core/src/kg/projections.rs     — Graph-specific projections
apps/core/tests/knowledge_graph.rs  — Integration tests
```

**Feature flag**: `knowledge-graph` (depends on `embedded`)

### Phase 2: Traversal Queries — P0

**Scope**: Multi-hop traversal, shortest path, subgraph extraction.

| Deliverable | Detail |
|-------------|--------|
| `neighbors()` | 1-hop with optional relation filter |
| `neighbors_within(depth)` | BFS up to N hops |
| `shortest_path()` | Unweighted BFS, weighted Dijkstra |
| `subgraph(center, depth)` | Ego network extraction |
| `connected_components()` | Find isolated subgraphs |
| `PathCacheProjection` | Optional projection caching frequent paths |

### Phase 3: Temporal Graph Queries — P1

**Scope**: Time-travel, history, diff.

| Deliverable | Detail |
|-------------|--------|
| `neighbors_as_of(timestamp)` | Graph state at past time |
| `node_history(id)` | Full audit trail of mutations |
| `diff(t1, t2)` | GraphDiff: added/removed nodes and edges |
| `timeline(id)` | Chronological event stream for entity |

### Phase 4: HTTP API + MCP Tools — P1

**Scope**: Standalone service mode, AI agent integration.

| Deliverable | Detail |
|-------------|--------|
| Axum HTTP server | Port 3905, REST API for all graph operations |
| MCP tool definitions | 6 tools for agent-driven graph construction |
| `McpToolTracker` integration | Auto-emit `mcp.tool.*` events for KG operations |
| Docker image | `ghcr.io/all-source-os/chronos-kg` |
| OpenAPI spec | Full API documentation |

### Phase 5: Schema Enforcement — P2

**Scope**: Validate node/edge types against registered JSON Schemas.

| Deliverable | Detail |
|-------------|--------|
| `register_node_schema(type, schema)` | Define valid properties per node type |
| `register_edge_schema(relation, schema)` | Define valid properties per edge type |
| Validation on ingest | Reject malformed nodes/edges at creation time |
| Schema evolution | Compatibility modes (backward, forward, full) |

### Phase 6: Semantic Search — P2

**Scope**: Vector embeddings for similarity-based graph queries.

| Deliverable | Detail |
|-------------|--------|
| `embed_node(id, vector)` | Attach embedding to node |
| `similar_to(id, top_k)` | Find semantically similar nodes |
| `vector_search(vector, top_k)` | Direct vector query |
| Integration with `EmbeddingVector` | Use existing Core vector support |

### Phase 7: Batch Import + Export — P2

**Scope**: Bulk operations for large graph construction.

| Deliverable | Detail |
|-------------|--------|
| `import_batch(nodes, edges)` | Atomic bulk import |
| `export_graphml(query)` | Export subgraph as GraphML |
| `export_cypher(query)` | Export as Cypher CREATE statements |
| CSV/JSON import | Common interchange formats |

---

## 9. Performance Targets

| Operation | Target | Mechanism |
|-----------|--------|-----------|
| Add node | < 50μs | Single event ingest + projection update |
| Add edge | < 50μs | Single event ingest + 2 projection updates |
| Get node (current state) | < 15μs | Projection lookup (DashMap) |
| Neighbors (1-hop) | < 20μs | Adjacency projection lookup |
| Neighbors (3-hop, 1000 nodes) | < 5ms | BFS over projections |
| Shortest path (10K node graph) | < 50ms | Dijkstra over projections |
| Time-travel query | < 100ms | Event replay with `as_of` filter |
| Batch import (10K nodes + 50K edges) | < 2s | Batch ingest + projection rebuild |
| Sync (1000 new events) | < 500ms | HTTP pull/push + CRDT resolution |

---

## 10. Competitive Analysis

The AI agent memory space is active and well-funded. Understanding what exists — and what's missing — positions AllSource KG precisely.

### 10.1 Direct Competitors

#### Zep / Graphiti

The closest competitor architecturally. [Graphiti](https://github.com/getzep/graphiti) is a Python framework for temporal knowledge graphs purpose-built for agent memory. [Paper: arXiv 2501.13956](https://arxiv.org/abs/2501.13956).

| Aspect | Detail |
|--------|--------|
| Language | Python 3.10+ |
| Storage | Requires Neo4j, FalkorDB, Kuzu, or Neptune as separate backend |
| Temporal model | Bi-temporal (event time + ingestion time) |
| Key strength | Edge invalidation for contradictions ("Alice works at X" supersedes "Alice works at Y") |
| Key weakness | **Requires an LLM call on every write** (entity extraction, relationship resolution). OpenAI API key is a hard dependency. Writes are ~1-2s, expensive, non-deterministic |
| Embeddable | No — requires a running graph database |
| Offline sync | No |

#### Mem0

[Mem0](https://github.com/mem0ai/mem0) raised $24M Series A (Oct 2025), chosen as AWS's exclusive memory provider. [Paper: arXiv 2504.19413](https://arxiv.org/abs/2504.19413).

| Aspect | Detail |
|--------|--------|
| Language | Python |
| Storage | Vector DB (Qdrant/Pinecone/pgvector) + graph backend (Neo4j/Memgraph) — two separate systems |
| Graph memory (Mem0g) | Extracts entities/relationships from conversations, stores as directed labeled graph |
| Key strength | Managed SaaS, SOC 2 / HIPAA, 91% lower p95 latency vs raw context stuffing |
| Key weakness | **Not event-sourced** — memories are mutable objects, no audit trail, no time-travel. Graph is a secondary index, not the source of truth |
| Embeddable | No — cloud-first |
| Offline sync | No |

#### Letta (MemGPT)

[Letta](https://github.com/letta-ai/letta) pioneered the OS-inspired memory hierarchy for agents.

| Aspect | Detail |
|--------|--------|
| Language | Python |
| Architecture | Core memory (in-context, self-edited) + recall memory (searchable history) + archival memory (vector/graph) |
| Key strength | Agent self-manages its own memory via tool calls. "Context Repositories" with git-based versioning (Feb 2026) |
| Key weakness | **Knowledge graph is bolted on** (Neo4j via MCP), not native. Memory model is text-centric, not relationship-centric |
| Embeddable | No — requires Letta server |
| Offline sync | No |

#### FalkorDB

[FalkorDB](https://www.falkordb.com/) is a Redis-based graph database optimized for AI workloads.

| Aspect | Detail |
|--------|--------|
| Language | C (Redis module) |
| Architecture | Sparse matrix algebra over Redis, sub-millisecond Cypher queries |
| Key strength | Raw speed — in-memory graph with full Cypher support |
| Key weakness | **No event sourcing, no temporality, no sync**. Fast graph DB, not an agent memory system. No audit trail |
| Embeddable | No — requires Redis |
| Offline sync | No |

#### Cognee

[Cognee](https://github.com/topoteretes/cognee) is a memory management framework with an ECL (Extract, Cognify, Load) pipeline.

| Aspect | Detail |
|--------|--------|
| Language | Python |
| Architecture | Pluggable backends (Neo4j, Kuzu, FalkorDB, NetworkX) + vector storage |
| Key strength | Flexible, integrates with multiple graph DBs and LLM providers |
| Key weakness | **Orchestration layer, not a database**. Depends on external graph DB + vector DB + LLM provider |
| Embeddable | Partially (NetworkX backend for local dev) |
| Offline sync | No |

### 10.2 Capability Matrix

```
                    Event-     Temporal   Embedded   Offline    No LLM     Audit    Write
                    Sourced    Queries    (no infra) Sync       on Write   Trail    Latency
                    ────────   ────────   ────────   ────────   ────────   ──────   ────────
Zep/Graphiti        partial¹   ✓          ✗          ✗          ✗          partial  ~1-2s
Mem0                ✗          ✗          ✗          ✗          ✗          ✗        ~200ms
Letta               ✗          partial    ✗          ✗          ✗          partial  ~500ms
FalkorDB            ✗          ✗          ✗          ✗          ✓          ✗        <1ms
Cognee              ✗          ✗          partial²   ✗          ✗          ✗        ~1-2s
AllSource KG        ✓          ✓          ✓          ✓ (CRDT)   ✓          ✓        <50μs
```

¹ Graphiti tracks "episodes" which are event-like, but the graph itself is mutable.
² Cognee supports NetworkX for local dev, but no durability or sync.

### 10.3 The Gap in the Market

Every existing solution follows the same pattern:

```
Python orchestration layer
    → LLM API call (entity extraction)     ← slow, expensive, non-deterministic
    → External graph DB (Neo4j/FalkorDB)   ← separate infrastructure
    → External vector DB (Qdrant/Pinecone) ← yet another system
    → No offline support                   ← cloud-only
    → No event sourcing                    ← mutations destroy history
```

**Nobody has built the database itself purpose-built for agent memory graphs.** They've all built Python middleware that glues together 2-3 external databases and an LLM API key.

AllSource KG inverts this:

```
Single Rust binary (or embedded library)
    → Structured node/edge API              ← 50μs writes, deterministic
    → Built-in storage (WAL + Parquet)      ← zero external dependencies
    → Built-in vector index                 ← no separate vector DB
    → Offline-first with CRDT sync          ← works anywhere
    → Event-sourced                         ← full history, time-travel, audit trail
    → MCP tools                             ← agents are callers, not on the write path
```

### 10.4 Positioning Statement

> "The event-sourced knowledge graph for AI agents. Every node and edge has a full audit trail. Time-travel to see your graph at any point in history. Runs embedded in your app — no Neo4j, no Redis, no API keys. Syncs offline-first with CRDT conflict resolution. 50μs writes, not 2-second LLM round-trips."

### 10.5 Features Inspired by Competitors

Several competitor features are worth adopting, adapted to AllSource's architecture:

| Feature | Origin | AllSource Adaptation |
|---------|--------|---------------------|
| Contradiction detection / edge invalidation | Zep/Graphiti | Projection that detects conflicting edges for same subject-predicate, emits `graph.edge.invalidated` event |
| Community detection (Leiden algorithm) | Graphiti | `CommunityProjection` that clusters densely connected nodes, updates incrementally |
| Episode-based ingestion | Graphiti | `kg.add_episode(text, source)` — batch operation that creates nodes + edges from structured input |
| Memory decay / relevance scoring | Mem0 | `RelevanceProjection` with configurable decay function (exponential, linear) based on access patterns |
| Self-editing memory (agent-driven compaction) | Letta/MemGPT | `kg.compact(entity)` — agent-triggered projection that merges redundant nodes |
| Hybrid retrieval (semantic + structural) | Mem0, Cognee | Combined vector similarity + graph traversal in single query |
| Conversation-aware context | Mem0 | `kg.context_for(conversation_id, top_k)` — retrieves relevant subgraph for current conversation |

---

## 11. Decision Record

| Decision | Rationale |
|----------|-----------|
| Graph layer on top of Core, not a fork | Zero new storage engines. Core handles durability, sync, projections. KG adds domain semantics. |
| `graph.` event namespace | Clear separation from user events. Enables prefix queries and merge strategy config. |
| Entity ID format `node:{type}:{id}` | Enables type-based queries via existing `entity_id` prefix matching. No new index needed. |
| Projections for graph indexes | Real-time updates on event ingestion. No separate indexing pipeline. |
| BFS/Dijkstra over projections, not events | Traversals must be fast. Replaying events per query is O(N) — projections give O(1) per hop. |
| Soft-delete, not hard delete | Event store is append-only. "Deleted" is a state, tracked with full provenance. |
| MCP tools in Phase 4, not Phase 1 | Get the data model right first. Agent integration is a consumer of the graph, not a driver of its design. |
| No Cypher/SPARQL query language | Complexity not justified for MVP. Rust builder API + REST endpoints cover 90% of use cases. |
| Feature flag `knowledge-graph` | Opt-in. Users who don't need graph capabilities pay zero cost. |

---

## 12. What's NOT Included

- **Custom query language** (Cypher, Gremlin, SPARQL) — defer until user demand justifies the parser complexity
- **Graph neural networks** — out of scope; export to PyG/DGL if needed
- **Real-time streaming of graph changes** — existing WebSocket infrastructure handles this; no KG-specific streaming needed
- **Multi-graph support** — use tenant isolation for multiple independent graphs
- **Undo/redo** — event sourcing gives you full history; "undo" is "add a compensating event"
- **Graph visualization** — frontend concern; export GraphML/JSON for visualization tools

---

## Appendix A: Docker Compose (Standalone Mode)

```yaml
services:
  allsource-kg:
    image: ghcr.io/all-source-os/chronos-kg:latest
    ports:
      - "3905:3905"
    environment:
      ALLSOURCE_DATA_DIR: /data
      ALLSOURCE_WAL_ENABLED: "true"
      ALLSOURCE_PARQUET_ENABLED: "true"
      KG_PORT: "3905"
    volumes:
      - kg-data:/data
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:3905/health"]
      interval: 10s
      timeout: 5s
      retries: 3

volumes:
  kg-data:
```

## Appendix B: Integration with Existing AllSource Stack

```
┌──────────────────────────────────────────────────────────┐
│                    Dashboard (Next.js)                     │
│          Graph visualization, node explorer, audit log     │
└──────────────────────┬───────────────────────────────────┘
                       │
┌──────────────────────▼───────────────────────────────────┐
│              Query Service (Elixir, port 3902)            │
│          Auth, billing, rate limiting, routing            │
└───────┬──────────────────────────────────┬───────────────┘
        │                                  │
┌───────▼───────────┐          ┌───────────▼───────────────┐
│ Core (port 3900)  │          │ KG Service (port 3905)    │
│ Raw event store   │◄────────►│ Graph operations          │
│ WAL + Parquet     │  sync    │ Uses Core as storage      │
└───────────────────┘          └───────────────────────────┘
        │
        │ WAL shipping
        ▼
┌───────────────────┐
│ Core Followers    │
│ (read replicas)   │
└───────────────────┘
```

The KG service can either embed Core directly (Mode A/B) or connect to an existing Core instance over HTTP. For the SaaS deployment, the Query Service routes `/api/v1/graph/*` requests to the KG service, which uses its own embedded Core instance that syncs with the primary Core cluster.
