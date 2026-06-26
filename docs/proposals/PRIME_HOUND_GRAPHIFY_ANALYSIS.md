# Prime Hound — Offering Graphify's Capabilities as a Prime Product Extension

**Status:** Analysis / design proposal (not yet scheduled)
**Author:** product analysis, 2026-06-26
**Scope:** Compare [graphify.net](https://graphify.net) (`safishamsi/graphify`) to AllSource Prime, then design a new product — working name **Prime Hound** — that offers everything Graphify does, built as an extension of the Prime engine.

---

## 0. TL;DR

- **Graphify is not a hosted graph engine — it's an open-source CLI / AI-assistant skill.** You run `/graphify .` inside Claude Code, it parses a folder with Tree-sitter (AST) plus an LLM pass for docs, and emits three portable files: `graph.json`, an interactive `graph.html`, and `GRAPH_REPORT.md`. No server, no accounts, no database, no embeddings, no multi-tenant state. Its assets are **distribution** (YC S26, ~70K GitHub stars, "skill" installers into AI assistants) and a **DX narrative** ("71.5× fewer tokens per query", local-first privacy, confidence-tagged relationships).
- **Prime is the opposite shape:** a durable, event-sourced, multi-tenant **runtime** graph + vector + recall engine over AllSource Core. It has the things a flat `graph.json` structurally cannot have — durability, hybrid vector+graph retrieval, provenance/time-travel, sync, and a hosted multi-tenant API.
- **They are complementary, not competing.** Graphify is a **build-time artifact**; Prime is a **runtime engine**. The product opportunity is to put Graphify's ingestion + DX **on top of** Prime's durable backend: *Graphify gives you a snapshot; Prime Hound gives you a living map.*
- **The work splits cleanly into "borrow" vs "already have."** We must build the ingestion/extraction/viz/analytics/distribution layer (Graphify's strengths). We get durability, hybrid recall, multi-tenancy, and provenance for free (Graphify's deliberate gaps — and our differentiators).
- **Hard architectural constraint:** per-tenant/per-repo graph compute must live in the **Prime app layer** (and a new ingestion worker), never in Core's global projection engine. This is enforced by the `tenant-isolation-check` gate and documented in `PER_TENANT_PROJECTIONS.md` ("Why not Core"). Prime's hosted mode (`HostedPrime` + `TenantProjectionCache`, stateless over Core) is already the correct place — we extend it, we do not touch Core's hot path.

---

## 1. What Graphify actually is (correcting the premise)

The request was "offer all that graphify does." First the premise needs sharpening, because it changes the design.

There are several unrelated products named "Graphify". The one at **graphify.net is `safishamsi/graphify`** — an MIT-licensed Python CLI, distributed on PyPI as `graphifyy`, backed by Y Combinator (S26). Its own homepage describes it as:

> "A Claude Code skill. Type `/graphify` in Claude Code — it reads your files, builds a knowledge graph, and gives you back structure you didn't know was there."

It is a **build-time tool that produces files**, not a service you call at runtime. This matters: "offer all that Graphify does" does **not** mean "build a competing SaaS graph database." It means "be able to turn any folder of code/docs into a rich, queryable knowledge graph for AI assistants — and then do the things Graphify deliberately chose not to."

### Graphify's capability surface (from the GitHub README, authoritative)

| Area | What it does |
|---|---|
| **Code ingestion** | Tree-sitter AST extraction across **13 languages** (Python, TS, JS, Go, Rust, Java, C, C++, Ruby, C#, Kotlin, Scala, PHP). 100% on-device; code never leaves the machine. ($0, no LLM.) *(Third-party writeups cite up to 33 grammars + SQL/infra — treat as aspirational vs the 13 the README ships.)* |
| **Multi-modal ingestion** | Docs (MD/HTML/rST), Office, Google Workspace, PDFs, images, audio/video (local Whisper transcription), YouTube, arXiv URLs. These go through an **LLM semantic-extraction** pass (only descriptions sent, not raw files). |
| **Data model** | NetworkX property graph → `graph.json`. Nodes: functions, classes, modules, docs, concepts. Typed edges: `calls`, `imports`, `references`, `depends_on`, `defines`, `explains`, `uses`, `extends`, `implements`. |
| **Confidence tagging** | Every relationship tagged `EXTRACTED` (AST-certain), `INFERRED` (LLM, with score), or `AMBIGUOUS`. Headline differentiator. |
| **Query** | `query "..."` (subgraph + LLM re-rank), `path "X" "Y"`, `explain "X"`. **No vector store** — "graph topology itself provides similarity signals." |
| **Analytics** | **Leiden community detection**; `GRAPH_REPORT.md` surfaces "god nodes" (most-connected), surprising cross-module links. |
| **Visualization** | Self-contained interactive `graph.html` (force-directed, click/filter/search/expand). ~5k-node practical ceiling. |
| **PR / impact** | `prs --triage` (AI-ranked review queue), `prs 42` (deep dive with graph impact), `--conflicts` (merge-order risk). *(Third-party; not in the conservative README.)* |
| **Git-native** | `hook install` → post-commit incremental rebuild of only changed files (AST-only = $0). Auto-merge driver so `graph.json` never conflicts. Relative paths → safe to commit. |
| **Exports** | `graph.json`, `graph.html`, `GRAPH_REPORT.md`, Markdown wiki, Obsidian vault, SVG, GraphML (Gephi/yEd), Cypher (Neo4j/FalkorDB), Mermaid call-flow HTML. |
| **API surface** | **MCP server mode** (stdio + HTTP) is the only programmatic interface. No hosted REST API. |
| **Distribution** | "Skill" installers into ~20 AI assistants (README emphasizes Claude Code). `graphify install --platform <name>` writes e.g. `.claude/skills/graphify/SKILL.md`. |
| **Pricing** | Free / MIT. Your only cost is **your own** LLM tokens for the doc pass. Commercial layer **"Penpax"** (always-on, on-device graph of meetings/email/browser/files/code) announced, unlaunched, no public pricing. |

### Graphify's *deliberate* gaps (these are the design opening)

1. **Static file, not a live service.** The graph is a snapshot; between commits it's stale. No real-time, no server, no shared state.
2. **No embeddings / no vectors.** Pure topology + LLM re-rank. No true semantic/hybrid retrieval.
3. **~5k-node viz ceiling.**
4. **No durable multi-user / multi-tenant state.** It's per-machine files.
5. **No real query language for end users.** Queries are LLM-mediated.
6. **No provenance / time-travel of the graph** beyond `git log` on `graph.json`.
7. **Code-only memory.** It models the codebase, not the working session, the agent's memory, or the team's accumulated knowledge.

Every one of these is something Prime already does.

---

## 2. What Prime is today

Source-verified inventory (see `apps/core/src/prime/`, `apps/prime-mcp/`, `apps/prime/`).

| Area | Prime today |
|---|---|
| **Shape** | A **runtime engine**, not a file. Property graph + vectors + hybrid recall over AllSource Core (durable event store: WAL + Parquet + DashMap). |
| **Data model** | Nodes `node:{type}:{id}` (typed, arbitrary JSON props, `domain`, `labels`, soft-delete, timestamps). Directed edges with `relation`, optional `properties`, optional `weight` (0.0–1.0). Everything stored as immutable events (`prime.node.created/updated/deleted`, `prime.edge.created/deleted`, `prime.vector.stored`). |
| **Graph ops** | `neighbors` (BFS ego network, direction + relation + depth), `shortest_path` (BFS/Dijkstra), `search` (by type). Materialized projections: NodeState, Adjacency, ReverseIndex, NodeTypeIndex, GraphStats, Schema, Contradiction, CrossDomain, DomainIndex, VectorIndex (HNSW). |
| **Vectors** | `all-MiniLM-L6-v2` (384-dim) via `fastembed` ONNX, in-process or remote (`PRIME_EMBED_ENDPOINT`). **HNSW** index (`instant-distance`). `embed`, `similar`, vector search. |
| **Recall** | `RecallEngine` — hybrid score `recency*0.2 + similarity*0.5 + proximity*0.3`. Tiered context L0/L1/L2 (stats → conversation → full hybrid). Auto-generated compressed markdown index (`prime_index`). |
| **Provenance / time-travel** | Full immutable audit trail (`prime_history`). Declarative projections (`define_projection` with LastWrite / HighestPriority / MostSpecific / MergeArray merge policies), `project_node`, `node_provenance` ("which event set this field?"). |
| **API** | 18 MCP tools (stdio + HTTP `/mcp`) **plus** a real REST API (`/api/v1/prime/nodes|edges|vectors|shortest-path|recall|graph|stats`, `graph.html`, `diff`). |
| **Deployment** | Two modes. **Local/embedded:** `allsource-prime --data-dir ~/.prime/memory --mode mcp`, single-writer (`prime.lock`), optional `--sync-to <Core> --api-key`. **Hosted/stateless:** multi-tenant `HostedPrime` + `TenantProjectionCache` (LRU, TTL) over remote Core; Fly app `allsource-prime`. |
| **Viz** | A self-contained `GET /api/v1/prime/graph.html` bubble viewer — exists but basic (no rich filter/expand/layout). |

### Prime's gaps vs Graphify

1. **No codebase ingestion** — no AST/Tree-sitter extraction of code into the graph.
2. **No multi-modal ingestion** — no PDF/image/video/doc → graph extraction pipeline.
3. **No confidence tagging** taxonomy (`EXTRACTED`/`INFERRED`/`AMBIGUOUS`).
4. **Weak visualization** — basic bubble viewer, no interactive exploration.
5. **No graph analytics** — no PageRank, no community detection (Leiden/Louvain), no "god node" surfacing.
6. **No PR/impact tooling.**
7. **No assistant-skill distribution** — no `/prime` slash-command installers across AI assistants.
8. **No git-hook incremental rebuild.**

---

## 3. Head-to-head

| Dimension | Graphify | Prime (today) | Prime Hound (proposed) |
|---|---|---|---|
| **Product shape** | Build-time CLI → files | Runtime engine + API | Runtime engine **with** Graphify-grade ingestion + DX |
| **Codebase → graph** | ✅ Tree-sitter, 13 langs | ❌ | ✅ (Phase 1) |
| **Multi-modal → graph** | ✅ LLM extraction | ❌ | ✅ (Phase 2) |
| **Confidence tagging** | ✅ | ❌ | ✅ via edge `weight`+`properties.confidence` |
| **Vectors / hybrid retrieval** | ❌ topology-only | ✅ HNSW + hybrid recall | ✅ **(differentiator)** |
| **Live / incremental** | ❌ static file | ✅ event-sourced | ✅ **(differentiator)** |
| **Multi-tenant / team graphs** | ❌ per-machine files | ✅ hosted stateless | ✅ **(differentiator)** |
| **Provenance / time-travel** | ⚠️ git-diff of JSON | ✅ event history + node_provenance | ✅ **(differentiator)** |
| **Graph analytics (PageRank/Leiden)** | ✅ Leiden, god-nodes | ❌ | ✅ (Phase 3) |
| **Interactive viz** | ✅ (≤5k nodes) | ⚠️ basic | ✅ (Phase 3, no 5k ceiling — server-side) |
| **PR impact analysis** | ✅ | ❌ | ✅ (Phase 3) |
| **Query interface** | LLM-mediated subgraph | MCP tools + REST | MCP + REST + (optional) graph query |
| **Assistant-skill distribution** | ✅ ~20 platforms | ❌ | ✅ (Phase 4) |
| **Git-hook rebuild** | ✅ | ❌ | ✅ (Phase 4) |
| **Local-first / privacy** | ✅ on-device | ✅ embedded mode + sync | ✅ embedded mode (parity) |
| **Cost (code-only)** | $0 | n/a | $0 (local AST, no LLM) |
| **Distribution moat** | ✅ 70K stars, YC | ❌ | ❌ (must earn) |

**The one-line read:** Graphify wins on distribution and on multi-modal ingestion breadth *today*. Prime wins on everything a static file cannot do. The extension closes our ingestion gap while keeping the runtime advantages — producing a strict superset of Graphify's *capabilities* (not its install base).

---

## 4. The product: Prime Hound

> **Positioning:** "The living knowledge graph for AI coding assistants. Graphify gives you a snapshot — Hound remembers."

Prime Hound is a Prime-backed product that ingests a codebase (and later docs/multi-modal sources) into a **durable, queryable, optionally-shared** knowledge graph, exposed to AI assistants over MCP — with hybrid vector+graph retrieval, provenance, and incremental live updates that a flat `graph.json` can't offer.

Crucially, Hound unifies two graphs that are separate everywhere else: **the code graph** (what Graphify builds) and **the agent's working memory** (Prime's original purpose) live in **one event store, one query surface**. An assistant can ask "what connects the login form to the users table?" and "what did we decide about auth last week?" against the same graph. Neither Graphify nor vanilla Prime does this.

### 4.1 Architecture (respecting the Core/QS/Prime boundary)

```
┌──────────────────────────────────────────────────────────────────────┐
│  Hound ingestion (NEW)  — runs client-side (CLI) or as a worker        │
│                                                                        │
│   ┌────────────────┐   ┌──────────────────────┐                       │
│   │ Tree-sitter AST │   │ LLM semantic extractor│  (docs/PDF/img/...)  │
│   │  (code, $0)     │   │  (BYO key / hosted)   │                       │
│   └───────┬─────────┘   └───────────┬──────────┘                       │
│           └──────────────┬──────────┘                                  │
│                          ▼                                             │
│           emits prime.node.created / prime.edge.created /              │
│           prime.vector.stored  (tenant-scoped, domain = repo)         │
└──────────────────────────┬─────────────────────────────────────────────┘
                           │ events (tenant-scoped)
                           ▼
┌──────────────────────────────────────────────────────────────────────┐
│  AllSource Core (Rust) — UNCHANGED                                     │
│  Durable event log (WAL+Parquet+DashMap), tenant-scoped. Stores the    │
│  Hound graph as ordinary prime.* events. No per-tenant Hound compute.  │
└──────────────────────────┬─────────────────────────────────────────────┘
                           │ tenant-scoped read
                           ▼
┌──────────────────────────────────────────────────────────────────────┐
│  allsource-prime app (extended)  — per-tenant compute lives HERE       │
│   • HostedPrime + TenantProjectionCache (exists)                       │
│   • NEW: graph analytics over the warm projections (PageRank, Leiden)  │
│   • NEW: Hound read APIs (impact, god-nodes, report)                   │
│   • Existing: neighbors / shortest_path / recall / vector search       │
└──────────────────────────┬─────────────────────────────────────────────┘
                           │ MCP + REST
                ┌──────────┴───────────┐
                ▼                      ▼
        AI assistants            Web viewer (apps/web)
        (/prime-hound skill)     interactive graph UI
```

**Why this placement is mandatory, not stylistic:** `CLAUDE.md` and `PER_TENANT_PROJECTIONS.md` forbid per-tenant read-model compute in Core's global projection engine (it's on the ingest hot path; enforced by `tenant-isolation-check`, override only via `// CORE_PROJECTION_OK:`). Hound graphs are per-tenant/per-repo, so **all Hound-specific folding and analytics run in the `allsource-prime` app** (already stateless-multi-tenant over Core) or in the ingestion worker — never in Core. Core only stores `prime.*` events, exactly as it does today.

