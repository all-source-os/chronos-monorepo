[PRD]
# PRD: Prime Graph Engine (M1)

## Overview

Build the foundational graph primitives for AllSource Prime — a unified agent memory engine. This PRD covers the Prime facade, graph data model (nodes + edges as events), projection-based indexing, and traversal algorithms. Everything is built on top of the existing `EmbeddedCore` with events stored in WAL + Parquet.

**Depends on:** Existing `EmbeddedCore` (`apps/core/src/embedded/core.rs`), `Projection` trait (`apps/core/src/application/services/projection.rs`), `CrdtResolver` (`apps/core/src/infrastructure/cluster/crdt.rs`).

**Design proposal:** `docs/proposals/ALLSOURCE_PRIME.md`

## Goals

- Establish the `prime` feature flag and module structure in `allsource-core`
- Implement node and edge CRUD as event-sourced operations under the `prime.` namespace
- Build projection-based indexes (adjacency list, reverse index, node state, type index) with snapshot persistence
- Implement graph traversal: 1-hop neighbors, N-hop BFS, shortest path (BFS + Dijkstra), subgraph extraction
- All graph operations under 50μs for graphs up to 100K nodes
- 40+ tests covering CRUD, projections, traversal, and edge cases

## Quality Gates

### Epic-Level (run once on epic completion)
General codebase checks that run ONCE when all stories are done:
- `cargo test -p allsource-core --features prime` — all Prime tests pass
- `cargo clippy -p allsource-core --features prime -- -D warnings` — no warnings

### Story-Level (checked per story)
- **Engine stories:** Run specific test filter (e.g. `cargo test -p allsource-core --features prime prime::types`)
- **Projection stories:** Verify projection rebuilds from WAL correctly after restart

## User Stories

### US-001: Add Projection Checkpointing to Core [Backend]
**Description:** As a developer building Prime, I need projections to snapshot their state to disk and restore on startup, so that large graphs don't require full WAL replay on every restart.

This is a prerequisite for Prime's graph projections. The existing `Projection` trait (`apps/core/src/application/services/projection.rs`) has no persistence — projections rebuild from all events on registration. Add checkpoint support.

**Acceptance Criteria:**
- [ ] `Projection` trait extended with `fn snapshot(&self) -> Option<Value>` and `fn restore(&self, snapshot: &Value) -> Result<()>` methods (with default no-op implementations for backwards compatibility)
- [ ] `ProjectionCheckpoint` struct added: `{ projection_name, state: Value, last_event_timestamp, event_count }`
- [ ] Checkpoint persistence to disk (JSON file in data directory, e.g. `{data_dir}/projections/{name}.checkpoint.json`)
- [ ] On projection registration, check for checkpoint → restore → replay only events after `last_event_timestamp`
- [ ] Configurable checkpoint interval (every N events or M seconds) in `EventStoreConfig` or projection config
- [ ] Existing projections (`EntitySnapshotProjection`, `EventCounterProjection`) continue to work unchanged (default no-op snapshot/restore)
- [ ] Test: register projection, process 1000 events, checkpoint, restart, verify only new events replayed
- [ ] `cargo test -p allsource-core projection::checkpoint` passes

Mark each item [x] as you complete it. Only close when all are checked.

### US-002: Prime Module Structure + Types [Backend]
**Description:** As a developer, I want the Prime module scaffolded with feature flag and core types, so that subsequent stories have a foundation to build on.

