---
title: "Archive Index"
status: CURRENT
last_updated: 2026-03-01
category: reference
---

# Archive Index

This document serves as a comprehensive index of all archived documentation in the AllSource monorepo. The archive contains historical documents that have been superseded by newer versions, completed phase documentation, and reference materials preserved for historical context.

## Archive Organization

The archive follows a date-based organization system with additional categorical subdirectories:

- **Date-prefixed files** (e.g., `2025-10-19_*.md`) indicate standalone archived documents with their archival date
- **Date-prefixed directories** (e.g., `2025-11-04/`) group related documents archived together
- **Categorical directories** (e.g., `apps-core-phases/`) organize documents by project phase or purpose

---

## Supersession Reference

The following table tracks which archived documents have been replaced by current versions:

| Archived Document | Superseded By | Date Archived | Notes |
|-------------------|---------------|---------------|-------|
| `archive/2025-11-04/ARCHITECTURE_OPTIMIZATION_v1.md` | `current/ARCHITECTURE_OPTIMIZATION.md` | 2025-11-04 | Version 1 replaced by streamlined current version |
| `archive/2025-10-21_ROADMAP.md` | `roadmaps/2025-10-22_COMPREHENSIVE_ROADMAP.md` | 2025-10-21 | Initial roadmap superseded by comprehensive roadmap |
| `archive/2025-10-19_OLD_README.md` | `README.md` (root) | 2025-10-19 | Original README replaced by updated project documentation |
| `archive/2025-10-19_ARCHITECTURE.md` | `current/CLEAN_ARCHITECTURE.md` | 2025-10-19 | Early architecture doc replaced by clean architecture guide |
| `archive/2026-03-01-v1-historical/FINAL_V1_REPORT.md` | N/A | 2026-03-01 | Oct 2025 session report — historical only |
| `archive/2026-03-01-v1-historical/V1_SESSION_SUMMARY.md` | N/A | 2026-03-01 | Oct 2025 session summary — historical only |
| `archive/2026-03-01-v1-historical/MIGRATION_VERIFICATION.md` | N/A | 2026-03-01 | Clojure→Elixir migration verification (completed) |
| `archive/2026-03-01-v1-historical/TEST_COVERAGE_REPORT.md` | N/A | 2026-03-01 | Oct 2025 test snapshot — superseded by CI |
| `archive/2026-03-01-v1-historical/QUERY_SERVICE_IMPLEMENTATION_GUIDE.md` | `current/TENANT_ARCHITECTURE.md` | 2026-03-01 | Nov 2025 QS implementation plan — superseded by v0.10.5 |

---

## Archive Directory Guide

### `archive/2024-11-03/` - Historical Migration Documentation

Contains very old historical documents from the initial project migration phase.

**Contents:**
- `MIGRATION_COMPLETE.md` - Documentation of early migration completion

**Purpose:** Preserves the earliest project documentation for historical reference.

---

### `archive/2025-10-19_*` - Early Documentation Versions

Standalone files representing the first generation of project documentation.

**Files:**
- `2025-10-19_ARCHITECTURE.md` - Original architecture documentation
- `2025-10-19_OLD_README.md` - Initial project README

**Purpose:** Historical reference for understanding the project's documentation evolution.

---

### `archive/2025-10-21_*` - Test Summaries and Initial Roadmaps

Documentation from the initial test coverage assessment and roadmap planning phase.

**Files:**
- `2025-10-21_COMPREHENSIVE_TEST_SUMMARY.md` - Full test coverage analysis
- `2025-10-21_FINAL_TEST_SUMMARY.md` - Consolidated test results
- `2025-10-21_ROADMAP.md` - Initial project roadmap (superseded)
- `2025-10-21_TEST_EXECUTION_LOG.md` - Detailed test execution records
- `2025-10-21_UPDATED_TEST_COVERAGE_REPORT.md` - Updated coverage metrics

**Purpose:** Reference for historical test coverage and early planning decisions.

---

### `archive/2025-10-22_*` - Full Architecture Documentation

Complete, detailed versions of architecture documentation before streamlining.

**Files:**
- `2025-10-22_CLEAN_ARCHITECTURE_FULL.md` - Complete clean architecture reference
- `2025-10-22_PERFORMANCE_FULL.md` - Comprehensive performance documentation
- `2025-10-22_SOLID_PRINCIPLES_FULL.md` - Detailed SOLID principles guide

**Purpose:** Deep-dive reference material when more detail is needed than current streamlined docs provide.

---

### `archive/2025-11-03-marketing/` - Marketing Materials