### 4.2 How each Graphify capability maps onto Prime

| Graphify capability | Prime Hound implementation | Build? |
|---|---|---|
| Tree-sitter AST → graph | New `hound-extract` crate (Rust, `tree-sitter` grammars) → emits `prime.node/edge.created`. Code nodes use `node:fn:`, `node:class:`, `node:module:` types; relations `calls`/`imports`/`defines`/etc. | **Build** |
| Multi-modal LLM extraction | Reuse the AI-inbox-style extractor pattern; emit nodes + edges + **vectors** (Prime embeds them → hybrid retrieval, which Graphify lacks). | **Build** |
| Confidence tags | Map `EXTRACTED→weight 1.0`, `INFERRED→weight=score`, `AMBIGUOUS→weight<0.5`; store the enum in edge `properties.confidence`. Prime edges already carry `weight` + `properties`. | **Trivial** |
| `query` / `path` / `explain` | Already exist: `prime_recall` (better — hybrid + vectors), `prime_shortest_path`, `prime_neighbors`. | **Have** |
| Leiden communities / PageRank / god-nodes | New read-side analytics over the materialized Adjacency/ReverseIndex projections in the prime app (tenant-scoped, cached). | **Build** |
| `graph.html` interactive viz | Upgrade existing `GET /api/v1/prime/graph.html` → real interactive viewer (filter/search/expand/layout); or a first-class page in `apps/web`. Server-side paging removes the ~5k ceiling. | **Build** |
| `GRAPH_REPORT.md` | Generate from `prime_stats` + new analytics (god-nodes, cross-domain edges via existing CrossDomain projection). | **Build (small)** |
| PR impact analysis | New: diff two graph snapshots (Prime already has `GET /api/v1/prime/diff`) + git integration to map changed files → impacted nodes. | **Build** |
| Git-hook incremental rebuild | New CLI subcommand `prime hound hook install`; reuse incremental SHA-cache idea; emit only deltas as events (event-sourcing makes deltas natural). | **Build** |
| Exports (wiki, Obsidian, GraphML, Cypher, Mermaid) | New serializers over the graph read API. Cheap, additive. | **Build (small)** |
| MCP server | **Already shipped** (`apps/prime-mcp`, stdio + HTTP). Add Hound-specific tools (`hound_ingest`, `hound_impact`, `hound_report`). | **Extend** |
| Assistant-skill installers | New `prime hound install --platform <name>` writing `.claude/skills/...`, `.cursor/rules/...`, etc. Mechanical; mirror Graphify's installer. | **Build** |
| Local-first / privacy | **Already have** — embedded mode parses locally; code-only graphs need no LLM and never leave the machine. Sync is opt-in. | **Have** |

