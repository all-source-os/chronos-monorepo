# AllSource — Vision Document

> **Version**: 1.0
> **Date**: 2026-03-02
> **Current release**: v0.12.0

---

## One Sentence

AllSource is the event-sourced database that becomes the memory engine for AI agents — vectors, relationships, and temporal history in one embedded binary, no infrastructure required.

---

## Where We've Been

AllSource started as a Rust event store: append-only, durable, fast. The thesis was simple — events are the purest form of data, and an event store purpose-built in Rust could be 100x faster than PostgreSQL-backed alternatives.

That thesis held. Core does 469K events/sec with 11.9μs query latency, backed by a three-tier storage architecture (DashMap for speed, WAL for durability, Parquet for archival) that fits in a 15.7 MB Docker image.

### The Timeline

```
2025-12     v0.6.0    Performance optimizations (1M+ events/sec target)
2026-01     v0.7.x    Docker optimization, build hardening
2026-02-10  v0.8.0    Workspace builds, release automation
2026-02-12  v0.9.x    RBAC, OAuth (GitHub/Google), stream discovery APIs
2026-02-14  v0.10.0   PostgreSQL removal from event path (ADR-005)
                      Vector search (fastembed + HNSW)
                      simd-json zero-copy deserialization
                      Native ARM64 CI
2026-02-17  v0.10.5   Server-side projections with fold-on-read (ADR-006)
2026-02-19  v0.10.6   Query Service fixes, auth bypass for dev
2026-02-25  v0.10.7   Query ergonomics, duplicate detection, consumer patterns
2026-03-01  v0.11.0   Embedded Core library — all 8 phases complete
                      83 tests, production-ready embeddable Rust library
2026-03-01  v0.12.0   Network sync transport (HTTP pull/push)
                      Configurable conflict resolution (LWW, FWW, AppendOnly)
                      MCP tool emission tracker
                      WebSocket backpressure with batching
```

**17 releases in 3 months.** Each one sharpened the same thesis: Core IS the database. No external dependencies. No PostgreSQL in the event path. No Redis. Just Rust, events, and projections.

---

## Where We Are

### What Works (v0.12.0)

**AllSource Core** — the database engine:

| Capability | Status | Detail |
|-----------|--------|--------|
| Event ingestion | Production | 469K events/sec, atomic batch, single-lock |
| Queries | Production | 11.9μs p99 via DashMap, time-travel via `as_of` |
| WAL durability | Production | CRC32 checksums, configurable fsync, crash recovery |
| Parquet archival | Production | Snappy compression, periodic flush, columnar analytics |
| Projections | Production | Fold-on-read + continuous, 9 built-in templates |
| Schema registry | Production | JSON Schema validation, compatibility modes |
| Snapshots | Production | Manual + automatic, snapshot-aware folding |
| Vector search | Production | fastembed (384-dim) + HNSW index |
| Keyword search | Production | BM25 via tantivy |
| EventQL | Production | SQL via DataFusion over event streams |
| Multi-tenancy | Production | Full isolation, per-tenant quotas |
| Embedded library | Production | `EmbeddedCore` facade, 83 tests, ~30MB rlib |
| Bidirectional sync | Production | HLC + CRDT, HTTP pull/push transport |
| Merge strategies | Production | AppendOnly, LastWriteWins, FirstWriteWins per event type |
| Replication | Designed | Leader-follower via WAL shipping |
| WebSocket streaming | Production | Backpressure, configurable batching |

**Full stack** — five services, four languages:

