[PRD]
# PRD: Prime MCP Server + Agent Features (M4-M6)

## Overview

Expose AllSource Prime to AI agents via MCP (stdio) and HTTP, then add agent-specific memory features: contradiction detection, relevance decay, memory compaction, conversation scoping, and offline sync. This PRD turns Prime from an embeddable engine into a zero-config agent memory server.

**Depends on:** PRD 1 (Graph Engine), PRD 2 (Vectors + Temporal).

**Design proposal:** `docs/proposals/ALLSOURCE_PRIME.md` — Milestones M4, M5, M6.

## Goals

- Ship `allsource-prime` MCP binary — one command gives any AI agent persistent structured memory
- Expose full Prime API over HTTP REST (port 3905)
- Build Docker image `ghcr.io/all-source-os/prime`
- Add contradiction detection, relevance decay, and memory compaction as projections
- Add conversation scoping for multi-conversation agents
- Add graph-aware offline sync with conflict reporting

## Quality Gates

### Epic-Level (run once on epic completion)
- `cargo test -p allsource-core --features prime-full` — all Prime engine tests pass
- `cargo test -p allsource-prime` — MCP + HTTP server tests pass (if separate crate)
- `cargo clippy --workspace -- -D warnings` — no warnings across workspace

### Story-Level (checked per story)
- **MCP stories:** Verify tool works via `echo '{"jsonrpc":"2.0",...}' | cargo run -p allsource-prime`
- **HTTP stories:** Verify endpoint via `curl` against running server
- **Projection stories:** Run specific `cargo test` filter

## User Stories

### US-001: MCP Binary Scaffold [Backend]
**Description:** As an AI agent developer, I want an `allsource-prime` binary that serves MCP tools over stdio, so I can add Prime to any agent's MCP config.

**Acceptance Criteria:**
- [ ] New crate `apps/prime-mcp/` with its own `Cargo.toml` (excluded from root workspace to avoid Dockerfile contamination, per monorepo rules)
- [ ] Binary name: `allsource-prime`
- [ ] CLI args: `--data-dir <path>` (required), `--log-level <level>` (optional, default info)
- [ ] On start: opens `Prime` at data-dir, registers MCP tools, serves over stdio
- [ ] Uses `rmcp` or equivalent Rust MCP SDK for stdio transport
- [ ] Graceful shutdown: on stdin EOF or SIGTERM, calls `prime.shutdown()`
- [ ] Test: binary starts and responds to MCP `initialize` handshake
- [ ] `cargo build -p allsource-prime` compiles

Mark each item [x] as you complete it. Only close when all are checked.

### US-002: MCP Tools — Graph Operations [Backend]
**Description:** As an AI agent, I want MCP tools for graph CRUD, so I can build knowledge graphs during conversations.

**Acceptance Criteria:**
- [ ] `prime_add_node` tool: params `{ type: string, properties: object }`, returns `{ node_id: string }`
- [ ] `prime_add_edge` tool: params `{ source: string, target: string, relation: string, properties?: object, weight?: number }`, returns `{ edge_id: string }`
- [ ] `prime_neighbors` tool: params `{ node_id: string, relation?: string, direction?: "incoming"|"outgoing"|"both", depth?: number }`, returns `{ nodes: [...], edges: [...] }`
- [ ] `prime_search` tool: params `{ type?: string, properties?: object }`, returns `{ nodes: [...] }`
- [ ] `prime_shortest_path` tool: params `{ from: string, to: string, relation?: string }`, returns `{ path: [...], weight?: number }`
- [ ] `prime_forget` tool: params `{ node_id: string }`, returns `{ deleted: true }`
- [ ] All tools return JSON-serializable results
- [ ] All tools include clear descriptions for agent tool-use (the agent sees these)
- [ ] Test: pipe MCP call for `prime_add_node` via stdin, verify JSON-RPC response

Mark each item [x] as you complete it. Only close when all are checked.

### US-003: MCP Tools — Vector + Recall Operations [Backend]
**Description:** As an AI agent, I want MCP tools for embedding storage and hybrid recall, so I can build and search semantic memory.

