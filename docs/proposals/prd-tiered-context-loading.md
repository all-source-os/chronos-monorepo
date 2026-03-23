[PRD]
# PRD: Tiered Context Loading — L0/L1/L2 Recall for Prime

## Overview

Prime's recall engine currently operates in two modes: compressed index only (~800 tokens) or full hybrid recall (vector + graph + index, 2000–5000 tokens). There is no middle ground. Every `prime_context` or `prime_recall` call pays the full retrieval cost even when the agent is doing routine follow-ups in the same conversation.

Tiered context loading adds an explicit `ContextTier` enum (`L0`, `L1`, `L2`) to `RecallContextQuery`. The recall engine short-circuits at the requested tier, skipping expensive work that isn't needed:

| Tier | What it returns | Token budget | When to use |
|------|----------------|-------------|-------------|
| **L0** | Stats + active tool schema only | ~100–200 | Tool-only calls, no memory needed |
| **L1** | L0 + recent conversation nodes + relevant edges | ~500–1500 | Follow-ups in same conversation, same domain |
| **L2** | L1 + compressed index + vector search + graph expansion | ~2000–5000 | New topic, cross-domain question, explicit "remember" |

**Key insight:** recall-bench shows `full-recall` (L2) achieves ~89% F1 but is overkill for ~60–70% of agent turns. L1 handles follow-ups using `ConversationScope` event tagging + `RelevanceDecayProjection` recency scoring — both already exist in the codebase but aren't wired into the recall path.

**Depends on:** Recall engine (complete), ConversationScope (partially implemented — event tagging exists, conversation history query exists, but not integrated into recall).

## Goals

- Add `ContextTier` enum and `tier` field to `RecallContextQuery` with backward-compatible default (`L2`)
- Implement L0 recall path that returns only `PrimeStats` + domain list (~100 tokens)
- Implement L1 recall path that adds conversation-scoped recent nodes + edges without vector search
- Wire `RecallEngine` to short-circuit at the requested tier
- Add `tier` parameter to `prime_context` MCP tool
- Add auto-tier selection heuristic (optional, behind feature flag) that chooses tier based on conversation state
- Extend recall-bench to measure per-tier token costs and accuracy tradeoffs
- Demonstrate ≥50% token reduction on follow-up queries with <5% accuracy loss vs L2

## Non-Goals

- Changing the existing L2 behavior — this is additive
- Auto-embedding or auto-tier selection in v1 (manual tier selection first, auto-tier as follow-up)
- Modifying the compressed index generation — tiers control *what* gets returned, not how indexes are built
- MCP protocol changes — this is a parameter addition to existing tools

## Quality Gates

### Epic-Level
- `make quality-rust` passes
- All existing recall tests still pass (L2 is the default, no regression)
- New tier-specific tests pass: `cargo test -p allsource-core --features prime recall::tier`
- recall-bench CrossRef suite shows L1 vs L2 accuracy comparison

### Story-Level
- Each story has specific `cargo test` filter
- MCP tool changes verified via stdio test

## User Stories

### US-001: ContextTier Enum and Query Extension [Backend]
**Description:** As a developer, I want to specify a retrieval tier when calling `recall.context()`, so that the engine can skip unnecessary work for simple queries.

**Acceptance Criteria:**
- [ ] `ContextTier` enum in `apps/core/src/prime/recall/types.rs`:
  ```rust
  #[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
  pub enum ContextTier {
      L0,
      L1,
      #[default]
      L2,
  }
  ```
- [ ] `RecallContextQuery` gains `pub tier: ContextTier` field (default `L2` for backward compat)
- [ ] Existing tests pass without modification (default tier = L2 = current behavior)
- [ ] `cargo test -p allsource-core recall::` passes

### US-002: L0 Recall Path — Stats Only [Backend]
**Description:** As an agent, when I only need to know the shape of memory (how many nodes, which domains exist), I want a minimal retrieval that costs <200 tokens.

**Acceptance Criteria:**
- [ ] When `tier == L0`, `RecallEngine::context()` returns:
  - `index`: empty string
  - `vectors`: empty
  - `nodes`: empty
  - `edges`: empty
  - New `stats` field: `Option<PrimeStats>` with node/edge counts, domain list
  - `token_count`: actual token count of the stats JSON (~100–200)
