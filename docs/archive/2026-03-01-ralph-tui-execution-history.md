# Ralph-TUI Execution History — AllSource Chronos

**Generated:** 2026-03-01
**Source:** `.ralph-tui/iterations/` logs, `.ralph-tui/progress.md`, `.beads/issues.jsonl`
**Total iterations executed:** ~280+
**Date range:** 2026-01-30 to 2026-02-28

## Overview

This document records all ralph-tui autonomous agent sessions run against the Chronos monorepo. Each session spawned agents to work through beads (task tickets), with work ranging from infrastructure deployment to Core engine features.

---

## Session Timeline

### Session 1 — Fly.io Deployment (2026-01-30 to 2026-02-01)

| Bead | Title | Iterations |
|------|-------|-----------|
| 700.1–700.15 | Configure Query Service for Fly.io Deployment | 16 |

**What was built:** `fly.toml` configuration, health check endpoints, deployment scripts for the Elixir Query Service on Fly.io. First production deployment of the query-service tier.

**Release:** Pre-v0.9.0 infrastructure work.

---

### Session 2 — CI/CD and Test Coverage (2026-02-01)

| Bead | Title | Iterations |
|------|-------|-----------|
| 5bc.1–5bc.10 | Remove redundant Go linting tools | 11 |
| qhw.1–qhw.15 | Audit Core component test coverage | 16 |

**What was built:**
- Eliminated duplicate `go vet` step from CI (already covered by golangci-lint)
- Analyzed Rust Core test coverage with `cargo tarpaulin`, identified gaps in event store, projections, and WAL modules

**Release:** v0.9.0 (2026-02-12)

---

### Session 3 — Core Domain & Query Service Integration (2026-02-02 to 2026-02-04)

| Bead | Title | Iterations |
|------|-------|-----------|
| 2q1.1–2q1.11 | Create domain value objects (EntityId, TenantId, EventType) | 33 |
| 198.1–198.8 | Implement zero-copy deserialization (simd-json) | 10 |
| 19g.1–19g.4 | Query Service Phase 2: Core Integration | 9 |
| 3jw.1–3jw.4 | Enhanced MCP tool descriptions with agent guidance | 8 |
| 1en.1–1en.4 | Implement vector search engine (fastembed) | 4 |
| o7p.1–o7p.7 | Native ARM64 CI runners (replace QEMU) | 7 |

**What was built:**
- Strongly-typed value objects (`EntityId`, `TenantId`, `PartitionKey`, `EventType`) replacing raw strings in Core's domain layer
- simd-json zero-copy deserialization for event payloads (~2x throughput improvement)
- `CoreWebSocketClient`, `ProjectionSync` GenServer, Broadway pipeline for QS↔Core real-time streaming
- Enhanced MCP tool descriptions with best practices and performance tips for AI agents
- Vector search engine using fastembed (pure Rust, no Python dependency) for semantic event queries
- Switched Docker CI from QEMU emulation to native ARM64 runners (~40 min build time reduction)

**Release:** v0.9.1 (2026-02-12), v0.10.0 (2026-02-14)

---

### Session 4 — WebSocket & Dependency Cleanup (2026-02-10 to 2026-02-12)

| Bead | Title | Iterations |
|------|-------|-----------|
| 1gk.1–1gk.6 | Phoenix Channels WebSocket endpoint | 3 |
| 2fn | Merge clean npm dependency PRs | 1 |

**What was built:**
- `/ws` WebSocket endpoint on Query Service for external client subscriptions (events, projections)
- Merged 5 clean Dependabot PRs (Next.js 16, tailwind-merge 3, biome 2, recharts 3)

**Release:** v0.9.1 (2026-02-12)

---

### Session 5 — MCP Event Tools & Core API Gaps (2026-02-12 to 2026-02-13)

| Bead | Title | Iterations |
|------|-------|-----------|
| 3en.1–3en.5 | Event management MCP tools | ~10 |
| 2w1q.1–2w1q.14 | Core API gap fixes | ~30 |
| 1u6h.1–1u6h.9 | SaaS launch prep (P0 gaps) | ~15 |
| 1wjk.1–1wjk.11 | SaaS launch prep (P1 features) | ~15 |
| 3bp.1–3bp.4 | Additional Core endpoints | ~5 |

**What was built:**
- 8 event management MCP tools: delete, archive, restore, redact, export, import, replay, validate
- Core endpoints for event-by-ID, projection delete/state/reset, fork event commit
- SaaS launch infrastructure: Fly.io configs, LemonSqueezy billing products, landing page foundations, OAuth provider integration

**Release:** v0.10.0 (2026-02-14)

---

### Session 6 — Control Plane & HAL Links (2026-02-14)

| Bead | Title | Iterations |
|------|-------|-----------|
| 4mc.1–4mc.2 | HAL link helper module for Control Plane | 30 |
| Various standalone beads | Additional CP endpoints and polish | (included above) |

