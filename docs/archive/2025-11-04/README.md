# Archive: November 4, 2025

**Status**: ⚠️ DEPRECATED
**Reason**: Corrected database assumptions + root-level documentation violations

---

## Archived Documents

### Database Corrections (archived during session)

#### 1. QUERY_SERVICE_ROADMAP_v1.md
**Status**: ⚠️ SUPERSEDED by [apps/query-service/ROADMAP.md](../../apps/query-service/ROADMAP.md)
**Date Archived**: November 4, 2025
**Reason**: Incorrectly planned PostgreSQL/Redis integration

**Key Issues:**
- Planned PostgreSQL for projection storage (not needed)
- Planned Redis for caching (Core's DashMap is faster)
- Planned Phoenix Channels rebuild (Core has WebSocket)
- Timeline: 8-12 weeks (optimized to 3-4 weeks)

---

#### 2. ARCHITECTURE_OPTIMIZATION_v1.md
**Status**: ⚠️ SUPERSEDED by [docs/current/ARCHITECTURE_OPTIMIZATION.md](../current/ARCHITECTURE_OPTIMIZATION.md)
**Date Archived**: November 4, 2025
**Reason**: Incorrectly assumed Core used 1 PostgreSQL instance

**Key Issues:**
- Stated "PostgreSQL instances: 2 → 1" (incorrect)
- Reality: "PostgreSQL instances: 0 → 0" (Core uses none)
- PostgreSQL exists in code but is feature-gated, not compiled/used
- Core uses: In-memory DashMap + Parquet + WAL (no databases)

---

#### 3. EXECUTIVE_SUMMARY_v1.md
**Status**: ⚠️ SUPERSEDED (removed, not recreated)
**Date Archived**: November 4, 2025
**Reason**: Based on incorrect database assumptions

**Key Issues:**
- Cost-benefit analysis based on 1 PostgreSQL instance
- Infrastructure section claimed "PostgreSQL: 2 → 1"
- Entire analysis needed correction

---

### Root-Level Documentation Violations (archived post-session)

#### 4. DOCUMENTATION_INDEX.md
**Status**: ⚠️ SUPERSEDED by [docs/README.md](../../README.md)
**Date Archived**: November 4, 2025 (post-session consolidation)
**Reason**: Violated documentation practices - index should not be at repository root

**Issue**: Created at root level instead of in `docs/` directory per documentation standards.

---

#### 5. SESSION_SUMMARY.md
**Status**: ⚠️ ARCHIVED (historical reference)
**Date Archived**: November 4, 2025 (post-session consolidation)
**Reason**: Session logs should not remain at repository root

**Content**: Monorepo refactoring completion, architecture review, zero-database discovery. Important findings integrated into `docs/current/ARCHITECTURE_OPTIMIZATION.md`.

---

#### 6. REFACTOR_PLAN.md
**Status**: ⚠️ ARCHIVED (historical)
**Date Archived**: November 4, 2025 (post-session consolidation)
**Reason**: One-time planning document, refactoring complete

**Content**: Original monorepo refactoring plan (services/ → apps/). Superseded by `REFACTOR_COMPLETE.md`.

---

#### 7. REFACTOR_COMPLETE.md
**Status**: ⚠️ ARCHIVED (historical)
**Date Archived**: November 4, 2025 (post-session consolidation)
**Reason**: One-time status document, should not remain at root

**Content**: Monorepo restructure summary, test status. Changes tracked in git history and `package.json`.

---

#### 8. STORAGE_CLARIFICATION.md
**Status**: ⚠️ ARCHIVED (content integrated)
**Date Archived**: November 4, 2025 (post-session consolidation)
**Reason**: Important content moved to proper location

**Content**: Zero-database architecture explanation. Integrated into `docs/current/ARCHITECTURE_OPTIMIZATION.md`.

---

## What Changed

### Phase 1: Database Corrections (during session)

#### Discovery
After detailed code review, discovered that:
1. **Core uses NO PostgreSQL** (it's optional, feature-gated)
2. **No Redis anywhere** (never used)
3. **Current storage**: DashMap (in-memory) + Parquet + WAL
4. **No external databases** at all

#### Impact
- ✅ **Better than planned**: Zero database dependencies
- ✅ **Faster than thought**: 11.9 μs queries (vs 1-5ms PostgreSQL)
- ✅ **Simpler deployment**: No external database to manage
- ✅ **Lower ops cost**: Zero database infrastructure

#### Corrections
- Timeline: 3-4 weeks (unchanged, still better than original 8-12)
- Performance: Even better (no DB network overhead)
- Infrastructure: Simpler (zero external dependencies)

---

### Phase 2: Documentation Consolidation (post-session)

#### Root-Level Cleanup
Following the principles in `docs/README.md`, removed 5 documentation files from repository root:

1. **DOCUMENTATION_INDEX.md** → Superseded by `docs/README.md`
2. **SESSION_SUMMARY.md** → Archived (historical reference)
3. **REFACTOR_PLAN.md** → Archived (historical planning)
4. **REFACTOR_COMPLETE.md** → Archived (historical status)
5. **STORAGE_CLARIFICATION.md** → Content integrated into current docs

#### Apps-Level Cleanup
Also archived 14 phase documents from `apps/core/`:
- Phase 1-5 planning and completion docs
- Session 2-3 summaries
- SierraDB implementation plan
- Control plane v0.1.0 README

**See**: `docs/archive/apps-core-phases/README.md` for details

---

## Current Documentation

**Active Documents:**
- [Documentation Index](../../README.md) - Central documentation hub
- [Query Service Roadmap v2.0](../../../apps/query-service/ROADMAP.md)
- [Architecture Optimization v2.0](../current/ARCHITECTURE_OPTIMIZATION.md)
- [Query Service Implementation Guide](../current/QUERY_SERVICE_IMPLEMENTATION_GUIDE.md)

**Archived (this directory):**
- Database-related: QUERY_SERVICE_ROADMAP_v1.md, ARCHITECTURE_OPTIMIZATION_v1.md, EXECUTIVE_SUMMARY_v1.md
- Root violations: DOCUMENTATION_INDEX.md, SESSION_SUMMARY.md, REFACTOR_PLAN.md, REFACTOR_COMPLETE.md, STORAGE_CLARIFICATION.md

**Archived (apps-core-phases):**
- Phase planning: 14 documents from `apps/core/`

---

## Lessons Learned

### Database Architecture

1. **Verify assumptions**: Don't assume databases are used without checking
2. **Check feature flags**: `#[cfg(feature = "postgres")]` means optional
3. **Review main.rs**: What's actually instantiated vs what's available
4. **In-memory is valid**: Modern event stores can be in-memory + WAL + Parquet

### Documentation Practices

1. **Minimal root-level**: Only `README.md` should remain at repository root
2. **Central docs/**: All documentation belongs in `docs/` directory
3. **No session logs at root**: Session summaries belong in `docs/archive/YYYY-MM-DD/`
4. **Archive promptly**: Move outdated docs to archive immediately, don't let them accumulate
5. **Single source of truth**: Consolidate duplicate information into one canonical location

---

**Archive Date**: November 4, 2025
**Archived By**: Claude Code
**Reason**: Database architecture clarification + documentation consolidation
