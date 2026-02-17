# CLAUDE.md — AllSource Chronos Monorepo

## Critical: Architecture Facts

**AllSource Core IS the database.** It is a purpose-built Rust event store with full durability:
- **WAL** (Write-Ahead Log): CRC32 checksums, configurable fsync (default 100ms), crash recovery
- **Parquet**: Columnar persistence with Snappy compression, periodic flush
- **DashMap**: In-memory concurrent map for 11.9μs queries and 469K events/sec throughput

**DO NOT** describe Core as "in-memory only", "dumb", "not production-ready", or claim "data is lost on restart." Event data is durable. Only Core's user/tenant metadata (a separate concern) is in-memory — and that responsibility belongs to the Query Service, not Core.

**DO NOT** suggest storing events in PostgreSQL. PostgreSQL is for operational metadata only (users, tenants, API keys, billing). The correct way to improve Core's availability is through Core replication (leader-follower, WAL shipping) — not by adding another database.

## Repository Structure

**See `docs/MONOREPO_STRUCTURE.md` for full rules.** Key directories:

```
apps/              — Deployable services ONLY
  core/            — Rust event store (AllSource Core)
  query-service/   — Elixir/Phoenix API gateway (auth, billing, routing to Core)
  web/             — Next.js frontend dashboard
  mcp-server-elixir/ — Elixir MCP server (separate from Rust MCP Docker binary)
sdks/              — Client SDKs — ALL languages, NO EXCEPTIONS
  rust/            — Rust SDK
  go/              — Go SDK
  python-client/   — Python SDK
  typescript/      — TypeScript SDK (@allsource/client)
packages/          — Shared internal packages (NOT SDKs)
  ui/              — Shared UI component library
docs/
  proposals/       — Design proposals (e.g., CORE_REPLICATION_DESIGN.md)
  use-cases/       — Use case documents
  current/         — Current architecture docs
deploy/            — K8s manifests, deployment configs
tooling/           — Developer tools (durability tests, etc.)
```

**DO NOT** put SDKs in `packages/`, `apps/`, or anywhere else. SDKs go in `sdks/`.

## Service Architecture

```
Clients → Query Service (Elixir, port 3902) → Core (Rust, port 3900)
               |                                    |
          PostgreSQL                          WAL + Parquet + DashMap
          (users, tenants,                    (events, projections,
           API keys, billing)                  snapshots, schemas)
```

- **Core** = the database. Source of truth for all event data.
- **Query Service** = API gateway. Source of truth for users, tenants, billing.
- **PostgreSQL** = operational metadata only. Never for events.

## Core API

All Core endpoints use the `/api/v1/` prefix. Key endpoints:
- `POST /api/v1/events` — ingest event (returns 200, not 201)
- `GET /api/v1/events/query` — query events (returns `{"events": [...], "count": N}`)
- `GET /api/v1/projections` — list projections (returns `{"projections": [...], "total": N}`)
- `GET /api/v1/snapshots` — list snapshots
- `GET /api/v1/schemas` — list schemas
- `GET /health` — health check (note: root path, not /api/v1/health)
- `GET /metrics` — Prometheus metrics

Core wraps responses in maps (`{"events": [...]}`, `{"projections": [...]}`). The Query Service's RustCoreClient unwraps these before passing to controllers.

## Query Service Config

- `CORE_URL` — Core connection URL (not RUST_CORE_URL — clean env var names, no implementation details)
- `CORE_WS_URL` — Core WebSocket URL for real-time streaming
- Config key in Elixir: `:core_url` (not `:rust_core_url`)

## Docker Stack

Defined in the wallet project: `/Users/decebaldobrica/Projects/alphaSigmaPro/wallet/docker/docker-compose.allsource.yml`

Services on `supabase_network_alpha-sigma-pro`:
- `allsource-core-leader` (port 3280 → 3900, replication on 3910)
- `allsource-core-follower-1` (port 3281 → 3900)
- `allsource-core-follower-2` (port 3282 → 3900)
- `allsource-query-service` (port 3283 → 3902)
- `allsource-mcp` (port 3904)

Building Docker on Apple Silicon: native arm64 only — QEMU cross-compilation to linux/amd64 fails on Erlang NIF.

## Scaling Strategy

See `docs/proposals/CORE_REPLICATION_DESIGN.md`:
- Leader-follower replication via WAL shipping
- Query Service routes writes to leader, reads round-robin across followers
- No Raft, no PostgreSQL in the event path, no multi-leader
