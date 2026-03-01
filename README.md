---
title: "AllSource Event Store - Monorepo"
status: CURRENT
last_updated: 2026-03-01
version: "0.12.0"
---

<div align="center">

# AllSource Event Store

**High-performance event sourcing platform with distributed architecture and AI-native tooling.**

[![CI](https://github.com/all-source-os/all-source/actions/workflows/ci.yml/badge.svg)](https://github.com/all-source-os/all-source/actions/workflows/ci.yml)
[![Container CI](https://github.com/all-source-os/all-source/actions/workflows/container-ci.yml/badge.svg)](https://github.com/all-source-os/all-source/actions/workflows/container-ci.yml)
[![Docker Build](https://github.com/all-source-os/all-source/actions/workflows/docker-build.yml/badge.svg)](https://github.com/all-source-os/all-source/actions/workflows/docker-build.yml)
[![Release](https://img.shields.io/github/v/release/all-source-os/all-source?label=release&color=blue)](https://github.com/all-source-os/all-source/releases/latest)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)

[![Core](https://img.shields.io/badge/Core-v0.12.0-orange?logo=rust&logoColor=white)](apps/core/)
[![Control Plane](https://img.shields.io/badge/Control_Plane-v0.12.0-00ADD8?logo=go&logoColor=white)](apps/control-plane/)
[![Query Service](https://img.shields.io/badge/Query_Service-v0.12.0-4B275F?logo=elixir&logoColor=white)](apps/query-service/)
[![Web](https://img.shields.io/badge/Web-v0.12.0-000000?logo=next.js&logoColor=white)](apps/web/)
[![MCP Server](https://img.shields.io/badge/MCP_Server-61_tools-8A2BE2)](apps/mcp-server-elixir/)

[![Core Image](https://img.shields.io/badge/ghcr.io-allsource--core:0.12.0-blue?logo=docker&logoColor=white)](https://ghcr.io/all-source-os/allsource-core)
[![Control Plane Image](https://img.shields.io/badge/ghcr.io-allsource--control--plane:0.12.0-blue?logo=docker&logoColor=white)](https://ghcr.io/all-source-os/allsource-control-plane)
[![Query Service Image](https://img.shields.io/badge/ghcr.io-allsource--query--service:0.12.0-blue?logo=docker&logoColor=white)](https://ghcr.io/all-source-os/allsource-query-service)
[![Web Image](https://img.shields.io/badge/ghcr.io-allsource--web:0.12.0-blue?logo=docker&logoColor=white)](https://ghcr.io/all-source-os/allsource-web)

</div>

---

## Quick Links

| | |
|---|---|
| **Get Started** | [Quick Start](docs/QUICK_START.md) · [Docker Guide](docs/deployment/DOCKER.md) · [Troubleshooting](docs/guides/TROUBLESHOOTING.md) |
| **Architecture** | [Clean Architecture](docs/current/CLEAN_ARCHITECTURE.md) · [Tenant Model](docs/current/TENANT_ARCHITECTURE.md) · [Replication Design](docs/proposals/CORE_REPLICATION_DESIGN.md) |
| **API & Specs** | [API Reference](docs/current/API_REFERENCE.md) · [Performance](docs/current/PERFORMANCE.md) · [Event Store Features](docs/current/EVENT_STORE_FEATURES.md) |
| **Operations** | [Release Guide](docs/guides/RELEASE.md) · [Quality Gates](docs/current/QUALITY_GATES.md) · [WebSocket Config](docs/guides/WEBSOCKET_CONFIGURATION.md) |
| **Services** | [Core](apps/core/) · [Control Plane](apps/control-plane/) · [Query Service](apps/query-service/) · [MCP Server](apps/mcp-server-elixir/) · [Web](apps/web/) |
| **Deploy** | [Helm Chart](deploy/helm/allsource/) · [Kubernetes](deploy/k8s/) · [Fly.io](apps/core/fly.toml) |

---

## Architecture

```
Clients --> Query Service (Elixir, :3902) --> Core (Rust, :3900)
                |                                  |
           Control Plane (Go, :3901)         WAL + Parquet + DashMap
           (auth, billing, policies)         (events, projections,
                                              snapshots, schemas)
```

**Core IS the database.** No PostgreSQL for events — just WAL (CRC32, fsync), Parquet (Snappy compression), and DashMap (concurrent in-memory reads). Zero external dependencies. [Full architecture docs →](docs/current/CLEAN_ARCHITECTURE.md)

### Monorepo Structure

```
apps/
  core/               # Rust event store          — the database
  control-plane/      # Go auth/billing/ops       — the gatekeeper
  query-service/      # Elixir API gateway        — the router
  mcp-server-elixir/  # MCP server (61 tools)     — the AI interface
  web/                # Next.js dashboard          — the UI

deploy/
  helm/               # Helm charts
  k8s/                # Kubernetes manifests
  cloudrun/           # Cloud Run configs
  prometheus/         # Monitoring config
  grafana/            # Grafana provisioning

tooling/
  data-flow-test/     # E2E data flow test
  durability-test/    # WAL/Parquet durability test
```

---

## Project Status & Roadmap (v0.12.0)

### What's New in v0.12.0

- **Network sync transport**: HTTP pull/push bidirectional sync with version vectors for offline-first embedded clients
- **Configurable conflict resolution**: `MergeStrategy` (AppendOnly, LastWriteWins, FirstWriteWins) with per-entity-type prefix matching
- **MCP tool event emission**: `McpToolTracker` with auto-timing feeding `ToolCallAuditProjection` end-to-end
- **WebSocket backpressure**: configurable batching, max batch size, lagged notification (backward compatible)
- **Embedded Core library** (v0.11.0): 8-phase implementation — use Core as an in-process library with TOON output for LLMs
- **Full dependency upgrade** (v0.11.0): arrow 57, datafusion 52, rand 0.10, reqwest 0.13, tantivy 0.25, fastembed 5

> Full roadmap: [Consolidated Roadmap](docs/roadmaps/2026-02-15_CONSOLIDATED_ROADMAP.md) · Known gaps: [Roadmap P0](docs/roadmaps/2026-02-15_CONSOLIDATED_ROADMAP.md#p0-fix-existing-gaps)

### Rust Core (port 3900) — [docs](apps/core/) · [features](docs/current/EVENT_STORE_FEATURES.md) · [perf](docs/current/PERFORMANCE.md)

The database. Source of truth for all event data.

- 469K events/sec ingestion, 11.9us query latency
- WAL (CRC32, fsync) + Parquet (Snappy) + DashMap for durability and speed
- **v0.10.4+**: persistence wiring fix — env vars now correctly configure WAL+Parquet on startup
- Leader-follower replication via WAL shipping ([design](docs/proposals/CORE_REPLICATION_DESIGN.md))
- Schema registry, stream processing pipelines, multi-tenancy with RBAC
- Vector search (fastembed + HNSW) and BM25 keyword search (tantivy)
- **Embedded API**: use Core as an in-process library (1489 tests, 8 phases complete) with TOON output, network sync, and conflict strategies

### Go Control Plane (port 3901) — [docs](apps/control-plane/)

Authentication, authorization, billing, and operational management.

- JWT auth & RBAC (4 roles, 7 permissions), policy enforcement with audit logging
- OAuth login (GitHub, Google) with CSRF-protected state cookies
- LemonSqueezy billing integration, HAL hypermedia API, OpenAPI spec
- OpenTelemetry distributed tracing

### Elixir Query Service (port 3902) — [docs](apps/query-service/) · [API ref](docs/allsource-qs-api-reference.md)

Stateless API gateway. Routes to Core, delegates auth to Control Plane.

- Fully stateless, no PostgreSQL dependency
- Server-side projections with fold-on-read and continuous folding via PubSub
- `POST /api/query/projected` — snapshot-aware fold endpoint
- `AUTH_DISABLED` mode for local dev (fully bypasses all auth)
- Tesla HTTP client with connection pooling, Broadway event processing
- OpenAPI specification via `open_api_spex`

### MCP Server (61 tools) — [docs](apps/mcp-server-elixir/)

AI-native interface for Claude Desktop or any MCP client.

- 61 tools across 11 categories (discover, search, drill-down, context, mutate, ops, tenants, schema, analytics, dev)
- TOON format responses (~50% fewer tokens than JSON)

### Web Dashboard (port 3000) — [docs](apps/web/)

- Next.js 16 + TypeScript + React + Tailwind + shadcn/ui
- Events, API Keys, Billing, Pipelines, Settings pages

### SDKs — [Rust](sdks/rust/) · [Go](sdks/go/) · [Python](sdks/python-client/) · [TypeScript](sdks/typescript/)

- Self-hosted SDK registry (`apps/registry`) serving Cargo, Go, npm, and PyPI protocols
- All SDKs distributed via `registry.all-source.xyz`

### What's Next

| Priority | Focus | Details |
|----------|-------|---------|
| **P0** | Fix existing gaps | 5 QS endpoints return 501, Core fork commit stubbed, MCP analytics are basic aggregations |
| **P1** | SaaS launch | Fly.io deploy, LemonSqueezy products, onboarding wizard, landing page |
| **P2** | QS Phase 3 | Phoenix Channels WebSocket, Broadway Kafka/RabbitMQ, distributed mode |
| **P3** | Future | Multi-node Raft, geo-replication (CRDT), GraphQL |

---

## Docker Images

All services ship at **v0.12.0**. Total production footprint: **~129 MB**.

| Service | Image | Size | Base |
|---------|-------|:----:|------|
| Core | `ghcr.io/all-source-os/allsource-core:0.12.0` | 15.7 MB | Distroless |
| Control Plane | `ghcr.io/all-source-os/allsource-control-plane:0.12.0` | 27.9 MB | Distroless |
| Query Service | `ghcr.io/all-source-os/allsource-query-service:0.12.0` | 35.1 MB | Alpine |
| Web | `ghcr.io/all-source-os/allsource-web:0.12.0` | ~50 MB | Alpine |

```bash
# Quick start
docker compose up -d

# Pull specific version
docker pull ghcr.io/all-source-os/allsource-core:0.12.0
```

See [Docker Guide](docs/deployment/DOCKER.md) · [Release Guide](docs/guides/RELEASE.md)

---

## Development

### Prerequisites

- **Rust** 1.92+ · **Go** 1.24+ · **Elixir** 1.17+ (OTP 27+) · **Bun** 1.3+

### Quick Start

```bash
git clone https://github.com/all-source-os/all-source.git
cd allsource-monorepo
docker compose up -d

# Or run individual services
cd apps/core && cargo run
cd apps/control-plane && go run .
cd apps/query-service && mix phx.server
```

### Testing

```bash
cd apps/core && cargo test --lib          # 1489 tests
cd apps/control-plane && go test ./...
cd apps/query-service && mix test
cd apps/mcp-server-elixir && mix test
```

### Quality Gates

```bash
make quality-rust       # fmt + clippy + test + doc
make quality-go         # vet + lint + test
make quality-elixir     # format + credo + test
make check-versions     # Verify version consistency
```

See [Quality Gates](docs/current/QUALITY_GATES.md) · [Quality Gates Setup](docs/guides/QUALITY_GATES_SETUP.md)

### Version Locations

| Service | File |
|---------|------|
| Core | `apps/core/Cargo.toml` |
| Control Plane | `apps/control-plane/main.go` |
| Query Service | `apps/query-service/mix.exs` |
| MCP Server | `apps/mcp-server-elixir/mix.exs` |
| K8s Manifests | `deploy/k8s/*.yaml` |

---

## License

[MIT License](LICENSE)

---

<div align="center">

[Issues](https://github.com/all-source-os/all-source/issues) · [Releases](https://github.com/all-source-os/all-source/releases) · [Docs Hub](docs/README.md)

</div>
