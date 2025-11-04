# Documentation Index

**Last Updated**: November 4, 2025
**Status**: ✅ CURRENT

---

## Quick Navigation

### 📋 Current Documents (Active)

#### Architecture & Planning
- **[Architecture Optimization](docs/current/ARCHITECTURE_OPTIMIZATION.md)** - Optimized architecture plan (v2.0, zero databases)
- **[Query Service Roadmap](apps/query-service/ROADMAP.md)** - Phase 2 roadmap (3-4 weeks timeline)
- **[Implementation Guide](docs/current/QUERY_SERVICE_IMPLEMENTATION_GUIDE.md)** - Week-by-week implementation steps
- **[Storage Clarification](STORAGE_CLARIFICATION.md)** - Explains zero-database architecture

#### Completion Status
- **[Refactoring Complete](REFACTOR_COMPLETE.md)** - Monorepo restructure summary
- **[Migration Complete](MIGRATION_COMPLETE.md)** - Clojure to Elixir migration

#### Project Overview
- **[README](README.md)** - Main project README
- **[Refactor Plan](REFACTOR_PLAN.md)** - Original refactoring plan

---

### ⚠️ Archived Documents (Historical)

**Location**: `docs/archive/2025-11-04/`

- **QUERY_SERVICE_ROADMAP_v1.md** - Original roadmap (superseded, had PostgreSQL/Redis)
- **ARCHITECTURE_OPTIMIZATION_v1.md** - Original analysis (superseded, assumed 1 PostgreSQL)
- **EXECUTIVE_SUMMARY_v1.md** - Original summary (superseded, incorrect database info)
- **README.md** - Archive index explaining why documents were archived

---

## Document Status Legend

- ✅ **CURRENT** - Active, up-to-date, authoritative
- ⚠️ **DEPRECATED** - Historical only, do not use
- 🔄 **SUPERSEDED** - Replaced by newer version
- 📝 **DRAFT** - Work in progress
- ⏳ **PLANNED** - Not yet implemented

---

## Key Changes (November 4, 2025)

### What Was Corrected

**Original Assumption (v1.0):**
- PostgreSQL instances: 2 → 1 (50% reduction)
- Redis instances: 1 → 0 (eliminated)
- Timeline: 8-12 weeks

**Actual Reality (v2.0):**
- PostgreSQL instances: 0 → 0 (NONE NEEDED)
- Redis instances: 0 → 0 (NONE NEEDED)
- Timeline: 3-4 weeks
- Storage: In-memory DashMap + Parquet + WAL

### Why the Change

