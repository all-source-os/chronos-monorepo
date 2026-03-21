# Prime Documentation & Use Cases Plan

> **Status**: Active
> **Date**: 2026-03-21
> **Goal**: Surface Prime to both AI agents (via MCP) and humans (via web, docs, video)

---

## Principle

Prime has two audiences that consume docs differently:
- **AI agents** read MCP tool descriptions, system prompts, and structured resources. They don't browse websites.
- **Humans** read READMEs, tutorials, interactive demos. They need *why* before *how*.

Every deliverable below is tagged with its audience.

---

## Phase 1: Agent-Facing Documentation (Week 1)

### 1.1 Improve MCP tool descriptions [Agent]

**Current state**: Tool descriptions are one-liners.
**Target**: Each tool description teaches *when* to use it and *what patterns work*.

**Files**: `apps/prime-mcp/src/tools.rs`

Example improvement:
```
prime_recall: "Hybrid recall: vectors + graph + temporal.
Use this as your PRIMARY retrieval tool when you need to answer
'what do I know about X?' Include a query embedding for best results.
Combine with prime_index for cross-domain questions like
'how does X relate to Y?'"
```

**Acceptance criteria**:
- [ ] Every tool description includes: what it does, when to use it, what to combine it with
- [ ] Descriptions use agent-optimized language (imperative, no marketing)
- [ ] `prime_index` description explains system prompt injection pattern
- [ ] `prime_recall` vs `prime_context` distinction is clear

### 1.2 Agent onboarding prompt template [Agent]

**File**: `docs/guides/PRIME_AGENT_PROMPT.md`

A copy-paste system prompt template for humans configuring their agent:

```markdown
You have access to AllSource Prime, a persistent memory engine.
Your knowledge is organized as a graph with domains.

## On conversation start
- Call prime_index to understand what you already know
- Use the compressed index to orient your responses

## When you learn something new
- prime_add_node to create the entity (tag with domain)
- prime_embed to make it semantically searchable
- prime_add_edge to connect to existing knowledge

## When answering questions
- prime_recall for specific factual queries (needs embedding)
- prime_context for cross-domain questions (includes compressed index)
- prime_neighbors to explore around a known entity

## When correcting knowledge
- prime_forget to soft-delete (preserved in history)
- prime_history to see what changed and when
```

**Acceptance criteria**:
- [ ] Prompt template covers: startup, learning, querying, correcting
- [ ] Tested with Claude Desktop — agent follows the patterns
- [ ] Includes Claude Desktop config JSON snippet

### 1.3 MCP cookbook resource [Agent]

**File**: `apps/prime-mcp/src/tools.rs` (add as MCP resource)

A `prime://cookbook` read-only resource that agents can access for usage patterns.

**Content**:
- Common patterns: learn → recall, cross-domain query, track changes
- Anti-patterns: orphaned vectors, using search instead of recall
- Token budget tips: when to use `max_tokens`, when to skip `include_index`

**Acceptance criteria**:
- [ ] `resources/list` returns `prime://cookbook` alongside `prime://auto-context`
- [ ] `resources/read` for `prime://cookbook` returns markdown cookbook
- [ ] Content is under 500 tokens (agents need it concise)

---

## Phase 2: Human-Facing Documentation (Week 1-2)

### 2.1 README with captured example output [Human]

**File**: `apps/prime-mcp/README.md` (update), `apps/core/README.md` (add Prime section)

Run each example, capture output, embed in README:

```bash
$ cargo run --features prime --example prime_graph
Created: Alice, Bob, Project
Graph: 3 nodes, 3 edges
Project team: Alice (engineer), Bob (manager)
```

**Acceptance criteria**:
- [ ] `prime_graph` output captured and shown
- [ ] `prime_vectors` output captured and shown
- [ ] `prime_recall` output captured and shown (compressed index visible)
- [ ] Root `README.md` has a "## Agent Memory (Prime)" section linking to examples

### 2.2 Hosted `cargo doc` API reference [Human]

**Deployment**: GitHub Actions → GitHub Pages at `all-source.xyz/docs/api/`

```yaml
- run: cargo doc --no-deps --features prime-recall,prime-vectors
- uses: peaceiris/actions-gh-pages@v4
  with:
    publish_dir: target/doc
```

**Acceptance criteria**:
- [ ] `cargo doc` builds without errors for `prime-recall` + `prime-vectors`
- [ ] GitHub Actions workflow publishes to Pages on release
- [ ] Link from `all-source.xyz/docs` to API reference
- [ ] Prime module docs have the Quick Start example visible

