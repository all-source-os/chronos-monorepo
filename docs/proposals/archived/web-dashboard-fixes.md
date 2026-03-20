# Web Dashboard Issues — Production (www.all-source.xyz)

Captured 2026-03-04 from browser console inspection of the live dashboard.

## Root Cause: CORS + Direct Backend Calls

Nearly every page fails with CORS errors because the frontend calls `https://allsource-query.fly.dev/api/...` directly instead of routing through the Next.js API proxy at `/api/...`. The proxy at `src/app/api/v1/[...path]/route.ts` exists but only covers `/api/v1/` routes. Many dashboard features call non-v1 endpoints (e.g., `/api/tenant/audit-logs`, `/api/team/invite`, `/api/api-keys`, `/api/billing/checkout`) that bypass the proxy entirely.

**Fix strategy**: Route ALL backend calls through the Next.js proxy to avoid CORS. Extend the proxy to handle all `/api/` paths, not just `/api/v1/`.

## Issues

### 1. CORS failures on multiple pages
**Affected pages**: Audit Log, Replay, Team Invite, API Keys, Analytics, Pipelines, Billing, Settings
**Error**: `Access to fetch at 'https://allsource-query.fly.dev/api/...' from origin 'https://www.all-source.xyz' has been blocked by CORS policy`
**Root cause**: API client (`src/lib/api/client.ts` or similar) constructs URLs pointing directly at the Query Service instead of using the Next.js proxy.
**Fix**: Update the API client to use relative URLs (`/api/...`) so requests go through the Next.js proxy. Extend the proxy catch-all route to handle all `/api/` paths.

### 2. Mock/demo data in production components
**Affected components**:
- `src/components/events/live-event-feed.tsx` — generates fake events (`user.signed_up`, `order.placed`, etc.) when WebSocket fails
- `src/components/demo/live-event-stream-panel.tsx` — elaborate simulation with `SIM_EVENT_TYPES` and `SIM_ENTITIES`
- `src/components/billing/usage-chart.tsx` — generates 30 random data points with `Math.random()` when no real data provided
**Requirement**: Zero mocks, zero stubs in production code. Only in test suites.
**Fix**: Remove all inline mock/simulation code. Show empty states or error messages when backend is unavailable.

### 3. Live Feed "(demo)" causes page reloads
**Component**: `src/components/events/live-event-feed.tsx`
**Issue**: The live feed badge shows "(demo)" and the component behavior causes unintentional page reloads. Should consume real data from backend WebSocket only.
**Fix**: Remove demo fallback. Show "Connecting..." or "No events" state when WebSocket is disconnected.

### 4. Replay page — calendar has too much empty space
**Component**: `src/app/dashboard/tools/replay/page.tsx`
**Issue**: The date/time inputs have excessive empty space to the right.
**Fix**: Adjust layout to fill available space or constrain the container width.

### 5. "Query It Back" / "Try It" returns 401
**Component**: `src/app/dashboard/demo/onboarding/page.tsx` (StepQueryBack, ~line 543)
**Issue**: The "Try It" button fires `GET /api/v1/events/query?event_type=user.signup&entity_id=user-001&limit=10` which returns HTTP 401.
**Fix**: Ensure the request includes the session auth token/cookie. The proxy should forward auth headers.

### 6. API Key creation modal — X button does nothing
**Component**: `src/components/api-keys/create-key-dialog.tsx`
**Issue**: Pressing "X" in the corner of the modal does nothing.
**Fix**: Debug the `handleClose` handler. Check for z-index / event propagation issues between the Card (`relative z-10`) and the backdrop.

### 7. Pipelines page — "No pipelines yet" button not actionable
**Component**: Pipelines page (likely `src/app/dashboard/pipelines/page.tsx`)
**Issue**: The empty state button is not clickable/functional.
**Fix**: Wire up the button to create a pipeline or navigate to docs.

### 8. Billing page — non-actionable, CORS failures
**Components**: `src/app/dashboard/billing/page.tsx`, `src/components/billing/plan-cards.tsx`, `src/components/billing/usage-chart.tsx`
**Issues**:
- CORS errors on `/api/tenant/usage` and `/api/billing/checkout`
- Usage chart shows random data (see issue #2)
- Plan upgrade buttons fail silently
**Fix**: Route through proxy (issue #1), remove mock chart data (issue #2).

### 9. Settings page improvements
**Component**: `src/app/dashboard/settings/page.tsx`
**Issues**:
- CORS error on user preferences endpoint
- "Workspace" section — consider removing or making functional (currently Save does a fake `setTimeout`, no real API call)
- Profile photo not uploadable
**Fix**: Route through proxy, implement real save, add photo upload functionality.

### 10. WebSocket streaming failures
**Error**: `WebSocket connection to 'wss://allsource-query.fly.dev/api/v1/events/stream' failed`
**Issue**: WebSocket connects directly to Query Service, bypassing proxy. Falls back to demo simulation.
**Fix**: Either proxy WebSocket through Next.js (complex) or configure CORS on the Query Service for WebSocket. At minimum, remove the demo fallback.

## Priority Order

1. **P0**: Fix API client to use proxy (fixes CORS on all pages) — Issue #1
2. **P0**: Remove all mock/demo data from production components — Issues #2, #3
3. **P1**: Fix auth on "Try It" query — Issue #5
4. **P1**: Fix modal X button — Issue #6
5. **P1**: Fix replay page layout — Issue #4
6. **P2**: Fix pipelines empty state — Issue #7
7. **P2**: Fix billing page — Issue #8
8. **P2**: Fix settings page — Issue #9
9. **P2**: Fix WebSocket streaming — Issue #10

## Quality Gates

### Epic-Level (run once on completion)
- `make ci` passes
- No `allsource-query.fly.dev` URLs remain in frontend API calls (all routed through proxy)
- `grep -r "mock\|demo\|simulation\|SIM_EVENT\|Math.random" apps/web/src --include="*.ts" --include="*.tsx" | grep -v test | grep -v __test__` returns zero results for production mock data

### Story-Level
- Each fixed page loads without console CORS errors
- Verify in browser using dev-browser skill
