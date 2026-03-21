# ADR-011: Typed EntityId Enum for Prime

**Status:** Accepted
**Date:** 2026-03-20
**Deciders:** Architecture Review

## Context

Prime uses stringly-typed entity IDs with a prefix convention: `node:{type}:{id}`, `edge:{id}`, `vec:{id}`. These are constructed by free functions (`node_entity_id()`, `edge_entity_id()`, `vector_entity_id()`) in `types.rs` and `vectors/types.rs`, and parsed back via ad-hoc `split(':')` calls scattered across the facade and projections.

This approach has several problems:

1. **Fragile parsing.** `node_entity_id("person", "abc")` produces `node:person:abc`, parsed back by splitting on `:` and taking indices. If `node_type` ever contains a colon (e.g., `schema:v2`), the split produces the wrong number of segments and silently corrupts the type/id extraction.

2. **No compile-time discrimination.** A `String` entity ID for a node is indistinguishable from one for an edge. Passing an edge entity ID to `get_node()` silently returns `None` instead of failing at compile time.

3. **Scattered format knowledge.** The `node:` / `edge:` / `vec:` prefix convention is encoded in at least five separate locations (two `*_entity_id()` functions, facade parsing, adjacency projection, vector index), with no single source of truth.

4. **No validation on construction.** Nothing prevents constructing an entity ID like `node:` (missing type and id) or `edge:` (missing id).

## Decision

Introduce an `EntityId` enum in `prime/types.rs`:

```rust
pub enum EntityId {
    Node { node_type: String, id: String },
    Edge { id: String },
    Vector { id: String },
    Schema { type_name: String },
}
```

The enum provides:

- **`EntityId::parse(wire: &str) -> Result<EntityId>`** — parses the wire format, using `splitn(3, ':')` for node IDs (handling colons in `node_type` is explicitly disallowed and returns an error) and `splitn(2, ':')` for edge/vec/schema prefixes.
- **`EntityId::to_wire(&self) -> String`** — serializes back to the `prefix:...` wire format for storage in the WAL.
- **Typed constructors** — `EntityId::node(node_type, id)`, `EntityId::edge(id)`, `EntityId::vector(id)`, `EntityId::schema(type_name)` that validate inputs at construction time (no empty strings, no colons in `node_type`).
- **`EntityId::as_str()`** — convenience that calls `to_wire()` for backward compatibility where a `&str` is needed.

The existing free functions `node_entity_id()`, `edge_entity_id()`, and `vector_entity_id()` are retained as deprecated wrappers that delegate to the typed constructors:

```rust
#[deprecated(note = "Use EntityId::node() instead")]
pub fn node_entity_id(node_type: &str, id: &str) -> String {
    EntityId::node(node_type, id).to_wire()
}
```

All internal code in projections and the facade migrates to accept/return `EntityId` directly, eliminating all `split(':')` call sites.

## Consequences

### Positive

- **Eliminates the colon-in-node-type bug.** Construction rejects colons in `node_type`; parsing uses `splitn(3, ':')` so extra colons in `id` are safe.
- **Compile-time type safety.** Functions that operate on nodes can accept `EntityId::Node` specifically (via pattern matching or a newtype wrapper), catching misuse at compile time.
- **Single source of truth.** The wire format is defined exactly once in `EntityId::parse()` / `to_wire()`.
- **Backward-compatible wire format.** The `node:type:id` / `edge:id` / `vec:id` strings in the WAL and Parquet files are unchanged. Existing data loads without migration.

### Negative

- **Churn across the facade and projections.** Every call site that constructs or destructures entity IDs must be updated. The facade alone has ~30 such sites.
- **Slight overhead.** `EntityId` is an enum allocation instead of a plain `String`. In practice this is negligible — entity ID construction is never on the hot path (event processing is).

### Risks

- **Serialization compatibility.** `EntityId` must serialize to the same wire string as the old format. A regression here would silently corrupt the entity_id column in Parquet and break WAL replay. Mitigation: implement `Serialize`/`Deserialize` via the wire format string and add round-trip property tests.
- **Deprecated wrapper removal timeline.** If wrappers are removed too aggressively, downstream code (SDK examples, integration tests) may break. Plan one full release cycle of deprecation warnings before removal.
