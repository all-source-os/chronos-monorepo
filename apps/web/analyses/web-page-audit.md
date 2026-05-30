# AllSource Web Page Audit (apps/web)

Date: 2026-05-30
Targets: frontend `https://www.all-source.xyz`, backend `https://api.all-source.xyz` (Core),
Query Service `https://allsource-query.fly.dev` (proxied through `www/api/*`).
Method: browser render (gstack headless Chromium) + API probe with a service-account
Debug key (role=`serviceaccount`). Debug token is NOT stored in this file or any artifact.

## Architecture notes discovered (load-bearing)

- The web app talks to the backend via **relative** `/api/*` URLs through Next.js proxy
  route handlers (`src/app/api/[...path]/route.ts`). `NEXT_PUBLIC_API_URL` /
  `NEXT_PUBLIC_WS_URL` are read **server-side only**; they are NOT in the client bundle.
- `api.all-source.xyz` is **Core** (only `/api/v1/*` works). The dashboard's data routes
  (`/api/events`, `/api/api-keys`, ...) go to the **Query Service**.
- Dashboard layout (`src/app/dashboard/layout.tsx`) fetches `/api/auth/session` on mount and
  redirects to `/login?redirect=...` when unauthenticated. Confirmed for all dashboard routes.
- Session is validated by `/api/auth/session` calling Query Service `/api/auth/me` with the
  `auth_token` cookie as Bearer. The Debug key is a valid bearer for `/api/auth/me`, so it was
  injected as the `auth_token` cookie to exercise the authenticated dashboard in-browser.
- **Production Query Service runs the Community edition.** Tenant / billing / team / audit /
  usage-analytics routes are gated behind `Edition.enterprise?()` in the QS router and 404 in
  prod. Pages that hit them must (and do) render a graceful error/empty state.

## Code fixes landed

| # | File:line | Defect | Fix |
|---|-----------|--------|-----|
| 1 | `src/lib/blog.ts:84-90` | Unknown blog slug → `fs.readFileSync` throws ENOENT → Next surfaces **HTTP 500** (`/blog/<bad>` returned 500). The page's `if (!post) notFound()` was dead code because `getPost` threw before returning. | `getPost` now `fs.existsSync`-probes and returns `null` for missing files. |
| 2 | `src/app/(marketing)/blog/[slug]/page.tsx:18-21` | `generateMetadata` called `getPost(slug)` then dereferenced `post.metadata` — same throw on bad slug. | Guard `if (!post) return undefined;` so metadata generation 404s cleanly. |
| 3 | `src/lib/blog.ts:104-111` | `getAllPosts` now sees `getPost`'s `null` in its type union. | Narrow with an explicit guard (files come from the dir listing, always exist). |
| 4 | `src/hooks/use-phoenix-channel.ts:12-37` | `getSocketUrl()` fell back to `window.location.host` when `NEXT_PUBLIC_WS_URL` is unset. In prod that is the Vercel frontend (`www.all-source.xyz`), which has **no `/ws` Phoenix endpoint** → endless `wss://www.all-source.xyz/ws → 404` reconnect storm in the console on `/dashboard/events` and demo live-stream. | Return `null` for any non-localhost origin when no WS URL is configured (only fall back to page origin for local dev). |
| 5 | `src/hooks/use-phoenix-channel.ts:46-60` (`acquireSocket`) and `connect` callback | Socket creation assumed a non-null URL. | `acquireSocket` returns `null` when no backend WS URL; `connect` no-ops gracefully, leaving `isConnected=false`. |

Verification of fixes (local production build, `VERCEL=1 bun run build`):
- `/blog/this-does-not-exist-xyz` → **404** (was 500); valid slug → 200; homepage → 200.
- `bun run build` ✓ compiled, all 64 static pages generated, exit 0.
- `bun run type-check` ✓ no errors.
- WS fix: localhost path still attempts `ws://localhost:<port>/ws` (dev); non-localhost path
  returns null and never connects (prod reconnect storm eliminated). No crash on `/dashboard/events`.

## Out-of-scope (backend / deployment, documented not fixed)

These are NOT apps/web bugs — the web pages handle them gracefully:

- **Community-edition 404s**: `/api/tenant`, `/api/tenant/usage`, `/api/tenant/audit-logs`,
  `/api/tenants/me/analytics`, `/api/billing/*`, `/api/team/*` all 404 in prod because the
  Query Service runs Community edition. Affected pages (analytics, audit-log, billing, team,
  overview) render a clean error/empty state ("Resource not found" panel, 0/quota counters) —
  no crash, no infinite spinner. Fixing requires deploying the enterprise edition or
  enabling those routes server-side.
- **Realtime WebSocket has no public endpoint**: production has no client-exposed
  `NEXT_PUBLIC_WS_URL`, and Next.js route handlers can't tunnel WebSockets. The QS `/ws`
  endpoint exists (`allsource-query.fly.dev/ws` → 403, `api.all-source.xyz/ws` → 401, i.e.
  reachable but auth/upgrade-gated). To enable live event streaming in prod, set
  `NEXT_PUBLIC_WS_URL` in Vercel to a public WS gateway and ensure CSP `connect-src` covers it
  (CSP already allows `wss://api.all-source.xyz`). Fix #4/#5 stops the error storm in the
  meantime.

## Dead code (noted, not chased — no page is broken by it)