```
┌──────────────────────────────────────────────────────────────┐
│                     Dashboard (Next.js)                        │
│  Events explorer · API keys · Billing · Pipelines · Demo zone │
└──────────────────────┬───────────────────────────────────────┘
                       │
┌──────────────────────▼───────────────────────────────────────┐
│              Query Service (Elixir/Phoenix, port 3902)        │
│  API gateway · Server-side projections · WebSocket proxy      │
└───────┬──────────────────────────────────┬───────────────────┘
        │                                  │
┌───────▼───────────┐           ┌──────────▼──────────────────┐
│ Control Plane     │           │ AllSource Core (Rust)        │
│ (Go, port 3901)   │           │ (port 3900)                  │
│                   │           │                              │
│ JWT/OAuth         │           │ DashMap + WAL + Parquet      │
│ RBAC (4 roles)    │           │ 469K events/sec              │
│ LemonSqueezy      │           │ 11.9μs queries              │
│ OpenTelemetry     │           │ 1,489 tests passing          │
└───────────────────┘           └──────────────────────────────┘

MCP Server (Elixir, 61 tools) — AI agent interface, TOON format

SDKs: Rust (crates.io) · Go · TypeScript · Python (GitHub registry)
Docker: 4 images, ~129 MB total, Distroless/Alpine base
Deploy: Docker Compose · Kubernetes/Helm · Cloud Run · Fly.io
```

### What's Being Built

| Item | Status | Issue |
|------|--------|-------|
| Embedded Core review fixes (14 items) | In progress | Forked from #73 |
| Rust rewrite of ralph-tui with rig-core + AllSource | Open | #77 |
| Dashboard: real data replacing all mocks | Recently shipped | — |
| E2E test suite (20+ Playwright specs) | Active | — |

### 10 Architecture Decisions That Define Us

| ADR | Decision | Why |
|-----|----------|-----|
| 005 | Remove PostgreSQL from event path | Core IS the database. No external deps for events. |
| 006 | Server-side projections (fold-on-read) | Query Service folds events, returns materialized views. |
| 007 | Domain value objects (EventType, EntityId, TenantId) | Compile-time invariant enforcement. |
| 008 | Vector search with fastembed | Pure Rust embeddings. No Python. No external service. |
| 009 | simd-json zero-copy deserialization | 2-3x JSON parse speedup in hot path. |
| 001 | Embedded Core as thin facade | String-based API over EventStore. Feature-gated. |
| 002 | Crash-safe token compaction | WAL append inside lock prevents recovery duplicates. |
| 003 | Batch ingestion with single lock | Acquire write lock once for N events, not N times. |
| 004 | Projection backfill on registration | New projections replay historical events automatically. |
| 010 | Native ARM64 CI | No QEMU. Erlang NIFs fail under emulation. |

---

## Where We're Going

### The Insight

Every AI agent memory framework today — Mem0 ($24M Series A), Zep/Graphiti, Letta, Cognee — follows the same pattern:

```
Python orchestration layer
  → LLM API call (entity extraction)      ← slow, expensive, non-deterministic
  → External graph DB (Neo4j/FalkorDB)    ← separate infrastructure
  → External vector DB (Qdrant/Pinecone)  ← yet another system
  → No offline support                    ← cloud-only
  → No event sourcing                     ← mutations destroy history
```

Three databases. Three APIs. Three failure modes. An LLM on the write path that costs money and adds 1-2 seconds of latency. No audit trail. No time-travel. No offline support.

**AllSource already has every primitive these frameworks glue together** — events (temporal history), projections (materialized views that can index vectors and graph structures), HLC + CRDT (offline sync), WAL + Parquet (durability), and an embeddable library API.

What's missing is the **domain layer** — an API that speaks nodes, edges, vectors, and recall instead of raw events.

### AllSource Prime

**The unified memory engine for AI agents.**

Vectors + relationships + events in one embedded binary. No Neo4j. No Pinecone. No API keys. No Docker. One data directory.

