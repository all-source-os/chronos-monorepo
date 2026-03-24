# Prime Ecosystem Integration Plan

**Status:** Active
**Date:** 2026-03-21
**Goal:** Integrate Prime into the existing Chronos ecosystem across CI, Docker, Core, Query Service, and SDKs.

## Phase 1: Foundation (immediate — no new features, just wiring)

### 1.1 CI Coverage for prime-mcp and recall-bench
- Add `prime` path filter to `ci.yml` change detection
- Add `prime-quality` job: clippy + test for `apps/prime-mcp/`
- Add `recall-bench-quality` job: clippy + build for `tooling/recall-bench/`
- **Effort:** 30 min | **Risk:** None

### 1.2 Docker Stack Entry
- Add `allsource-prime` service to `docker-compose.allsource.yml`
- Port 3905, HTTP mode, shared network, volume for /data
- Health check: `curl -f http://localhost:3905/health`
- **Effort:** 15 min | **Risk:** None

### 1.3 Core HTTP Proxy for Prime API
- Add `/api/v1/prime/*` routes to Core's axum server
- Prime runs embedded (same process, no HTTP hop) when `prime` feature enabled
- Tenant isolation via domain prefix in node properties
- Auth middleware reused from existing Core endpoints
- **Effort:** 3 hours | **Risk:** Low — feature-gated, no impact when disabled

## Phase 2: Service Integration (next sprint)

### 2.1 Query Service → Prime Client
- New Elixir adapter: `PrimeClient` (HTTP client to Prime :3905)
- New routes: `POST /api/analytics/prime/recall`, `GET /api/analytics/prime/index`
- Tenant-scoped: `domain: "tenant:#{tenant_id}:#{domain}"`
- **Effort:** 4 hours | **Risk:** Low

### 2.2 TypeScript SDK Prime Client
- New class `PrimeClient` in `sdks/typescript/`
- Methods: `addNode()`, `addEdge()`, `recall()`, `embed()`, `neighbors()`, `index()`
- Targets Prime HTTP API at configurable base URL
- **Effort:** 3 hours | **Risk:** Low

## Phase 3: User-Facing (backlog)

### 3.1 Web Dashboard Graph Page
### 3.2 Chronis Task Memory Backend
### 3.3 Combined MCP Workflow Docs