**Net:** ~9 things to build, 4 of them small; 4 things we already have (and 4 of the "have"s are the differentiators Graphify can't match).

### 4.3 What we get for free that Graphify can never have

These are the headline differentiators — they're not roadmap items, they're inherent to building on Prime:

1. **Hybrid retrieval.** Graphify is topology-only and explicitly has no vectors. Hound embeds every node/doc (`all-MiniLM-L6-v2`, HNSW) and ranks with `recency + similarity + graph-proximity`. Semantic queries that miss on pure topology hit on Hound.
2. **Living graph.** Event-sourced ⇒ incremental updates, real-time subscription (Core WS), no stale snapshot, no merge conflicts on a JSON blob.
3. **Team / shared graphs.** Hosted multi-tenant mode ⇒ a team shares one repo graph; Graphify's files are per-machine.
4. **Provenance + time-travel.** `prime_history` + `node_provenance` ⇒ "when did this edge appear, and which commit/event created it?" Graphify offers only `git log graph.json`.
5. **Code graph + agent memory in one store.** The unique combination: the assistant's durable memory and the codebase map are the same graph.

---

## 5. Phased roadmap

Each phase is independently shippable and mirrors Graphify's own wedge-then-expand path.

**Phase 1 — Hound Code (the wedge).** Tree-sitter AST extractor (start with the languages our users actually use — Rust, TS/JS, Python, Go, Elixir), emitting `prime.*` events into local embedded Prime. `prime hound <path>` CLI. Code-only ⇒ $0, on-device, privacy parity with Graphify. Queryable immediately via existing MCP tools; opt-in sync to hosted. *Exit: a developer can `prime hound .` and ask their assistant graph questions about their repo, durably.*

**Phase 2 — Semantic & multi-modal.** LLM semantic-extraction pass for docs/PDF/etc., emitting nodes + edges + **vectors**. Confidence tagging. This is where Hound visibly beats Graphify: hybrid vector+graph recall instead of topology-only. BYO LLM key (like Graphify) or hosted usage-billed.

**Phase 3 — Analytics, viz, impact.** PageRank + Leiden communities + god-node surfacing (read-side, in the prime app). Interactive web viewer (no 5k ceiling). `GRAPH_REPORT.md` generator. PR impact analysis via graph diff + git. *Exit: feature parity with Graphify's analytics/report/PR story.*

**Phase 4 — Distribution & collaboration.** Assistant-skill installers across the major AI assistants. Git-hook incremental rebuild. Team/shared hosted graphs (the thing Graphify structurally cannot do). Export formats. *Exit: a developer can install the `/prime-hound` skill in their assistant and a team can share one living graph.*

---

## 6. Go-to-market & pricing

Mirror Graphify's funnel, monetize where Prime adds what a file can't:

- **Free local CLI** (Phase 1): code-only graphs, on-device, $0 — same wedge as Graphify, but durable and syncable. This is the adoption driver; do not paywall it.
- **BYO-key semantic extraction** (Phase 2): free tool, user pays their own LLM tokens — exactly Graphify's model, removes a cost objection.
- **Hosted team graphs** (paid): per-seat or per-graph. This is the real revenue — multi-tenant shared, always-fresh graphs are the capability Graphify gave up. Bundles naturally with existing AllSource/Prime customers.
- **Hosted semantic extraction** (usage-billed): for teams that don't want to manage keys.

Anchor pricing to the **team/living-graph** value, not the local tool — the local tool is the top of the funnel.

---

## 7. Risks & honest assessment

1. **Distribution is Graphify's real moat, and we don't have it.** 70K stars + YC + installers into ~20 assistants is a genuine lead we will not out-distribute quickly. *Mitigation:* don't try to out-distribute on day one — win on capabilities a file can't have (live/team/hybrid/provenance), and seed adoption through existing AllSource/Prime/chronis users where we already have a relationship.
2. **Multi-modal extraction is a lot of surface.** Graphify spent real effort on PDF/image/video/Whisper pipelines. *Mitigation:* Phase it; ship code-only first (where Tree-sitter is conventional and $0), add modalities by demand.
3. **Tree-sitter grammar breadth is ongoing work.** 13+ languages is real maintenance. *Mitigation:* start with the handful our users use; grammars are off-the-shelf, not novel research.
4. **Boundary discipline.** It is tempting to fold Hound analytics into Core's projection engine for speed. **Do not** — `tenant-isolation-check` will fail and it's the wrong architecture. All per-tenant Hound compute lives in the `allsource-prime` app or the ingestion worker.
5. **Penpax is the signal to watch.** Graphify's unlaunched commercial layer (always-on, on-device graph of meetings/email/browser/files/code) overlaps directly with Prime's *memory* use case — more than the free CLI does. If Penpax ships, the competition moves onto Prime's home turf (durable personal/agent memory), not just code graphs. Track it as the real competitive intent.

---

## 8. Recommendation

Build **Prime Hound** as a phased extension, starting with **Phase 1 (Hound Code)** — the highest-leverage, lowest-cost wedge that proves the thesis (durable, queryable code graph for assistants) using only Tree-sitter + the Prime engine we already run. It is a strict capability superset of Graphify with no new database, no Core changes, and immediate differentiation (durability + hybrid recall) the moment a graph exists.

The strategic bet: Graphify proved the demand and the DX; Prime already has the durable runtime that Graphify deliberately went without. We are not copying a competitor — we are giving their best idea the backend it's missing.

---

### Appendix A — Source notes

- **Graphify primary source:** `github.com/safishamsi/graphify` README (authoritative). `graphify.net` returns 403 to automated fetchers (Cloudflare). README ships **13 languages, Claude Code skill, 71.5× token-reduction claim** (their claim, unverified). Third-party writeups (AITroveX, Augment Code, GoPenAI) cite higher numbers (up to 33 grammars, ~20 platforms, PR-triage commands) — treated here as aspirational/marketing vs the conservative README.
- **Prime inventory:** source-verified in `apps/core/src/prime/` (`types.rs`, `vectors/`, `recall/`, `projections/declarative.rs`, `hosted.rs`, `tenant_cache.rs`), `apps/prime-mcp/src/tools.rs`, and the hosted deployment in `apps/prime/`.
- **Boundary constraint:** `docs/proposals/PER_TENANT_PROJECTIONS.md` and `CLAUDE.md` ("Per-tenant read-model compute lives in the Query Service, NOT Core"); enforced by `tooling/tenant-isolation-check/`.
