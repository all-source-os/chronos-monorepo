---
title: "AllSource Event Store - Consolidated Roadmap"
status: CURRENT
last_updated: 2026-02-15
category: roadmap
supersedes:
  - "archive/2026-02-02_CONSOLIDATED_ROADMAP.md"
  - "archive/SAAS_LAUNCH_ROADMAP.md"
---

# AllSource Event Store - Consolidated Roadmap

**Date**: 2026-02-15
**Version**: 5.0

---

## Where We Are — Honest Assessment

The codebase is real and substantial: Rust Core, Go Control Plane, Elixir Query Service, Elixir MCP Server (61 tools), Next.js Web dashboard, Helm/K8s deployment, CI/CD. However, a code audit reveals gaps between what's claimed as complete and what actually works end-to-end. This roadmap addresses those gaps first, then lists new work.

### What Works

- Core event ingestion, WAL durability, Parquet storage, DashMap queries
- Leader-follower replication via WAL shipping (fully implemented)
- Control Plane: JWT/RBAC, policy engine, OpenTelemetry, HAL responses, OpenAPI spec
- LemonSqueezy billing integration (checkout, webhooks, usage reporting)
- Query Service: stateless gateway, Tesla HTTP client to Core, Broadway pipeline, OpenAPI
- MCP Server: 61 tool definitions, TOON encoder, conversation context, Core/Control Plane clients
- Web: auth pages, dashboard skeleton, real React/Next.js pages
- Infrastructure: Docker images, Helm charts, K8s manifests, GitHub Actions CI/CD

### What Has Cracks

These items were previously marked as complete but have known gaps that need addressing before they can be considered production-ready.

---

## P0: Fix Existing Gaps

### 1. Query Service API Completeness

Four projection endpoints and one event endpoint return `501 Not Implemented`:

- [ ] `GET /api/events/:id` — event detail lookup (Core lacks single-event-by-ID endpoint)
- [ ] `DELETE /api/projections/:name` — projection deletion (Core lacks endpoint)
- [ ] `GET /api/projections/:name/state` — projection state query (Core lacks endpoint)
- [ ] `POST /api/projections/:name/reset` — projection reset (Core lacks endpoint)
- [ ] `GET /api/projections/rebuild_stats` — returns hardcoded zeros instead of real data

**Root cause**: Core needs these endpoints added first. Then Query Service controllers can call them.

### 2. Core: Fork Event Commit

`MergeForkUseCase.execute()` marks a fork as merged but does NOT commit events back to the parent store when `commit_events=true`. There's a TODO:
```
// TODO: If commit_events is true, apply events to parent/main store
```

- [ ] Inject EventRepository into MergeForkUseCase
- [ ] Implement actual event transfer from fork to main store
- [ ] Add integration test for fork → merge → verify events in main store

### 3. Core: KMS Cloud Providers

Only Local KMS is implemented. AWS, Azure, GCP, Vault all return `"not yet implemented"` error.

- [ ] Implement AWS KMS provider (or remove from KmsProvider enum if not planned)
- [ ] Decide: remove cloud KMS stubs or implement them. Don't ship dead code paths.

### 4. Core: Storage Integrity Gaps

- [ ] Parquet metadata checksum verification (TODO in `storage_integrity.rs`)
- [ ] License compliance scanning is a no-op (`check_licenses()` always returns empty)

### 5. Core: Vector Search Test Coverage

Mock repository methods for vector search use `unimplemented!()` for most operations. The production code may work, but it can't be properly tested.

- [ ] Complete mock repository implementations for semantic search tests
- [ ] Add integration test: ingest events → embed → semantic search → verify results

### 6. MCP Analytics Tool Quality

8 analytics tools (cohort, correlation, forecast, segment, path, attribution, churn, LTV) call basic Core endpoints and do simple client-side math. They are callable but not sophisticated.

- [ ] Audit each analytics tool against real-world use cases
- [ ] Either improve server-side analytics in Core or document limitations honestly
- [ ] `get_query_advice` is a hardcoded lookup table — improve or document as basic

### 7. MCP Schema Tools: Client-Side Only

`migrate_schema`, `infer_schema`, `schema_diff` compute results client-side without Core API support.

- [ ] Add Core endpoints for schema migration, inference, diff (or accept client-side as design choice and document it)

---

## P1: SaaS Launch