### 2.3 Guides section on all-source.xyz [Human]

**Location**: `apps/web/src/app/(marketing)/docs/prime/`

Pages:
1. **Quickstart** — install, Claude Desktop config, first node+edge+query
2. **Concepts** — compressed index, domains, cross-domain reasoning, time-travel
3. **MCP Setup** — detailed Claude Desktop integration with screenshots
4. **HTTP API** — endpoint reference with curl examples
5. **Embedded Usage** — using `allsource-core` as a Rust library

**Acceptance criteria**:
- [ ] 5 docs pages created with MDX content
- [ ] Navigation sidebar includes Prime section
- [ ] Each page has "Edit on GitHub" link
- [ ] Code blocks have copy buttons

---

## Phase 3: Use Case Pages (Week 2)

### 3.1 Use case content [Human]

**Location**: `apps/web/src/app/(marketing)/solutions/agent-memory/`

Story-driven pages, not feature lists:

| Use Case | Story | Key Demo Moment |
|----------|-------|-----------------|
| **Personal AI assistant** | Claude remembers your project context across sessions | Next day, Claude recalls yesterday's decisions |
| **Multi-agent knowledge sharing** | Three agents work on different parts, share findings | Agent B recalls what Agent A discovered |
| **Incident response memory** | Oncall agent remembers every past incident | "What happened last time this alert fired?" |
| **Research assistant** | Read 50 papers, build knowledge graph, find connections | Compressed index surfaces unexpected cross-domain link |
| **Code review context** | Agent remembers past review feedback | "Last time you said X about error handling" |

**Acceptance criteria**:
- [ ] 5 use case pages with narrative structure
- [ ] Each includes: problem → solution → code snippet → result
- [ ] SEO meta tags: "AI agent memory", "persistent agent", "agent knowledge graph"
- [ ] Link to interactive demo (Phase 4)

### 3.2 Comparison page [Human]

**Location**: `apps/web/src/app/(marketing)/solutions/agent-memory/compare/`

Pull from `docs/articles/zer0dex-comparison.md`:

| Feature | zer0dex | Mem0 | Letta | Zep | **AllSource Prime** |
|---------|---------|------|-------|-----|---------------------|
| Compressed index | Manual | No | No | No | Auto-generated |
| Temporal queries | No | No | No | Yes | Yes |
| Provenance | No | No | Partial | Partial | Full event audit |
| Cross-domain recall | 80% | ~50% | ~37% | ~85% | 80%+ |
| Offline/embedded | Yes | No | No | Optional | Yes + sync |
| Latency | 70ms | Variable | Variable | Variable | 12μs |
| Cost | $0 | $0-249/mo | Cloud | Cloud | $0 |

**Acceptance criteria**:
- [ ] Comparison table rendered with proper styling
- [ ] Sources cited with links
- [ ] "How we measured" section explaining benchmarks
- [ ] CTA: "Try it yourself" → install command

---

## Phase 4: Interactive Demo (Week 2-3)

### 4.1 Deploy Prime HTTP server [Infra]

**Target**: `allsource-prime.fly.dev` or custom subdomain

```bash
allsource-prime --mode http --port 3905 --data-dir /data/prime
```

Pre-seed with revenue/engineering/product dataset from `prime_recall` example.

**Acceptance criteria**:
- [ ] Fly.io deployment with persistent volume
- [ ] Health check at `/health` returns 200
- [ ] CORS configured for `all-source.xyz`
- [ ] Pre-seeded with demo data on first boot

### 4.2 Interactive playground component [Human]

**Location**: `apps/web/src/app/dashboard/demo/prime/`

Three-panel layout:

```
┌─────────────────┬──────────────────┬─────────────────┐
│  Graph Builder   │  Compressed      │  Ask a Question │
│                  │  Index           │                 │
│  [+ Node]        │  (auto-updates)  │  [query input]  │
│  [+ Edge]        │                  │  [results]      │
│  [vis: D3/Cyto]  │                  │                 │
└─────────────────┴──────────────────┴─────────────────┘
```

**Left**: Force-directed graph (Cytoscape.js). Add nodes via form, drag to connect. Nodes colored by domain.

**Middle**: Compressed index markdown, auto-regenerated when graph changes. Shows token count.

**Right**: Query input → calls `/api/v1/prime/recall` → shows ranked results with domain highlighting.

**Acceptance criteria**:
- [ ] Users can add nodes with domain tags
- [ ] Users can draw edges between nodes
- [ ] Compressed index updates within 1 second of graph change
- [ ] Query returns results with relevance scores
- [ ] Pre-loaded with demo data (can be reset)
- [ ] Works on mobile (responsive layout)