**Acceptance Criteria:**
- [ ] `prime` feature added to `apps/core/Cargo.toml`: `prime = ["embedded"]`
- [ ] Module `apps/core/src/prime/` created with `mod.rs`, `types.rs`, `error.rs`
- [ ] `types.rs` defines: `NodeId(String)`, `EdgeId(String)`, `Node { id: NodeId, node_type: String, properties: Value, created_at: DateTime, updated_at: DateTime }`, `Edge { id: EdgeId, source: NodeId, target: NodeId, relation: String, properties: Option<Value>, weight: Option<f64>, created_at: DateTime }`, `Direction { Incoming, Outgoing, Both }`
- [ ] `error.rs` defines `PrimeError` enum: `NodeNotFound`, `EdgeNotFound`, `DuplicateNode`, `InvalidTraversal`, `ProjectionError`, `CoreError(anyhow::Error)`
- [ ] Event type constants defined: `prime.node.created`, `prime.node.updated`, `prime.node.deleted`, `prime.edge.created`, `prime.edge.deleted`
- [ ] Entity ID format functions: `node_entity_id(type, id) -> "node:{type}:{id}"`, `edge_entity_id(id) -> "edge:{id}"`
- [ ] All types derive `Debug, Clone, Serialize, Deserialize` where appropriate
- [ ] `cargo test -p allsource-core --features prime prime::types` passes
- [ ] Module is conditionally compiled: `#[cfg(feature = "prime")]`

Mark each item [x] as you complete it. Only close when all are checked.

### US-003: Prime Facade [Backend]
**Description:** As a developer, I want a `Prime` struct that wraps `EmbeddedCore` and registers graph projections, so that all Prime operations go through a single entry point.

**Acceptance Criteria:**
- [ ] `Prime` struct in `apps/core/src/prime/mod.rs` wrapping `EmbeddedCore`
- [ ] `Prime::open(path: impl AsRef<Path>) -> Result<Self>` constructor that:
  - Creates `EmbeddedConfig` with durable storage at `path`
  - Configures merge strategies: `prime.node.created` → FirstWriteWins, `prime.node.updated` → LastWriteWins, `prime.edge.created` → AppendOnly, `prime.edge.deleted` → LastWriteWins
  - Opens `EmbeddedCore`
  - Registers all Prime projections (added in later stories — for now, just the skeleton)
- [ ] `Prime::open_in_memory() -> Result<Self>` for testing (no disk persistence)
- [ ] `Prime::shutdown(self) -> Result<()>` delegates to `EmbeddedCore::shutdown()`
- [ ] `Prime::stats() -> PrimeStats` returns node count, edge count, event count
- [ ] Test: open Prime, verify it initializes without error, shutdown cleanly
- [ ] Test: open Prime with path, ingest a raw event, verify it persists
- [ ] `cargo test -p allsource-core --features prime prime::facade` passes

Mark each item [x] as you complete it. Only close when all are checked.

### US-004: NodeState Projection [Backend]
**Description:** As a developer, I want a projection that maintains the current merged state of each node, so that `get_node` is O(1) without replaying events.

**Acceptance Criteria:**
- [ ] `NodeStateProjection` in `apps/core/src/prime/projections/node_state.rs`
- [ ] Implements `Projection` trait (including new `snapshot`/`restore` methods from US-001)
- [ ] Processes `prime.node.created`: inserts full node state into DashMap keyed by `NodeId`
- [ ] Processes `prime.node.updated`: merges properties into existing node state (deep merge, not replace)
- [ ] Processes `prime.node.deleted`: marks node as deleted (soft delete — retains in map with `deleted: true` flag)
- [ ] `get_state(entity_id)` returns current `Node` as `Value`
- [ ] `snapshot()` serializes entire DashMap to JSON Value
- [ ] `restore(snapshot)` rebuilds DashMap from JSON Value
- [ ] Test: create node, update properties, verify merged state
- [ ] Test: delete node, verify soft-deleted
- [ ] Test: snapshot, clear, restore, verify state matches
- [ ] `cargo test -p allsource-core --features prime prime::projections::node_state` passes

Mark each item [x] as you complete it. Only close when all are checked.

### US-005: NodeTypeIndex Projection [Backend]
**Description:** As a developer, I want a projection that indexes nodes by type, so that I can efficiently query "all nodes of type X".

