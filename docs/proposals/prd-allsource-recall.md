[PRD]
# PRD: AllSource Recall — Compressed Index + Agent Memory API

## Overview

AllSource Recall is the agent-facing memory API built on Prime. It adds the key insight from zer0dex — a compressed, token-efficient index of what the agent knows — but makes it automatic (projection-based, LLM-assisted) and durable (event-sourced with temporal queries).

The compressed index solves the hardest agent memory problem: cross-domain retrieval. When an agent asks "How does X relate to Y?", vector similarity finds X or Y — rarely both. zer0dex proved that a structured index bridging domains achieves 80% cross-reference accuracy where pure vector RAG scores 37.5%. Recall brings this to Prime with zero manual maintenance.

**Depends on:** Prime Graph Engine (M1), Prime Vectors + Temporal (M2-M3), Prime MCP Server (M4 — at minimum MCP scaffold + graph/vector tools).

**Reference:** `docs/articles/zer0dex-comparison.md`, `docs/proposals/ALLSOURCE_PRIME.md`

## Goals

- Build a `CompressedIndexProjection` that auto-generates a token-efficient markdown summary (~800 tokens) of the agent's knowledge, organized by domain with cross-references
- Use LLM calls to compress graph state into natural-language prose (not just heuristic formatting)
- Add domain tagging to nodes/edges for organizing the compressed index
- Expose `recall.index()` and `recall.context()` APIs that combine compressed index + vector search + temporal context
- Add pre-message context injection to MCP tools (agent gets relevant memory injected automatically)
- Build a benchmark harness for LoCoMo and LongMemEval evaluation
- Achieve >80% cross-reference accuracy (matching or exceeding zer0dex's benchmark)

## Quality Gates

### Epic-Level (run once on epic completion)
- `make quality-rust` passes (includes fmt, clippy, tests, release build, docs)
- All Recall tests pass: `cd apps/core && cargo +nightly test --locked --lib --all-features` includes recall tests
- Zero clippy warnings with recall feature enabled

### Story-Level (checked per story)
- **Engine stories:** Run specific `cargo test` filter (e.g. `cargo test -p allsource-core --features prime-recall recall::`)
- **MCP stories:** Verify tool works via MCP stdio test
- **Benchmark stories:** Verify benchmark harness runs and produces output

## User Stories

### US-001: Domain Tagging for Nodes and Edges [Backend]
**Description:** As a developer, I want to tag nodes and edges with a domain (e.g. "revenue", "engineering", "compliance"), so that the compressed index can organize knowledge by domain.

**Acceptance Criteria:**
- [ ] `Node` struct gains optional `domain: Option<String>` field
- [ ] `prime.add_node()` accepts optional `domain` parameter
- [ ] `prime.remember()` accepts optional `domain` parameter
- [ ] `DomainIndexProjection` in `apps/core/src/prime/projections/domain_index.rs` — maintains `DashMap<String, Vec<NodeId>>` (domain -> nodes)
- [ ] Implements Projection trait with snapshot/restore
- [ ] `prime.domains() -> Vec<String>` — list all known domains
- [ ] `prime.nodes_by_domain(domain: &str) -> Vec<Node>` — nodes in a domain
- [ ] Edges inherit domain from their source node (or can be explicitly tagged)
- [ ] Test: add nodes with domains "revenue" and "engineering", verify domain index
- [ ] Test: nodes_by_domain returns correct results
- [ ] `cargo test -p allsource-core --features prime recall::domain` passes

Mark each item [x] as you complete it. Only close when all are checked.

### US-002: Cross-Domain Relationship Tracking [Backend]
**Description:** As a developer, I want to track which domains are connected by edges, so that the compressed index can generate cross-references.

**Acceptance Criteria:**
- [ ] `CrossDomainProjection` in `apps/core/src/prime/projections/cross_domain.rs`
- [ ] Maintains `DashMap<(String, String), Vec<EdgeId>>` — (domain_a, domain_b) -> edges connecting them
- [ ] Processes `prime.edge.created`: if source and target nodes have different domains, records the cross-domain link
- [ ] Processes `prime.edge.deleted`: removes from cross-domain index
- [ ] `prime.cross_domain_links() -> Vec<CrossDomainLink>` where `CrossDomainLink { domain_a, domain_b, edge_count, sample_relations: Vec<String> }`
- [ ] Implements Projection trait with snapshot/restore
- [ ] Test: node A (domain "revenue") -> node B (domain "engineering"), verify cross-domain link recorded
- [ ] Test: same-domain edges don't appear in cross-domain index
- [ ] `cargo test -p allsource-core --features prime recall::cross_domain` passes

Mark each item [x] as you complete it. Only close when all are checked.

### US-003: Recall Feature Flag + Types [Backend]
**Description:** As a developer, I want a `prime-recall` feature flag that gates the compressed index and LLM-assisted features.

**Acceptance Criteria:**
- [ ] `prime-recall` feature added to `apps/core/Cargo.toml`: `prime-recall = ["prime-full"]`
- [ ] Recall-specific modules gated with `#[cfg(feature = "prime-recall")]`
- [ ] `RecallContext` struct: `{ index: String, vectors: Vec<VectorSearchResult>, nodes: Vec<ScoredNode>, edges: Vec<Edge>, token_count: usize }`
- [ ] `IndexConfig` struct: `{ max_tokens: usize, llm_endpoint: Option<String>, llm_model: Option<String>, refresh_interval_events: usize, refresh_interval_seconds: u64 }`
- [ ] `CompressedIndex` struct: `{ markdown: String, token_count: usize, domains: Vec<String>, cross_references: Vec<CrossDomainLink>, last_updated: DateTime, event_count_at_generation: usize }`
- [ ] Types in `apps/core/src/prime/recall/types.rs`
- [ ] `cargo build -p allsource-core --features prime-recall` compiles
- [ ] `cargo build -p allsource-core --features prime-full` still compiles without recall deps

Mark each item [x] as you complete it. Only close when all are checked.

### US-004: CompressedIndexProjection — Heuristic Scaffold [Backend]
**Description:** As a developer, I want a projection that maintains raw material for the compressed index (domain summaries, node counts, relationship stats), so that the LLM compressor has structured input.

**Acceptance Criteria:**
- [ ] `CompressedIndexProjection` in `apps/core/src/prime/recall/index_projection.rs`
- [ ] Implements Projection trait with snapshot/restore
- [ ] Maintains per-domain state: `{ domain: String, node_count: usize, node_types: HashSet<String>, sample_entities: Vec<String>, edge_count: usize }`
- [ ] Maintains cross-domain summary: list of (domain_a, domain_b, relation_types, edge_count)
- [ ] `raw_summary() -> IndexRawSummary` — returns structured data for LLM consumption
- [ ] `heuristic_index() -> String` — generates a basic markdown index WITHOUT LLM (fallback when no LLM configured): lists domains, node counts, cross-references as bullet points
- [ ] Heuristic output stays under 1000 tokens
- [ ] Test: add nodes across 3 domains with cross-domain edges, verify heuristic_index produces valid markdown
- [ ] Test: verify token count estimate is reasonable (within 20% of tiktoken)
- [ ] `cargo test -p allsource-core --features prime-recall recall::index_projection` passes

Mark each item [x] as you complete it. Only close when all are checked.

### US-005: LLM-Assisted Index Compression [Backend]
**Description:** As a developer, I want the compressed index to be generated by an LLM that summarizes the agent's knowledge into natural-language prose with cross-domain pointers, achieving higher recall than heuristic formatting.

**Acceptance Criteria:**
- [ ] `IndexCompressor` in `apps/core/src/prime/recall/compressor.rs`
- [ ] Accepts `IndexRawSummary` (from projection) and calls an LLM endpoint to generate compressed markdown
- [ ] LLM prompt template: "Summarize this agent's knowledge into a compressed markdown index. Organize by domain. Include cross-references between domains. Target ~800 tokens. Focus on relationships between concepts, not individual facts."
- [ ] Configurable LLM endpoint via `IndexConfig.llm_endpoint` (default: `http://localhost:11434/api/generate` for Ollama)
- [ ] Configurable model via `IndexConfig.llm_model` (default: `mistral`)
- [ ] Falls back to `heuristic_index()` if LLM is unavailable or errors
- [ ] Caches generated index until `refresh_interval_events` new events arrive or `refresh_interval_seconds` elapsed
- [ ] `prime.refresh_index() -> Result<CompressedIndex>` — forces regeneration
- [ ] Uses `reqwest` for HTTP calls (already in allsource-core deps)
- [ ] Test: mock LLM endpoint, verify compressor sends correct prompt and parses response
- [ ] Test: LLM unavailable, verify fallback to heuristic index
- [ ] Test: caching — verify index not regenerated before threshold
- [ ] `cargo test -p allsource-core --features prime-recall recall::compressor` passes

Mark each item [x] as you complete it. Only close when all are checked.

### US-006: Recall API — index() and context() [Backend]
**Description:** As a developer, I want `prime.index()` and `prime.context()` methods that combine the compressed index with vector search and temporal context into a single response optimized for agent consumption.

**Acceptance Criteria:**
- [ ] `prime.index(agent_id: Option<&str>) -> Result<CompressedIndex>` — returns the current compressed index (generates on first call, returns cached thereafter)
- [ ] `prime.context(query: RecallContextQuery) -> Result<RecallContext>` — combines compressed index excerpt + vector results + graph neighbors + temporal context
- [ ] `RecallContextQuery` struct: `{ query: String, vector: Option<Vec<f32>>, agent_id: Option<String>, top_k: usize, as_of: Option<DateTime>, include_index: bool, max_tokens: Option<usize> }`
- [ ] When `include_index` is true, prepends relevant excerpt from compressed index (filtered to domains matching the query's vector results)
- [ ] `RecallContext.token_count` estimates total token usage of the response
- [ ] If `max_tokens` is set, truncates results to fit within budget (index first, then vectors, then graph)
- [ ] Delegates to existing `prime.recall()` for vector + graph scoring, wraps with index
- [ ] Test: context() with include_index=true returns index + vector results
- [ ] Test: context() with max_tokens=500 truncates appropriately
- [ ] Test: context() with as_of returns temporal state
- [ ] `cargo test -p allsource-core --features prime-recall recall::api` passes

Mark each item [x] as you complete it. Only close when all are checked.

### US-007: MCP Tool — prime_index [Backend]
**Description:** As an AI agent, I want a `prime_index` MCP tool that returns my compressed knowledge index for injection into my system prompt.

**Acceptance Criteria:**
- [ ] `prime_index` tool added to MCP server: params `{ agent_id?: string }`, returns `{ index: string, token_count: number, domains: string[], last_updated: string }`
- [ ] Tool description: "Get a compressed summary of everything stored in memory, organized by domain with cross-references. Use this to understand the shape of your knowledge before searching for specifics."
- [ ] Returns heuristic index if LLM is not configured
- [ ] Returns cached index if fresh, regenerates if stale
- [ ] Test: pipe MCP call for prime_index, verify markdown response

Mark each item [x] as you complete it. Only close when all are checked.

### US-008: MCP Tool — prime_context [Backend]
**Description:** As an AI agent, I want a `prime_context` MCP tool that combines my compressed index with semantic search results for maximum recall.

**Acceptance Criteria:**
- [ ] `prime_context` tool added to MCP server: params `{ query: string, vector?: number[], agent_id?: string, top_k?: number, include_index?: boolean, max_tokens?: number }`
- [ ] Returns `{ index_excerpt?: string, vectors: [...], nodes: [...], edges: [...], token_count: number }`
- [ ] Tool description: "Search memory with hybrid recall: compressed index (cross-domain awareness) + semantic vectors (similarity) + graph (relationships) + temporal (recency). Use this instead of prime_recall when you want the compressed index included."
- [ ] `include_index` defaults to true
- [ ] Test: pipe MCP call for prime_context with a query, verify response includes index excerpt + vector results

Mark each item [x] as you complete it. Only close when all are checked.

### US-009: MCP Pre-Message Context Injection [Backend]
**Description:** As an AI agent developer, I want the MCP server to optionally auto-inject relevant memory context before each agent message, matching zer0dex's pre-message injection pattern.

**Acceptance Criteria:**
- [ ] New MCP server config flag: `--auto-inject` (default: false)
- [ ] When enabled, the MCP server exposes a `prime_auto_context` resource (MCP resource, not tool) that returns the compressed index
- [ ] Resource updates whenever the index is refreshed
- [ ] Agent MCP config example documented: how to use the resource for system prompt injection
- [ ] Configurable `--auto-inject-max-tokens <N>` (default: 1000) to cap injection size
- [ ] Test: start MCP server with --auto-inject, verify resource returns compressed index
- [ ] Test: index updates when new memories are added and threshold crossed

Mark each item [x] as you complete it. Only close when all are checked.

### US-010: Benchmark Harness — LoCoMo + LongMemEval [Integration]
**Description:** As a developer, I want a benchmark harness that evaluates Recall against LoCoMo and LongMemEval datasets, so we can publish comparable results.

**Acceptance Criteria:**
- [ ] Benchmark crate in `tooling/recall-bench/` with its own Cargo.toml
- [ ] Downloads LoCoMo dataset (from HuggingFace or direct URL) on first run
- [ ] Downloads LongMemEval dataset on first run
- [ ] Implements the standard evaluation protocol for each benchmark:
  - LoCoMo: multi-session conversation memory, measures recall/precision on fact retrieval
  - LongMemEval: long-term memory evaluation across conversation turns
- [ ] Runs Recall's `remember()` to ingest benchmark conversations, then `context()` to retrieve
- [ ] Compares retrieved facts against ground truth, computes recall/precision/F1
- [ ] Outputs results as markdown table + JSON for CI integration
- [ ] `cargo run -p recall-bench -- --dataset locomo` runs LoCoMo evaluation
- [ ] `cargo run -p recall-bench -- --dataset longmemeval` runs LongMemEval evaluation
- [ ] Includes a `--baseline vector-only` flag to run without compressed index (for ablation)
- [ ] Test: benchmark harness runs on a small subset (10 conversations) and produces output
- [ ] Results include comparison row for zer0dex published numbers (80% cross-ref, 91.2% recall)

Mark each item [x] as you complete it. Only close when all are checked.

### US-011: Cross-Reference Accuracy Test Suite [Integration]
**Description:** As a developer, I want a focused test suite that measures cross-domain retrieval accuracy, replicating zer0dex's benchmark methodology.

**Acceptance Criteria:**
- [ ] Test suite in `tooling/recall-bench/src/cross_ref.rs`
- [ ] Seeds Prime with multi-domain knowledge (minimum 5 domains, 50 facts, 20 cross-domain relationships)
- [ ] Runs 30+ cross-domain queries ("How does X relate to Y?" where X and Y are in different domains)
- [ ] Measures: cross-reference accuracy (did the response contain facts from BOTH domains?)
- [ ] Compares three modes: (a) vector-only, (b) vector + graph, (c) vector + graph + compressed index
- [ ] Outputs accuracy table showing improvement from each layer
- [ ] Target: >80% cross-reference accuracy with compressed index (matching zer0dex)
- [ ] `cargo run -p recall-bench -- --dataset cross-ref` runs the cross-reference suite
- [ ] Results reproducible (seeded random, deterministic ordering)

Mark each item [x] as you complete it. Only close when all are checked.

## Functional Requirements

- FR-1: The compressed index MUST be auto-generated from graph state — zero manual maintenance
- FR-2: The compressed index MUST stay under 1000 tokens to be practical for system prompt injection
- FR-3: Index generation MUST use an LLM when configured, falling back to heuristic when not
- FR-4: The LLM endpoint MUST be configurable (Ollama, OpenAI-compatible, Anthropic) — no vendor lock-in
- FR-5: Index refresh MUST be lazy (triggered by event threshold or time interval), not on every write
- FR-6: `context()` MUST combine compressed index + vector results + graph + temporal in a single response
- FR-7: Token budget MUST be respected — if max_tokens is set, truncate gracefully (index first, then vectors)
- FR-8: All domain and cross-domain state MUST be maintained as projections (event-sourced, snapshotable)
- FR-9: Benchmark harness MUST use standard evaluation datasets (LoCoMo, LongMemEval) — no custom benchmarks
- FR-10: MCP tools MUST include clear descriptions explaining when to use `prime_context` vs `prime_recall` vs `prime_index`

## Non-Goals

- Built-in LLM / embedding model (agents provide vectors, index uses external LLM endpoint)
- Automatic entity extraction on the write path (agents decide what to remember)
- Custom benchmark datasets (use published standards only)
- Python SDK wrapper for Recall (defer to separate SDK PRD)
- Full LoCoMo/LongMemEval analysis paper (this PRD builds the harness; analysis is a separate workstream)

## Technical Considerations

- **LLM for index compression:** The compressed index is regenerated periodically (not on every write). A single LLM call processes ~2-5KB of structured summary into ~800 tokens of prose. At default refresh intervals (every 100 events or 5 minutes), this is negligible cost. Ollama runs locally for zero-cost development.
- **Token counting:** Use a simple heuristic (words * 1.3) for Rust-side token estimation. Exact counts require a tokenizer; the heuristic is sufficient for budget enforcement.
- **Index staleness:** The projection tracks `event_count_at_generation`. When `current_event_count - event_count_at_generation > refresh_interval_events`, the index is stale. Staleness is cheap to check (O(1) counter comparison).
- **Benchmark datasets:** LoCoMo is on HuggingFace (`snap-stanford/locomo`). LongMemEval may need manual download. The harness should cache datasets locally after first download.
- **Feature flag layering:** `prime-recall` implies `prime-full` which implies `prime` + `prime-vectors`. This ensures all graph, vector, and temporal primitives are available.

## Success Metrics

- Compressed index generation under 5 seconds (LLM call) or 50ms (heuristic)
- Cross-reference accuracy >80% (matching zer0dex's published numbers)
- LoCoMo recall score published and comparable to Mem0/Zep/Letta baselines
- `prime_context` MCP tool returns combined results in under 100ms (excluding LLM call if index is cached)
- Index stays under 1000 tokens for graphs up to 10K nodes

## Open Questions

- Should the compressed index include temporal annotations ("as of March 2026, Alice leads...")?
- Should domain assignment be inferred from node type when not explicitly provided?
- What Ollama model produces the best index compression? (Mistral 7B, Llama 3, Phi-3?)
- Should the benchmark harness also evaluate Mem0 and Zep for direct comparison, or just Recall + ablation?
[/PRD]