- [ ] `RecallContext` struct gains `pub stats: Option<PrimeStats>` field
- [ ] `RecallEngine` needs access to `GraphStatsProjection` (add to constructor, or accept stats as parameter)
- [ ] Test: L0 context returns stats, zero vectors, zero nodes, token_count < 200
- [ ] Test: L0 context does NOT call `self.index().await` (no compressed index generation)

### US-003: L1 Recall Path — Conversation-Scoped Recent Context [Backend]
**Description:** As an agent in an ongoing conversation, I want recent nodes and edges from this conversation without paying for vector search or full graph expansion.

**Acceptance Criteria:**
- [ ] `RecallContextQuery` gains `pub conversation_id: Option<String>` field
- [ ] When `tier == L1`, `RecallEngine::context()`:
  1. Includes L0 stats
  2. Queries recent events tagged with `conversation_id` (last N events, configurable, default 20)
  3. Extracts nodes and edges from those events
  4. Includes 1-hop edges from those nodes (via adjacency projection, no BFS)
  5. Does NOT run vector search
  6. Does NOT generate compressed index
- [ ] `RecallEngine` needs access to `NodeStateProjection` + `AdjacencyListProjection` for node/edge lookups
- [ ] `RecallEngine` needs access to event store for conversation-scoped event queries
- [ ] Token budget enforced: if conversation context exceeds `max_tokens`, truncate oldest events first
- [ ] Test: L1 with conversation_id returns only nodes from that conversation + 1-hop neighbors
- [ ] Test: L1 without conversation_id returns most recent N nodes across all conversations
- [ ] Test: L1 token_count < L2 token_count for same knowledge base

### US-004: RecallEngine Short-Circuit Wiring [Backend]
**Description:** As a developer, I want `RecallEngine::context()` to dispatch to the correct tier implementation based on the query's `tier` field.

**Acceptance Criteria:**
- [ ] `RecallEngine::context()` match-dispatches on `query.tier`:
  ```rust
  match query.tier {
      ContextTier::L0 => self.context_l0(&query).await,
      ContextTier::L1 => self.context_l1(&query).await,
      ContextTier::L2 => self.context_l2(&query).await,
  }
  ```
- [ ] `context_l2()` is the existing `context()` implementation (extracted, not duplicated)
- [ ] Each tier method is independently testable
- [ ] Test: calling `context()` with default query (L2) produces identical output to before
- [ ] Test: L0 is measurably faster than L2 (no I/O, no compression)

### US-005: MCP Tool — Tier Parameter [MCP]
**Description:** As an AI agent using Prime via MCP, I want to specify a retrieval tier in `prime_context` calls so I can control token costs.

**Acceptance Criteria:**
- [ ] `prime_context` tool gains `tier` parameter:
  ```json
  "tier": { "type": "string", "enum": ["L0", "L1", "L2"], "description": "Retrieval depth. L0=stats only (~100 tokens). L1=recent conversation context (~500-1500 tokens). L2=full hybrid recall (~2000-5000 tokens). Default: L2." }
  ```
- [ ] `prime_context` tool gains `conversation_id` parameter:
  ```json
  "conversation_id": { "type": "string", "description": "Scope L1 retrieval to this conversation. Ignored for L0/L2." }
  ```
- [ ] `call_context()` in `tools.rs` maps string tier to `ContextTier` enum
- [ ] MCP tool description updated to explain tier selection guidance
- [ ] Test: MCP call with `tier: "L1"` returns conversation-scoped context

### US-006: Recall-Bench Tier Comparison [Tooling]
**Description:** As a developer, I want recall-bench to measure accuracy and token cost per tier, so I can validate the accuracy/cost tradeoff.

**Acceptance Criteria:**
- [ ] CrossRef benchmark runs each query at all three tiers
- [ ] Output table includes per-tier columns:
  ```
  | Tier | Precision | Recall | F1   | Avg Tokens | Avg Latency |
  |------|-----------|--------|------|------------|-------------|
  | L0   | —         | —      | —    | 120        | 0.1ms       |
  | L1   | 72.0%     | 75.0%  | 73.5%| 800        | 0.5ms       |
  | L2   | 87.5%     | 91.2%  | 89.3%| 2300       | 2.3ms       |
  ```
