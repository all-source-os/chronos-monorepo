# tenant-isolation-check

CI gate against cross-tenant data spill on the Query Service WebSocket path.

Every user-facing event/projection PubSub topic must be **tenant-scoped**
(`events:<tenant>:...`, `projections:<tenant>:...`). A global topic
(`events:all`, `events:<entity>`, `events:type:<type>`, `projections:<name>`)
lets any authenticated client receive every tenant's events — the spill this
gate prevents from regressing.

It scans `apps/query-service/lib` for `Phoenix.PubSub.broadcast` / `subscribe`
calls whose topic is a non-tenant-scoped `events:`/`projections:` topic. Each
must be fixed or carry an inline `ISOLATION_OK: <reason>` justification nearby.
The tool prints every justified exception (the audit surface to review) and
exits non-zero on any un-justified one.

```sh
# from the repo root
cargo run --manifest-path tooling/tenant-isolation-check/Cargo.toml
```

Exit 0 = clean (or all exceptions justified); exit 1 = an un-justified global
topic was introduced. Wire into CI alongside the Elixir/Rust/Go test gates.