```rust
let prime = Prime::open("~/.agent/memory").await?;

// Vectors — semantic recall
prime.embed("doc-1", "CRDTs enable conflict-free replication", vector).await?;
let similar = prime.similar("doc-1", 10).await?;

// Graph — structured relationships
let alice = prime.add_node("person", json!({"name": "Alice"})).await?;
let crdt = prime.add_node("concept", json!({"name": "CRDT"})).await?;
prime.add_edge(&alice, &crdt, "expert_in", None).await?;
let experts = prime.neighbors(&crdt, Some("expert_in"), Direction::Incoming).await?;

// Temporal — full history, time-travel
let history = prime.history(&alice).await?;                    // audit trail
let past = prime.neighbors_as_of(&alice, None, last_week).await?;  // time-travel

// Hybrid recall — all three combined
let context = prime.recall("who knows about CRDTs?", top_k=10).await?;
// → semantic similarity + graph traversal + temporal recency in one query

// Offline sync — CRDT merge with cloud
prime.sync("https://cloud.example.com").await?;
```

The key insight: vectors, graph nodes, graph edges, and domain events are **all events** at the storage layer. The differentiation happens at the **projection layer** — different projections maintain different indexes (HNSW for vectors, adjacency lists for graph, node state for current properties) over the same unified event stream.

```
                What agents need          How Prime delivers it
                ─────────────────         ─────────────────────
                Semantic recall      →    VectorIndexProjection (HNSW over events)
                Structured memory    →    AdjacencyList + ReverseIndex projections
                "When did I learn X" →    Event log (it's already there — it's an event store)
                Offline + merge      →    HLC + CRDT (already built in v0.12.0)
                "Forget this"        →    Soft-delete event (append-only, auditable)
                Hybrid recall        →    Combined vector + graph + temporal query
```

**No new storage engine.** Prime is a feature flag on `allsource-core` — a `prime/` module alongside `embedded/`, using the same EventStore, WAL, Parquet, and DashMap that already pass 1,489 tests.

### Competitive Position

```
              Vectors  Graph  Events  Temporal  Embedded  Offline   LLM-free
              ───────  ─────  ──────  ────────  ────────  ───────   ────────
Pinecone      ✓        ✗      ✗       ✗         ✗         ✗         ✓
Qdrant        ✓        ✗      ✗       ✗         ✗         ✗         ✓
LanceDB       ✓        ✗      ✗       partial   ✓         ✗         ✓
Chroma        ✓        ✗      ✗       ✗         ✓(Py)     ✗         ✓
Neo4j         plugin   ✓      ✗       ✗         ✗         ✗         ✓
Mem0          ✓        ✓      ✗       ✗         ✗         ✗         ✗
Graphiti      ✓        ✓      partial ✓         ✗         ✗         ✗
Letta         ✓        bolt   ✗       partial   ✗         ✗         ✗
───────────────────────────────────────────────────────────────────────────
Prime         ✓        ✓      ✓       ✓         ✓         ✓(CRDT)   ✓
```

**Nobody occupies the center of the Venn diagram.** Every product is strong in one silo (vectors OR graph OR events) and absent from the others. Prime is the unified engine.

The pitch:

> Chroma for vectors. Neo4j for graphs. Neither for history. **Prime for all three.**

### How Agents Adopt Prime

**Step 1** — install one binary:
```bash
brew install allsource-prime
```

**Step 2** — add to MCP config:
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

**Step 3** — the agent has persistent memory. Forever. With receipts.

Ten MCP tools: `prime_embed`, `prime_similar`, `prime_add_node`, `prime_add_edge`, `prime_neighbors`, `prime_recall`, `prime_history`, `prime_search`, `prime_forget`, `prime_shortest_path`.

No Docker. No API keys. No Pinecone bill. No Neo4j instance. 50μs writes, not 2-second LLM round-trips.

---

## Roadmap

### 2026 Q1 (Done)

**Theme: Foundation + Embedded**

