# ADR 019: Edition Gating is Compile-Time, Not Runtime

- **Status:** Accepted
- **Date:** 2026-06-01
- **Fix commit:** `2645daf` (query-service)
- **Relates to:** the `QueryServiceEx.Edition` module, `apps/query-service/lib/query_service_ex_web/router.ex`

## Context

### The incident

The dashboard **Analytics** page (and **Billing**, **Team**, **Audit Log**, and
the events/queries counters on **Overview**) showed **"Resource not found"** and
all-zero stats on the deployed, hosted gateway — even though `fly.toml` correctly
sets `ALLSOURCE_EDITION = "enterprise"` in `[env]` and the app boots as enterprise.

### Root cause

The router gates enterprise-only route scopes behind a **module-level**
conditional:

```elixir
if QueryServiceEx.Edition.enterprise?() do
  scope "/api/tenants/me", QueryServiceExWeb do
    get("/analytics", UsageAnalyticsController, :show)
  end
  # ...team, audit-logs, tenant/usage, billing
end
```

`QueryServiceEx.Edition.enterprise?()` reads `Application.get_env(:query_service_ex,
:edition)`. Phoenix evaluates a module-level `if` **when the router module is
compiled**, not at request time. So the value of `:edition` *at compile time* is
what decides whether those routes exist in the release at all.

Two things set `:edition`:

- `config.exs` — read **at compile time**. It hardcoded `:edition, :community`.
- `runtime.exs` — read **at boot**. It sets `:edition` from `ALLSOURCE_EDITION`.

The Dockerfile ran `mix compile` with `config.exs`'s `:community` and **without**
`ALLSOURCE_EDITION` in the build environment. Result: the enterprise scopes were
**compiled out of the release binary**. `runtime.exs` and the fly `[env]` var then
flipped `:edition` to `:enterprise` at boot — too late to bring back routes that
no longer exist. The router answered `/api/tenants/me/analytics` with the
fallback 404 ("Resource not found"), and the frontend rendered zeros + the error.

This is invisible in local `mix phx.server` / tests, because `test.exs` sets
`edition: :enterprise` and local dev compiles with whatever env is present.

## Decision

**The build must compile with the edition it will run as.** Compile-time edition
now honors `ALLSOURCE_EDITION`:

1. `config.exs` reads `ALLSOURCE_EDITION` at compile time (default still
   `:community`) instead of hardcoding.
2. The Dockerfile exports the `ALLSOURCE_EDITION` build `ARG` as `ENV` **before**
   `mix compile`, so the compile sees it.
3. `fly.toml` `[build.args]` passes `ALLSOURCE_EDITION = "enterprise"`, alongside
   the existing runtime `[env]` var. **Both are required:** the build arg compiles
   the routes *in*; the runtime env keeps `TenantContext` multi-tenant.

### Verification

`mix phx.routes` is the proof, not the running app:

```
ALLSOURCE_EDITION=community  mix phx.routes | grep tenants/me/analytics   # → absent (the bug)
ALLSOURCE_EDITION=enterprise mix phx.routes | grep tenants/me/analytics   # → usage_analytics_path present
```

Pages repaired by building enterprise: Analytics, Billing, Team, Audit Log, and
the events/queries counters on Overview (`/api/tenant/usage`). Memory, Pipelines,
Replay, API Keys, and Overview's projections/metrics were never edition-gated and
were unaffected (empty there means no data, not a 404).

## Consequences / rules

- **Never assume a runtime env var can turn a compiled-out route back on.** A
  module-level `if` in the router, or any `@attr` / `def` guarded by
  `Application.compile_env` / `Application.get_env` at module scope, is frozen at
  compile time.
- **An enterprise image must be BUILT enterprise.** `ALLSOURCE_EDITION` belongs in
  both `[build.args]` (compile) and `[env]` (runtime) of `fly.toml`. Dropping the
  build arg silently reintroduces this bug — the app boots "enterprise" but serves
  404s on every gated route.
- **Verify route presence with `mix phx.routes`** for the target edition, not by
  hitting the running app in dev (dev/test compile enterprise and hide the bug).
- If a future refactor wants edition to be *purely* runtime-switchable, the gating
  must move out of the router's module body into per-request plug logic (e.g. a
  plug that 404s gated paths when `Edition.community?()`), so all routes compile in
  unconditionally. That's a larger change; the build-time fix is correct and
  minimal for the current module-level design.
