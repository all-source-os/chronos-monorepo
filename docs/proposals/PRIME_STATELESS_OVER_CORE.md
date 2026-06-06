# Prime as a stateless engine over Core

*Date: 2026-06-06*
*Status: proposal / ADR — supersedes the "tenant-isolate the single store" framing of bead t-10f876.*
*Context: `docs/current/PROD_DATA_FLOW_C4.md` (the trace + live verification that surfaced this).*

---

## Decision

**The hosted `allsource-prime` app must hold no durable data of its own. It becomes a stateless Prime engine that reads and writes Core over the network — the same architectural paradigm as the Query Service.** Core stays the single source of truth (event store); Prime is a materialized view over Core's `prime.*` events, computed per-tenant on demand and cached in memory.

This is the principle from CLAUDE.md applied consistently: *Core IS the database; everything else is a stateless service that talks to Core over the network.* The Query Service already lives by it (`RustCoreClient` → HTTP → Core, every op tenant-scoped). chronis already lives by it (`apps/chronis/src/infrastructure/http_core_client.rs`). The hosted Prime app is the one service that violates it.

---

## Why this is the right model

### What's wrong today (root cause)

`Prime` is hard-wired to a concrete, local, on-disk store:

```rust
// apps/core/src/prime/facade.rs:49
pub struct Prime {
    core: EmbeddedCore,           // ← concrete in-process WAL/Parquet/DashMap
    node_state: Arc<NodeStateProjection>,
    adjacency: Arc<AdjacencyListProjection>,
    vector_index: Arc<VectorIndexProjection>,
    // …
}
```

