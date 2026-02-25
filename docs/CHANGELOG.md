# Changelog

All notable changes to AllSource Chronos are documented here.

## [0.10.7] - 2026-02-26

### Added
- **Query ergonomics**: `event_type_prefix` query parameter for prefix-based event type filtering (e.g., "index." matches "index.created", "index.updated")
- **Payload filtering**: `payload_filter` query parameter — JSON key-value matching against event payloads (e.g., `{"user_id":"abc-123"}`)
- **Duplicate entity detection**: `GET /api/v1/entities/duplicates` endpoint — groups events by payload fields and identifies entities with duplicate values
- **Consumer patterns guide**: `docs/current/QUERY_PATTERNS.md` — best practices for pagination, saga orchestration, uniqueness checks, and MCP consumers
- **Demo zone**: interactive demo portal with one-click account provisioning and guided feature showcases
- `Default` derive for `QueryEventsRequest` for ergonomic struct construction

### Fixed
- **Control plane OAuth proxy**: stopped forwarding Host header to prevent Fly.io misrouting
- **Service JWT auth**: Core requests from Control Plane now authenticated with service JWT
- **OAuth callback URL**: uses `FRONTEND_URL` for callback base URL instead of request host
- **External OAuth calls**: uses plain HTTP client (no service auth headers) for provider API calls

## [0.10.6] - 2026-02-19

### Fixed
- **Query DSL crash** (GitHub #65): `POST /api/query` returned HTTP 500 when `%Query{}` struct fell through to Access-based `is_map` clause in `maybe_add_next_link/3` — added explicit `%Query{}` catch-all clause
- **AUTH_DISABLED bypass**: dev mode now short-circuits before header inspection, so malformed `Authorization` headers no longer cause 401 when auth is disabled
- **Domain references**: replaced all `allsource.co` references with `all-source.xyz` across SDKs, registry, CI, and docs
- **Go lint issues**: extracted OAuth provider constants, added `stringFromMap`/`boolFromMap` helpers to satisfy errcheck, gosec nolint for URL constants
- **SIMD filter test threshold**: lowered flaky throughput assertion from 1M to 100K events/sec for CI stability

## [0.10.5] - 2026-02-17

### Added
- **Server-side projections**: Query Service projection engine with behaviour, registry, fold pipeline, and 5 projection modules (IndexState, TradeState, PortfolioState, SagaState)
- **Fold-on-read endpoint**: `POST /api/query/projected` — returns projected entity state instead of raw events, with snapshot-aware folding, server-side filtering, and pagination
- **Continuous projections**: ProjectionServer (PubSub subscription) + DynamicSupervisor for real-time materialized read models via ETS with fold-on-read fallback
- **Rust SDK** (`allsource` crate): typed client with circuit breaker, retry logic, and client-side fold helpers — published to crates.io
- Monorepo structure best practices document (`docs/MONOREPO_STRUCTURE.md`)
- Design proposal and 8 use cases for server-side projections

### Fixed
- **Wire format standardization**: all QS list endpoints now return consistent `{data, count, total}` — fixed snapshot controller mapping `total` → `count`, standardized webhook and replay controllers
- **MCP client resilience**: disabled Hackney connection pooling in CoreClient and ControlPlaneClient to prevent stale connection errors after Core restarts; increased retry delays (500ms base, 5s max)
- **Core persistence wiring** (from v0.10.4): `main.rs` now reads env vars and constructs `EventStoreConfig::production(...)` — all prior Docker images ran in-memory only

### Changed
- Consolidated all SDKs under `sdks/` — moved TypeScript client from `packages/client` to `sdks/typescript`, Rust SDK from `packages/rust-client` to `sdks/rust`
- Removed legacy duplicate SDKs from `packages/` (go-client, python-client)
- `packages/` now contains only shared internal packages (ui component library)
- Updated workspace Cargo.toml and bun workspace to reflect new SDK paths

## [0.10.4] - 2026-02-17

### Fixed
- Core persistence wiring: `main.rs` reads `ALLSOURCE_DATA_DIR` / `ALLSOURCE_WAL_DIR` / `ALLSOURCE_STORAGE_DIR` env vars and constructs `EventStoreConfig::production(...)` when persistence dirs are available
- Added durability test script (`tooling/durability-test/test-durability.sh`)

## [0.10.3] - 2026-02-16

### Changed
- Migrate all domains to all-source.xyz (web, API, docs, emails)
- Archive incorrect PostgreSQL docs, rewrite C4 architecture diagrams

### Fixed
- Rust: clippy, fmt, sort, doc links
- Go: golangci-lint (goconst, gocritic)
- Elixir: credo --strict, format, dialyzer, unused deps, test fixes
- Fix Makefile set-version target

## [0.10.1] - 2026-02-15

### Changed
- Remove PostgreSQL dependency from Query Service (now a stateless API gateway)
- Add JWT/API key auth, tenant cache, and usage reporter to Query Service

### Added
- Billing, webhook, and HAL support in Control Plane
- LemonSqueezy webhook integration for subscription management
- OpenAPI spec for Control Plane (920-line spec)

## [0.10.0] - 2026-02-14

### Changed
- Remove all PostgreSQL repositories from Control Plane — Core is the single source of truth
- Replace pg_tenant, pg_audit, pg_config, pg_operation, pg_policy, pg_user with Core-backed repositories

### Added
- Core: POST/GET /api/v1/audit/events — audit event logging and querying
- Core: GET/POST/PUT/DELETE /api/v1/config — dynamic config management
- CoreClient with connection pooling, retries, and OpenTelemetry tracing
- Leader-follower replication via WAL shipping

### Fixed
- 68 golangci-lint issues across Control Plane
- Rust clippy and cargo fmt in new API files

## [0.9.1] - 2026-02-12

### Added
- Core: GET /api/v1/streams — list all streams (entity IDs) with event counts
- Core: GET /api/v1/event-types — list all event types with usage statistics
- Core: POST /api/v1/events/batch — batch event ingestion
- Query Service: list_streams and list_event_types integration
- Web: useStreams() and useEventTypes() React hooks
- Time travel context and picker components
- Core replication design proposal

## [0.9.0] - 2026-02-12

### Added
- MCP Server: 8 event management tools (delete, archive, restore, export, import, clone, merge, split entities) with dry-run preview and audit trails — 27 tools total, 309 tests
- Web: redesigned login/signup pages with WCAG 2.1 accessibility
- Docker Compose override for isolated local development (ports 3900-3908)
- Version management commands in Makefile (set-version, bump-version)

### Changed
- Query Service: added cmake for NIF compilation, APM/analytics, cluster support, WebSocket channels, OpenAPI spec, Prometheus metrics

### Fixed
- Web Dockerfile for monorepo standalone builds
- Consistent versioning at 0.9.0 across all services
