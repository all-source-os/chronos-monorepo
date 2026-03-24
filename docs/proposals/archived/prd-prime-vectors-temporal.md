[PRD]
# PRD: Prime Vectors + Temporal Queries (M2-M3)

## Overview

Add vector search and temporal query capabilities to AllSource Prime. This PRD builds on the graph engine (PRD 1) to add semantic recall (embed, similar, vector_search), temporal queries (history, as_of, diff), and the hybrid recall feature that combines all three signals — the killer differentiator.

**Depends on:** PRD 1 (Prime Graph Engine) — Prime facade, projections, node/edge CRUD, traversal.

**Design proposal:** `docs/proposals/ALLSOURCE_PRIME.md` — Milestones M2 + M3.

## Goals

- Implement vector storage as events (`prime.vector.stored`, `prime.vector.deleted`)
- Build HNSW-based vector index as a projection with snapshot persistence
- Implement temporal queries: entity history, time-travel graph state, graph diff
- Build hybrid recall combining vector similarity + graph proximity + temporal recency
- Vector search under 5ms for 100K vectors
- Configurable embedding dimensionality (384 to 1536+)

## Quality Gates

### Epic-Level (run once on epic completion)
- `cargo test -p allsource-core --features prime-full` — all Prime tests pass (graph + vectors + temporal)
- `cargo clippy -p allsource-core --features prime-full -- -D warnings` — no warnings

### Story-Level (checked per story)
- **Vector stories:** Run `cargo test -p allsource-core --features prime-full prime::vectors`
- **Temporal stories:** Run `cargo test -p allsource-core --features prime-full prime::temporal`
- **Recall stories:** Run `cargo test -p allsource-core --features prime-full prime::recall`

## User Stories

### US-001: Feature Flags for Vectors [Backend]
**Description:** As a developer, I want feature flags that gate vector search dependencies, so that users who only need the graph don't pay the compilation cost.

**Acceptance Criteria:**
- [ ] `prime-vectors` feature added to `apps/core/Cargo.toml`: `prime-vectors = ["prime", "dep:instant-distance"]` (or chosen HNSW crate)
- [ ] `prime-full` feature added: `prime-full = ["prime", "prime-vectors"]`
- [ ] Vector-related modules gated with `#[cfg(feature = "prime-vectors")]`
- [ ] `cargo build -p allsource-core --features prime` compiles without vector dependencies
- [ ] `cargo build -p allsource-core --features prime-full` compiles with vector dependencies
- [ ] `cargo test -p allsource-core --features prime` still passes (graph-only)

Mark each item [x] as you complete it. Only close when all are checked.

### US-002: Vector Types + Event Schema [Backend]
**Description:** As a developer, I want vector-specific types and event definitions, so that embeddings are stored as first-class events.

**Acceptance Criteria:**
- [ ] `VectorEntry` struct: `{ id: String, text: Option<String>, dimensions: usize, metadata: Option<Value> }`
- [ ] `VectorSearchResult` struct: `{ id: String, score: f64, text: Option<String>, metadata: Option<Value> }`
- [ ] Event type constants: `prime.vector.stored`, `prime.vector.deleted`
- [ ] Entity ID format: `vec:{id}`
- [ ] Event payload for `prime.vector.stored`: `{ text, dimensions, metadata }` with the embedding vector stored in event metadata (not payload — keeps payload human-readable)
- [ ] Types in `apps/core/src/prime/vectors/types.rs`
- [ ] `cargo test -p allsource-core --features prime-full prime::vectors::types` passes

Mark each item [x] as you complete it. Only close when all are checked.

### US-003: VectorIndex Projection (HNSW) [Backend]
**Description:** As a developer, I want a projection that maintains an HNSW index over stored embeddings, so that similarity search is fast.

