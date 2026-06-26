# tenant-isolation-check

CI gate against two tenant-isolation regressions. Runs both checks in one pass;
exits non-zero if either fails.

```sh
# from the repo root
cargo run --manifest-path tooling/tenant-isolation-check/Cargo.toml
```

## Check 1 — QS PubSub topics must be tenant-scoped

Every user-facing event/projection PubSub topic in the Query Service must be
**tenant-scoped** (`events:<tenant>:...`, `projections:<tenant>:...`). A global
topic (`events:all`, `events:<entity>`, `events:type:<type>`, `projections:<name>`)
lets any authenticated client receive every tenant's events — the spill this
gate prevents from regressing.

It scans `apps/query-service/lib` for `Phoenix.PubSub.broadcast` / `subscribe`
calls whose topic is a non-tenant-scoped `events:`/`projections:` topic. Each
must be fixed or carry an inline `ISOLATION_OK: <reason>` justification nearby.

## Check 2 — per-tenant projection compute must NOT live in Core

Per-tenant projections (epic `t-822210`) are a **Query Service** concern: QS
folds a tenant's event stream into read-models. **Core IS the durable event
store** — it may *store* the enabled set as opaque tenant metadata
(`metadata.projections.enabled`), but it must never *compute or serve* per-tenant
projection state. Doing so would rewire Core's ingest hot path and break the
Core/QS role split (see `CLAUDE.md` and
`docs/proposals/PER_TENANT_PROJECTIONS.md` "Why not Core").

This check scans `apps/core/src` and FAILS on lines that signal per-tenant
projection compute:

- a `list_projections_for_tenant` / `projection_for_tenant` accessor;
- Core folding `metadata.projections.enabled` (the `projections.enabled` /
  `projections_enabled` token — *interpreting* the set, not just storing it);
- a **tenant-keyed projection state** — a line that mentions both a
  projection-state token (`projection_state`, `projection ... get_state`,
  `state_cache`) **and** a tenant token (`tenant_id`, `tenant.id`, `by_tenant`).

Core's GLOBAL engine projections (`entity_snapshots`, `event_counters`, Prime's
9, the embedded demo set) are internal database read-models — they key state by
`"{name}:{entity_id}"` with **no tenant dimension**, so they are not flagged. The
rule targets the *intersection* of "projection" and "tenant", not either word
alone (Core legitimately uses both).

A genuine exception must carry an inline `CORE_PROJECTION_OK: <reason>` comment
within 6 lines (mirrors `ISOLATION_OK`). Exceptions are printed as an audit
surface — **overridable, never silent**.

## Exit codes & CI

Exit 0 = both checks clean (or all hits justified); exit 1 = an un-justified
global topic, or per-tenant projection compute in Core, was introduced. Wire
into CI alongside the Elixir/Rust/Go test gates.
</content>
