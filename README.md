# AllSource Event Store - Chronos Monorepo

[![Rust v1.0](https://img.shields.io/badge/Rust%20Core-v1.0-green.svg)](services/core/)
[![Go Control Plane](https://img.shields.io/badge/Go%20Control%20Plane-active-blue.svg)](services/control-plane/)
[![Phase 1.5](https://img.shields.io/badge/Phase%201.5-70%25-orange.svg)](docs/roadmaps/2025-10-22_PHASE_1.5_PROGRESS.md)

High-performance event sourcing platform with Clean Architecture implementation.

---

## Quick Links

| Category | Links |
|----------|-------|
| **Documentation** | [Index](docs/INDEX.md) · [Architecture](docs/current/CLEAN_ARCHITECTURE.md) · [SOLID Principles](docs/current/SOLID_PRINCIPLES.md) · [Performance](docs/current/PERFORMANCE.md) |
| **Roadmaps** | [Comprehensive Roadmap](docs/roadmaps/2025-10-22_COMPREHENSIVE_ROADMAP.md) · [Phase 1.5 Progress](docs/roadmaps/2025-10-22_PHASE_1.5_PROGRESS.md) · [TDD Results](docs/roadmaps/2025-10-22_PHASE_1.5_TDD_RESULTS.md) |
| **Guides** | [Quick Start](docs/guides/QUICK_START.md) · [Demo](docs/guides/DEMO.md) |
| **Services** | [Rust Core](services/core/docs/README.md) · [Go Control Plane](services/control-plane/docs/README.md) |

---

## Project Status

### v1.0 (Completed)
- ✅ **86/86 tests passing** (100% pass rate)
- ✅ **Baseline performance**: 469K events/sec, 11.9μs query latency
- ✅ **Multi-tenancy support** with RBAC and policies
- ✅ **Parquet storage** with efficient columnar format
- ✅ **Full audit logging** and compliance features

### Phase 1.5 (In Progress - 70% Complete)
- ✅ **Domain layer** with pure entities and validation
- ✅ **Application layer** with use cases and DTOs
- ✅ **Repository abstractions** (EventRepository, EventReader, EventWriter)
- ⏳ **Infrastructure layer** (30% - structure created)
- ⏳ **Performance optimization** (simd-json, async batching pending)
- ⏳ **Go Control Plane refactoring** (planned for weeks 6-8)

### v1.2 Targets
- 🎯 **1M+ events/sec** (+113% from baseline)
- 🎯 **<5μs query latency** (-58% from baseline)
- 🎯 **<4ms concurrent writes** (-50% from baseline)

---

## Architecture

### Rust Core (`services/core`)

```
src/
├── domain/              ✅ Layer 1: Business entities
│   ├── entities/       - Pure domain logic
│   └── repositories/   - Repository trait abstractions
├── application/         ✅ Layer 2: Use cases
│   ├── dto/           - Data Transfer Objects
│   └── use_cases/     - Business logic orchestration
├── infrastructure/      ⏳ Layer 3: Adapters (30%)
│   └── [to be organized]
└── [legacy modules]     ⏳ Being refactored
```

**Status**: Clean Architecture implementation with TDD approach
**Documentation**: [Rust Core Docs](services/core/docs/README.md)

### Go Control Plane (`services/control-plane`)

**Current**: Traditional structure
**Planned**: Clean Architecture migration in Phase 1.5 (weeks 6-8)
**Documentation**: [Control Plane Docs](services/control-plane/docs/README.md)

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

### Pending Optimizations
- ⏳ simd-json integration (+40%)
- ⏳ Async I/O batching (+700%)
- ⏳ Batch processing optimization (+1300%)

**Full details**: [Performance Guide](docs/current/PERFORMANCE.md)

---

## Development

### Prerequisites
- **Rust**: 1.75+
- **Go**: 1.21+
- **Node.js**: 18+ (for MCP server)

### Quick Start

```bash
# Clone the repository
git clone https://github.com/allsource/chronos-monorepo.git
cd chronos-monorepo

# Run Rust tests
cd services/core
cargo test --lib

# Run Go tests
cd ../control-plane
go test ./...

# Run benchmarks
cd ../core
cargo bench --bench performance_benchmarks
```

**Detailed guide**: [Quick Start](docs/guides/QUICK_START.md)

---

## Testing

### Rust Core
```bash
cd services/core

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
cd services/control-plane

# All tests
go test ./...

# With coverage
go test ./... -cover

# Verbose
go test -v ./...
```

**Status**: ✅ All tests passing, 23.2% coverage

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

## Services

### Core Event Store (Rust)
High-performance event sourcing engine with Parquet storage.

- **Language**: Rust
- **Storage**: Apache Parquet
- **Performance**: 469K events/sec baseline
- **Tests**: 86/86 passing
- **Documentation**: [Core Docs](services/core/docs/README.md)

### Control Plane (Go)
Multi-tenant control plane with RBAC and policy engine.

- **Language**: Go
- **Features**: Tenancy, RBAC, Policies, Audit logs
- **Tests**: 23.2% coverage
- **Recent Fix**: Policy engine tenant evaluation (2025-10-22)
- **Documentation**: [Control Plane Docs](services/control-plane/docs/README.md)

### MCP Server (Node.js)
Model Context Protocol server for AI integration.

- **Language**: TypeScript/Node.js
- **Status**: In development
- **Documentation**: [MCP Docs](packages/mcp-server/)

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

### Phase 1.0 (✅ Complete)
- Basic event storage and retrieval
- Multi-tenancy with RBAC
- Parquet storage backend
- 92% test coverage

### Phase 1.5 (⏳ 70% Complete - Current)
- Clean Architecture refactoring
- Performance optimization
- Domain-driven design
- Go Control Plane migration

### Phase 2.0 (📋 Planned)
- Distributed event processing
- Real-time streaming
- Advanced analytics
- Clojure query service

**Full roadmap**: [Comprehensive Roadmap](docs/roadmaps/2025-10-22_COMPREHENSIVE_ROADMAP.md)

---

## License

[MIT License](LICENSE)

---

## Support

- **Issues**: [GitHub Issues](https://github.com/allsource/chronos-monorepo/issues)
- **Documentation**: [docs/INDEX.md](docs/INDEX.md)
- **Maintainers**: @allsource-team

---

**Last Updated**: 2025-10-22
**Version**: v1.0 (Rust), v1.5-dev (Phase 1.5 in progress)