Marketing content and social media materials from launch preparation.

**Files:**
- `2025-11-03-QUICK_START.md` & progress update - Quick start guide drafts
- `2025-11-03-linkedin-post.md` & progress update - LinkedIn content
- `2025-11-03-x-post.md` & progress update - X (Twitter) content
- `2025-11-03-SUMMARY.md` - Marketing summary
- `2025-11-03-visual-assets.md` - Visual asset specifications
- `TWITTER_THREAD.md` - Twitter thread content

**Purpose:** Reference for marketing messaging and brand voice consistency.

---

### `archive/2025-11-04/` - Session Summaries and v1 Documents

Documentation from the November 4th development session including v1 documents that have been updated.

**Files:**
- `ARCHITECTURE_OPTIMIZATION_v1.md` - First version of optimization guide (superseded)
- `EXECUTIVE_SUMMARY_v1.md` - v1 executive summary
- `QUERY_SERVICE_ROADMAP_v1.md` - v1 query service roadmap
- `REFACTOR_PLAN.md` & `REFACTOR_COMPLETE.md` - Refactoring documentation
- `SESSION_SUMMARY.md` - Development session notes
- `STORAGE_CLARIFICATION.md` - Storage architecture clarifications
- `DOCUMENTATION_INDEX.md` - Previous documentation index
- `README.md` - Directory-specific readme

**Purpose:** Reference for understanding the v1 to v2 documentation evolution and session context.

---

### `archive/apps-core-phases/` - Phase 1-5 Development Progress

Comprehensive documentation of the phased development approach for the core applications.

**Files:**
- **Phase 1:** `PHASE_1_PROGRESS_SUMMARY.md`, `PHASE_1_COMPLETE.md`
- **Phase 3:** `PHASE3_PLAN.md`
- **Phase 4:** `PHASE4_PLAN.md`, `PHASE4A_SUMMARY.md`, `PHASE4B_SUMMARY.md`, `PHASE4_COMPLETE_SUMMARY.md`
- **Phase 5:** `PHASE5_PLAN.md`, `PHASE5A_PROGRESS.md`
- **Supporting:** `CLEAN_ARCHITECTURE_REFACTORING.md`, `IMPLEMENTATION_SUMMARY.md`, `SIERRADB_IMPLEMENTATION_PLAN.md`
- **Session Notes:** `SESSION_2_CONTINUATION_SUMMARY.md`, `SESSION_3_EVENT_REFACTORING_COMPLETE.md`
- **Legacy:** `control-plane-README_v0.1.0.md`, `README.md`

**Purpose:** Complete historical record of phased development progress and decisions.

---

### `archive/2026-03-01-v1-historical/` - V1 Historical Documents

Documents from Oct-Nov 2025 that are purely historical and no longer reflect the current architecture (v0.10.0+).

**Files:**
- `FINAL_V1_REPORT.md` — Oct 2025 development session report
- `V1_SESSION_SUMMARY.md` — Oct 2025 session summary
- `MIGRATION_VERIFICATION.md` — Clojure→Elixir migration verification (completed)
- `TEST_COVERAGE_REPORT.md` — Oct 2025 test coverage snapshot (superseded by CI)
- `QUERY_SERVICE_IMPLEMENTATION_GUIDE.md` — Nov 2025 QS implementation plan (superseded by v0.10.5 stateless architecture)

**Purpose:** Preserves early development documentation for historical reference. All documents reference PostgreSQL-backed QS or pre-v0.10.0 architecture.

---

### `archive/2026-02-16-postgres-cleanup/` - PostgreSQL Removal

Documentation from the PostgreSQL removal effort where Query Service was made stateless.

**Purpose:** Records the transition from PostgreSQL-backed tenant storage to Core-backed event-sourced metadata.

---

### `archive/2026-03-01-embedded-core-hardening.md` - Embedded Core Hardening