**Acceptance Criteria:**
- [ ] `NodeTypeIndexProjection` in `apps/core/src/prime/projections/node_type_index.rs`
- [ ] Implements `Projection` trait with snapshot/restore
- [ ] Maintains `DashMap<String, HashSet<NodeId>>` — type → set of node IDs
- [ ] Processes `prime.node.created`: adds node ID to type set
- [ ] Processes `prime.node.deleted`: removes node ID from type set
- [ ] Public method `nodes_by_type(type: &str) -> Vec<NodeId>`
- [ ] Public method `node_types() -> Vec<String>` — list all known types
- [ ] Test: add 3 "person" nodes and 2 "project" nodes, verify `nodes_by_type("person")` returns 3
- [ ] Test: delete a person node, verify count drops to 2
- [ ] `cargo test -p allsource-core --features prime prime::projections::node_type_index` passes

Mark each item [x] as you complete it. Only close when all are checked.

### US-006: AdjacencyList + ReverseIndex Projections [Backend]
**Description:** As a developer, I want projections that maintain outgoing and incoming edges per node, so that neighbor lookups are O(1).

**Acceptance Criteria:**
- [ ] `AdjacencyListProjection` in `apps/core/src/prime/projections/adjacency.rs`
- [ ] Maintains `DashMap<NodeId, Vec<(String, NodeId, EdgeId, Option<f64>)>>` — source → [(relation, target, edge_id, weight)]
- [ ] Processes `prime.edge.created`: appends to source node's adjacency list
- [ ] Processes `prime.edge.deleted`: removes from source node's adjacency list
- [ ] Implements snapshot/restore
- [ ] `ReverseIndexProjection` in `apps/core/src/prime/projections/reverse_index.rs`
- [ ] Maintains `DashMap<NodeId, Vec<(String, NodeId, EdgeId, Option<f64>)>>` — target → [(relation, source, edge_id, weight)]
- [ ] Processes same events, indexed by target instead of source
- [ ] Implements snapshot/restore
- [ ] Test: add edges A→B, A→C, D→B. Verify adjacency[A] = [B, C], reverse[B] = [A, D]
- [ ] Test: delete edge A→B, verify adjacency and reverse updated
- [ ] `cargo test -p allsource-core --features prime prime::projections::adjacency` passes

Mark each item [x] as you complete it. Only close when all are checked.

### US-007: Node CRUD Operations [Backend]
**Description:** As a developer, I want to create, read, update, and delete graph nodes through the Prime API, with each mutation stored as an immutable event.

**Acceptance Criteria:**
- [ ] `prime.add_node(type: &str, properties: Value) -> Result<NodeId>` — generates UUID, ingests `prime.node.created` event, returns `NodeId`
- [ ] `prime.get_node(id: &NodeId) -> Result<Option<Node>>` — reads from `NodeStateProjection`
- [ ] `prime.update_node(id: &NodeId, properties: Value) -> Result<()>` — ingests `prime.node.updated` event, returns error if node doesn't exist or is deleted
- [ ] `prime.delete_node(id: &NodeId) -> Result<()>` — ingests `prime.node.deleted` event, also deletes all edges connected to this node (ingests `prime.edge.deleted` for each)
- [ ] `prime.nodes_by_type(type: &str) -> Vec<Node>` — reads from `NodeTypeIndexProjection` + `NodeStateProjection`
- [ ] Event payload includes `node_type`, `properties`, and `labels` (empty vec by default)
- [ ] Test: full CRUD lifecycle — create, read, update, read (verify merge), delete, read (verify gone)
- [ ] Test: delete node with edges, verify edges also deleted
- [ ] Test: update non-existent node returns `NodeNotFound`
- [ ] `cargo test -p allsource-core --features prime prime::node` passes

Mark each item [x] as you complete it. Only close when all are checked.

### US-008: Edge CRUD Operations [Backend]
**Description:** As a developer, I want to create, read, and delete directed edges between nodes, with each mutation stored as an immutable event.

