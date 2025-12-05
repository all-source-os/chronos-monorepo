# AllSource Event Store - Chronos Monorepo

[![crates.io](https://img.shields.io/crates/v/allsource-core.svg)](https://crates.io/crates/allsource-core)
[![docs.rs](https://docs.rs/allsource-core/badge.svg)](https://docs.rs/allsource-core)
[![Rust Core](https://img.shields.io/badge/Rust%20Core-v0.2.0-green.svg)](apps/core/)
[![Go Control Plane](https://img.shields.io/badge/Go%20Control%20Plane-v1.0-blue.svg)](apps/control-plane/)
[![Elixir Query Service](https://img.shields.io/badge/Elixir%20Query-Phase%201%20Complete-purple.svg)](apps/query-service/)
[![MCP Server](https://img.shields.io/badge/MCP%20Server-Active-orange.svg)](apps/mcp-server-elixir/)

High-performance event sourcing platform with distributed architecture and AI-native tooling.

---

## Quick Links

| Category | Links |
|----------|-------|
| **Documentation** | [📋 Docs Hub](docs/README.md) · [Architecture](docs/current/CLEAN_ARCHITECTURE.md) · [SOLID Principles](docs/current/SOLID_PRINCIPLES.md) · [Performance](docs/current/PERFORMANCE.md) |
| **Bug Fixes** | [✅ Critical Bugs Fixed](docs/current/CRITICAL_BUGS_FIXED.md) · AllFrame Integration Unblocked (Nov 30, 2025) |
| **Quality** | [✅ Quality Gates](docs/current/QUALITY_GATES.md) · [Setup Guide](docs/guides/QUALITY_GATES_SETUP.md) · `make check` before commit |
| **Query Service** | [Roadmap](docs/roadmaps/query-service-roadmap.md) · [Implementation Guide](docs/current/QUERY_SERVICE_IMPLEMENTATION_GUIDE.md) · [Architecture Optimization](docs/current/ARCHITECTURE_OPTIMIZATION.md) |
| **Roadmaps** | [Comprehensive Roadmap](docs/roadmaps/2025-10-22_COMPREHENSIVE_ROADMAP.md) · [Phase 1.5 Progress](docs/roadmaps/2025-10-22_PHASE_1.5_PROGRESS.md) · [TDD Results](docs/roadmaps/2025-10-22_PHASE_1.5_TDD_RESULTS.md) |
| **Guides** | [Quick Start](docs/guides/QUICK_START.md) · [Demo](docs/guides/DEMO.md) |
| **Services** | [Rust Core](apps/core/README.md) · [Go Control Plane](apps/control-plane/README.md) · [Query Service](apps/query-service/README.md) · [MCP Server (Elixir)](apps/mcp-server-elixir/README.md) · [Web](apps/web/README.md) |

---

## Project Status

### ✅ Current State (November 2025)

**Rust Core (v0.1.0)**
- Event store with 469K events/sec throughput
- Schema registry with JSON Schema validation
- Event replay engine for point-in-time rebuilds
- Stream processing pipelines (Filter, Map, Reduce, Window, Branch, Enrich)
- Multi-tenancy, RBAC, audit logging
- Parquet storage + WAL for durability
- **Zero external database dependencies** (DashMap in-memory + Parquet)

**Go Control Plane (v1.0)**
- JWT authentication & RBAC (4 roles, 7 permissions)
- Policy enforcement engine (5 default policies)
- OpenTelemetry distributed tracing
- Complete audit logging
- Authenticated proxying to Core

**Elixir Query Service (Phase 1 Complete)**
- 242 tests passing (7 doctests + 235 tests)
- Query DSL with fluent Elixir pipes
- GenServer-based projections with OTP supervision
- Event pipelines (Filter, Transform, Enrich, Validate, Route, Aggregate)
- Phoenix HTTP API (11 endpoints)
- Tesla client with Core integration
- **Phase 2**: 3-4 weeks (WebSocket integration, Broadway refinement)

**MCP Server (Elixir)**
- AI-native interface to event store via Claude Desktop
- 11 core tools for event operations
- **TOON format** by default (~50% fewer tokens than JSON)
- Real-time streaming and projections
- OTP supervision for fault tolerance
- Migrated from TypeScript to Elixir for better stack alignment
- v2.0 planned: 55+ tools

**Web Demo**
- Interactive Next.js showcase
- Real-time event visualization

---

## Architecture

### Monorepo Structure

```
apps/
├── core/               # Rust event store (port 3900)
├── control-plane/      # Go control plane (port 3901)
├── query-service/      # Elixir query service (port 3902)
├── mcp-server-elixir/ # MCP server (Elixir)
└── web/                # Next.js web app (port 3000)

packages/
└── ui/                 # Shared UI components

tooling/
├── biome/             # Linting config
└── e2e/               # E2E tests
```

### Service Architecture

**Distributed Services:**
- **Rust Core** (port 3900): High-performance event storage and processing
- **Go Control Plane** (port 3901): Enterprise orchestration and management
- **Elixir Query Service** (port 3902): Fault-tolerant query processing with OTP
- **MCP Server (Elixir)**: AI-native interface for Claude Desktop
- **Web Demo** (port 3000): Interactive Next.js showcase

**Key Principle: Zero External Databases**
- Core uses DashMap (in-memory) + Parquet files + WAL
- 11.9 μs query latency
- No PostgreSQL, no Redis, no external dependencies

### Rust Core (`apps/core`)

**Version**: v0.2.0 · [![crates.io](https://img.shields.io/crates/v/allsource-core.svg)](https://crates.io/crates/allsource-core) · [![docs.rs](https://docs.rs/allsource-core/badge.svg)](https://docs.rs/allsource-core)

```bash
# Add to your Cargo.toml (pin to minor version)
cargo add allsource-core@0.2
```

```toml
[dependencies]
allsource-core = "0.2"  # Pin to minor version for stability
```

**Features**: Schema registry, event replay, stream processing pipelines (6 operators)
**Storage**: DashMap + Parquet + WAL (zero external databases)
**Documentation**: [Core README](apps/core/README.md) · [Changelog](apps/core/docs/CHANGELOG.md) · [Features](apps/core/docs/FEATURES.md) · [Security](apps/core/docs/SECURITY.md) · [docs.rs](https://docs.rs/allsource-core)

### Go Control Plane (`apps/control-plane`)

**Version**: v1.0
**Features**: JWT auth, RBAC (4 roles, 7 permissions), policy enforcement, audit logging, OpenTelemetry tracing
**Documentation**: [Control Plane README](apps/control-plane/README.md)

### Elixir Query Service (`apps/query-service`)

**Status**: Phase 1 Complete, Phase 2 in planning (3-4 weeks)
**Port**: 3902
**Features**: Query DSL, GenServer projections, event pipelines, Phoenix HTTP API
**Tests**: 242 passing (7 doctests + 235 tests)
**Documentation**: [Query Service README](apps/query-service/README.md) · [Roadmap](docs/roadmaps/query-service-roadmap.md)

### MCP Server (`apps/mcp-server-elixir`)

**Status**: Active, migrated from TypeScript to Elixir
**Features**: 11 core tools, AI-native interface via Claude Desktop, OTP supervision, **TOON format** for ~50% token reduction
**Documentation**: [MCP README](apps/mcp-server-elixir/README.md) · [Setup Guide](docs/guides/mcp-server/CLAUDE_DESKTOP_SETUP.md)

### Web Demo (`apps/web`)

**Status**: Interactive showcase
**Port**: 3000
**Features**: Real-time event visualization
**Documentation**: [Web README](apps/web/README.md)

---

## Performance

### Current (v1.0)
- **Ingestion**: 469,000 events/sec
- **Query p99**: 11.9μs
- **Concurrent writes**: 7.98ms (8 threads)

### Optimizations Applied
- ✅ Lock-free data structures (DashMap)
- ✅ Zero-cost field access (public fields)
- ✅ No validation in hot path
- ✅ Batch processing support

**Full details**: [Performance Guide](docs/current/PERFORMANCE.md) · [Architecture Optimization](docs/current/ARCHITECTURE_OPTIMIZATION.md)

---

## Development

### Prerequisites
- **Rust**: 1.75+
- **Go**: 1.21+
- **Elixir**: 1.19+ (with Erlang/OTP 27+)
- **Node.js**: 18+ (for MCP server)

### Quick Start

```bash
# Clone the repository
git clone https://github.com/allsource/chronos-monorepo.git
cd chronos-monorepo

# Run Rust tests
cd apps/core
cargo test --lib

# Run Go tests
cd ../control-plane
go test ./...

# Run Elixir tests
cd ../query-service
mix test

# Run benchmarks
cd ../core
cargo bench --bench performance_benchmarks
```

**Detailed guide**: [Quick Start](docs/guides/QUICK_START.md)

---

## Testing

### Rust Core
```bash
cd apps/core

# All tests
cargo test --lib

# Specific module
cargo test --lib -- domain::

# With coverage
cargo tarpaulin --lib --out Html
```

**Status**: ✅ 86/86 tests passing (100%)
**Coverage**: 100% for domain and application layers

### Go Control Plane
```bash
cd apps/control-plane

# All tests
go test ./...

# With coverage
go test ./... -cover

# Verbose
go test -v ./...
```

**Status**: ✅ All tests passing, 23.2% coverage

### Elixir Query Service
```bash
cd apps/query-service

# All tests
mix test

# Watch mode
mix test.watch

# With coverage
mix test --cover
```

**Status**: ✅ 281/281 tests passing (100%)

---

## Documentation Organization

All documentation uses **timestamp-based organization** with clear deprecation markers:

```
docs/
├── current/          # ✅ Active documentation
├── archive/          # ⚠️ Historical/deprecated docs
├── roadmaps/         # 📋 Planning and progress
├── guides/           # 📚 How-to guides
├── architecture/     # 🏗️ ADRs
└── operations/       # 🔧 Ops guides
```

### Status Markers
- ✅ **CURRENT** - Active, up-to-date
- ⚠️ **DEPRECATED** - Historical only
- 🔄 **SUPERSEDED** - Replaced by newer doc
- 📝 **DRAFT** - Work in progress
- ⏳ **PLANNED** - Not yet implemented

**Full index**: [Documentation Index](docs/INDEX.md)

---

## Key Design Principles

### Clean Architecture
- **Layer 1 (Domain)**: Pure business entities with zero dependencies
- **Layer 2 (Application)**: Use cases orchestrating domain logic
- **Layer 3 (Infrastructure)**: Concrete implementations of abstractions
- **Layer 4 (Frameworks)**: Web servers, databases, external services

### SOLID Principles
- **SRP**: Each module has one reason to change
- **OCP**: Open for extension via traits
- **LSP**: Subtypes are substitutable
- **ISP**: Segregated read/write interfaces
- **DIP**: Depend on abstractions, not concretions

**Full details**: [SOLID Principles](docs/current/SOLID_PRINCIPLES.md)

---

## Additional Resources

**Documentation**:
- [Documentation Hub](docs/README.md) - Central documentation index
- [Architecture Guides](docs/current/) - Current architecture and design patterns
- [Roadmaps](docs/roadmaps/) - All roadmaps centralized
- [How-To Guides](docs/guides/) - Step-by-step guides

**Service-Specific**:
- [Rust Core](apps/core/README.md) · [Changelog](apps/core/docs/CHANGELOG.md) · [Features](apps/core/docs/FEATURES.md)
- [Go Control Plane](apps/control-plane/README.md)
- [Elixir Query Service](apps/query-service/README.md) · [Roadmap](docs/roadmaps/query-service-roadmap.md)
- [MCP Server](apps/mcp-server/README.md) · [Setup Guide](docs/guides/mcp-server/CLAUDE_DESKTOP_SETUP.md)
- [Web Demo](apps/web/README.md)

---

## Contributing

We follow a TDD approach for all refactoring:
1. Let tests guide the refactoring
2. Fix compilation errors systematically
3. Use tests as validation
4. Move aggressively - don't worry about backward compatibility during refactoring

**Detailed guide**: [Contributing](docs/guides/CONTRIBUTING.md)

---

## Roadmap

**Current Focus (November 2025):**
- ✅ Rust Core v0.1.0 with schema registry, replay engine, stream processing
- ✅ Go Control Plane v1.0 with enterprise features
- ✅ Elixir Query Service Phase 1 complete (242 tests passing)
- 📋 Query Service Phase 2 (3-4 weeks): WebSocket integration, Broadway refinement
- 📋 MCP Server v2.0: Expanding from 11 to 55+ tools

**Detailed Roadmaps:**
- [Query Service Roadmap](docs/roadmaps/query-service-roadmap.md) - Phase 2 planning (zero databases)
- [MCP v2 Enhancements](docs/roadmaps/mcp-v2-enhancements.md) - AI-native tooling expansion
- [Comprehensive Roadmap](docs/roadmaps/2025-10-22_COMPREHENSIVE_ROADMAP.md) - Overall vision

---

## License

[MIT License](LICENSE)

---

## Support

- **Issues**: [GitHub Issues](https://github.com/allsource/chronos-monorepo/issues)
- **Documentation**: [Documentation Hub](docs/README.md)
- **Maintainers**: @allsource-team

---

**Last Updated**: November 4, 2025
**Versions**: Rust Core v0.1.0 · Go Control Plane v1.0 · Elixir Query Service Phase 1 Complete