- [x] Core event store with full durability (WAL + Parquet + DashMap)
- [x] Query Service API gateway (Elixir)
- [x] Control Plane with auth/billing (Go)
- [x] MCP Server with 61 tools (Elixir)
- [x] Next.js dashboard with real data
- [x] 4 SDKs (Rust, Go, TypeScript, Python)
- [x] Embedded Core library — 8 phases, 83 tests
- [x] Bidirectional sync with HLC + CRDT
- [x] Network sync transport (HTTP pull/push)
- [x] Configurable merge strategies
- [x] WebSocket backpressure
- [x] Vector search + keyword search
- [x] Server-side projections with fold-on-read
- [x] 1,489 Core tests passing, CI green
- [x] v0.6.0 → v0.12.0 (17 releases)

### 2026 Q2

**Theme: Prime + SaaS Launch**

| Milestone | Target | Detail |
|-----------|--------|--------|
| **Prime M1** | v0.13.0 | Graph primitives + traversal (nodes, edges, BFS, Dijkstra, subgraph) |
| **Prime M2** | v0.14.0 | Vector index projection + hybrid recall |
| **Prime M3** | v0.14.0 | Temporal graph queries (history, as_of, diff) |
| **Prime M4** | v0.15.0 | MCP server binary + HTTP API + Docker image |
| **Fix P0 gaps** | v0.13.0 | Query Service 501 endpoints, Core fork commit, KMS stubs |
| **SaaS MVP** | — | Fly.io deployment, landing page, onboarding |
| **Replication** | v0.13.0 | Leader-follower WAL shipping (complete implementation) |

### 2026 Q3

**Theme: Agent Memory Specialization**

| Milestone | Target | Detail |
|-----------|--------|--------|
| **Prime M5** | v0.16.0 | Contradiction detection, relevance decay, memory compaction |
| **Prime M6** | v0.17.0 | Schema enforcement for graph types |
| **Prime M7** | v0.17.0 | Semantic search + hybrid retrieval (vector + graph + temporal) |
| **Automatic failover** | v0.16.0 | Sentinel process, runtime leader promotion |
| **Community detection** | v0.17.0 | Leiden clustering as incremental projection |

### 2026 Q4

**Theme: Scale + Enterprise**

| Milestone | Target | Detail |
|-----------|--------|--------|
| **Prime M8** | v0.18.0 | Offline sync with graph-aware conflict reporting |
| **Prime M9** | v0.18.0 | Batch import/export (GraphML, Cypher, CSV) |
| **Phoenix Channels** | — | External WebSocket for non-Core clients |
| **Message queues** | — | Kafka/RabbitMQ integration via Broadway |
| **Multi-region** | — | Geo-replication with CRDT |

### 2027+

**Theme: Platform**

- Custom query language (if demand warrants the parser complexity)
- Multi-node clustering (Raft or simpler consensus)
- Enterprise compliance (SOC 2, HIPAA)
- GraphQL API
- Redis protocol compatibility (RESP3)

---

## The Product Line

```
┌────────────────────────────────────────────────────────────────────┐
│                                                                    │
│                         AllSource                                  │
│                                                                    │
│  ┌────────────────────────────────────────────────────────────┐    │
│  │                                                            │    │
│  │  AllSource Core                                            │    │
│  │  The event-sourced database engine                         │    │
│  │                                                            │    │
│  │  WAL · Parquet · DashMap · Projections · Schemas ·         │    │
│  │  Snapshots · Replication · Vector Search · EventQL         │    │
│  │                                                            │    │
│  │  ┌──────────────────────────────────────────────────┐      │    │
│  │  │                                                  │      │    │
│  │  │  AllSource Prime                                 │      │    │
│  │  │  The unified memory engine for AI agents         │      │    │
│  │  │                                                  │      │    │
│  │  │  Vectors · Graph · Temporal · Hybrid Recall ·    │      │    │
│  │  │  Contradiction Detection · Memory Decay ·        │      │    │
│  │  │  Offline Sync · MCP Tools                        │      │    │
│  │  │                                                  │      │    │
│  │  └──────────────────────────────────────────────────┘      │    │
│  │                                                            │    │
│  └────────────────────────────────────────────────────────────┘    │
│                                                                    │
│  ┌──────────┐  ┌──────────────┐  ┌───────────┐  ┌────────────┐   │
│  │ Query    │  │ Control      │  │ MCP       │  │ Dashboard  │   │
│  │ Service  │  │ Plane        │  │ Server    │  │ (Next.js)  │   │
│  │ (Elixir) │  │ (Go)         │  │ (Elixir)  │  │            │   │
│  └──────────┘  └──────────────┘  └───────────┘  └────────────┘   │
│                                                                    │
│  SDKs: Rust · Go · TypeScript · Python                            │
│                                                                    │
└────────────────────────────────────────────────────────────────────┘
```