**Acceptance Criteria:**
- [ ] `prime.add_edge(source: &NodeId, target: &NodeId, relation: &str, properties: Option<Value>) -> Result<EdgeId>` — generates UUID, ingests `prime.edge.created` event
- [ ] `prime.add_edge_weighted(source, target, relation, weight: f64, properties: Option<Value>) -> Result<EdgeId>` — same but with weight
- [ ] `prime.get_edge(id: &EdgeId) -> Result<Option<Edge>>` — reads from events or a lightweight edge state projection
- [ ] `prime.delete_edge(id: &EdgeId) -> Result<()>` — ingests `prime.edge.deleted` event
- [ ] Validates source and target nodes exist and are not deleted before creating edge
- [ ] Event payload includes `source`, `target`, `relation`, `properties`, `weight`
- [ ] Test: create edge between two nodes, verify edge exists
- [ ] Test: create edge to non-existent node returns error
- [ ] Test: delete edge, verify it's gone from adjacency + reverse indexes
- [ ] `cargo test -p allsource-core --features prime prime::edge` passes

Mark each item [x] as you complete it. Only close when all are checked.

### US-009: Neighbor Queries [Backend]
**Description:** As a developer, I want to query a node's neighbors with direction and relation filters, so that agents can traverse the graph.

**Acceptance Criteria:**
- [ ] `prime.neighbors(id: &NodeId, relation: Option<&str>, direction: Direction) -> Result<Vec<Node>>` — 1-hop neighbors
- [ ] Direction::Outgoing reads from `AdjacencyListProjection`
- [ ] Direction::Incoming reads from `ReverseIndexProjection`
- [ ] Direction::Both combines both (deduplicated)
- [ ] Optional `relation` filter: only return neighbors connected by matching relation type
- [ ] Returns full `Node` objects (resolved via `NodeStateProjection`)
- [ ] Skips deleted nodes in results
- [ ] Test: A→B (works_on), A→C (knows), D→A (manages). `neighbors(A, None, Outgoing)` = [B, C]. `neighbors(A, Some("works_on"), Outgoing)` = [B]. `neighbors(A, None, Incoming)` = [D]. `neighbors(A, None, Both)` = [B, C, D]
- [ ] `cargo test -p allsource-core --features prime prime::traversal::neighbors` passes

Mark each item [x] as you complete it. Only close when all are checked.

### US-010: BFS Traversal + Subgraph Extraction [Backend]
**Description:** As a developer, I want multi-hop traversal and ego network extraction, so that agents can explore graph neighborhoods.

**Acceptance Criteria:**
- [ ] `prime.neighbors_within(id: &NodeId, depth: usize, relation: Option<&str>, direction: Direction) -> Result<Vec<(Node, usize)>>` — BFS up to N hops, returns nodes with their depth
- [ ] `prime.subgraph(center: &NodeId, depth: usize) -> Result<SubGraph>` — returns `SubGraph { nodes: Vec<Node>, edges: Vec<Edge> }` for the ego network
- [ ] BFS uses visited set to avoid cycles
- [ ] Respects direction and relation filters
- [ ] Depth 0 = just the center node, depth 1 = center + immediate neighbors, etc.
- [ ] Test: build diamond graph A→B→D, A→C→D. `neighbors_within(A, 2, None, Outgoing)` = [B(1), C(1), D(2)]
- [ ] Test: `subgraph(A, 1)` returns nodes [A, B, C] and edges [A→B, A→C]
- [ ] Test: cyclic graph A→B→C→A, verify BFS terminates and visits each node once
- [ ] `cargo test -p allsource-core --features prime prime::traversal::bfs` passes

Mark each item [x] as you complete it. Only close when all are checked.

### US-011: Shortest Path [Backend]
**Description:** As a developer, I want to find the shortest path between two nodes using BFS (unweighted) or Dijkstra (weighted), so agents can discover how entities are connected.

**Acceptance Criteria:**
- [ ] `prime.shortest_path(from: &NodeId, to: &NodeId, relation: Option<&str>) -> Result<Option<Vec<Node>>>` — BFS for unweighted graphs, returns ordered path including start and end
- [ ] `prime.shortest_path_weighted(from: &NodeId, to: &NodeId, relation: Option<&str>) -> Result<Option<(Vec<Node>, f64)>>` — Dijkstra, returns path + total weight
- [ ] Returns `None` if no path exists
- [ ] Optional relation filter restricts which edges are traversable
- [ ] Traversal module in `apps/core/src/prime/traversal.rs`
- [ ] Test: A→B→C→D, shortest_path(A, D) = [A, B, C, D]
- [ ] Test: A→B (weight 1), A→C (weight 3), B→D (weight 1), C→D (weight 1). Dijkstra(A, D) = [A, B, D] with weight 2 (not [A, C, D] with weight 4)
- [ ] Test: disconnected nodes, verify returns None
- [ ] Test: path to self returns [A]
- [ ] `cargo test -p allsource-core --features prime prime::traversal::shortest_path` passes

