---
title: "AllSource Event Store - AllSource Monorepo"
status: CURRENT
last_updated: 2026-02-11
version: "0.9.1"
---

# AllSource Event Store - AllSource Monorepo

[![CI](https://github.com/all-source-os/allsource-monorepo/actions/workflows/ci.yml/badge.svg)](https://github.com/all-source-os/allsource-monorepo/actions/workflows/ci.yml)
[![Container CI](https://github.com/all-source-os/allsource-monorepo/actions/workflows/container-ci.yml/badge.svg)](https://github.com/all-source-os/allsource-monorepo/actions/workflows/container-ci.yml)
[![Docker Build](https://github.com/all-source-os/allsource-monorepo/actions/workflows/docker-build.yml/badge.svg)](https://github.com/all-source-os/allsource-monorepo/actions/workflows/docker-build.yml)
[![Release](https://img.shields.io/github/v/release/all-source-os/allsource-monorepo?label=release)](https://github.com/all-source-os/allsource-monorepo/releases/latest)
[![Rust Core](https://img.shields.io/badge/Rust%20Core-v0.9.0-green.svg)](apps/core/)
[![Go Control Plane](https://img.shields.io/badge/Go%20Control%20Plane-v0.9.0-blue.svg)](apps/control-plane/)
[![Elixir Query Service](https://img.shields.io/badge/Elixir%20Query-v0.9.0-purple.svg)](apps/query-service/)
[![MCP Server](https://img.shields.io/badge/MCP%20Server-43%20Tools-orange.svg)](apps/mcp-server-elixir/)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)

High-performance event sourcing platform with distributed architecture and AI-native tooling.

---

## Quick Links

| Category | Links |
|----------|-------|
| **Documentation** | [Docs Hub](docs/README.md) · [Architecture](docs/current/CLEAN_ARCHITECTURE.md) · [API Reference](docs/current/API_REFERENCE.md) |
| **Deployment** | [Docker Guide](docs/deployment/DOCKER.md) · [Helm Chart](deploy/helm/allsource/) · [Kubernetes](deploy/k8s/) |
| **Services** | [Rust Core](apps/core/README.md) · [Control Plane](apps/control-plane/README.md) · [Query Service](apps/query-service/README.md) · [MCP Server](apps/mcp-server-elixir/README.md) · [Web](apps/web/README.md) |
| **Roadmaps** | [SaaS Launch Roadmap](docs/roadmaps/SAAS_LAUNCH_ROADMAP.md) · [Consolidated Roadmap](docs/roadmaps/2026-02-02_CONSOLIDATED_ROADMAP.md) |

---

## Docker Images

Production-optimized containers with minimal footprint:

| Service | Image | Version | Size | Base |
|---------|-------|:-------:|:----:|------|
| **Core** | `ghcr.io/all-source-os/allsource-core` | [![v0.9.0](https://img.shields.io/badge/ghcr-v0.9.0-blue?logo=github)](https://github.com/all-source-os/allsource-monorepo/pkgs/container/allsource-core) | **15.7 MB** | Distroless |
| **Control Plane** | `ghcr.io/all-source-os/allsource-control-plane` | [![v0.9.0](https://img.shields.io/badge/ghcr-v0.9.0-blue?logo=github)](https://github.com/all-source-os/allsource-monorepo/pkgs/container/allsource-control-plane) | **27.9 MB** | Distroless |
| **Query Service** | `ghcr.io/all-source-os/allsource-query-service` | [![v0.9.0](https://img.shields.io/badge/ghcr-v0.9.0-blue?logo=github)](https://github.com/all-source-os/allsource-monorepo/pkgs/container/allsource-query-service) | **35.1 MB** | Alpine |
| **Web** | `ghcr.io/all-source-os/allsource-web` | [![v0.9.0](https://img.shields.io/badge/ghcr-v0.9.0-blue?logo=github)](https://github.com/all-source-os/allsource-monorepo/pkgs/container/allsource-web) | **~50 MB** | Alpine |

**Total production footprint: ~129 MB** (excluding database)

```bash
# Quick Start with Docker Compose
docker compose up -d

# Or pull specific version from GHCR
docker pull ghcr.io/all-source-os/allsource-core:0.9.0
docker pull ghcr.io/all-source-os/allsource-control-plane:0.9.0
docker pull ghcr.io/all-source-os/allsource-query-service:0.9.0
docker pull ghcr.io/all-source-os/allsource-web:0.9.0
```

---

## Project Status

### Current Release: v0.9.1 (February 2026)

**Rust Core**
- Event store with 469K events/sec throughput
- Schema registry with JSON Schema validation
- Stream processing pipelines (Filter, Map, Reduce, Window, Branch, Enrich)
- Multi-tenancy, RBAC, audit logging
- Parquet storage + WAL for durability
- **Zero external database dependencies**

**Go Control Plane**
- JWT authentication & RBAC (4 roles, 7 permissions)
- Policy enforcement engine
- OpenTelemetry distributed tracing
- Authenticated proxying to Core

**Elixir Query Service**
- Query DSL with fluent Elixir pipes
- GenServer-based projections with OTP supervision
- Event pipelines (Filter, Transform, Enrich, Validate, Route, Aggregate)
- Phoenix HTTP API with OpenAPI spec
- WebSocket channels for real-time updates
- Prometheus metrics and APM integration

**MCP Server (43 Tools)**
- AI-native interface via Claude Desktop
- **Core Tools** (19): queries, time series, funnel analysis, anomaly detection, projections, schemas, snapshots
- **Event Management** (8): delete, archive, restore, export, import, clone, merge, split
- **Operational** (10): storage compaction, WAL status, backups, deep health checks, performance reports, audit logs
- **Tenant Management** (6): create, update, usage, quotas, suspend, export
- Dry-run preview mode and audit trails on all operations
- 429 tests passing

**Web Dashboard**
- Modern login/signup with OAuth (Google, GitHub)
- Accessible design (WCAG 2.1 compliant)
- Real-time event visualization

---

## Architecture

### Monorepo Structure

```
apps/
├── core/               # Rust event store (port 3900)
├── control-plane/      # Go control plane (port 3901)
├── query-service/      # Elixir query service (port 3902)
├── mcp-server-elixir/  # MCP server for AI agents
└── web/                # Next.js dashboard (port 3000/3908)

packages/
└── ui/                 # Shared UI components

deploy/
├── helm/               # Helm charts
├── k8s/                # Kubernetes manifests
└── prometheus/         # Monitoring config
```

### Service Ports (Local Development)

| Service | Default Port | Override Port |
|---------|:------------:|:-------------:|
| Core | 3900 | 3900 |
| Control Plane | 8080 | 3901 |
| Query Service | 3902 | 3902 |
| PostgreSQL | 5432 | 3903 |
| Web | 3000 | 3908 |
| Redis | 6379 | 3905 |
| Prometheus | 9090 | 3906 |
| Grafana | 3000 | 3907 |

Use `docker-compose.override.yml` for isolated local development with override ports.

---

## Performance

- **Ingestion**: 469,000 events/sec
- **Query p99**: 11.9μs
- **Concurrent writes**: 7.98ms (8 threads)

**Optimizations**:
- Lock-free data structures (DashMap)
- Zero-cost field access
- No validation in hot path
- Batch processing support

---

## Development

### Prerequisites
- **Rust**: 1.92+
- **Go**: 1.24+
- **Elixir**: 1.17+ (with Erlang/OTP 27+)
- **Bun**: 1.3+ (for web apps)

### Quick Start

```bash
# Clone and start all services
git clone https://github.com/all-source-os/allsource-monorepo.git
cd allsource-monorepo
docker compose up -d

# Or run individual services
cd apps/core && cargo run
cd apps/control-plane && go run .
cd apps/query-service && mix phx.server
```

### Testing

```bash
# Rust Core
cd apps/core && cargo test --lib

# Go Control Plane
cd apps/control-plane && go test ./...

# Elixir Query Service
cd apps/query-service && mix test

# MCP Server
cd apps/mcp-server-elixir && mix test
```

---

## Roadmap

### Completed (v0.9.1)
- 43 MCP tools (19 core + 8 event management + 10 operational + 6 tenant)
- Event Management Tools (delete, archive, restore, export, import, clone, merge, split)
- Operational Tools (storage, WAL, backups, health, performance, audit)
- Tenant Management Tools (CRUD, usage, quotas, suspend, export)
- Web dashboard with OAuth, onboarding, billing UI
- Consistent versioning across all services
- OpenAPI specification for Query Service
- WebSocket channels and real-time updates

### In Progress
- SaaS launch (self-service signup, billing integration)
- Core WAL-based replication (leader-follower)
- Go Control Plane PostgreSQL migration

### Planned
- Event sourcing SDK for popular languages
- Multi-region deployment
- GraphQL API layer

**Detailed Roadmaps**:
- [SaaS Launch Roadmap](docs/roadmaps/SAAS_LAUNCH_ROADMAP.md)
- [Consolidated Roadmap](docs/roadmaps/2026-02-02_CONSOLIDATED_ROADMAP.md)
- [MCP Server Changelog](apps/mcp-server-elixir/CHANGELOG.md)

---

## Version Management

All services maintain consistent versioning at **v0.9.0**.

### Version Reference

| Service | File | Current |
|---------|------|:-------:|
| Core | `apps/core/Cargo.toml` | 0.9.0 |
| Control Plane | `apps/control-plane/main.go` | 0.9.0 |
| Query Service | `apps/query-service/mix.exs` | 0.9.0 |
| MCP Server | `apps/mcp-server-elixir/mix.exs` | 0.9.0 |
| K8s Manifests | `deploy/k8s/*.yaml` | 0.9.0 |

### Commands

```bash
make check-versions    # Verify consistency
make set-version VERSION=0.10.0  # Update all
make bump-version      # Interactive bump
```

---

## License

[MIT License](LICENSE)

---

## Support

- **Issues**: [GitHub Issues](https://github.com/all-source-os/allsource-monorepo/issues)
- **Releases**: [GitHub Releases](https://github.com/all-source-os/allsource-monorepo/releases)
- **Documentation**: [Docs Hub](docs/README.md)

---

**Last Updated**: February 12, 2026
**Version**: v0.9.1