**Acceptance Criteria:**
- [ ] `VectorIndexProjection` in `apps/core/src/prime/vectors/index.rs`
- [ ] Implements `Projection` trait with snapshot/restore
- [ ] Uses an HNSW implementation (evaluate `instant-distance`, `hnsw_rs`, or `usearch` crates — pick the one with best Rust-native support and no C++ deps)
- [ ] Processes `prime.vector.stored`: extracts embedding from event metadata, adds to HNSW index
- [ ] Processes `prime.vector.deleted`: removes from index (or marks as deleted for lazy cleanup)
- [ ] Configurable: max dimensions, ef_construction, M parameter (HNSW tuning)
- [ ] Snapshot: serializes HNSW index to bytes for disk persistence
- [ ] Restore: deserializes HNSW index from bytes, replays only new events
- [ ] Test: insert 100 random vectors, verify index.len() == 100
- [ ] Test: snapshot, clear, restore, verify search still works
- [ ] `cargo test -p allsource-core --features prime-full prime::vectors::index` passes

Mark each item [x] as you complete it. Only close when all are checked.

### US-004: Embed + Similar Operations [Backend]
**Description:** As a developer, I want `prime.embed()` and `prime.similar()` methods, so that agents can store and find semantically similar content.

**Acceptance Criteria:**
- [ ] `prime.embed(id: &str, text: &str, vector: Vec<f32>) -> Result<()>` — ingests `prime.vector.stored` event with embedding in metadata
- [ ] `prime.embed_with_metadata(id: &str, text: &str, vector: Vec<f32>, metadata: Value) -> Result<()>` — same but with additional metadata
- [ ] `prime.similar(id: &str, top_k: usize) -> Result<Vec<VectorSearchResult>>` — finds top_k most similar vectors to the given ID's vector
- [ ] `prime.vector_search(query: &[f32], top_k: usize) -> Result<Vec<VectorSearchResult>>` — direct vector search without reference document
- [ ] `prime.delete_vector(id: &str) -> Result<()>` — ingests `prime.vector.deleted` event
- [ ] `prime.get_vector(id: &str) -> Result<Option<VectorEntry>>` — retrieves stored vector entry
- [ ] Cosine similarity as default distance metric
- [ ] Test: embed 3 documents, query similar to doc-1, verify doc-2 (close) scores higher than doc-3 (distant)
- [ ] Test: embed, delete, verify search no longer returns deleted vector
- [ ] Test: vector_search with raw query vector returns ranked results
- [ ] `cargo test -p allsource-core --features prime-full prime::vectors::ops` passes

Mark each item [x] as you complete it. Only close when all are checked.

### US-005: Entity History [Backend]
**Description:** As a developer, I want to retrieve the full audit trail for any entity (node, edge, or vector), so that agents have provenance for every piece of knowledge.

**Acceptance Criteria:**
- [ ] `prime.history(entity_id: &str) -> Result<Vec<HistoryEntry>>` — returns all events for the entity in chronological order
- [ ] `HistoryEntry` struct: `{ event_type: String, timestamp: DateTime, payload: Value, source: Option<String> }`
- [ ] Works for nodes: returns created, updated, deleted events
- [ ] Works for edges: returns created, deleted events
- [ ] Works for vectors: returns stored, deleted events
- [ ] Delegates to `EmbeddedCore::query()` with entity_id filter
- [ ] Test: create node, update twice, delete. `history()` returns 4 entries in order
- [ ] Test: history of non-existent entity returns empty vec (not error)
- [ ] Module: `apps/core/src/prime/temporal.rs`
- [ ] `cargo test -p allsource-core --features prime prime::temporal::history` passes

Mark each item [x] as you complete it. Only close when all are checked.

### US-006: Time-Travel Graph State [Backend]
**Description:** As a developer, I want to query the graph as it existed at a past timestamp, so that agents can answer "what did I know before X?".

