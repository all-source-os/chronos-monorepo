# Archive: November 4, 2025

**Status**: ⚠️ DEPRECATED
**Reason**: Corrected to reflect zero PostgreSQL dependency

---

## Archived Documents

### 1. QUERY_SERVICE_ROADMAP_v1.md
**Status**: ⚠️ SUPERSEDED by [apps/query-service/ROADMAP.md](../../apps/query-service/ROADMAP.md)
**Date Archived**: November 4, 2025
**Reason**: Incorrectly planned PostgreSQL/Redis integration

**Key Issues:**
- Planned PostgreSQL for projection storage (not needed)
- Planned Redis for caching (Core's DashMap is faster)
- Planned Phoenix Channels rebuild (Core has WebSocket)
- Timeline: 8-12 weeks (optimized to 3-4 weeks)

---

### 2. ARCHITECTURE_OPTIMIZATION_v1.md
**Status**: ⚠️ SUPERSEDED by [docs/current/ARCHITECTURE_OPTIMIZATION.md](../current/ARCHITECTURE_OPTIMIZATION.md)
**Date Archived**: November 4, 2025
**Reason**: Incorrectly assumed Core used 1 PostgreSQL instance

**Key Issues:**
- Stated "PostgreSQL instances: 2 → 1" (incorrect)
- Reality: "PostgreSQL instances: 0 → 0" (Core uses none)
- PostgreSQL exists in code but is feature-gated, not compiled/used
- Core uses: In-memory DashMap + Parquet + WAL (no databases)

---

### 3. EXECUTIVE_SUMMARY_v1.md
**Status**: ⚠️ SUPERSEDED (removed, not recreated)
**Date Archived**: November 4, 2025
**Reason**: Based on incorrect database assumptions

**Key Issues:**
- Cost-benefit analysis based on 1 PostgreSQL instance
- Infrastructure section claimed "PostgreSQL: 2 → 1"
- Entire analysis needed correction

---

## What Changed

### Discovery
After detailed code review, discovered that:
1. **Core uses NO PostgreSQL** (it's optional, feature-gated)
2. **No Redis anywhere** (never used)
3. **Current storage**: DashMap (in-memory) + Parquet + WAL
4. **No external databases** at all

### Impact
- ✅ **Better than planned**: Zero database dependencies
- ✅ **Faster than thought**: 11.9 μs queries (vs 1-5ms PostgreSQL)
- ✅ **Simpler deployment**: No external database to manage
- ✅ **Lower ops cost**: Zero database infrastructure

### Corrections
- Timeline: 3-4 weeks (unchanged, still better than original 8-12)
- Performance: Even better (no DB network overhead)
- Infrastructure: Simpler (zero external dependencies)

---

## Current Documentation

**Active Documents:**
- [Query Service Roadmap v2.0](../../apps/query-service/ROADMAP.md)
- [Architecture Optimization v2.0](../current/ARCHITECTURE_OPTIMIZATION.md)
- [Storage Clarification](../../STORAGE_CLARIFICATION.md)

**Archived (this directory):**
- QUERY_SERVICE_ROADMAP_v1.md
- ARCHITECTURE_OPTIMIZATION_v1.md
- EXECUTIVE_SUMMARY_v1.md

---

## Lessons Learned

1. **Verify assumptions**: Don't assume databases are used without checking
2. **Check feature flags**: `#[cfg(feature = "postgres")]` means optional
3. **Review main.rs**: What's actually instantiated vs what's available
4. **In-memory is valid**: Modern event stores can be in-memory + WAL + Parquet

---

**Archive Date**: November 4, 2025
**Archived By**: Claude Code
**Reason**: Database architecture clarification
