# ADR-007: Domain Value Objects for Type Safety

**Status:** Accepted
**Date:** 2026-02-02
**Release:** v0.9.1

## Context

Core's internal API used raw `String` and `&str` for entity IDs, tenant IDs, event types, and partition keys. This led to:
1. No compile-time protection against swapping arguments (e.g., passing entity_id where tenant_id was expected)
2. No validation at construction time
3. Unclear API signatures: `fn query(String, String, String)` — which is which?

## Decision

Introduce strongly-typed value objects in `domain/value_objects/`:

- `EntityId` — wraps entity identifier, validates non-empty
- `TenantId` — wraps tenant identifier, validates non-empty
- `EventType` — wraps event type string, validates format (alphanumeric + dots)
- `PartitionKey` — wraps partition key for sharding

Each type implements `From<&str>`, `Display`, `AsRef<str>`, and validation in `new()`.

The `Event::from_strings()` constructor accepts raw strings for external callers (HTTP handlers, embedded API) and constructs value objects internally.

## Consequences

### Positive
- Compile-time prevention of argument swapping
- Validation at construction — invalid event types rejected early
- Self-documenting function signatures
- `Event::from_strings()` provides ergonomic bridge for string-based callers

### Negative
- More boilerplate for internal code (`.as_ref()`, `.to_string()` at boundaries)
- Two construction paths: value objects for internal, `from_strings()` for external