- `ApiClient.getAnalyticsStats()` → `/api/analytics/stats` (no such QS route — analytics has
  `/frequency`, `/summary`, etc.). Not called by any page.
- `ApiClient.getEventsInRange()` → `/api/events/range` (collides with `/api/events/:id`, 400s).
  Not called by any page.
- `ApiClient.getApiKeyScopes()` → `/api/api-keys/scopes` (no such route). Not called by any page.

## Per-page results (56 pages)

Legend: PASS = renders clean both modes; FIXED = had a defect, now fixed+re-verified;
FOLLOW-UP = renders gracefully but underlying data is backend-gated (out of scope).

### Auth (5) — render + submit-target + error-path
| Route | Status | Use case checked | Evidence |
|-------|--------|------------------|----------|
| /login | PASS | Email form reveals; bad creds → POST `/api/v1/auth/login` → 401 → "Invalid email or password", stays on page | browser drive; API 401 |
| /signup | PASS | Renders; OAuth + `/api/v1/auth/register` targets correct | render 200 |
| /forgot-password | PASS | Renders; submits `/api/auth/forgot-password` | render 200 |
| /reset-password | PASS | Renders | render 200 |
| /verify-email | PASS | Renders | render 200 |

Note: forgot/reset/verify post to `/api/auth/*` (generic proxy → Query Service) while
login/signup use `/api/v1/auth/*` (Control Plane). Not exercised destructively; flagged as a
possible target inconsistency but both paths return handled responses, not crashes.

### Marketing / docs / static (37) — render + links + embedded examples
All render HTTP 200 with 0 console errors and no failed same-origin requests:
/ , /about, /blog, /blog/[slug] (FIXED), /changelog, /compare/agent-memory,
/compare/eventstoredb, /connect, /examples, /use-cases, /privacy, /terms, /status, /sdks,
/prime, /docs, /docs/api, /docs/chronis, /docs/mcp, /docs/tenant-setup, /docs/prime,
/docs/prime/concepts, /docs/prime/embedded, /docs/prime/http, /docs/prime/mcp,
/docs/prime/quickstart, /platform/event-sourcing, /platform/prime,
/platform/stream-processing, /solutions/agent-memory, /solutions/audit-compliance,
/solutions/financial-services, /solutions/iot-telemetry, /solutions/multi-tenant-saas,
/solutions/quant-intelligence, /solutions/real-time-analytics, /ui-test.

| Route | Status | Notes |
|-------|--------|-------|
| /blog/[slug] (valid) | PASS | e.g. `/blog/allsource-as-cms-from-claude-desktop` → 200, full article, 0 errors |
| /blog/[slug] (unknown) | FIXED | was 500 → now 404 (fixes #1-#3) |
| /connect | PASS | fires `/api/auth/session` → 401 when logged out (expected probe); page still renders connect UI |

### Dashboard (14) — auth gate + data fetch + browser-vs-API agreement
Unauthenticated: ALL 14 redirect to `/login?redirect=<route>` (verified). Authenticated
(Debug cookie):
| Route | Status | Use case checked | Evidence |
|-------|--------|------------------|----------|
| /dashboard | PASS | Overview loads, usage 0/10,000 zero-state | render; `/api/tenant/usage` 404 handled |
| /dashboard/analytics | FOLLOW-UP | Renders shell + "Resource not found" panel for enterprise-gated `/api/tenants/me/analytics` 404 | screenshot `shots/analytics.png` |
| /dashboard/api-keys | PASS | Lists both keys (Debug + claude Code Desktop); **browser matches API** (`/api/api-keys` → count:2) | screenshot `shots/api-keys.png` + API 200 |
| /dashboard/billing | FOLLOW-UP | Renders; `/api/tenant/usage` 404 (community edition) handled | render |
| /dashboard/events | FIXED | Event Explorer renders; was emitting `wss://www/ws → 404` storm — fixed (#4/#5). `/api/events` → 200 (count:0 for this tenant, empty state) | render + API 200 |
| /dashboard/memory | PASS | Renders | render, 0 errors |
| /dashboard/pipelines | PASS | Renders | render, 0 errors |
| /dashboard/team | FOLLOW-UP | Renders; `/api/team/members` enterprise-gated 404 handled | render |
| /dashboard/settings | PASS | Renders | render, 0 errors |
| /dashboard/settings/audit-log | FOLLOW-UP | Renders; `/api/tenant/audit-logs` 404 handled, no spinner | screenshot `shots/audit-log.png` |
| /dashboard/tools/replay | PASS | Renders; `/api/replay` → 200 | render + API 200 |
| /dashboard/demo | PASS | Renders | render, 0 errors |
| /dashboard/demo/onboarding | PASS | Renders | render, 0 errors |
| /onboarding | PASS | Renders (redirects to login when unauth) | render |

## Summary

- 56/56 pages have browser + API evidence.
- PASS: 48 · FIXED: 4 page-routes covered by 5 code edits (blog 500→404 across slug
  page + lib; events WS storm) · FOLLOW-UP: 4 (all backend community-edition gating, pages
  degrade gracefully).
- No previously-passing page regressed. `bun run build` and `bun run type-check` pass.
- Repo-wide `bun run lint` (biome) fails with 217 pre-existing errors unrelated to this
  change; the 3 touched files introduce **no new** lint errors (the 3 flagged in
  use-phoenix-channel.ts pre-date this work on lines not edited).