Detailed code review revealed:
1. Core uses NO PostgreSQL (it's feature-gated, not compiled/used)
2. Current storage: DashMap (11.9 μs) + Parquet + WAL
3. No external databases at all
4. Architecture is simpler and faster than initially assessed

---

## Phase 2 Implementation

### Timeline: 3-4 Weeks

**Week 1**: Core Projection API (Rust)
- Add projection state endpoints to Core
- Use existing DashMap (no new dependencies)
- 15+ tests

**Week 2**: WebSocket Integration (Elixir)
- Subscribe to Core's WebSocket
- PubSub distribution
- 10+ tests

**Week 2-3**: Projection State Sync (Elixir)
- Sync to Core's DashMap API
- ETS local cache
- 15+ tests

**Week 3-4**: Broadway Refinement (Elixir)
- Production-ready producer
- Cursor tracking
- 15+ tests

### Success Metrics
- ✅ Zero external databases
- ✅ 11.9 μs projection reads
- ✅ 10K+ events/sec Broadway
- ✅ <100ms event delivery

---

## Infrastructure Summary

### Current Architecture

```
┌─────────────────────────────────────┐
│ Rust Core (Port 3900)              │
│ Storage:                           │
│ • DashMap (in-memory, 11.9 μs)     │
│ • Parquet (optional persistence)   │
│ • WAL (durability)                 │
│                                    │
│ External Dependencies: ZERO ✅      │
└─────────────────────────────────────┘
           │
           │ WebSocket + HTTP
           ▼
┌─────────────────────────────────────┐
│ Elixir Query Service (Port 3902)   │
│ • WebSocket client to Core         │
│ • GenServer/ETS cache (local)      │
│ • Broadway (batch processing)      │
│                                    │
│ External Dependencies: ZERO ✅      │
└─────────────────────────────────────┘
```

**Total PostgreSQL Instances**: 0
**Total Redis Instances**: 0
**Total External Databases**: 0

---

## Service Ports

- **3900**: Rust Core (event store)
- **3901**: Go Control Plane (orchestration)
- **3902**: Elixir Query Service (query layer)
- **3000**: Next.js Web (UI)

---

## Test Status

### Rust Core
- **Tests**: 86/86 passing (100%)
- **Coverage**: 100% for domain/application layers

### Go Control Plane
- **Tests**: All passing
- **Coverage**: 23.2%

### Elixir Query Service
- **Tests**: 281/281 passing (100%)
- **Coverage**: Full coverage for Phase 1

**Total Tests**: 367+ across all services

---

## Getting Started

### For Developers

1. **Read**: [README.md](README.md) - Project overview
2. **Understand**: [Architecture Optimization](docs/current/ARCHITECTURE_OPTIMIZATION.md) - Why zero databases
3. **Implement**: [Implementation Guide](docs/current/QUERY_SERVICE_IMPLEMENTATION_GUIDE.md) - Week-by-week steps
4. **Reference**: [Query Service Roadmap](apps/query-service/ROADMAP.md) - Full roadmap

### For Stakeholders

1. **Overview**: [README.md](README.md)
2. **Status**: [Refactoring Complete](REFACTOR_COMPLETE.md)
3. **Storage**: [Storage Clarification](STORAGE_CLARIFICATION.md)
4. **Roadmap**: [Query Service Roadmap](apps/query-service/ROADMAP.md)

---

## Related Documentation

### Service-Specific Docs
- [Rust Core Docs](apps/core/docs/README.md)
- [Go Control Plane Docs](apps/control-plane/docs/README.md)
- [Query Service Docs](apps/query-service/README.md)

### Roadmaps
- [Comprehensive Roadmap](docs/roadmaps/2025-10-22_COMPREHENSIVE_ROADMAP.md)
- [Phase 1.5 Progress](docs/roadmaps/2025-10-22_PHASE_1.5_PROGRESS.md)

### Guides
- [Quick Start](docs/guides/QUICK_START.md)
- [Demo](docs/guides/DEMO.md)

---

## Document Maintenance

### When to Archive
- Document superseded by newer version
- Information becomes incorrect/outdated
- Major architectural change

### Archiving Process
1. Create dated directory: `docs/archive/YYYY-MM-DD/`
2. Move old document: `mv doc.md docs/archive/YYYY-MM-DD/doc_v1.md`
3. Add status markers: ⚠️ DEPRECATED, 🔄 SUPERSEDED
4. Create archive README explaining changes
5. Update this index

### Status Markers
Use these in document headers:
```markdown
**Status**: ✅ CURRENT | ⚠️ DEPRECATED | 🔄 SUPERSEDED
**Version**: X.X
**Supersedes**: [Link to old version]
**Superseded By**: [Link to new version]
```

---

## Questions?

- **Architecture**: See [Architecture Optimization](docs/current/ARCHITECTURE_OPTIMIZATION.md)
- **Implementation**: See [Implementation Guide](docs/current/QUERY_SERVICE_IMPLEMENTATION_GUIDE.md)
- **Storage**: See [Storage Clarification](STORAGE_CLARIFICATION.md)
- **Roadmap**: See [Query Service Roadmap](apps/query-service/ROADMAP.md)

---

**Maintained By**: Development Team
**Last Audit**: November 4, 2025
**Next Audit**: After Phase 2 completion
