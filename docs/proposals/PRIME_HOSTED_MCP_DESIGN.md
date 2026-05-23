# Hosted Prime MCP Server — Design Proposal

## Status: Architecture approved (Option A) — 2026-05-23. Open questions below still pending.
## Date: 2026-05-23
## Owner: needs assignment

## Problem

To wire Prime into Claude Desktop today, a user installs the `allsource-prime` binary locally, edits `claude_desktop_config.json`, and provides an API key. This works but it has three structural costs:

1. **Per-user binary distribution.** Every user runs their own local process; every new platform we want to support means another build target.
2. **Memory lives on the laptop.** The local data directory holds the WAL and Parquet. If the user works on multiple machines, memory diverges unless they pay attention to the `--sync-to` flag — and even then sync is one-directional (local → remote, never remote → local).
3. **Onboarding has a manual step.** Even with `/connect` minting the key and `install.sh` automating the rest, the user still runs a curl-pipe on their machine. Some users won't.

Claude Desktop's [Connectors UI](https://www.anthropic.com/news/integrations) accepts remote MCP servers over HTTP. If we shipped one, the user flow would shrink to: paste a URL in Claude Desktop → sign in → done. No binary, no config file, no curl. The cost: we run the MCP server, not the user.

## What Exists Today

| Component | Mode | Auth | Data isolation | Reach |
|---|---|---|---|---|
| `apps/prime-mcp` stdio server | MCP stdio (newline JSON-RPC) | none (local trust) | single-tenant per process (`--data-dir`) | runs on user's machine |
| `apps/prime-mcp` HTTP server (Fly: `allsource-prime`) | REST | none today | single-tenant | exposes `/api/v1/prime/*` for the dashboard memory tab |
| `apps/core` (Fly: `allsource-core-*`) | REST | Bearer API key → tenant | multi-tenant via tenant_id on every event | already serves the gateway and panel |
| Query Service (Fly: `allsource-query`) | REST + WS | Bearer API key → tenant | multi-tenant | wraps Core; rate-limits + billing per call |