- [ ] L1 accuracy within 20 percentage points of L2 for same-domain queries
- [ ] L1 token cost ≤50% of L2 on average

### US-007: Auto-Tier Heuristic (Optional, Feature-Gated) [Backend]
**Description:** As an agent framework developer, I want Prime to automatically select the optimal tier based on conversation state, so agents don't need to manually choose.

**Acceptance Criteria:**
- [ ] Behind `prime-auto-tier` feature flag (not enabled by default)
- [ ] `AutoTierSelector` struct with `fn select(&self, query: &RecallContextQuery, conversation_state: &ConversationState) -> ContextTier`
- [ ] Heuristic rules:
  - No `conversation_id` + no `query` text → L0
  - Same `conversation_id` as last call + query domain unchanged → L1
  - New topic detected (query domain differs from recent nodes) → L2
  - Explicit `include_index: true` → L2 (override)
- [ ] `ConversationState` tracks: last conversation_id, last query domains, turn count
- [ ] When `tier` is not explicitly set and `prime-auto-tier` is enabled, auto-select
- [ ] Test: auto-tier selects L1 for follow-up, L2 for topic switch

## Architecture

### RecallEngine Dependencies (After)

```
RecallEngine
├── domain_index: Arc<DomainIndexProjection>      (existing)
├── cross_domain: Arc<CrossDomainProjection>       (existing)
├── compressor: IndexCompressor                     (existing)
├── node_state: Arc<NodeStateProjection>            (NEW — for L1 node lookups)
├── adjacency: Arc<AdjacencyListProjection>         (NEW — for L1 edge expansion)
├── graph_stats: Arc<GraphStatsProjection>          (NEW — for L0 stats)
└── event_store: Arc<EventStore>                    (NEW — for L1 conversation queries)
```

The `Prime` facade already holds all these projections. The change is passing them through to `RecallEngine` (currently it only receives `DomainIndexProjection` + `CrossDomainProjection`).

### Data Flow Per Tier

```
L0:  query → graph_stats.stats() → serialize → return          (~0.1ms)
L1:  query → L0 + event_store.query(conversation_id, limit=20)
           → node_state.get(ids) + adjacency.neighbors(ids, depth=1)
           → serialize → return                                  (~0.5ms)
L2:  query → L1 + compressor.compress() + vector_search(query)
           + bfs_expand(matches, depth) → serialize → return    (~2-5ms)
```

### Wire Format

`RecallContext` response gains one field:

```rust
pub struct RecallContext {
    pub index: String,            // L2 only (empty for L0/L1)
    pub vectors: Vec<RankedMemory>, // L2 only (empty for L0/L1)
    pub nodes: Vec<Node>,         // L1+L2
    pub edges: Vec<Edge>,         // L1+L2
    pub stats: Option<PrimeStats>, // L0+L1+L2 (NEW)
    pub tier: ContextTier,        // Which tier was used (NEW)
    pub token_count: usize,       // Actual tokens returned
}
```

## Implementation Order

1. **US-001** — Type additions (ContextTier, query field). No behavior change.
2. **US-002** — L0 path. Simplest, validates the short-circuit pattern.
3. **US-004** — Refactor `context()` into tier dispatch. Extract L2.
4. **US-003** — L1 path. Needs the most new wiring (event store access).
5. **US-005** — MCP tool parameter. Expose to agents.
6. **US-006** — Benchmark validation. Proves the tradeoff.
7. **US-007** — Auto-tier. Only after manual tiers are validated.

## Risks

| Risk | Mitigation |
|------|-----------|
| L1 accuracy too low for useful agent work | Benchmark gate: L1 must be within 20pp of L2 for same-domain queries. If not, increase conversation window or add domain-filtered index excerpt. |
| RecallEngine constructor bloat (too many Arc params) | Bundle projections into a `RecallDependencies` struct. Single parameter. |
| ConversationScope event query is slow (scans WAL) | Add conversation_id to event metadata index. Or: maintain a small in-memory ring buffer of recent conversation events per conversation_id. |
| Auto-tier heuristic makes wrong choices | Feature-gated, off by default. Manual tier selection is the primary interface. |
