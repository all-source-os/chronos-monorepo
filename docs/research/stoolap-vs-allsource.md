---
title: "stoolap vs AllSource — Research Dossier"
status: RESEARCH
phase: "learn (feeds prompt 020: /vs/stoolap page + article)"
last_updated: 2026-06-22
audience: internal
benchmark_machine: "Apple M2 Max, 12 cores, 64 GB RAM, macOS 25.2.0 (Darwin)"
---

# stoolap vs AllSource

> **Purpose.** Honest, citable raw material for a balanced `/vs/stoolap` page and
> a long-form article. Honesty is the product: where stoolap is faster, simpler,
> or a better fit, it is recorded here in plain terms. Every AllSource claim is
> grounded in this repo; every stoolap claim is grounded in a cited URL or a
> benchmark this dossier ran on one machine. Anything we could not verify is
> labelled **unverified** or **vendor-reported**.

---

## TL;DR (the honest verdict)

**stoolap and AllSource are different categories of product that happen to share
an AI-era pitch (embedded, vector search, temporal queries, an MCP server).**

- **stoolap** is an **embedded relational SQL DBMS** (HTAP / OLAP-leaning) in pure
  Rust. You `cargo add stoolap`, write `CREATE TABLE` / `INSERT` / `SELECT`, and it
  runs *in your process*. Mutable rows, MVCC, B-tree/Hash/Bitmap/HNSW indexes, full
  SQL (JOINs, CTEs, window functions), `AS OF` time-travel, native vectors. There is
  **no server mode** — it is a library, like SQLite/DuckDB.
  ([stoolap.io](https://stoolap.io), [github.com/stoolap/stoolap](https://github.com/stoolap/stoolap))
- **AllSource** is an **append-only event store** plus an agent-memory engine
  (Prime) and a hosted multi-tenant SaaS. Core runs as a **Rust server** (and can be
  embedded as a library); data is an immutable WAL + Parquet log; "current state" is a
  *projection* you fold from events. (`apps/core/README.md`, root `README.md`, `CLAUDE.md`)

If you want **SQL over mutable tables, embedded in one app**, pick stoolap — it is
simpler and there is nothing to operate. If you want an **immutable, replayable
store of record with full provenance, served over the network or self-hosted as
SaaS with per-call billing**, that is AllSource's shape, not stoolap's.

The few axes that are *truly* comparable (in-memory point-read latency, single-row
ingestion throughput) come out **roughly the same order of magnitude** — see the
benchmark. Neither is dramatically faster than the other on the fair, like-for-like
workload run here on the same laptop.

---

## 1. What stoolap is

**Category:** Embedded relational SQL database, HTAP (hybrid
transactional/analytical), OLAP-leaning. An independent review calls it
"an embedded OLAP (Online Analytical Processing) database."
([Better Stack](https://betterstack.com/community/guides/ai/stoolap-vs-sqlite/))
The Database-of-Databases encyclopedia classifies it as **Relational**,
**Shared-Everything**, **MVCC**, SQL interface.
([dbdb.io/db/stoolap](https://dbdb.io/db/stoolap))

**One-paragraph summary.** stoolap is "A Modern Embedded SQL Database in Pure Rust"
([repo tagline](https://github.com/stoolap/stoolap)) — a SQLite/DuckDB-class library
you compile into your application. It provides ACID transactions with MVCC and
snapshot isolation, a cost-based optimizer, parallel query execution (Rayon), a rich
SQL surface (JOINs, subqueries, CTEs, window functions, 101+ built-in functions),
multiple index types (B-tree, Hash, Bitmap, multi-column, and HNSW for vectors),
built-in `AS OF` temporal/time-travel queries, native `VECTOR(N)` columns with a
built-in `EMBED()` function (no external embedding API), and a storage engine with a
"hot/cold volume" architecture inspired by Iceberg/Delta Lake — an in-memory MVCC hot
buffer with WAL for active writes, and immutable columnar cold files (zone maps, bloom
filters, dictionary encoding, LZ4). (`src/lib.rs` of `stoolap-0.4.0`;
[github.com/stoolap/stoolap](https://github.com/stoolap/stoolap))

**Who it's for.** Developers who want an embeddable SQL engine with analytical speed
and modern AI features (vectors, semantic search) *inside a single application*,
without standing up Postgres/DuckDB or calling out to an embedding service. Its own
positioning is "the SQL database that ships with your app"
([stoolap.io](https://stoolap.io)).

**Language / license / maturity.**

| Attribute | Value | Source |
|---|---|---|
| Language | Rust (99.8%), pure Rust, "zero dependencies" stated | [repo](https://github.com/stoolap/stoolap) |
| License | Apache 2.0 (with patent grants) | [repo](https://github.com/stoolap/stoolap), `LICENSE` in crate |
| Stars / forks / issues | ~1.2k ★ / 43 forks / 7 open issues | [repo](https://github.com/stoolap/stoolap) (2026-06) |
| Latest release | **v0.4.0** (2026-04-01) | [repo](https://github.com/stoolap/stoolap) |
| Author / origin | Yasar Alev (Turkey); started 2025 | [dbdb.io](https://dbdb.io/db/stoolap) |
| Project type | Open-source, single primary maintainer; "hobby and open-source" per dbdb | [dbdb.io](https://dbdb.io/db/stoolap) |
| Maturity signal | "still in its early days, as evidenced by the NPM installation bug" | [Better Stack](https://betterstack.com/community/guides/ai/stoolap-vs-sqlite/) |

> **Heads-up on stale "Go" data.** Several pages (dbdb.io, an older Hacker News
> thread, some search snippets) describe stoolap as "pure Go." That is **historical**
> — stoolap began as a Go prototype (`stoolap-go`) and **migrated to Rust**; the
> current crate and all active development are Rust (`v0.4.0`, 99.8% Rust). When the
> article cites maturity, prefer the repo/crate over dbdb.io for language facts.

**Drivers / deployment model.** Embedded-only — there is **no standalone server or
daemon**. Bindings exist for Rust (native), Node.js (NAPI-RS), Python, Go, Java, C#,
PHP, Ruby, Swift, a C FFI, WASM, and — notably — an **MCP server** (`@stoolap/mcp`).
([stoolap.io installation docs](https://stoolap.io/docs/getting-started/installation/),
[MCP driver docs](https://stoolap.io/docs/drivers/mcp/))

**stoolap's MCP server matters for fairness.** It is a real, first-party feature: "30
tools, 2 resources, and 1 prompt" that "let AI assistants query, manage, and analyze
Stoolap databases with full access to all SQL features," compatible with Claude
Desktop, Claude Code, Cursor, Windsurf, Cline.
([MCP driver docs](https://stoolap.io/docs/drivers/mcp/)) So "has an MCP server" is
**not** an AllSource-exclusive axis here, unlike against mem0/letta/zep. The
difference is *what the tools do*: stoolap's 30 tools are a SQL surface (query,
execute, transactions, schema, vacuum); AllSource's tools are event-store/agent-memory
verbs (ingest events, recall, projections, anomaly detection). (`apps/core/README.md`,
[stoolap MCP docs](https://stoolap.io/docs/drivers/mcp/))

---

## 2. AllSource — grounded facts (for the comparison)

All from this repo. Core is a **durable event store**, *not* "in-memory only":
WAL (CRC32 checksums, configurable fsync) + Parquet (Snappy) + DashMap (in-memory
concurrent reads). (`CLAUDE.md`, `apps/core/README.md` lines 79, 209)

| Fact | Value | Source |
|---|---|---|
| Category | Append-only event store + agent-memory engine (Prime) + hosted SaaS | `apps/core/README.md`, `CLAUDE.md` |
| Data model | Immutable events; current state via projections / `as_of` | `CLAUDE.md`, `apps/core/README.md` |
| Durability | WAL (CRC32, fsync) + Parquet (Snappy) + DashMap | `CLAUDE.md`, `apps/core/README.md:79` |
| Headline ingestion | **469K events/sec** (batch-processor path) | `apps/web/src/lib/config.ts` `stats`, `README.md:12,99` |
| Recall latency | **11.9µs** p99 (DashMap reads) | `siteConfig.stats`, `docs/current/PERFORMANCE.md` |
| Concurrent writes (8 threads) | 7.98 ms | `docs/current/PERFORMANCE.md` |
| Deployment | Rust **server** (:3900) **and** embeddable library (`cargo add allsource-core`) | `README.md`, `apps/core/README.md:105,214` |
| MCP | 43–61 tools (event-store/agent-memory verbs) | `siteConfig.stats` (43), `apps/core/README.md:127` (61) |
| Vector / semantic | fastembed + HNSW; BM25 (tantivy); Prime agent memory | `apps/core/README.md:213` |
| Temporal | `as_of` projections by replaying the immutable log | `CLAUDE.md` (Core API), `README.md` |
| License | Community **Apache 2.0**; enterprise features **BSL 1.1** (→ Apache 2.0 on 2029-03-01). (Note: `siteConfig`/older README still say "MIT" in places.) | `README.md:18,416`; `siteConfig.pricing` |
| Pricing | Self-host free; hosted $19/$79/$299/mo + Enterprise; x402 per-call overage $0.0001/call | `apps/web/src/lib/config.ts` `pricing` |
| Footprint | ~129 MB all services; Core image 15.7 MB | `siteConfig.stats`, `apps/core/README.md` |

> **Note the licence nuance:** `siteConfig.faqs` and the v0.11 README still say
> "MIT," but the current root `README.md` (v0.22) states **Apache 2.0 community /
> BSL 1.1 enterprise**. Prompt 020 should use Apache 2.0 / BSL 1.1 and flag that the
> hosted SaaS adds commercial terms. Both stoolap and AllSource-community are
> permissive OSS, so "open source, self-hostable" is true of **both**.

---

## 3. Comparison matrix

Two kinds of rows: **comparable** (a fair side-by-side exists) and
**different category** (the honest answer is "these don't line up — here's why").
Cells cite a repo path or URL; the benchmark rows cite §4.

### 3a. Comparable axes

| Axis | AllSource | stoolap | Notes / source |
|---|---|---|---|
| Written in | Rust | Rust | `README.md`; [repo](https://github.com/stoolap/stoolap) |
| Embeddable in-process | Yes (`allsource-core`) | Yes (`stoolap`) — its *only* mode | `apps/core/README.md:105`; [install docs](https://stoolap.io/docs/getting-started/installation/) |
| In-memory point-read latency | 11.9µs p99 (recall) | **~0.5–0.9µs** (point read by PK) | §4 (both measured/cited) |
| Single-row in-memory ingestion | 469K events/sec (batch path); ~200–500K measured | **~290–450K rows/sec** measured | §4 — same order of magnitude |
| Durable (survives restart) | Yes — WAL + Parquet | Yes — WAL + cold columnar files | `CLAUDE.md`; [repo](https://github.com/stoolap/stoolap) |
| Vector / HNSW search | Yes (fastembed + HNSW) | Yes (`VECTOR(N)`, HNSW, built-in `EMBED()`) | `apps/core/README.md:213`; [vector blog](https://stoolap.io/blog/2026/02/27/vector-and-semantic-search-in-sql/) |
| Built-in embeddings, no external API | Yes (local fastembed) | Yes (built-in sentence-transformer `EMBED()`) | `apps/core/README.md:105,213`; [vector blog](https://stoolap.io/blog/2026/02/27/vector-and-semantic-search-in-sql/) |
| Temporal / time-travel | Yes — `as_of` projections (replay log) | Yes — `AS OF TIMESTAMP` / `AS OF TRANSACTION` (MVCC) | `CLAUDE.md`; `src/lib.rs` of crate, [repo](https://github.com/stoolap/stoolap) |
| MCP server for AI agents | Yes — 43–61 event/memory tools | Yes — 30 SQL tools (`@stoolap/mcp`) | `apps/core/README.md:127`; [MCP docs](https://stoolap.io/docs/drivers/mcp/) |
| License (OSS core) | Apache 2.0 (enterprise BSL 1.1) | Apache 2.0 | `README.md:416`; [repo](https://github.com/stoolap/stoolap) |
| Maturity (stars) | n/a — primary repo private/org `all-source-os` | ~1.2k ★, v0.4.0, 1 primary author | `siteConfig.links.github`; [repo](https://github.com/stoolap/stoolap) |

### 3b. Different-category axes (do **not** force a contest)

| Axis | AllSource | stoolap | Why they don't line up |
|---|---|---|---|
| Core data model | Immutable append-only **event log**; state = projection | Mutable **relational tables** (rows you UPDATE/DELETE) | Fundamentally different abstraction. "Throughput" means different units: events appended vs rows mutated. |
| Query language | HTTP/MCP event queries; EventQL (DataFusion SQL) for analytics | **Full SQL** front and center (JOINs, CTEs, windows) | stoolap is a SQL DBMS; AllSource is an event store with SQL analytics bolted on. If you want SQL ergonomics, stoolap wins outright. |
| Provenance / replay | First-class: every change is a replayable event | Not the model — `AS OF` reads MVCC versions, retained per cleanup, not a permanent audit log | AllSource keeps the *full* immutable history by design; stoolap's MVCC versions are a concurrency mechanism, garbage-collected, not an audit ledger. |
| Deployment topology | Server (:3900) + embedded + **hosted multi-tenant SaaS** | **Embedded library only** — no server/daemon | You cannot point a fleet of clients at a stoolap "instance" over the network; it lives in one process. AllSource is built to be a shared service. |
| Multi-tenancy / RBAC / billing | Yes (Control Plane: tenants, RBAC, LemonSqueezy, x402) | No — it's a library; tenancy/billing is your app's job | Different problem space entirely. |
| Distribution / replication | Leader-follower WAL shipping (enterprise) | Single-process; no replication | stoolap is not a distributed system; comparing HA is meaningless. |
| Pricing model | Free self-host + hosted tiers + x402 per-call | Free (Apache 2.0); no hosted offering | stoolap has nothing to price — there's no SaaS. |
| Agent-memory product | Prime (graph + vector + recall, dedicated MCP) | None — it's a database, not a memory layer | stoolap gives you a SQL+vector store; assembling "agent memory" on top is your job. |

---

## 4. Benchmark — fair, like-for-like, reproduced on one machine

### 4.1 Why this workload

Both tools' real strength is **append-heavy ingestion + fast point reads**. That is
the only axis where a *fair* head-to-head exists (a SQL JOIN benchmark would flatter
stoolap; an event-replay benchmark would flatter AllSource — both would be
misleading). So the micro-benchmark is: **insert 100,000 single rows/events, then do
100,000 point reads by primary key**, in release mode, single thread, same laptop.

Because AllSource Core is a **durable** store, stoolap was measured in **both** modes:
`memory://` (in-memory) and `file://…?sync_mode=normal` (durable, fsync on the WAL).
The durable number is the honest one for "survives a restart."

### 4.2 Hardware & versions (identical for both)

| | |
|---|---|
| Machine | Apple **M2 Max**, 12 cores, 64 GB RAM |
| OS | macOS, Darwin 25.2.0 (arm64) |
| Toolchain | rustc/cargo **1.92.0-nightly**, `--release` |
| AllSource | this repo @ branch `main` (root `README.md` reports v0.22.0), harness `tooling/performance/src/main.rs` |
| stoolap | **0.4.0** (from crates.io; `Cargo.lock` pinned `stoolap 0.4.0`), `stoolap = "0.4"` |

### 4.3 Exact commands

**AllSource** (from repo root):
```bash
cargo run --release -p allsource-performance
```

**stoolap** (standalone harness in `/tmp/stoolap-bench`; full source in §4.7):
```bash
cd /tmp/stoolap-bench
cargo build --release        # compiles stoolap 0.4.0 from source, ~3m15s
./target/release/stoolap-bench
```

### 4.4 Raw output — stoolap (captured)

Run #1:
```
=== stoolap in-memory (memory://) ===
Insert rows: 100000
Insert duration: 343.467334ms
Insert rate: 291149 rows/sec
Point reads: 100000
Read duration: 87.861416ms
Read rate: 1138156 reads/sec
Read avg latency: 0.879 us

=== stoolap durable file-backed (sync_mode=normal) ===
Insert rows: 100000
Insert duration: 1.267644375s
Insert rate: 78886 rows/sec
Point reads: 100000
Read duration: 102.932042ms
Read rate: 971515 reads/sec
Read avg latency: 1.029 us
```

Run #2 (stability check — note real variance):
```
=== stoolap in-memory (memory://) ===
Insert rate: 447663 rows/sec
Read rate: 1595387 reads/sec
Read avg latency: 0.627 us
=== stoolap durable file-backed (sync_mode=normal) ===
Insert rate: 97344 rows/sec
Read rate: 2094793 reads/sec
Read avg latency: 0.477 us
```

### 4.5 Raw output — AllSource (captured)

Run #1 (clean machine), the ingestion-relevant stages:
```
=== Batch Processor Performance ===
Total events: 100000
Duration: 197.6265ms
Events/sec: 506005                 # ← the "469K" headline path

=== Full Pipeline Performance (Concurrent) ===
Total events: 100000  Threads: 4
Events/sec: 957556

=== Sustained Throughput Test ===
Total events: 693000  (2 s wall)
Events/sec: 495978
```

Run #2 (under load from the concurrent stoolap build — shows variance):
```
=== Batch Processor Performance ===  Events/sec: 218748
=== Full Pipeline Performance (Concurrent) ===  Events/sec: 405279
=== Sustained Throughput Test ===  Events/sec: 201395
```

Other AllSource hot-path stages (run #1, for context only — not comparable to stoolap):
SIMD JSON 1.12M docs/sec · lock-free queue push 42M/sec · sharded queue 2.0M/sec ·
arena 29.9M allocs/sec · SIMD filter 9.66M/sec.

### 4.6 Fair like-for-like table

Single-thread, 100k ops, M2 Max, `--release`. AllSource ingestion = batch-processor
path (its headline unit); stoolap ingestion = single-row prepared INSERTs.

| Metric | AllSource | stoolap (in-memory) | stoolap (durable, fsync) |
|---|---|---|---|
| Ingestion throughput | **~200K–506K events/sec** (run-to-run) | **~291K–448K rows/sec** | **~79K–97K rows/sec** |
| Headline / published | 469K events/sec (`siteConfig`) | 191x DuckDB batch insert *(vendor, §4.8)* | — |
| Point-read latency (in-mem) | 11.9µs p99 *(published; not re-measured here)* | **~0.5–0.9µs avg** (measured) | ~0.5–1.0µs avg (measured) |
| Point-read throughput | n/a (recall figure is latency) | ~1.1M–1.6M reads/sec | ~0.97M–2.1M reads/sec |

**Reading this honestly:**

- **In-memory ingestion is a wash.** Both land in the **low-hundreds-of-thousands per
  second** on this machine. AllSource's batch path edges ahead on a clean machine
  (~506K vs stoolap's ~290–450K), but the ranges overlap and depend on background
  load. Nobody is "10x faster" here.
- **stoolap's in-memory point reads look faster on paper** (sub-microsecond average vs
  AllSource's published 11.9µs p99) — but these measure *different things*: stoolap is
  an `avg` of a prepared SQL point-select on 100k rows; AllSource's 11.9µs is a *p99*
  recall figure from `PERFORMANCE.md` that this dossier did **not** re-measure with the
  same harness. **Do not** present "0.6µs vs 11.9µs" as stoolap winning 20x — the
  metrics (avg vs p99), datasets, and code paths differ. The honest statement: *both do
  point reads in the single-digit-microsecond-or-better range in memory.*
- **Durability has a real cost for stoolap.** With `sync_mode=normal`, single-row
  insert throughput drops to **~79–97K rows/sec** (fsync per the WAL config). AllSource's
  469K is the in-memory batch-processor pipeline; this dossier did **not** isolate
  AllSource's fsync-to-disk per-event rate either (its published "concurrent writes (8
  threads): 7.98ms" in `PERFORMANCE.md` is the closest durable figure). So **neither
  side's headline ingestion number is a pure synchronous-durable single-row rate** —
  state this plainly in the article.

### 4.7 stoolap benchmark source (for reproducibility)

`/tmp/stoolap-bench/Cargo.toml`:
```toml
[package]
name = "stoolap-bench"
version = "0.1.0"
edition = "2021"
[[bin]]
name = "stoolap-bench"
path = "main.rs"
[dependencies]
stoolap = "0.4"
[profile.release]
opt-level = 3
```

`main.rs` (abridged — full logic): open `Database::open(dsn)`,
`CREATE TABLE events (id INTEGER PRIMARY KEY, event_type TEXT, entity_id TEXT, value INTEGER)`,
prepare `INSERT … VALUES ($1,$2,$3,$4)` and loop 100k single-row `.execute(...)`
(timed), then prepare `SELECT * FROM events WHERE id = $1`, warm up 1k, then 100k
timed `.query(...)` consuming the first row. Run once for `memory://`, once for
`file://<tmp>?sync_mode=normal`. (API verified against `stoolap-0.4.0/src/api/database.rs`
and the crate's own `examples/benchmark.rs`.)

### 4.8 Vendor-reported stoolap numbers (NOT independently reproduced)

stoolap's `BENCHMARKS.md` (v0.4.0, Apple Silicon, in-memory, 10k rows) reports it
beating SQLite and DuckDB on most operations:

- "STOOLAP vs SQLite: 46 wins / 7 losses (87% win rate)"; "vs DuckDB: 52 wins / 1 loss (98%)"
- SELECT by ID: 0.12µs (vs SQLite 0.21µs, DuckDB 145.55µs) → "1213x" vs DuckDB
- Batch INSERT (100 rows): 77.94µs (vs DuckDB 14920.25µs) → "191x"
- COUNT DISTINCT: 0.37µs (vs SQLite 105.98µs) → "286x"

Source: [BENCHMARKS.md](https://github.com/stoolap/stoolap/blob/main/BENCHMARKS.md).

**Treat these as vendor-reported and partly inflated.** An independent Better Stack
test found stoolap's *real* OLAP advantage over SQLite to be **~4.12x at 100k rows,
~6.47x at 1M rows** — and explicitly: *"the dramatic numbers from its marketing
materials should be taken with a grain of salt, as performance can vary greatly,"*
noting stoolap's blog "advertises 138x speedup on COUNT DISTINCT, yet testing showed
only 4.12x at 100K rows." It also flags stoolap as "still in its early days, as
evidenced by the NPM installation bug."
([Better Stack](https://betterstack.com/community/guides/ai/stoolap-vs-sqlite/))
For the article, prefer **our captured numbers (§4.6)** over either vendor's
multipliers.

### 4.9 Methodology & caveats (read before quoting any number)

- **Same machine, same day, `--release`** for both. Laptop, not a tuned server — run-to-run
  variance is real (see runs #1/#2); background load (the two builds running concurrently)
  visibly depressed the second readings. Treat all figures as *ranges*, not points.
- **Different units, stated honestly.** AllSource "events/sec" (immutable append, batch
  processor) ≠ stoolap "rows/sec" (single-row SQL INSERT into a mutable table). They are
  *comparable in spirit* (per-record ingestion cost) but not identical operations.
- **stoolap was actually installed and run** here (crate `0.4.0`, compiled from source,
  benchmark executed twice). This is **not** a vendor-numbers-only comparison.
- **AllSource point-read latency (11.9µs) was NOT re-measured** with a matching harness;
  it is taken from `docs/current/PERFORMANCE.md` / `siteConfig`. stoolap's read latency
  **was** measured. So the read-latency row mixes one measured and one published figure —
  do not over-claim a winner there.
- **Neither headline ingestion number is a synchronous-durable single-row rate.** Both
  vendors quote their best in-memory/batch path. The only durable single-row number in
  this dossier is stoolap's (~79–97K/sec with fsync); AllSource's durable per-event rate
  was not isolated.
- A skeptical engineer can reproduce all of §4.4–4.5 with the commands in §4.3 and the
  source in §4.7.

---

## 5. Where each genuinely wins

### Pick **stoolap** when…
- You want **real SQL** — JOINs, CTEs, window functions, a cost-based optimizer — over
  **mutable tables**. AllSource makes you fold projections; stoolap just runs the query.
  ([repo](https://github.com/stoolap/stoolap))
- You want a database **embedded in one app with zero ops** — no server, no Docker, no
  network hop. `cargo add stoolap` (or npm/pip/etc.) and ship.
  ([install docs](https://stoolap.io/docs/getting-started/installation/))
- You need **analytical (OLAP) queries** — GROUP BY, DISTINCT, aggregations — fast,
  locally, without DuckDB. Even discounted to the independent ~4–6x, it's quick.
  ([Better Stack](https://betterstack.com/community/guides/ai/stoolap-vs-sqlite/))
- You want **vector search + built-in embeddings inside SQL** (`VECTOR(N)`, HNSW,
  `EMBED()`) with no Python and no external embedding API.
  ([vector blog](https://stoolap.io/blog/2026/02/27/vector-and-semantic-search-in-sql/))
- You want **fully permissive Apache-2.0** with no commercial/BSL strings on any
  feature. ([repo](https://github.com/stoolap/stoolap))
- Your AI use case is **"let Claude query my SQL database"** — the `@stoolap/mcp` server
  (30 tools) does exactly that, in-process. ([MCP docs](https://stoolap.io/docs/drivers/mcp/))
- You value **broad language reach from one engine** (Rust, Node, Python, Go, Java, C#,
  PHP, Ruby, Swift, C, WASM). ([install docs](https://stoolap.io/docs/getting-started/installation/))

### Pick **AllSource** when…
- You need an **immutable, replayable store of record** — full event provenance and
  the ability to reconstruct any past state by replay, as an audit ledger, not a
  garbage-collected MVCC version. (`CLAUDE.md`, `apps/core/README.md`)
- You need the store to be a **shared network service** — many clients/agents, a Rust
  server on :3900, leader-follower replication — not a library trapped in one process.
  (`apps/core/README.md`, `CLAUDE.md`)
- You want a **hosted, multi-tenant SaaS** with RBAC, billing (LemonSqueezy), and
  **x402 per-call agent payments** out of the box. (`apps/web/src/lib/config.ts`,
  Control Plane in `apps/core/README.md:216`)
- You want a **purpose-built agent-memory engine (Prime)** — knowledge graph + vector
  + recall with its own MCP tools — rather than assembling memory yourself over SQL.
  (`apps/core/README.md` Agent Memory section)
- Your workload is **event sourcing / CQRS / time-travel as the primary model** —
  append-only ingestion at scale with `as_of` projections as a first-class query.
  (`README.md`, `CLAUDE.md`)
- You want a **proven-reproducible throughput claim** you can run yourself
  (`cargo run --release -p allsource-performance`) and a documented ~129 MB footprint
  for the whole stack. (`README.md`, `siteConfig.stats`)

---

## 6. Recommended framing for prompt 020 (the spine)

**Lead with category honesty, not a contest.** The page's credibility (and AI
citation) comes from saying out loud: *"stoolap is an embedded SQL database; AllSource
is an event store / agent-memory service. They overlap on AI features but solve
different problems."* This differs from `/vs/mem0|letta|zep`, where the others are
agent-memory tools in AllSource's lane — so reuse the `ComparisonTable` +
`competitors.ts` structure, but **add a "different category" treatment** and **do not**
claim AllSource-only on MCP, vectors, embeddings, or temporal — stoolap has all four
(cite §1, §3a).

**The 2–3 axes that matter most to a buyer:**

1. **Data model — mutable SQL tables vs immutable event log.** This is *the* decision.
   "Do you want to run SQL and update rows (stoolap), or keep an append-only, replayable
   history and project state from it (AllSource)?" Everything else follows.
2. **Deployment — embedded library vs network service + SaaS.** stoolap lives in one
   process (zero ops, zero sharing); AllSource is a server you can self-host or buy
   hosted, with multi-tenancy, RBAC, and per-call billing. Pick by whether the store is
   *yours alone in one app* or *a shared backbone for a fleet of agents*.
3. **Ingestion/latency are a near-wash; don't bluff.** On the fair micro-benchmark both
   sit in the same order of magnitude (§4.6). Make the **honest "roughly comparable, run
   it yourself"** point a *feature* of the page — it's exactly what an AAA reader and an
   LLM will trust. Lead performance with reproducibility, not a cherry-picked multiplier.

**Honest verdict line for the page:** *"If you want SQL in your app, use stoolap. If you
want a replayable event store and agent-memory backbone you can serve or host, use
AllSource. On raw in-memory speed they're close — so choose by data model and
deployment, not by a benchmark."*

**Pitfalls to avoid in 020:**
- Don't call Core "in-memory only" (it's WAL + Parquet + DashMap) — `CLAUDE.md`.
- Don't present stoolap's vendor multipliers (191x/1213x) as fact; cite Better Stack's
  ~4–6x correction and our §4.6 numbers.
- Don't imply stoolap lacks MCP/vectors/temporal — it has all three.
- Fix the licence story: AllSource community is **Apache 2.0** (enterprise BSL 1.1), not
  "MIT," when comparing to stoolap's Apache 2.0.

---

## 7. Sources

**AllSource (this repo):**
- `CLAUDE.md` — architecture facts (WAL + Parquet + DashMap; "Core IS the database").
- `apps/core/README.md` — durability, 469K events/sec, embedded API, MCP (61 tools), Prime, vectors, BSL/Apache editions.
- `README.md` (root) — v0.22 positioning, benchmark table (M2 Max), Apache 2.0 / BSL 1.1 licence.
- `apps/web/src/lib/config.ts` — `siteConfig.stats` (469K, 11.9µs, 43 MCP tools, 129MB) and `pricing` (tiers, x402 overage).
- `docs/current/PERFORMANCE.md` — 469K ingestion, 11.9µs p99, concurrent-writes 7.98ms.
- `tooling/performance/src/main.rs` — the benchmark harness.
- `apps/web/src/app/(marketing)/vs/_data/competitors.ts`, `_components/ComparisonTable.tsx` — existing /vs/ structure to mirror.

**stoolap (external):**
- Site: <https://stoolap.io>
- Install / drivers: <https://stoolap.io/docs/getting-started/installation/>
- MCP server: <https://stoolap.io/docs/drivers/mcp/>
- Vector/semantic blog: <https://stoolap.io/blog/2026/02/27/vector-and-semantic-search-in-sql/>
- Repo: <https://github.com/stoolap/stoolap>
- Vendor benchmarks: <https://github.com/stoolap/stoolap/blob/main/BENCHMARKS.md>
- Encyclopedia: <https://dbdb.io/db/stoolap>
- Independent review (strengths + caveats): <https://betterstack.com/community/guides/ai/stoolap-vs-sqlite/>
- crate `stoolap 0.4.0` source (local): `~/.cargo/registry/src/.../stoolap-0.4.0/` (`src/lib.rs`, `src/api/database.rs`, `examples/benchmark.rs`, `BENCHMARKS.md`, `README.md`).

**Benchmark captured by this dossier:** §4.4 (stoolap, 2 runs), §4.5 (AllSource, 2 runs),
machine = Apple M2 Max / 12c / 64 GB / macOS Darwin 25.2.0, rustc 1.92.0-nightly,
`--release`; stoolap harness source in §4.7.