**Acceptance Criteria:**
- [ ] `prime.neighbors_as_of(id: &NodeId, relation: Option<&str>, timestamp: DateTime) -> Result<Vec<Node>>` — returns neighbors that existed at the given timestamp
- [ ] `prime.get_node_as_of(id: &NodeId, timestamp: DateTime) -> Result<Option<Node>>` — returns node state at timestamp (replays events up to that point)
- [ ] Implementation: query events with `timestamp <= as_of`, replay to build point-in-time state
- [ ] Handles: nodes created after timestamp (excluded), nodes deleted before timestamp (excluded), edges added/removed over time
- [ ] Test: create node A at t1, add edge A→B at t2, add edge A→C at t3. `neighbors_as_of(A, None, t2)` = [B] (not C)
- [ ] Test: create node, delete at t2. `get_node_as_of(id, t1)` returns node, `get_node_as_of(id, t3)` returns None
- [ ] `cargo test -p allsource-core --features prime prime::temporal::time_travel` passes

Mark each item [x] as you complete it. Only close when all are checked.

### US-007: Graph Diff [Backend]
**Description:** As a developer, I want to see what changed in the graph between two timestamps, so that agents can track knowledge evolution.

**Acceptance Criteria:**
- [ ] `prime.diff(from: DateTime, to: DateTime) -> Result<GraphDiff>`
- [ ] `GraphDiff` struct: `{ nodes_added: Vec<NodeId>, nodes_updated: Vec<NodeId>, nodes_deleted: Vec<NodeId>, edges_added: Vec<EdgeId>, edges_deleted: Vec<EdgeId>, vectors_stored: Vec<String>, vectors_deleted: Vec<String> }`
- [ ] Implementation: query all `prime.*` events in the time range, categorize by type
- [ ] `prime.timeline(entity_id: &str, from: Option<DateTime>, to: Option<DateTime>) -> Result<Vec<HistoryEntry>>` — chronological event stream for an entity within a time range
- [ ] Test: add 3 nodes, 2 edges between t1 and t2. `diff(t1, t2)` shows nodes_added=3, edges_added=2
- [ ] Test: timeline with range filter returns only events in range
- [ ] `cargo test -p allsource-core --features prime prime::temporal::diff` passes

Mark each item [x] as you complete it. Only close when all are checked.

### US-008: Hybrid Recall [Backend]
**Description:** As a developer, I want a single `prime.recall()` operation that combines vector similarity, graph proximity, and temporal recency into ranked results — this is Prime's killer feature.

**Acceptance Criteria:**
- [ ] `RecallQuery` struct: `{ text: Option<String>, vector: Option<Vec<f32>>, node_type: Option<String>, depth: usize, top_k: usize, recency_weight: f64, similarity_weight: f64, proximity_weight: f64 }`
- [ ] `RecallResult` struct: `{ nodes: Vec<ScoredNode>, vectors: Vec<VectorSearchResult>, edges: Vec<Edge> }`
- [ ] `ScoredNode`: `{ node: Node, score: f64, depth: usize, components: ScoreComponents }` where `ScoreComponents` breaks down the contribution of each signal
- [ ] `prime.recall(query: RecallQuery) -> Result<RecallResult>`
- [ ] Scoring algorithm:
  - Vector similarity: cosine score (0-1) if query vector provided
  - Graph proximity: 1.0 / (1.0 + depth) for BFS distance from matched nodes
  - Temporal recency: exponential decay based on last_updated timestamp
  - Final score: `similarity_weight * sim + proximity_weight * prox + recency_weight * recency` (normalized)
- [ ] When `text` provided without `vector`: returns graph-only results filtered by node type + sorted by recency
- [ ] When `vector` provided: starts from vector matches, then expands graph neighborhood up to `depth`
- [ ] Module: `apps/core/src/prime/recall.rs`
- [ ] Test: embed 3 docs linked to graph nodes. `recall(vector=query, depth=1)` returns vector matches + their graph neighbors, ranked by combined score
- [ ] Test: verify recency_weight=1.0 makes newest results rank highest
- [ ] Test: verify depth=0 returns only direct vector matches (no graph expansion)
- [ ] `cargo test -p allsource-core --features prime-full prime::recall` passes

Mark each item [x] as you complete it. Only close when all are checked.

