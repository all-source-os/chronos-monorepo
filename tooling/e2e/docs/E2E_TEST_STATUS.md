---
title: "E2E Test Suite — Status Report"
date: 2026-03-01
status: GREEN (284/287 pass)
---

# E2E Test Suite — Status Report

## Current Status: GREEN

| Metric | Value |
|--------|-------|
| Total tests | 287 |
| Passed | 284 |
| Skipped | 3 (expected) |
| Failed | 0 |
| Duration | ~5.7 minutes |
| Last run | 2026-03-01 |
| Runner | Playwright + Chromium |

---

## Architecture

```
Playwright (tooling/e2e/)
    │
    ├── global-setup.ts          ← Health gate: fails fast if stack is down
    ├── playwright.config.ts     ← Config: baseURL, projects, globalSetup
    ├── fixtures/
    │   ├── auth.ts              ← Email/password login fixture
    │   ├── demo-auth.ts         ← Shared demoLogin + authenticateAndGoToDashboard
    │   └── pages.ts             ← Page object fixtures
    ├── page-objects/            ← Page object models
    └── tests/
        ├── dashboard/           ← 17 spec files (authenticated dashboard pages)
        ├── smoke/               ← 6 spec files (cross-service integration)
        ├── local-only/          ← 1 spec file (dev-token auth, requires AUTH_DISABLED)
        └── ui-components/       ← 1 spec file (shared UI component library)
```

### Service Dependencies

| Service | Default URL | Env Var | Required |
|---------|-------------|---------|----------|
| Web App | `https://all-source.xyz` | `BASE_URL` | Yes |
| Control Plane | `http://localhost:3901` | `CONTROL_PLANE_URL` | Yes |
| Core | `http://localhost:3900` | `CORE_URL` | Local only |
| Query Service | `http://localhost:3902` | `QS_URL` | Local only |

---

## Running the Tests

### Against production (default)

```bash
cd tooling/e2e
bunx playwright test
```

### Against local Docker stack

```bash
# 1. Start the stack (Core on 3280, QS on 3283, CP on 3901, Web on 3000)
# 2. Run with env overrides:
BASE_URL=http://localhost:3000 \
  CONTROL_PLANE_URL=http://localhost:3901 \
  CORE_URL=http://localhost:3280 \
  QS_URL=http://localhost:3283 \
  bunx playwright test
```

### Single test file

```bash
bunx playwright test tests/dashboard/overview.spec.ts
```

### View report

```bash
bunx playwright show-report
```

---

## Health Check Gate

The global setup (`global-setup.ts`) runs before any tests and verifies all required services are healthy:

```
🏥 Checking service health before running tests...

  ✅ Web App              https://all-source.xyz/ — healthy (200)
  ✅ Control Plane        http://localhost:3901/health — healthy (200)
  ✅ Core                 http://localhost:3280/health — healthy (200)
  ✅ Query Service        http://localhost:3283/api/health — healthy (200)
```

If any required service is down, the suite fails immediately with:

```
Error: Cannot run e2e tests: 2 required service(s) are down: Control Plane, Core.
Start the stack first: docker compose up -d (or see docs/deployment/DOCKER.md)
```

---

## Test Categories

### Dashboard Tests (17 files, ~135 tests)

All dashboard tests use the demo login flow:
1. `POST /api/v1/demo/start` → creates ephemeral demo credentials
2. `POST /api/v1/auth/login` → authenticates, returns JWT
3. `GET /api/auth/callback?token=...` → sets browser cookie
4. Navigate to dashboard page under test