The HTTP server on Prime today is single-tenant. It's deployed because the dashboard's memory tab calls it for graph queries, but it talks to **one** Prime store (the Fly machine's `/data` volume). It has no authentication; it's protected by Fly's private network and only reachable from the dashboard backend.

The MCP server today is stdio-only. There is no MCP-over-HTTP transport in our codebase.

## The Gap

A hosted Prime MCP requires four things we don't have:

1. **MCP transport over HTTP** — Streamable HTTP per the MCP spec (formerly SSE). Wraps the same `tools/call` handlers that `transport.rs` exposes over stdio.
2. **Auth on the MCP transport** — Bearer API key in the `Authorization` header → tenant resolution.
3. **Multi-tenancy in the Prime data layer** — one process serves N tenants, each with isolated graph/vector/recall state. Today the `Prime` struct in `allsource-core` is bound to a single data directory.
4. **Billing per MCP call** — already exists for Core event ingest; needs to extend to Prime tool calls.

## Options

### Option A: New process — `prime-mcp-gateway` (Recommended)

A new app under `apps/` that exposes MCP-over-HTTP and routes calls into per-tenant Prime instances. Keeps the existing stdio server and local data path unchanged.

```
┌─────────────────────┐                              ┌──────────────────────┐
│  Claude Desktop     │                              │  prime-mcp-gateway   │
│  (Connectors UI)    │  ──── MCP-over-HTTP ────→    │  (new Fly app)       │
│                     │       Bearer auth            │                      │
└─────────────────────┘                              │  ┌────────────────┐  │
                                                     │  │ tenant router  │  │
                                                     │  │  (API key →    │  │
                                                     │  │   tenant_id)   │  │
                                                     │  └────────────────┘  │
                                                     │           │          │
                                                     │   ┌───────┴─────┐    │
                                                     │   │ Prime store │    │
                                                     │   │ per tenant  │    │
                                                     │   └─────────────┘    │
                                                     └──────────────────────┘
                                                                │
                                                                ▼
                                                       ┌──────────────────┐
                                                       │ allsource-core   │
                                                       │ (events sink)    │
                                                       └──────────────────┘
```

**Internals:**

- One MCP server per Fly machine. On connection: read bearer token → call Control Plane to resolve tenant + scopes (cached). Reject if not authenticated.
- Each tenant has an in-memory `Prime` instance with its own DashMap-backed graph/vector state. Cold start: on first connection for a tenant, hydrate from Core's event stream by replaying `prime.*` events filtered to that tenant_id. Hot tenants stay in memory; LRU evicts cold ones to disk.
- Tool calls remain side-effect free reads OR emit `prime.*` events to Core. Writes go through Core (single source of truth); reads can be served from the in-memory Prime.
- The existing `apps/prime-mcp` stays as-is for local-only use. Codebase stays standalone.

**Pros:**
- Clean separation: hosted vs local don't share a process or risk-profile
- Multi-tenancy lives in one place, easy to reason about
- Failure mode: a misbehaving tenant only impacts their own in-memory Prime
- Existing Core auth/billing/rate-limit primitives reused

**Cons:**
- New deployable, new fly.toml, new Dockerfile, new CI pipeline
- Cold-start latency on first connection per tenant (could be seconds for large memories)
- Memory pressure: every active tenant holds a Prime instance — capacity planning matters

### Option B: Extend the existing `allsource-prime` Fly app

Add MCP-over-HTTP to `apps/prime-mcp` itself, multiplex on the same axum server that serves REST today. Make the Prime layer multi-tenant in-process.

**Pros:**
- One less deployable
- Reuses the existing Dockerfile, fly.toml, deploy pipeline

**Cons:**
- The existing prime-mcp is currently single-tenant; making it multi-tenant is invasive
- Mixing local and hosted code paths in one binary — the stdio server has no use for tenant routing, but it'd still be compiled in
- Higher risk of regressing the dashboard's memory-tab queries (which talk to the same process)
- The crate is excluded from the workspace specifically to keep its blast radius small; adding cross-tenant concerns expands that

### Option C: Per-tenant containers / fly machines

Spin up one Fly machine per tenant on first connection (Fly's `machines.create` API + `auto_stop_machines`). Each container runs the existing single-tenant prime-mcp.

**Pros:**
- True data isolation (per-process, per-volume)
- Reuses every line of existing single-tenant code
- Tenant memory pressure stays bounded — each machine is `~256MB`

**Cons:**
- One Fly machine per active user is expensive — even at `auto_stop_machines = true` with 1m idle, the per-tenant baseline is ≥ $1/month
- Connection latency to wake a stopped machine is several seconds
- Free-tier abuse surface (creating accounts to consume machine slots)
- Operating N machines is operationally heavier than operating one multi-tenant process

### Option D: Pre-provisioned remote MCP (no multi-tenancy)

Single shared Prime store; everyone reads/writes the same memory. Cheapest. Obviously wrong — we'd be the LLM equivalent of a shared Google Doc with no permissions.

Listed here so it's clear we considered and rejected it.

## Recommendation

Option A. Multi-tenancy belongs in a new dedicated service rather than retrofitted into either the stdio server (Option B) or per-tenant infrastructure (Option C). The new service can reuse Core's auth/billing/rate-limit code paths instead of duplicating them, and the cold-start latency concern is solvable with an LRU + warmup heuristic before it becomes load-bearing.

## Detailed Design (Option A)

### Transport

The MCP spec's [Streamable HTTP transport](https://modelcontextprotocol.io/docs/concepts/transports#streamable-http) (replaced SSE in late 2025): one HTTP endpoint per session, server-sent events for responses, bidirectional via POST. Standard Anthropic primitives.

Endpoint shape:

```
POST /mcp                Authorization: Bearer ask_...
                         Content-Type: application/json
                         Mcp-Session-Id: <session uuid>

→ JSON-RPC request body, e.g. {"method":"tools/call","params":{...}}
← chunked text/event-stream response (one event per partial result)
```

`Mcp-Session-Id` is generated server-side on the first request without one, and the client echoes it on subsequent requests in the same session. This lets us bind a session to a tenant for the lifetime of the connection.

### Authentication

Bearer token in `Authorization`. Same `ask_*` key format used everywhere else.

```
Authorization: Bearer ask_<tenant>_<keyid>_<secret>
```

Validation flow:

1. Pull token from header
2. Hash → look up in Control Plane's API key cache (already exists for the gateway path)
3. Resolve to `(tenant_id, scopes, tier)`
4. Reject if scopes don't include the MCP scope (proposed new: `prime:invoke`)
5. Bind tenant_id to the session, cache for the connection's lifetime
6. Per-tool-call: increment usage counter (existing Core billing path) and check rate limit

API keys generated by `/connect` today have `events:write` + `events:read`. We need to add `prime:invoke` as a new scope, and `/connect` should request it when minting the Claude Desktop key.

### Multi-tenancy

Each tenant gets its own `Prime` struct instance, indexed by tenant_id. Backing store:

- **Hot tenants** — Prime instance in memory. DashMap-backed, same as today.
- **Cold tenants** — flushed to per-tenant Parquet snapshots in object storage (S3/Tigris). First connection rehydrates by reading the snapshot + replaying recent `prime.*` events from Core.
- **Eviction** — LRU by last activity time. Target: keep top-N active tenants in memory where N is sized to ~50% of machine RAM budget.

Cold-start cost is the main risk. Mitigations:
- Smaller-than-expected memories warm fast (most tenants are <10MB of nodes/edges)
- Background prefetch: when a tenant signs into the dashboard, pre-warm their Prime
- LRU eviction policy can favor evicting tenants whose snapshots are recent

### Event flow

Writes (e.g., `prime_add_node`) emit a `prime.node.created` event to Core via the existing Core ingest path, with `tenant_id` set by the gateway. The local in-memory Prime updates immediately (for read-your-writes consistency within the session). The event is the source of truth.

Reads serve from the in-memory Prime. Eventual consistency between machines (if we ever shard) is fine because each tenant is pinned to a machine at session start.

### Billing & rate limits

Existing Core billing meters event ingest. Hosted MCP tool calls are additional cost — reads especially, since they don't emit events. Three new meters:

- `prime.tool.read` — per call (cheap, ~$0.0001 per call equivalent)
- `prime.tool.write` — per call (already covered by the event ingest meter)
- `prime.session.minutes` — per minute of active connection (capacity cost)

Free tier: 10k reads / 10k writes / 30 minutes session per day. Same shape as the existing free-tier event allowance.

### Capacity planning

Single Fly machine (`shared-cpu-2x@2GB`) targets:
- ~100 hot tenants in memory (assumes ~10MB per tenant)
- ~500 cold tenants snapshotted to object storage
- Connection ceiling: ~50 concurrent sessions per machine

Scale-out: add machines, route by tenant_id hash. Sticky sessions for the lifetime of a connection. Cross-machine writes via Core (the single source of truth) keep coherence.

### Migration story for existing local users

Local `allsource-prime` users today have `--sync-to https://api.all-source.xyz` pushing their `prime.*` events into Core. When hosted MCP ships, those events are already in the right place — switching to hosted means:

1. Disconnect the local Prime from Claude Desktop's config
2. Add the hosted Prime connector URL
3. Hosted Prime hydrates from the user's existing Core events on first connection

No data loss. Local Prime can continue running as a personal cache if the user wants.

### Open questions

1. **Region selection.** Single region (iad like Core) or multi-region? Multi-region means data residency wins but also cross-region replication concerns. Recommend: start single-region, defer multi-region until customer ask.

2. **OAuth vs Bearer.** Bearer is simpler. OAuth is what Claude Desktop's connector UI may eventually expect (per MCP authorization spec). Recommend: ship Bearer first, add OAuth if Claude Desktop drops Bearer support.

3. **Per-call billing granularity.** Per-call meter is high-volume — every `prime_recall` call writes a billing event. Aggregate per-minute or per-session instead? Recommend: per-session aggregation (one event per session close) to keep Core's event volume sane.

4. **Connection limits per tier.** Free tier: 1 concurrent session? Pro: 5? Need to land before we publish pricing.

5. **Cold-start UX in Claude Desktop.** If a user's first call after a cold start takes 5s, does Claude Desktop time out? Need to test. Mitigation: respond with a "warming up" partial event within 1s.

## Out of Scope (for this proposal)

- Selling Prime as a managed service to non-AllSource users (would change the auth story)
- E2E encryption of in-memory data (different threat model)
- Federated Prime instances (cross-tenant queries — explicitly not happening)

## Effort Estimate

Rough estimate, broken into shippable atoms:

| Atom | Description | Effort |
|---|---|---|
| 1 | Add `prime:invoke` scope; `/connect` requests it | 1d |
| 2 | Make `Prime` struct cleanly per-tenant in allsource-core | 3-5d |
| 3 | New `apps/prime-mcp-gateway` scaffolding (fly.toml, Dockerfile, axum app) | 2d |
| 4 | MCP-over-HTTP Streamable transport implementation | 3-5d |
| 5 | Tenant router with hot/cold LRU + snapshot store | 5-7d |
| 6 | Billing meter integration + rate limits | 2-3d |
| 7 | Claude Desktop end-to-end testing + docs update | 2d |
| **Total** | | **~3-4 weeks of focused work** |

This is a real product investment, not a weekend. Worth checking that the demand is there (or that we're confident enough in the wedge) before starting.

## Decision Status

- [x] Sign off on Option A (new `prime-mcp-gateway` service) — approved 2026-05-23
- [ ] Confirm the billing model (per-session aggregation vs per-call)
- [ ] Confirm free-tier limits (sessions, concurrent connections)
- [ ] Owner assignment (who builds this and when)
- [ ] Sequencing vs other roadmap work (DXT is shipping, Bearer-MCP is months out)

Implementation should not start until the remaining decisions are made — billing in particular needs to be settled before atoms 5/6 of the effort estimate.