**Acceptance Criteria:**
- [ ] `prime_embed` tool: params `{ id: string, text: string, vector: number[], metadata?: object }`, returns `{ stored: true }`
- [ ] `prime_similar` tool: params `{ id: string, top_k?: number }`, returns `{ results: [{ id, score, text }] }`
- [ ] `prime_recall` tool: params `{ text?: string, vector?: number[], node_type?: string, depth?: number, top_k?: number }`, returns `{ nodes: [...], vectors: [...], edges: [...] }`
- [ ] `prime_history` tool: params `{ entity_id: string }`, returns `{ events: [{ type, timestamp, payload }] }`
- [ ] Tool descriptions explain when to use each (embed for storage, similar for finding related, recall for hybrid search)
- [ ] Test: pipe MCP calls for embed → similar flow, verify results

Mark each item [x] as you complete it. Only close when all are checked.

### US-004: HTTP Server Mode [Backend]
**Description:** As a developer deploying Prime as a service, I want an HTTP REST API so that non-MCP clients (web apps, SDKs) can use Prime.

**Acceptance Criteria:**
- [ ] `allsource-prime --mode http --port 3905 --data-dir <path>` starts HTTP server
- [ ] Uses `axum` (consistent with Core's HTTP stack)
- [ ] Endpoints per design proposal section 6.3:
  - `POST /api/v1/prime/nodes` — create node
  - `GET /api/v1/prime/nodes/:id` — get node
  - `PATCH /api/v1/prime/nodes/:id` — update node
  - `DELETE /api/v1/prime/nodes/:id` — soft-delete node
  - `POST /api/v1/prime/edges` — create edge
  - `DELETE /api/v1/prime/edges/:id` — soft-delete edge
  - `GET /api/v1/prime/nodes/:id/neighbors` — traverse (query params: relation, direction, depth)
  - `POST /api/v1/prime/shortest-path` — find path
  - `GET /api/v1/prime/nodes/:id/subgraph` — ego network (query param: depth)
  - `POST /api/v1/prime/vectors` — store embedding
  - `POST /api/v1/prime/vectors/search` — vector search
  - `DELETE /api/v1/prime/vectors/:id` — remove embedding
  - `GET /api/v1/prime/nodes/:id/history` — audit trail
  - `GET /api/v1/prime/diff` — graph diff (query params: from, to)
  - `POST /api/v1/prime/recall` — hybrid recall
  - `GET /api/v1/prime/stats` — memory statistics
  - `GET /health` — health check
- [ ] JSON request/response bodies
- [ ] Proper HTTP status codes: 200 OK, 201 Created, 404 Not Found, 400 Bad Request
- [ ] Test: `curl -X POST localhost:3905/api/v1/prime/nodes -d '{"type":"person","properties":{"name":"Alice"}}'` returns 201 with node_id
- [ ] Test: `curl localhost:3905/health` returns 200

Mark each item [x] as you complete it. Only close when all are checked.

### US-005: Dockerfile + Docker Image [Integration]
**Description:** As a DevOps engineer, I want a Docker image for Prime, so I can deploy it alongside the existing Chronos stack.

**Acceptance Criteria:**
- [ ] `apps/prime-mcp/Dockerfile` — multi-stage build (builder + runtime)
- [ ] Build context is `apps/prime-mcp/` (standalone, per monorepo isolation rules — no `COPY apps/<other>/`)
- [ ] May `COPY crates/` for shared Rust crates if needed
- [ ] Runtime image based on `debian:bookworm-slim` or `distroless`
- [ ] Exposes port 3905 (HTTP mode)
- [ ] Default CMD: `allsource-prime --mode http --port 3905 --data-dir /data`
- [ ] Volume mount for `/data` (WAL + Parquet + projection checkpoints)
- [ ] Health check: `HEALTHCHECK CMD curl -f http://localhost:3905/health || exit 1`
- [ ] Image name: `ghcr.io/all-source-os/prime`
- [ ] `docker build -t prime-test apps/prime-mcp/` succeeds on arm64
- [ ] `docker run -v prime-data:/data prime-test` starts and responds to health check

Mark each item [x] as you complete it. Only close when all are checked.

### US-006: Contradiction Detection Projection [Backend]
**Description:** As an agent memory engine, Prime should detect when new edges contradict existing ones, so agents can resolve conflicting knowledge.

**Acceptance Criteria:**
- [ ] `ContradictionDetectionProjection` in `apps/core/src/prime/projections/contradiction.rs`
- [ ] Implements `Projection` trait with snapshot/restore
- [ ] Detects contradictions: two edges from the same source with the same relation type but different targets, where the relation is marked as "exclusive" (e.g., "is_ceo_of" — a person can only be CEO of one company)
- [ ] Configurable exclusive relations: `prime.configure_exclusive("is_ceo_of")`, `prime.configure_exclusive("capital_of")`
- [ ] When contradiction detected: emits `prime.contradiction.detected` event with `{ entity_id, existing_edge, conflicting_edge, relation }`
- [ ] `prime.contradictions() -> Vec<Contradiction>` — list unresolved contradictions
- [ ] `prime.resolve_contradiction(id: &str, keep: &EdgeId) -> Result<()>` — resolves by deleting the other edge
- [ ] Test: add "Alice is_ceo_of CompanyA", then "Alice is_ceo_of CompanyB" → contradiction detected
- [ ] Test: non-exclusive relations (e.g., "knows") don't trigger contradictions
- [ ] `cargo test -p allsource-core --features prime prime::projections::contradiction` passes

Mark each item [x] as you complete it. Only close when all are checked.

### US-007: Relevance Decay Projection [Backend]
**Description:** As an agent memory engine, Prime should score nodes/edges by relevance (recency + access frequency), so that recall results prioritize active knowledge.

**Acceptance Criteria:**
- [ ] `RelevanceDecayProjection` in `apps/core/src/prime/projections/relevance.rs`
- [ ] Implements `Projection` trait with snapshot/restore
- [ ] Maintains per-entity scores: `DashMap<String, RelevanceScore>`
- [ ] `RelevanceScore`: `{ base_score: f64, last_accessed: DateTime, access_count: u64, decay_rate: f64 }`
- [ ] Score increases on: entity creation, entity update, entity accessed via `recall()` or `neighbors()`
- [ ] Score decays exponentially over time: `score * e^(-decay_rate * hours_since_access)`
- [ ] `prime.relevance(entity_id: &str) -> f64` — current relevance score
- [ ] `prime.touch(entity_id: &str) -> Result<()>` — manually boost relevance (ingests `prime.memory.accessed` event)
- [ ] Recall integration: `recency_weight` in `RecallQuery` uses this projection's scores
- [ ] Configurable decay rate per node type (default: 0.01 per hour)
- [ ] Test: create node, wait (simulated), verify score decayed
- [ ] Test: access node via recall, verify score boosted
- [ ] `cargo test -p allsource-core --features prime prime::projections::relevance` passes

Mark each item [x] as you complete it. Only close when all are checked.

### US-008: Memory Compaction [Backend]
**Description:** As an agent, I want to merge redundant nodes into a single canonical node, so that my knowledge graph stays clean over time.

**Acceptance Criteria:**
- [ ] `prime.compact(target: &NodeId, sources: &[NodeId]) -> Result<()>` — merges source nodes into target
- [ ] Merges properties: target gets union of all properties (target values take precedence on conflict)
- [ ] Redirects edges: all edges pointing to/from source nodes are redirected to target
- [ ] Soft-deletes source nodes
- [ ] Ingests `prime.memory.compacted` event with `{ target, merged_from: [source_ids], merged_properties }`
- [ ] If vectors exist for source nodes, keeps target's vector (deletes source vectors)
- [ ] Test: nodes A, B, C with overlapping properties and edges. Compact B+C into A. Verify A has merged properties, all edges redirected, B+C deleted
- [ ] Test: compaction is reversible via event history (can see pre-compaction state via `as_of`)
- [ ] `cargo test -p allsource-core --features prime-full prime::compact` passes

Mark each item [x] as you complete it. Only close when all are checked.

### US-009: Conversation Scoping [Backend]
**Description:** As an agent running multiple conversations, I want to associate memory mutations with a conversation ID, so I can query "what did I learn in this conversation?".

**Acceptance Criteria:**
- [ ] `prime.with_conversation(id: &str) -> ConversationScope` — returns a scoped handle
- [ ] `ConversationScope` has all Prime methods but automatically adds `conversation_id` to every event's metadata
- [ ] `prime.conversation_history(conversation_id: &str) -> Result<Vec<HistoryEntry>>` — all events from a conversation
- [ ] `prime.conversation_diff(conversation_id: &str) -> Result<GraphDiff>` — what changed in this conversation
- [ ] `RecallQuery` gets optional `conversation_id` field to scope recall to a conversation
- [ ] MCP tools accept optional `conversation_id` parameter
- [ ] Test: two conversations add different nodes. `conversation_history(conv1)` only shows conv1's events
- [ ] Test: recall with conversation_id scopes results
- [ ] `cargo test -p allsource-core --features prime prime::conversation` passes

Mark each item [x] as you complete it. Only close when all are checked.

### US-010: Graph-Aware Offline Sync [Backend]
**Description:** As a developer running Prime on multiple devices, I want to sync knowledge graphs with graph-aware conflict reporting.

**Acceptance Criteria:**
- [ ] `prime.sync(remote: &str) -> Result<SyncReport>` — high-level sync using existing `EmbeddedCore::sync_to()`
- [ ] `SyncReport`: `{ pushed: usize, pulled: usize, conflicts: Vec<SyncConflict> }`
- [ ] `SyncConflict`: `{ entity_id: String, conflict_type: ConflictType, local_event: Event, remote_event: Event, resolution: MergeStrategy }`
- [ ] Graph-aware conflict types: `NodePropertyConflict` (same node updated differently), `EdgeContradiction` (conflicting edges synced), `DuplicateNode` (same logical entity, different IDs)
- [ ] `prime.sync_preview(remote: &str) -> Result<SyncPreview>` — shows what would change without applying
- [ ] `SyncPreview`: `{ will_push: usize, will_pull: usize, potential_conflicts: Vec<SyncConflict> }`
- [ ] Conflict resolution uses configured CRDT merge strategies (FirstWriteWins, LastWriteWins, AppendOnly per event type)
- [ ] Test: two Prime instances, add different nodes to each, sync, verify both have all nodes
- [ ] Test: both update same node with different properties, sync, verify LWW resolution applied
- [ ] Test: sync_preview shows accurate counts without modifying state
- [ ] `cargo test -p allsource-core --features prime prime::sync` passes

Mark each item [x] as you complete it. Only close when all are checked.

### US-011: Schema Enforcement per Node/Edge Type [Backend]
**Description:** As a developer, I want to define JSON schemas per node type and edge relation, so that agents can't store malformed data.

**Acceptance Criteria:**
- [ ] `prime.register_schema(node_type: &str, schema: Value) -> Result<()>` — registers a JSON Schema for a node type
- [ ] `prime.register_edge_schema(relation: &str, schema: Value) -> Result<()>` — registers schema for edge properties
- [ ] On `add_node` / `update_node`: validates properties against registered schema, returns `ValidationError` on failure
- [ ] On `add_edge`: validates properties against edge schema if registered
- [ ] Schemas stored as events (`prime.schema.registered`) — they're part of the event stream
- [ ] `prime.schemas() -> Vec<SchemaEntry>` — list registered schemas
- [ ] Schemas are optional — if no schema registered for a type, any properties accepted
- [ ] Test: register person schema requiring "name" field. `add_node("person", {})` fails validation. `add_node("person", {"name": "Alice"})` succeeds
- [ ] `cargo test -p allsource-core --features prime prime::schema` passes

Mark each item [x] as you complete it. Only close when all are checked.

### US-012: Batch Import/Export [Backend]
**Description:** As a developer, I want to bulk import and export graph data, so that I can seed Prime from existing knowledge or back up memory.

**Acceptance Criteria:**
- [ ] `prime.export_json(writer: impl Write) -> Result<ExportStats>` — exports all nodes, edges, vectors as JSON lines
- [ ] Export format: one JSON object per line, `{ "type": "node"|"edge"|"vector", "data": {...} }`
- [ ] `prime.import_json(reader: impl Read) -> Result<ImportStats>` — imports from JSON lines format
- [ ] Import ingests proper `prime.*` events (not raw inserts — maintains event sourcing)
- [ ] `ExportStats` / `ImportStats`: `{ nodes: usize, edges: usize, vectors: usize }`
- [ ] Import is idempotent: re-importing same data doesn't create duplicates (uses entity IDs + FirstWriteWins)
- [ ] Test: create graph, export, clear, import, verify graph matches original
- [ ] Test: import twice, verify no duplicates
- [ ] `cargo test -p allsource-core --features prime-full prime::import_export` passes

Mark each item [x] as you complete it. Only close when all are checked.

## Functional Requirements

- FR-1: The MCP binary MUST work with zero configuration beyond `--data-dir` — no API keys, no Docker required
- FR-2: All MCP tools MUST include human-readable descriptions that help AI agents understand when to use each tool
- FR-3: The HTTP API MUST follow the same URL patterns as Core (`/api/v1/prime/...`) for consistency
- FR-4: Contradiction detection MUST be opt-in per relation type (not all relations are exclusive)
- FR-5: Relevance decay MUST integrate with the existing `recall()` scoring (PRD 2 US-008)
- FR-6: Memory compaction MUST preserve full event history (the compaction itself is an event)
- FR-7: Sync MUST use existing CRDT infrastructure (`CrdtResolver`, `HLC`, merge strategies)
- FR-8: Schema validation MUST happen before event ingestion (fail fast)
- FR-9: The Docker image MUST be standalone (no dependency on Core or Query Service containers)

## Non-Goals

- Authentication / authorization on the HTTP API (add in SaaS mode behind Query Service)
- Rate limiting (Query Service responsibility)
- Multi-leader sync (single-leader CRDT first)
- Custom query language or GraphQL
- Graph visualization UI
- Community detection (Leiden clustering) — deferred to future PRD
- Auto-embedding (built-in embedding model)

## Technical Considerations

- **MCP SDK:** Use `rmcp` crate for Rust MCP server. If not mature enough, use raw JSON-RPC over stdio (simple — tool calls are just request/response).
- **Workspace isolation:** `apps/prime-mcp/` must be excluded from root `Cargo.toml` workspace (like chronis, registry) to prevent Dockerfile cross-contamination. It depends on `allsource-core` with `features = ["prime-full"]`.
- **Docker build context:** `apps/prime-mcp/` as build context. May need to COPY `crates/` if allsource-core is a workspace dependency. Alternative: publish allsource-core to crates.io first, depend on published version.
- **Contradiction detection latency:** The projection processes every `prime.edge.created` event. For exclusive relations, it checks for existing edges with the same source+relation. This is O(edges per source) — acceptable for agent-scale graphs.
- **Relevance decay computation:** Don't recompute all scores on every access. Compute lazily when `relevance()` is called. The projection stores raw data (last_accessed, access_count), the decay formula is applied at read time.

## Success Metrics

- Agent can build and query a knowledge graph across multiple MCP sessions with zero infrastructure
- HTTP API passes integration tests for all endpoints
- Docker image starts in under 2 seconds, health check passes
- Contradiction detection catches conflicting exclusive edges with zero false positives
- Sync between two Prime instances preserves all data with correct conflict resolution

## Open Questions

- Should the MCP binary also support HTTP mode (dual mode), or keep them as separate launch flags?
- Should contradiction resolution be automated (newest wins) or always require explicit resolution?
- For batch import, should we support GraphML format in addition to JSON lines?
- Should the Docker image default to MCP mode (stdio) or HTTP mode?
[/PRD]