The hosted app then does `Prime::open(PRIME_DATA_DIR)` (`apps/prime-mcp/src/main.rs:155`) against a Fly volume. So the `allsource-prime` app *is* a second database — with its own store, its own (single, untenanted) graph, and a `/data` volume. The live instance proves it: one shared seeded graph of 17 nodes / 8 edges, no tenant partition (see the C4 doc's live-probe table).

This single fact is the source of every Prime contradiction in the C4 trace:
- the "four Prime stores" problem (A local, B Core events, C this volume, D Core-embedded),
- the cross-tenant leak risk (one shared store behind a tenant-authenticated edge),
- the orphaned/seeded data (C never sees real tenant memory because that flows to Core as events),
- the broken gateway routes (they proxy to a `prime`-featured Core that prod doesn't build).

### The model already exists — for one view

The Query Service's `PrimeController.graph` does exactly the right thing for the graph view: it queries Core for the tenant's `prime.*` events and folds them into a graph in memory (`apps/query-service/.../prime_controller.ex` + `graph_fold.ex`), holding nothing durable. This proposal **generalizes that one view into the whole Prime engine** (graph + vectors + recall + projections + provenance), hosted in the Rust `allsource-prime` app reusing the existing `prime` crate, instead of re-implemented per-view in Elixir.

### What it dissolves

- **Tenant isolation stops being a partitioning project.** There is no shared single store to carve up. Events come from Core queries already filtered by `tenant_id` (Core enforces tenant isolation on every query). Each tenant gets its own in-memory projection, built from its own events. Isolation reduces to "query Core scoped to the caller's tenant" — which is what QS and chronis already do.
- **The store count collapses to one.** Core's `prime.*` events are the only durable store. The `/data` volume (store C) is removed. Core does **not** need the `prime` cargo feature (store D) — the `prime` crate runs *in the stateless app*, not inside the Core database binary.
- **The local-first story is untouched.** The stdio dev binary keeps its embedded local store + push-sync. Only the *hosted http mode* becomes stateless-over-Core.

---

## Architecture

```
LOCAL (developer laptop)            HOSTED (Fly)
┌────────────────────────┐          ┌─────────────────────────────┐
│ allsource-prime (stdio)│          │ Control Plane (public edge) │
│  Prime<EmbeddedCore>   │          │  authenticates, forwards    │
│  local WAL/Parquet     │          │  tenant identity            │
│        │ push-sync     │          └──────────────┬──────────────┘
└────────┼───────────────┘                         │ /api/v1/prime/* (+ tenant)
         │ prime.* events                          ▼
         │ (tenant key)              ┌─────────────────────────────┐
         ▼                           │ allsource-prime (http)      │
┌────────────────────────┐          │  Prime<HttpCore>  STATELESS │
│ Core (event store)      │◀─────────│  per-tenant warm cache      │
│  prime.* events,        │  query   │  no /data volume            │
│  tenant-stamped         │  events  └─────────────────────────────┘
│  SOURCE OF TRUTH        │  by tenant
└─────────────────────────┘
```

### The one change that enables it: `EventStore` trait

Abstract Prime's `core` field behind a trait with two implementations:

```rust
// new: a minimal event source/sink Prime depends on
#[async_trait]
trait EventStore {
    async fn ingest(&self, event: IngestEvent<'_>) -> Result<()>;
    async fn ingest_batch(&self, batch: &[IngestEvent<'_>]) -> Result<()>;
    async fn query(&self, q: Query) -> Result<Vec<Event>>;
}

struct Prime<S: EventStore> { core: S, /* projections… */ }
```

- **`EmbeddedCore`** implements it (today's behavior) — used by the local stdio binary. Local-first unchanged.
- **`HttpCore`** implements it over Core's HTTP API (mirror `apps/chronis/src/infrastructure/http_core_client.rs`: `query_events`, `ingest_event`) — used by the hosted http mode. Reads/writes `prime.*` events on the remote Core, tenant-stamped.

`Prime`'s projection logic (graph fold, vector index, recall, provenance) is unchanged — it already operates over the events the `core` field yields. Only the *source* of those events changes.

### Per-tenant projections + warm cache (the load-bearing part)

Hosted Prime serves many tenants from one process, so it cannot hold one global projection set. Instead:

1. A request arrives with a tenant id (forwarded by the Control Plane, exactly as to QS/Core — never client-supplied).
2. Look up the tenant's projection bundle in an in-memory cache.
3. **Cache miss** → query Core for that tenant's `prime.*` events, build the projections (graph + vector index), insert into the cache.
4. Serve from the warm bundle.
5. **Writes** → ingest the `prime.*` event to Core (tenant-stamped) **and** apply it to the cached bundle.
6. Evict cold tenants (LRU + TTL) to bound memory. Restart = empty cache, rebuilt lazily from Core.

This is the **same shape as Core's own multi-tenant warm-set** (lazy per-tenant hydration from Parquet, LRU eviction) — a proven pattern in this codebase, not a new invention.

---

## Risks & the genuinely hard part

1. **Recall latency vs. statelessness (the crux).** Prime's value is fast in-memory recall (vector HNSW + graph traversal). Cold-building a tenant's vector index from a *remote* Core's full event history — including re-embedding or loading stored vectors over the network — is expensive. The per-tenant warm cache (above) is what makes stateless-on-disk viable; without it, recall latency dies. **This cache is the load-bearing design element and the main implementation effort.** Mitigations: lazy hydrate on first touch; keep hot tenants warm (`min_machines_running`); store vectors in the `prime.*` events so they don't need re-embedding on rebuild; cap rebuild cost with pagination/snapshots.
2. **Write path adds a network hop.** Writes go to remote Core instead of local WAL. Acceptable — QS already writes events to Core over HTTP on the request path.
3. **No "subscribe" over HTTP.** Embedded projections can tail the event stream cheaply; over HTTP you query-and-fold a snapshot per tenant. Fine for build-on-miss; if near-real-time cross-client updates are needed, add Core change-streaming later (out of scope).
4. **Embedding model placement.** fastembed stays in the prime app (server-side embed); only the event store moves remote. No change.
5. **Cache coherence across replicas.** If the hosted app scales to >1 machine, two replicas may hold stale projections for the same tenant after a write on the other. Start single-machine (it already is); add tag-based invalidation or sticky routing when scaling.

---

## Impact on the open Neotoma-parity beads

- **t-10f876** — rewritten. From "partition the single shared store by tenant" → **"make hosted Prime stateless over Core (`EventStore` trait + `HttpCore` + per-tenant warm cache); drop the `/data` volume."** Tenant isolation becomes a property of querying Core by tenant, not a partitioning effort. Smaller and safer than the original framing.
- **t-be6360** (hosted MCP-over-HTTP auth) — clean once t-10f876 lands: the `/mcp` handler takes the CP-forwarded tenant, and the engine reads Core scoped to it. No new isolation work.
- **t-2ac8 / t-9501 / t-8bf4** (Core REST prime routes + gateway proxy + SDK) — reframed. The prime HTTP/SDK surface should be served by the **stateless prime app reading Core**, and the Control Plane should route `/api/v1/prime/*` there. Core does **not** get the `prime` feature. The projection/provenance primitives I built in `apps/core/src/prime/projections/` remain valid as *library* code used by the engine — what changes is *which service hosts the HTTP surface and where its events come from*. (Note: those routes/SDK methods are currently dead in prod — they target a `prime`-featured Core that isn't built. This work is what makes them live.)
- **Core's `prime` cargo feature** — can be retired from the Core deployable entirely; it only ever made sense as an embedded convenience. The crate stays; the Core *binary* stops shipping it.

---

## Non-goals

- Do **not** change the local stdio dev binary's local-first store + sync. That story is correct and stays.
- Do **not** add a database to the prime app. The whole point is it has none.
- Do **not** enable the `prime` feature on the Core deployable as the fix — that would re-embed a second Prime store inside the DB and recreate the confusion. Core stays the event store; Prime stays a stateless reader.

---

## Sequence (when built)

1. `EventStore` trait + make `Prime` generic over it; `EmbeddedCore` impl (no behavior change) + tests green.
2. `HttpCore` impl over Core's HTTP API (reuse the chronis client pattern); tenant-scoped queries/ingest.
3. Per-tenant projection cache (lazy hydrate + LRU/TTL) in the hosted http path.
4. Hosted app: tenant from CP-forwarded identity; drop `PRIME_DATA_DIR` volume from `apps/prime-mcp/fly.toml`.
5. Cross-tenant isolation tests (tenant A cannot see B's nodes/recall/projections) at engine + `/mcp` + via CP.
6. Point Control Plane `/api/v1/prime/*` at the stateless prime app; retire QS GraphFold (or have it defer) and Core's `prime` feature.
7. Only then: deploy + live smoke-test (no deploy before isolation tests pass).
