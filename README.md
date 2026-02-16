---
title: "AllSource Event Store - Monorepo"
status: CURRENT
last_updated: 2026-02-15
version: "0.10.3"
---

# AllSource Event Store

[![CI](https://github.com/all-source-os/all-source/actions/workflows/ci.yml/badge.svg)](https://github.com/all-source-os/all-source/actions/workflows/ci.yml)
[![Container CI](https://github.com/all-source-os/all-source/actions/workflows/container-ci.yml/badge.svg)](https://github.com/all-source-os/all-source/actions/workflows/container-ci.yml)
[![Docker Build](https://github.com/all-source-os/all-source/actions/workflows/docker-build.yml/badge.svg)](https://github.com/all-source-os/all-source/actions/workflows/docker-build.yml)
[![Release](https://img.shields.io/github/v/release/all-source-os/all-source?label=release)](https://github.com/all-source-os/all-source/releases/latest)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)

High-performance event sourcing platform with distributed architecture and AI-native tooling.

---

## Quick Links

| Category | Links |
|----------|-------|
| **Documentation** | [Quick Start](docs/QUICK_START.md) · [Docs Hub](docs/README.md) · [Architecture](docs/current/CLEAN_ARCHITECTURE.md) · [API Reference](docs/current/API_REFERENCE.md) |
| **Deployment** | [Docker Guide](docs/deployment/DOCKER.md) · [Helm Chart](deploy/helm/allsource/) · [Kubernetes](deploy/k8s/) |
| **Services** | [Core](apps/core/README.md) · [Control Plane](apps/control-plane/README.md) · [Query Service](apps/query-service/README.md) · [MCP Server](apps/mcp-server-elixir/README.md) · [Web](apps/web/README.md) |
| **Roadmap** | [Consolidated Roadmap](docs/roadmaps/2026-02-15_CONSOLIDATED_ROADMAP.md) |

---

## Project Status (v0.10.3)

### Rust Core (port 3900)

The database. Source of truth for all event data.

- Event ingestion at 469K events/sec, 11.9us query latency
- WAL (CRC32, fsync) + Parquet (Snappy) + DashMap for durability and speed
- Leader-follower replication via WAL shipping
- Schema registry with JSON Schema validation
- Stream processing pipelines (Filter, Map, Reduce, Window, Branch, Enrich)
- Multi-tenancy with quotas, RBAC, audit logging
- Vector search (fastembed + HNSW) and BM25 keyword search (tantivy)
- Zero external database dependencies

**Known gaps**: Fork event commit is stubbed (TODO), cloud KMS providers unimplemented (Local only), Parquet metadata checksum verification missing. See [roadmap P0](docs/roadmaps/2026-02-15_CONSOLIDATED_ROADMAP.md#p0-fix-existing-gaps).

### Go Control Plane (port 3901)

Authentication, authorization, billing, and operational management.

- JWT authentication & RBAC (4 roles, 7 permissions)
- Policy enforcement engine with audit logging
- LemonSqueezy billing integration (checkout, webhooks, usage reporting)
- Authenticated proxying to Core
- HAL hypermedia API responses, OpenAPI specification
- OpenTelemetry distributed tracing

### Elixir Query Service (port 3902)

Stateless API gateway. Routes to Core, handles auth delegation to Control Plane.

- No PostgreSQL dependency — fully stateless
- JWT & API key auth delegated to Control Plane
- Tesla HTTP client to Core with connection pooling
- Broadway event processing pipeline
- OpenAPI specification via `open_api_spex`

**Known gaps**: 5 endpoints return 501 Not Implemented (`GET /api/events/:id`, projection delete/state/reset/rebuild_stats). These require Core API additions first. Phoenix Channels WebSocket for external clients not yet built (internal WS client to Core works). Broadway Kafka/RabbitMQ dependencies exist but are not wired. See [roadmap](docs/roadmaps/2026-02-15_CONSOLIDATED_ROADMAP.md#1-query-service-api-completeness).

### MCP Server (61 tools)

AI-native interface for Claude Desktop or any MCP client.

- 61 tool definitions across 11 categories (discover, search, drill-down, context, mutate, event management, operations, tenants, schema, analytics, dev tools)
- TOON format responses (~50% fewer tokens than JSON)
- Conversation context for multi-turn sessions

**Known gaps**: Analytics tools (cohort, correlation, forecast, churn, LTV, etc.) are basic client-side aggregations, not sophisticated ML. Schema tools (migrate, infer, diff) compute client-side without Core API support. `get_query_advice` is a hardcoded lookup table. See [roadmap](docs/roadmaps/2026-02-15_CONSOLIDATED_ROADMAP.md#6-mcp-analytics-tool-quality).

### Web Dashboard (port 3000)

- Next.js 16 + TypeScript + React + Tailwind + shadcn/ui
- Auth pages (login, signup, forgot/reset password, verify email)
- Dashboard pages: Events, API Keys, Billing, Pipelines, Settings
- OAuth UI scaffolded (Google, GitHub buttons present)

---

## Architecture

```
Clients --> Query Service (Elixir, port 3902) --> Core (Rust, port 3900)
                |                                     |
           Control Plane (Go, port 3901)        WAL + Parquet + DashMap
           (auth, billing, policies)            (events, projections,
                                                 snapshots, schemas)
```

### Monorepo Structure

```
apps/
  core/               # Rust event store
  control-plane/      # Go auth/billing/operations
  query-service/      # Elixir API gateway
  mcp-server-elixir/  # MCP server for AI agents
  web/                # Next.js dashboard

deploy/
  helm/               # Helm charts
  k8s/                # Kubernetes manifests
  cloudrun/           # Cloud Run configs
  prometheus/         # Monitoring config
  grafana/            # Grafana provisioning
```

---

## Releases & Docker Images

All services maintain consistent versioning at **v0.10.3**.

| Service | Image | Size | Base |
|---------|-------|:----:|------|
| Core | `ghcr.io/all-source-os/allsource-core:0.10.3` | 15.7 MB | Distroless |
| Control Plane | `ghcr.io/all-source-os/allsource-control-plane:0.10.3` | 27.9 MB | Distroless |
| Query Service | `ghcr.io/all-source-os/allsource-query-service:0.10.3` | 35.1 MB | Alpine |
| Web | `ghcr.io/all-source-os/allsource-web:0.10.3` | ~50 MB | Alpine |

**Total production footprint: ~129 MB**

### Version Locations

| Service | File |
|---------|------|
| Core | `apps/core/Cargo.toml` |
| Control Plane | `apps/control-plane/main.go` |
| Query Service | `apps/query-service/mix.exs` |
| MCP Server | `apps/mcp-server-elixir/mix.exs` |
| K8s Manifests | `deploy/k8s/*.yaml` |

```bash
# Quick start
docker compose up -d

# Pull specific version
docker pull ghcr.io/all-source-os/allsource-core:0.10.3

# Version management
make check-versions           # Verify consistency
make set-version VERSION=X.Y.Z  # Update all locations
make bump-version             # Interactive bump
```

See [Release Guide](docs/guides/RELEASE.md) for the full release process.

---

## Development

### Prerequisites

- **Rust**: 1.92+
- **Go**: 1.24+
- **Elixir**: 1.17+ (Erlang/OTP 27+)
- **Bun**: 1.3+ (for web)

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
cd apps/core && cargo test --lib
cd apps/control-plane && go test ./...
cd apps/query-service && mix test
cd apps/mcp-server-elixir && mix test
```

---

## Roadmap

Full details: [Consolidated Roadmap](docs/roadmaps/2026-02-15_CONSOLIDATED_ROADMAP.md)

### P0: Fix Existing Gaps

Items previously marked complete that have known cracks:

- **Query Service**: 5 endpoints return 501 (event by ID, projection delete/state/reset/rebuild_stats)
- **Core**: Fork event commit stubbed, cloud KMS unimplemented, Parquet checksum TODO
- **Core**: Vector search mock repositories incomplete for testing
- **MCP**: Analytics tools are basic client-side aggregations, schema tools lack Core API backing

### P1: SaaS Launch

- Deploy to Fly.io, create LemonSqueezy products, landing page
- Onboarding wizard, quick start docs, usage warning emails
- JavaScript SDK, then Python and Go SDKs
- Simple customer dashboard, webhook delivery, status page

### P2: Query Service Phase 3

- Phoenix Channels WebSocket for external clients
- Wire Broadway Kafka/RabbitMQ integrations
- Distributed mode (libcluster + Horde)
- Grafana dashboards, Swagger UI

### P3: Future (2027)

- Multi-node clustering (simplified Raft)
- Geo-replication (CRDT conflict resolution)
- EventQL query language, GraphQL API

---

## License

[MIT License](LICENSE)

---

## Support

- **Issues**: [GitHub Issues](https://github.com/all-source-os/all-source/issues)
- **Releases**: [GitHub Releases](https://github.com/all-source-os/all-source/releases)
- **Documentation**: [Docs Hub](docs/README.md)