**What was built:**
- Reusable HAL link builder (`internal/interfaces/http/hal.go`) for standardized `_links` in all Control Plane responses
- PostgreSQL removal from event path — Control Plane refactored to use Core as sole data store
- Event-sourced metadata pattern: users, tenants, API keys stored as events in Core

**Release:** v0.10.0 (2026-02-14)

---

### Session 7 — Production Gaps & Usage Dashboard (2026-02-15 to 2026-02-16)

| Bead | Title | Iterations |
|------|-------|-----------|
| 1w99.1–1w99.11 | Fix production gaps (P0) | ~12 |
| dhgn.1–dhgn.14 | SaaS launch tasks (P1) | ~16 |
| 10eg.1–10eg.6 | Build usage dashboard | 15 |
| 3j0v.1–3j0v.6 | Query Service enhancements | (included above) |
| 2t5w.1–2t5w.2 | Additional QS fixes | (included above) |

**What was built:**
- Fixed Core missing endpoints for event-by-ID, projection management (delete/state/reset)
- Usage dashboard in Next.js web app: event counts, API call history, plan quota visualization
- Domain migration from allsource.co to all-source.xyz across all SDKs, CI, and docs

**Release:** v0.10.3 (2026-02-16), v0.10.4 (2026-02-17)

---

### Session 8 — Server-Side Projections (2026-02-17)

| Bead | Title | Iterations |
|------|-------|-----------|
| 3o0c.1–3o0c.10 | Projection behaviour and registry module | 10 |

**What was built:**
- Elixir projection behaviour contract (`Projection` behaviour with `init/1`, `handle_event/2`, `get_state/1`)
- Compile-time projection registry mapping names to modules
- Fold-on-read pipeline with snapshot-aware delta folding
- 5 projection modules: IndexState, TradeState, PortfolioState, SagaState
- `POST /api/query/projected` endpoint for server-side projected reads

**Release:** v0.10.5 (2026-02-17)

---

### Session 9 — Demo Seeding & Query Ergonomics (2026-02-23)

| Bead | Title | Iterations |
|------|-------|-----------|
| 159t.1–159t.18 | Demo seed endpoint and query ergonomics | 24 |

**What was built:**
- `POST /api/v1/demo/seed` Core endpoint auto-seeding diverse demo data with vector embeddings
- `event_type_prefix` query parameter for prefix-based event type filtering
- `payload_filter` query parameter for JSON key-value matching against event payloads
- `GET /api/v1/entities/duplicates` endpoint for duplicate entity detection
- Demo zone portal with interactive showcases

**Release:** v0.10.7 (2026-02-25)

---

### Session 10 — E2E Tests & Tenant Sync Fix (2026-02-28)

| Bead | Title | Iterations |
|------|-------|-----------|
| 2xz.1–2xz.20 | Dashboard E2E test suite (Playwright) | 5 (ralph-tui) + manual |
| 1d0.1–1d0.6 | Fix demo account ↔ QS tenant sync | (included above) |

**What was built:**
- Comprehensive Playwright E2E test suite covering all 11 dashboard pages
- Fixed 16 failing E2E tests caused by tenant_id mismatch between Control Plane and Query Service
- Added `Email`/`Name` fields to Control Plane JWT Claims
- ETS-backed API key management in Query Service for demo accounts

**Release:** Unreleased (post-v0.10.7)

---

### Session 11 — Embedded Core Hardening (2026-02-28 to 2026-03-01)

| Bead | Title | Iterations |
|------|-------|-----------|
| 3ih.1–3ih.14 | Fix embedded Core review findings (round 1) | manual |
| oni.1–oni.11 | Embedded Core hardening (round 2) | manual |

**What was built:**
- Crash-safe WAL compaction (WAL append inside write lock)
- Cross-tenant compaction guard
- True batch ingestion (`EventStore::ingest_batch()` with single lock)
- Projection backfill on registration
- Concurrency and crash recovery integration tests
- TOON round-trip tests
- Removed duplicate replicant projection module
- SDK quickstart example

**Release:** Unreleased (post-v0.10.7)

---

## Cumulative Statistics

| Metric | Value |
|--------|-------|
| Total ralph-tui sessions | 11 |
| Total iterations | ~280 |
| Total beads (tasks) completed | 62+ |
| Releases shipped | 9 (v0.9.0 through v0.10.7) |
| Date range | 2026-01-30 to 2026-03-01 |
| Services touched | 5 (Core, Query Service, Control Plane, Web, MCP Server) |
| Languages | Rust, Elixir, Go, TypeScript |

## Learnings Captured in progress.md

Key patterns documented during E2E testing sessions:
- **Auth fixture pattern**: `authenticatedPage` fixture with `storageState` reuse
- **Demo login pattern**: `demoLogin(request)` via Control Plane API
- **QS response format**: All endpoints wrap responses in `{data: ...}`
- **CP JWT claims**: QS reads `email`, `name`, `tenant_id`, `sub`, `role` from JWT
- **Core auth defaults**: `register_handler` defaults `tenant_id` to `"default"` if not provided
