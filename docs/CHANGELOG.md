# Changelog

All notable changes to AllSource Chronos are documented here.

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