**Core** is the database. Everything else is a view, a gateway, or a domain layer on top.

**Prime** is the highest-value domain layer — it turns Core from "an event store" into "the memory engine that AI agents can't live without."

The transition from Core to Prime is a feature flag, not a rewrite. Prime is built on the same 1,489-test foundation. Every vector is an event. Every graph node is an event. Every relationship is an event. The storage engine doesn't change. The projections do.

---

## Principles

These have held from v0.6.0 to v0.12.0 and will continue to hold:

1. **Core IS the database.** No PostgreSQL in the event path. No Redis. No external dependencies for data storage. If we need a capability, we build it into Core — not by adding another database.

2. **Events are the source of truth.** Everything — vectors, graph nodes, graph edges, configuration, audit logs — is an immutable event. History is never lost. Time-travel is always possible.

3. **Projections are the read model.** Different views of the same event stream. Vector indexes, adjacency lists, node state, statistics — all projections. One ingestion path, many read paths.

4. **Embeddable first.** If it can't run as a library in a Tauri app or a CLI tool, the design is wrong. Server mode is a deployment option, not a requirement.

5. **No LLM on the write path.** Agents decide what to store. Prime stores exactly what's sent — fast, deterministic, cheap. Entity extraction is the agent's job, not the database's.

6. **Offline-first.** Build knowledge locally, sync when ready. CRDT conflict resolution means multiple agents can work independently and merge without coordination.

7. **Zero infrastructure to start.** `brew install allsource-prime` → add to MCP config → agent has persistent memory. No Docker. No API keys. No cloud account.

---

## Metrics That Matter

### Today (v0.12.0)

| Metric | Value |
|--------|-------|
| Core tests passing | 1,489 |
| Event ingestion throughput | 469K events/sec |
| Query latency (p99) | 11.9μs |
| Docker image size (Core) | 15.7 MB |
| Total Docker footprint | ~129 MB |
| Embedded rlib size | ~30 MB |
| MCP tools | 61 |
| SDKs | 4 languages |
| Releases shipped | 17 (in 3 months) |
| Architecture decisions documented | 10 ADRs |

### Targets (v0.15.0 — Prime MCP Launch)

| Metric | Target |
|--------|--------|
| Prime graph operations | < 50μs |
| Prime vector search (100K vectors) | < 5ms |
| Prime hybrid recall | < 10ms |
| MCP tools (Prime) | 10 agent-specific tools |
| Time to first agent memory | < 2 minutes (install + config) |
| External dependencies | 0 |

---

## The Opportunity

The AI agent infrastructure market is $100B+ and growing. Agent memory is a core primitive that every framework needs but nobody has built properly.

**Mem0** raised $24M to glue together Qdrant + Neo4j + Python. **Zep** built Graphiti on top of Neo4j with an LLM on every write. **Letta** bolted Neo4j onto MemGPT via MCP.

They're all orchestration layers. They all require 2-3 external databases. They all put an LLM on the write path.

**AllSource Prime is the database itself.** One binary. One data directory. 50μs writes. Full history. Offline sync. No API keys.

> "Stop gluing together three databases. Prime is one engine — vectors, relationships, and events — with full temporal history and offline sync. The memory engine AI agents deserve."