### US-009: Remember + Forget Convenience Methods [Backend]
**Description:** As a developer, I want high-level `remember` and `forget` methods that combine vector + graph storage in a single call, optimized for agent memory workflows. Includes domain tagging, source provenance, and confidence for Recall integration.

**Acceptance Criteria:**
- [ ] `prime.remember(text: &str, vector: Vec<f32>, node_type: &str, properties: Value, relations: Vec<(NodeId, &str)>) -> Result<NodeId>` — creates a node, stores its embedding, and connects it to existing nodes in one call
- [ ] Optional `domain: Option<&str>` parameter — tags the node with a knowledge domain (e.g. "revenue", "engineering") for compressed index organization
- [ ] Optional `source: Option<&str>` parameter — records provenance (e.g. "analysis-session-42")
- [ ] Optional `confidence: Option<f64>` parameter — records confidence score in node properties
- [ ] Internally: ingests `prime.node.created` + `prime.vector.stored` + N × `prime.edge.created` events
- [ ] `prime.forget(id: &NodeId) -> Result<()>` — soft-deletes node, its edges, and its vector in one call
- [ ] Internally: ingests `prime.node.deleted` + `prime.vector.deleted` + N × `prime.edge.deleted` events
- [ ] Test: `remember` creates node + vector + edges atomically, all retrievable
- [ ] Test: `remember` with domain sets domain on node, retrievable via `nodes_by_domain()`
- [ ] Test: `forget` removes node from graph queries and vector search
- [ ] `cargo test -p allsource-core --features prime-full prime::memory` passes

Mark each item [x] as you complete it. Only close when all are checked.

## Functional Requirements

- FR-1: Embeddings MUST be stored as event metadata, with source text in event payload
- FR-2: Vector index MUST be maintained as a projection, rebuilt from events on cold start (with snapshot acceleration)
- FR-3: Similarity search MUST use cosine similarity as default metric
- FR-4: `recall()` MUST combine at least two of three signals (vector, graph, temporal) — never just one
- FR-5: All temporal queries MUST work by replaying events, not by maintaining separate time-indexed structures
- FR-6: `remember()` and `forget()` MUST be atomic from the caller's perspective (all events ingested or none)
- FR-7: Vector operations MUST be gated behind the `prime-vectors` feature flag
- FR-8: Temporal queries MUST work with `prime` feature alone (no vector dependency)

## Non-Goals

- Built-in embedding model (agents provide their own vectors)
- Multi-modal embeddings (images, audio) — store any vector, but no encoder
- LLM-powered entity extraction on the write path
- Real-time streaming of recall results
- MCP or HTTP exposure (PRD 3)
- Contradiction detection or relevance decay (PRD 3)

## Technical Considerations

- **HNSW crate selection:** Evaluate `instant-distance` (pure Rust, simple API), `hnsw_rs` (more features), and `usearch` (C++ binding, fastest). Prefer pure Rust for portability. `instant-distance` is likely sufficient for v1.
- **Embedding storage:** Vectors as `Vec<f32>` in event metadata (JSON). For 1536-dim embeddings, this is ~6KB per event. WAL handles this fine. For 100K+ vectors, Parquet columnar storage is efficient.
- **Recall scoring:** The weight normalization should ensure weights sum to 1.0. If only some signals are available (e.g., no vector), redistribute weights proportionally.
- **Temporal replay performance:** For large entity histories, consider caching replayed states. The existing `SnapshotManager` could be leveraged — snapshot entity state periodically, replay from last snapshot.

## Success Metrics

- Vector search under 5ms for 100K vectors (384-dim)
- Hybrid recall returns meaningful ranked results in under 10ms
- All temporal queries produce correct results verified by event-level assertions
- 30+ new tests passing

## Open Questions

- Should `recall()` accept a raw text string and use a stored embedding model, or always require pre-computed vectors?
- What's the maximum practical vector count for embedded use? (100K? 1M?)
- Should temporal queries support HLC timestamps or wall-clock only?
[/PRD]