### Phase 0: Ship What You Have (Week 1)
- [ ] Deploy to Fly.io
- [ ] Create LemonSqueezy products (Free, Pro $29, Team $99)
- [ ] Landing page on `/` (static HTML — hero + pricing + signup)
- [ ] Waitlist → OAuth flow

### Phase 1: First 10 Customers (Weeks 2-4)
- [ ] Onboarding wizard API (`/api/onboard/start`)
- [ ] Quick start curl examples (welcome email)
- [ ] Usage warning emails (80% quota)
- [ ] Stripe as backup payment processor
- [ ] JavaScript SDK (`@allsource/client`)
- [ ] API docs on `/docs`

### Phase 2: Product-Market Fit (Months 2-3)
- [ ] Simple dashboard (usage chart, API keys, upgrade button)
- [ ] Webhook delivery (push events to customer URLs)
- [ ] Python SDK
- [ ] Status page
- [ ] Changelog (`/changelog`)
- [ ] Feedback widget

### Phase 3: Scale Revenue (Months 3-6)
- [ ] Annual billing discount
- [ ] Team seats
- [ ] Customer-facing audit logs
- [ ] Usage analytics for customers
- [ ] Event replay from UI
- [ ] Go SDK

---

## P2: Query Service Phase 3 (Q2-Q3 2026)

### Phoenix Channels WebSocket (1-2 weeks)
Expose WebSocket endpoint for external clients (currently only internal WS client to Core exists).

- [ ] Phoenix.Socket and Channel implementation
- [ ] `/ws` endpoint on port 3902
- [ ] EventChannel for `events:all`, `events:{entity_id}`, `events:type:{type}`
- [ ] ProjectionChannel for real-time projection state updates
- [ ] JWT authentication for WebSocket connections
- [ ] Presence tracking

### Distributed Mode (2-3 weeks)
- [ ] libcluster for multi-node (dependency exists, not wired)
- [ ] Distributed registry via Horde (dependency exists, not wired)
- [ ] Consistent hashing

### Advanced Analytics (2-3 weeks)
- [ ] Leverage Core's `/api/v1/analytics/*` endpoints
- [ ] Time-window aggregations
- [ ] Statistical functions

### Message Queue Integration (2-3 weeks)
- [ ] Kafka integration (Broadway Kafka dependency exists, not wired)
- [ ] RabbitMQ integration (Broadway RabbitMQ dependency exists, not wired)

### Monitoring & Observability (1-2 weeks)
- [ ] Prometheus exporter (Telemetry dependency exists, needs dashboards)
- [ ] Grafana dashboards (provisioning config exists, no dashboards)
- [ ] APM integration

### API Documentation (1 week)
- [ ] Swagger UI endpoint (OpenAPI spec exists, no UI)

---

## P3: Optional / Future

### Redis Protocol (v1.2) — LOW
- [ ] RESP3 server implementation
- [ ] Redis command mapping (XADD, XRANGE, SUBSCRIBE)
- [ ] Integration tests with redis-cli

### Enterprise Features (2027)

#### v1.8: Multi-Node Clustering (Q1 2027)
- [ ] Term-based consensus (simplified Raft)
- [ ] Deterministic leader selection
- [ ] Partition replication
- [ ] Manual failover
- [ ] Cluster membership management

#### v1.9: Geo-Replication (Q2 2027)
- [ ] Cross-region event replication
- [ ] CRDT-based conflict resolution
- [ ] Hybrid logical clocks
- [ ] Regional/automatic failover

#### v2.0: Advanced Features (Q3-Q4 2027)
- [ ] EventQL (SQL-like query language)
- [ ] GraphQL API
- [ ] Geospatial queries
- [ ] Exactly-once stream processing
- [ ] Autonomous schema evolution

---

## Priority Summary

| Priority | Item | Items | Notes |
|----------|------|:-----:|-------|
| **P0** | Fix existing gaps | 15 | Cracks in "complete" features |
| **P1** | SaaS Launch | 22 | Revenue path |
| **P2** | Query Service Phase 3 | 14 | Feature expansion |
| **P3** | Redis / Enterprise | 19 | Future |

---

## References

- Archived roadmaps: `docs/roadmaps/archive/`
- Analysis docs: `ALLSOURCE_VS_TURSO_COMPARISON.md`, `CHRONOS_VS_LANCEDB_COMPARISON.md`
- Future research: `FUTURE_VECTOR_EMBEDDING_DESIGN.md`

---

**Document Status**: CURRENT
**Last Updated**: 2026-02-15
**Next Review**: After P0 gaps are resolved