Mark each item [x] as you complete it. Only close when all are checked.

### US-012: GraphStats Projection [Backend]
**Description:** As a developer, I want a projection that tracks graph statistics incrementally, so that `prime.stats()` is O(1).

**Acceptance Criteria:**
- [ ] `GraphStatsProjection` in `apps/core/src/prime/projections/stats.rs`
- [ ] Tracks: total_nodes, total_edges, nodes_by_type (HashMap), edges_by_relation (HashMap), deleted_nodes, deleted_edges
- [ ] Processes all `prime.*` events and updates counters
- [ ] Implements snapshot/restore
- [ ] `prime.stats()` reads from this projection, returns `PrimeStats { total_nodes, total_edges, nodes_by_type, edges_by_relation, event_count }`
- [ ] Test: add 5 nodes (3 person, 2 project), 4 edges (2 works_on, 2 knows), delete 1 node. Verify all stats correct.
- [ ] `cargo test -p allsource-core --features prime prime::projections::stats` passes

Mark each item [x] as you complete it. Only close when all are checked.

## Functional Requirements

- FR-1: Every graph mutation (node create/update/delete, edge create/delete) MUST be stored as an immutable event in the `prime.` namespace
- FR-2: Graph reads MUST be served from projections (DashMap), not by replaying events
- FR-3: Projections MUST support snapshot persistence to disk and restore on startup with WAL replay from checkpoint
- FR-4: Node deletion MUST cascade to connected edges (delete all edges where node is source or target)
- FR-5: All graph operations MUST be under 50μs for graphs up to 100K nodes (excluding I/O)
- FR-6: The `prime` feature flag MUST NOT affect compilation or behavior when disabled
- FR-7: CRDT merge strategies MUST be configured for all `prime.*` event types to support future sync
- FR-8: Entity IDs MUST follow the format `node:{type}:{uuid}` and `edge:{uuid}`

## Non-Goals

- Vector storage or search (PRD 2)
- Temporal queries — history, as_of, diff (PRD 2)
- Hybrid recall (PRD 2)
- MCP server or HTTP API (PRD 3)
- Contradiction detection, relevance decay, compaction (PRD 3)
- Offline sync (PRD 3)
- Graph visualization or export formats
- Custom query language

## Technical Considerations

- **Projection registration:** `EmbeddedCore` currently registers projections via its internal `ProjectionManager`. Prime needs access to register its own projections during `Prime::open()`. May need to expose a registration method or use the existing pattern.
- **Snapshot persistence path:** Projection checkpoints should go in `{data_dir}/projections/` alongside WAL and Parquet directories.
- **DashMap concurrency:** All projections use `DashMap` for lock-free reads. Edge deletion cascade (US-007) requires careful ordering — delete edges first, then mark node deleted.
- **Feature flag gating:** Use `#[cfg(feature = "prime")]` on the module. The `prime` feature implies `embedded` since Prime wraps `EmbeddedCore`.

## Success Metrics

- 40+ tests passing under `cargo test -p allsource-core --features prime`
- All CRUD + traversal operations benchmarked under 50μs on a 10K-node graph
- Projection checkpoint/restore works correctly (verified by restart test)
- Zero clippy warnings

## Open Questions

- Should `Prime::open()` accept a full `EmbeddedConfig` for advanced users, or always build its own config from a path?
- Should edge weights default to 1.0 (for Dijkstra) or None (truly unweighted)?
- Should `SubGraph` include edge metadata (properties, weight) or just connectivity?
[/PRD]
