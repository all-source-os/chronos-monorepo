# ADR-005: PostgreSQL Removal and Event-Sourced Metadata

**Status:** Accepted
**Date:** 2026-02-14
**Release:** v0.10.0

## Context

The original architecture had three separate stores for users and tenants:
1. Core's in-memory auth manager
2. Query Service's PostgreSQL database
3. Control Plane's in-memory Go store

This created synchronization problems — a user created in one store didn't exist in the others. The 16 failing E2E tests (epic `chronos-monorepo-1d0`) were a direct consequence: Control Plane created demo accounts that Query Service couldn't find because they were never provisioned in PostgreSQL.

## Decision

Remove PostgreSQL from the event data path entirely. Store operational metadata (users, tenants, API keys) as events in Core using the event-sourcing pattern.

- **Core** is the single source of truth for all data, including users and tenants
- **Query Service** fetches tenant data from Core via `RustCoreClient.get_tenant()` with ETS cache (5-min TTL)
- **PostgreSQL** reserved exclusively for future billing/usage metering (not yet implemented)
- **Control Plane** provisions tenants in Core during `POST /api/v1/demo/start`, ensuring the tenant exists before issuing JWTs

## Consequences

### Positive
- Single source of truth eliminates sync bugs
- Query Service becomes stateless — easier horizontal scaling
- No PostgreSQL dependency for core functionality (simpler deployment)

### Negative
- Core must handle auth/tenant queries in addition to event queries
- ETS cache introduces 5-minute eventual consistency for tenant data
- If Core is down, Query Service cannot authenticate users (no local fallback)

### Risks
- Core becomes a single point of failure for both events AND metadata
- Mitigated by planned leader-follower replication (see `docs/proposals/CORE_REPLICATION_DESIGN.md`)