---

## Phase 5: Video & Social (Week 3)

### 5.1 Terminal recordings [Human]

Use asciinema to record the three examples:

```bash
asciinema rec prime-graph.cast -c "cargo run --features prime --example prime_graph"
asciinema rec prime-vectors.cast -c "cargo run --features prime-full --example prime_vectors"
asciinema rec prime-recall.cast -c "cargo run --features prime-recall --example prime_recall"
```

Embed on docs pages using `<asciinema-player>`.

**Acceptance criteria**:
- [ ] 3 recordings created and uploaded
- [ ] Embedded in quickstart docs page
- [ ] Playback speed set to 1.5x (engineers are impatient)

### 5.2 Demo video: 60 seconds [Human + Social]

Screen recording of Claude Desktop with Prime:
1. User asks "what do you know about our project?"
2. Claude calls `prime_index` → compressed index shown
3. User says "Alice is now leading the security team"
4. Claude calls `prime_add_node` + `prime_add_edge`
5. User asks "how does security relate to engineering?"
6. Claude calls `prime_context` → cross-domain result

Narrated with captions. Post to X, YouTube, dev.to.

**Acceptance criteria**:
- [ ] Under 60 seconds
- [ ] Captions (accessibility + muted autoplay on social)
- [ ] Shows the "aha" moment: cross-domain recall working
- [ ] Link to install in video description

### 5.3 Technical walkthrough: 5 minutes [Human]

Architecture diagram → event store → projections → compressed index → cross-domain recall.

Code-level walkthrough for HN/dev.to audience:
- Show the `Projection` trait
- Show `CompressedIndexProjection` generating markdown
- Show `RecallEngine.context()` combining layers
- Show benchmark results

**Acceptance criteria**:
- [ ] Under 5 minutes
- [ ] Code is readable (large font, dark theme)
- [ ] Published on YouTube + embedded on docs
- [ ] Blog post companion piece

---

## Phase 6: Content Marketing (Week 3-4)

### 6.1 Blog posts [Human + SEO]

| Post | Target Keywords | Source |
|------|----------------|--------|
| "Why Your AI Agent's Memory is Broken" | AI agent memory, agent memory framework | Problem framing |
| "AllSource Recall: Compressed Index Doubles Cross-Domain Recall" | compressed index, cross-domain recall, zer0dex | `docs/articles/zer0dex-comparison.md` |
| "Building Agent Memory in Rust" | rust event sourcing, agent memory rust | Architecture decisions |
| "From zer0dex to AllSource: What We Learned" | zer0dex comparison | Honest comparison |
| "12μs Agent Memory: How We Got There" | performance, microsecond queries | Technical deep-dive |

**Acceptance criteria**:
- [ ] 5 blog posts published on `all-source.xyz/blog/`
- [ ] Each has Open Graph images
- [ ] Cross-posted to dev.to with canonical URL
- [ ] First post submitted to Hacker News

### 6.2 X/Twitter launch campaign [Social]

**Thread 1** (launch): 10-tweet thread from `docs/articles/zer0dex-x-thread.md`
**Thread 2** (demo): 4-tweet thread with video + benchmark numbers
**Thread 3** (technical): Architecture diagram + "here's how we built it"

**Acceptance criteria**:
- [ ] 3 threads drafted and reviewed
- [ ] Demo video attached to Thread 2
- [ ] Links to blog posts and GitHub
- [ ] Posted during US business hours (9am-12pm PT)

---

## Tracking

| Phase | Deliverable | Status | Owner |
|-------|------------|--------|-------|
| 1.1 | MCP tool descriptions | TODO | |
| 1.2 | Agent onboarding prompt | TODO | |
| 1.3 | MCP cookbook resource | TODO | |
| 2.1 | README with example output | TODO | |
| 2.2 | Hosted cargo doc | TODO | |
| 2.3 | Docs pages (5) | TODO | |
| 3.1 | Use case pages (5) | TODO | |
| 3.2 | Comparison page | TODO | |
| 4.1 | Fly.io deployment | TODO | |
| 4.2 | Interactive playground | TODO | |
| 5.1 | Terminal recordings (3) | TODO | |
| 5.2 | 60-second demo video | TODO | |
| 5.3 | 5-minute walkthrough | TODO | |
| 6.1 | Blog posts (5) | TODO | |
| 6.2 | X launch campaign (3 threads) | TODO | |