Execution summary of 25 findings across two principal engineer review rounds of the Embedded Core Library (Issue #73). Covers crash-safety fixes, batch ingestion, projection backfill, concurrency tests, and TOON round-trip verification.

**Purpose:** Complete record of all hardening work done on the embedded Core API before release.

---

### `archive/2026-03-01-ralph-tui-execution-history.md` - Ralph-TUI Session History

Comprehensive log of all 11 ralph-tui autonomous agent sessions (~280 iterations) from 2026-01-30 to 2026-03-01. Maps each session to beads completed, features built, and releases shipped.

**Purpose:** Historical record of autonomous agent contributions and velocity tracking.

---

### `adr/` - Architecture Decision Records

10 ADRs documenting major technical decisions from v0.9.1 through unreleased. Covers embedded Core API, crash-safe compaction, batch ingestion, projection backfill, PostgreSQL removal, server-side projections, domain value objects, vector search, simd-json, and native ARM64 CI.

**Purpose:** Immutable record of architectural decisions and their rationale.

---

## When to Archive

Documents should be moved to the archive when:

1. **Superseded by newer version** - A document has been replaced by an updated version with significant changes
2. **Phase completed** - Development phase documentation that is no longer actively referenced
3. **Historical value** - Content that may be useful for reference but is no longer current
4. **Marketing cycles** - Campaign materials after launch completion
5. **Session-specific** - Development session notes after integration into main documentation

### Archiving Process

1. Create a date-prefixed file or directory (format: `YYYY-MM-DD_` or `YYYY-MM-DD/`)
2. Move the document(s) to the appropriate archive location
3. Update this index with the supersession reference if applicable
4. Update any links in current documentation that referenced the archived file

---

## When to Delete

**Never delete documentation without creating a backup first.**

Documents may be considered for deletion only when:

1. **Duplicates** - Exact duplicates exist elsewhere in the archive
2. **Empty/placeholder** - Files with no meaningful content
3. **Generated artifacts** - Auto-generated files that can be recreated
4. **Sensitive information** - Documents containing credentials or secrets (after secure backup)

### Deletion Process

1. **Verify** the document meets deletion criteria
2. **Create backup** in a secure location outside the repository
3. **Document** the deletion in commit message with rationale
4. **Remove** references to the deleted file from other documentation
5. **Update** this index if the file was listed

### What to Never Delete

- Session summaries (historical context)
- Architecture decisions (ADRs)
- Phase completion documentation
- Version history documents (v1, v2, etc.)
- Marketing materials (brand consistency reference)

---

## Quick Reference

## Documents Updated In-Place (2026-03-01)

The following documents were updated with staleness notes or corrections rather than archived, as they contain useful forward-looking recommendations:

| Document | Action Taken | Key Changes |
|----------|-------------|-------------|
| `current/C4_ARCHITECTURE_ANALYSIS.md` | Updated | MCP Elixir:4000, tenant metadata WAL-durable, Gap 1 downgraded |
| `current/TENANT_ARCHITECTURE.md` | Rewritten (v2.0) | Event-sourced system streams, QS stateless, no PostgreSQL |
| `ARCHITECTURE_REVIEW.md` | Staleness note | QS stateless since v0.10.0 (not v0.10.3), references ADR-005 |
| `proposals/SERVICE_RESPONSIBILITY_REALIGNMENT.md` | Status → Partially Superseded | QS no longer uses PostgreSQL |
| `proposals/better-auth-migration.md` | Staleness note | QS has no PostgreSQL to share |
| `proposals/SERVER_SIDE_PROJECTIONS.md` | Status → Implemented (v0.10.5) | References ADR-006 |
| `MCP_SERVER_ENHANCEMENT_PLAN.md` | Status → Superseded | Clojure DSL removed, MCP is now Elixir |
| `docker-images.md` | Updated | Image names chronos-*, MCP port 4000, version v0.10.7 |
| `QUICK_START.md` | Updated | URLs → all-source.xyz, API paths → /api/v1/ |
| `deployment/DOCKER.md` | Rewritten | GHCR org, no DATABASE_URL, CORE_URL, CP port 3901 |
| `launch/SOFT_LAUNCH_CHECKLIST.md` | Rewritten | No PostgreSQL steps, added CP deployment |
| `checklists/NEXT_STEPS.md` | Section 2 rewritten | Architecture cleanup reflects current state |

---

| Need | Look In |
|------|---------|
| Old architecture details | `archive/2025-10-22_*_FULL.md` files |
| Test history | `archive/2025-10-21_*` files |
| Development phases | `archive/apps-core-phases/` |
| Marketing content | `archive/2025-11-03-marketing/` |
| v1 documents | `archive/2025-11-04/` |
| Migration history | `archive/2024-11-03/` |
| PostgreSQL cleanup | `archive/2026-02-16-postgres-cleanup/` |
| V1 historical docs | `archive/2026-03-01-v1-historical/` |
| Embedded Core hardening | `archive/2026-03-01-embedded-core-hardening.md` |
| Ralph-TUI execution history | `archive/2026-03-01-ralph-tui-execution-history.md` |
| Architecture decisions | `adr/README.md` |

---

*This index should be updated whenever documents are archived or deleted.*