| File | Tests | What it covers |
|------|-------|----------------|
| `overview.spec.ts` | 10 | Stats cards, plan card, usage charts, API keys, quick actions |
| `sidebar-navigation.spec.ts` | 8 | All 11 nav links, active state, collapse/expand |
| `events.spec.ts` | 7 | Event list, filters, detail drawer, export |
| `settings.spec.ts` | 7 | General, Workspace, Danger Zone tabs |
| `analytics.spec.ts` | 6 | Charts, summary cards, time range toggle |
| `api-keys.spec.ts` | 6 | Create, list, rotate, revoke keys |
| `audit-log.spec.ts` | 6 | Filters, pagination, log entries |
| `billing.spec.ts` | 6 | Plan cards, period toggle, usage charts |
| `feedback-widget.spec.ts` | 6 | Floating button, modal, categories, submission |
| `header.spec.ts` | 6 | Theme toggle, user menu |
| `pipelines.spec.ts` | 6 | Pipeline list, create, status |
| `replay.spec.ts` | 6 | Replay form, history, remove |
| `team.spec.ts` | 5 | Member table, invite modal, seats |
| `time-travel.spec.ts` | 5 | Time slider, event replay |
| `demo-zone.spec.ts` | 4 | Demo seeding, MCP showdown view |
| `websocket-streaming.spec.ts` | 4 | Real-time event feed |
| `logout.spec.ts` | 3 | Logout flow, session clear, redirect |

### Smoke Tests (6 files, ~80 tests)

| File | Tests | What it covers |
|------|-------|----------------|
| `demo-zone.spec.ts` | ~50 | MCP Showdown, Speed Race, Cost Calculator, Onboarding Wizard, responsive layout |
| `backend-integration.spec.ts` | 6 | Full-stack: CP health, QS session, events E2E, API keys, team, audit log |
| `navigation.spec.ts` | 8 | Protected routes redirect, 404 handling, breadcrumbs |
| `dashboard.spec.ts` | 6 | Dashboard renders, sidebar, events page |
| `auth-staging.spec.ts` | 5 | Login UI, demo flow, OAuth (skipped without creds) |
| `auth.spec.ts` | 1 | Authenticated fixture smoke test |

### UI Component Tests (1 file, 36 tests)

Tests the shared `@allsource/ui` component library rendered on `/ui-test`:
- Button variants (5): default, destructive, outline, secondary, ghost
- Badge variants (4): default, secondary, destructive, outline
- Card component: header, title, content
- Accessibility: keyboard focus, ARIA roles, heading hierarchy

### Local-Only Tests (1 file, 5 tests)

Requires `AUTH_DISABLED=true` on the Query Service (dev mode):
- Login page renders
- Invalid token error handling
- Dev-token authentication flow
- Session management
- Logout

---

## Skipped Tests (3)

| Test | Reason |
|------|--------|
| `auth-staging.spec.ts` — OAuth flow | `E2E_OAUTH_EMAIL`/`PASSWORD` not set |
| `auth-staging.spec.ts` — Demo banner | TODO: QS `/api/auth/me` doesn't return tenant data |
| `auth-staging.spec.ts` — OAuth login | `E2E_OAUTH_PROVIDER` not set |

---

## Issue Fixed: 171 Failures (2026-03-01)

### Root Cause

All 171 test failures had the same root cause: `ECONNREFUSED ::1:3901`. The `demoLogin()` function called `request.post()` to the Control Plane, which threw an unhandled network error in `test.beforeAll()`. This crashed the test suite before `test.skip(!token)` could execute.

### Contributing Factor

`demoLogin()` was copy-pasted across 21 test files with no try-catch. Each copy independently failed with the same unhandled exception.

### Fix Applied

| Change | File | Impact |
|--------|------|--------|
| Global health gate | `global-setup.ts` (new) | Fails fast before any tests run |
| Shared fixture | `fixtures/demo-auth.ts` (new) | Single source of truth with try-catch |
| Config wiring | `playwright.config.ts` | `globalSetup: "./global-setup.ts"` |
| 21 test files refactored | `tests/dashboard/*.spec.ts`, `tests/smoke/*.spec.ts` | Import from shared fixture, ~490 lines removed |

### Commit

```
6e906c8 fix(e2e): add health check gate and extract shared demoLogin fixture
```

---

## Maintenance Notes

- **Adding a new dashboard test**: Import `demoLogin` from `../../fixtures/demo-auth` instead of defining it inline
- **Adding a new service check**: Add to `global-setup.ts` services array
- **Tests hang**: Check if the web app dev server started (Playwright starts it automatically for local runs via `webServer` config)
- **`bun.lock` changes**: The e2e suite has its own `package.json` in `tooling/e2e/`; `bun install` may update the root lockfile
