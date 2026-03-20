# PRD: Dashboard E2E Test Suite — Production Safety Net

## Overview
Create a comprehensive Playwright E2E test suite that exercises every interactive element on the AllSource Chronos dashboard at `https://all-source.xyz`. Tests run against production with a dedicated test account. Every button, form, modal, dropdown, drawer, toggle, and navigation link must be covered. Bugs discovered during test writing are documented in `docs/e2e-issues.md` and fixed in subsequent stories.

## Goals
- Achieve 100% interactive element coverage across all authenticated dashboard pages
- Catch regressions before users do — "safer than a plane"
- Document every broken element discovered during test authoring in `docs/e2e-issues.md`
- Fix all discovered bugs so the full suite passes green
- Validate real-time WebSocket event streaming works end-to-end
- Run reliably in CI via `make ci`

## Quality Gates

### Epic-Level (run once on epic completion)
General codebase checks that run ONCE when all stories are done:
- `make ci` — full CI pipeline passes

### Story-Level (checked per story)
- Run that story's Playwright spec file: `cd tooling/e2e && bunx playwright test tests/<spec-file>.spec.ts`

## User Stories

### US-001: Scaffold Playwright E2E project [Integration]
Set up the Playwright test infrastructure in `tooling/e2e/`.

**Acceptance Criteria:**
- [ ] `tooling/e2e/package.json` exists with `@playwright/test` dependency
- [ ] `tooling/e2e/playwright.config.ts` configured for Chromium only, base URL `https://all-source.xyz`
- [ ] `tooling/e2e/.env.test` template exists with `TEST_EMAIL` and `TEST_PASSWORD` placeholders (gitignored)
- [ ] `tooling/e2e/.gitignore` ignores `.env.test`, `test-results/`, `playwright-report/`
- [ ] `tooling/e2e/fixtures/auth.ts` exports a reusable `authenticatedPage` fixture that logs in once and reuses session via `storageState`
- [ ] `tooling/e2e/fixtures/auth.ts` handles login via email/password form at `/login`
- [ ] A smoke test `tests/smoke/auth.spec.ts` verifies login succeeds and redirects to `/dashboard`
- [ ] `cd tooling/e2e && bunx playwright test tests/smoke/auth.spec.ts` passes

Mark each item [x] as you complete it. Only close when all are checked.

### US-002: Test sidebar navigation and layout [UI]
Verify all 11 sidebar nav links route correctly and the sidebar collapse/expand works.

**Acceptance Criteria:**
- [ ] Test file: `tests/dashboard/sidebar-navigation.spec.ts`
- [ ] Test clicks each of the 11 sidebar links (Overview, Events, Pipelines, Analytics, Demo Zone, API Keys, Team, Replay, Audit Log, Billing, Settings) and asserts correct URL
- [ ] Test verifies active state highlighting on the current page's nav link
- [ ] Test clicks sidebar collapse button and verifies sidebar collapses (aria-label changes to "Expand sidebar")
- [ ] Test clicks expand button and verifies sidebar expands back
- [ ] Test verifies logo link navigates to `/dashboard`
- [ ] `cd tooling/e2e && bunx playwright test tests/dashboard/sidebar-navigation.spec.ts` passes

Mark each item [x] as you complete it. Only close when all are checked.

### US-003: Test header elements — theme, user menu, command palette [UI]
Cover all header interactive elements.

**Acceptance Criteria:**
- [ ] Test file: `tests/dashboard/header.spec.ts`
- [ ] Test clicks theme toggle button and verifies theme class changes on `<html>` or body
- [ ] Test clicks user avatar → verifies dropdown shows name, email, Profile, Settings, Log out
- [ ] Test clicks "Profile" in user menu → navigates to `/dashboard/settings`
- [ ] Test clicks "Settings" in user menu → navigates to `/dashboard/settings`
- [ ] Test verifies Cmd+K opens command palette with search input visible
- [ ] Test types in command palette search and verifies filtered results
- [ ] Test clicks a command palette navigation item (e.g., "Go to Events") and verifies navigation
- [ ] Test verifies Escape closes command palette
- [ ] Test verifies notifications bell button is clickable
- [ ] `cd tooling/e2e && bunx playwright test tests/dashboard/header.spec.ts` passes

Mark each item [x] as you complete it. Only close when all are checked.

### US-004: Test time travel picker [UI]
Cover the time travel popover and all its interactions.

**Acceptance Criteria:**
- [ ] Test file: `tests/dashboard/time-travel.spec.ts`
- [ ] Test clicks time travel trigger button → popover opens
- [ ] Test clicks a quick preset (e.g., "1 hour ago") → popover closes, trigger shows historical date in amber
- [ ] Test verifies historical mode banner appears when time travel is active
- [ ] Test clicks "Return to Present" → trigger returns to "Time Travel" text
- [ ] Test sets custom date and time inputs, clicks "Travel to this time" → popover closes, trigger shows custom date
- [ ] Test verifies Escape closes popover
- [ ] Test verifies Cmd+T opens/closes popover
- [ ] `cd tooling/e2e && bunx playwright test tests/dashboard/time-travel.spec.ts` passes

Mark each item [x] as you complete it. Only close when all are checked.

### US-005: Test dashboard overview page [UI]
Cover stats, quick actions, API key preview, and usage charts on `/dashboard`.

**Acceptance Criteria:**
- [ ] Test file: `tests/dashboard/overview.spec.ts`
- [ ] Test navigates to `/dashboard` and verifies stats cards render with numeric values
- [ ] Test verifies current plan card shows plan name and usage bars
- [ ] Test verifies usage charts render (SVG/canvas elements present)
- [ ] Test clicks "View All" link in API keys section → navigates to `/dashboard/api-keys`
- [ ] Test clicks each of the 6 quick action cards and verifies navigation/action:
  - Create Event → `/dashboard/events?action=create`
  - Generate API Key → `/dashboard/api-keys?action=create`
  - View Documentation → external link (verify `target="_blank"`)
  - API Reference → external link
  - Join Discord → external link
  - Changelog → `/blog`
- [ ] Test verifies recent events section renders
- [ ] `cd tooling/e2e && bunx playwright test tests/dashboard/overview.spec.ts` passes

Mark each item [x] as you complete it. Only close when all are checked.

### US-006: Test events page — search, filters, export, list, detail drawer [UI]
Full coverage of the Event Explorer page.

**Acceptance Criteria:**
- [ ] Test file: `tests/dashboard/events.spec.ts`
- [ ] Test navigates to `/dashboard/events` and verifies event list renders
- [ ] Test types in search input and verifies list filters (or shows "no results")
- [ ] Test clicks "Filters" toggle → filter panel expands showing Entity ID, Event Type, Date Range inputs
- [ ] Test fills in Entity ID filter → verifies list filters
- [ ] Test clicks "X" clear filters button → filters reset, badge count disappears
- [ ] Test clicks "Export" button → verifies file download triggers (`.json` file)
- [ ] Test clicks an event row → event detail drawer slides up
- [ ] In event detail drawer: test clicks "Copy Event ID" → clipboard write succeeds (verify button state change)
- [ ] In event detail drawer: test clicks "Copy Entity ID"
- [ ] In event detail drawer: test clicks "Copy JSON" → clipboard write succeeds
- [ ] In event detail drawer: test clicks "Close" → drawer closes
- [ ] Test verifies live event feed section renders with connection status indicator
- [ ] Test clicks pause button on live feed → feed pauses (button toggles to play)
- [ ] Test clicks clear (trash) button → feed clears
- [ ] `cd tooling/e2e && bunx playwright test tests/dashboard/events.spec.ts` passes

Mark each item [x] as you complete it. Only close when all are checked.

### US-007: Test API keys page — CRUD and copy [UI]
Full coverage of API key creation, listing, copying, rotation, and revocation.

**Acceptance Criteria:**
- [ ] Test file: `tests/dashboard/api-keys.spec.ts`
- [ ] Test navigates to `/dashboard/api-keys` and verifies key table renders
- [ ] Test clicks "Create Key" → modal opens with Name input, Description input, 7 scope toggles, Expiration dropdown
- [ ] Test fills in key name "E2E Test Key", toggles `events:read` + `events:write` scopes, selects "30 days" expiration
- [ ] Test clicks "Create Key" in modal → success state shows generated key
- [ ] Test clicks show/hide toggle on generated key → key text toggles visibility
- [ ] Test clicks "Copy" on generated key → clipboard write succeeds
- [ ] Test clicks "Done" → modal closes, new key appears in table
- [ ] Test clicks copy prefix button on a key row → clipboard succeeds
- [ ] Test clicks "..." overflow menu on the new key → shows "Rotate Key" and "Revoke Key"
- [ ] Test clicks "Revoke Key" → confirmation modal appears with "Cancel" and "Revoke Key" buttons
- [ ] Test clicks "Cancel" → modal closes, key still exists
- [ ] Test clicks "..." → "Revoke Key" → "Revoke Key" confirm → key is removed from table
- [ ] `cd tooling/e2e && bunx playwright test tests/dashboard/api-keys.spec.ts` passes

Mark each item [x] as you complete it. Only close when all are checked.

### US-008: Test pipelines page [UI]
Cover pipeline listing, status cards, and overflow menu actions.

**Acceptance Criteria:**
- [ ] Test file: `tests/dashboard/pipelines.spec.ts`
- [ ] Test navigates to `/dashboard/pipelines` and verifies page renders
- [ ] If pipelines exist: test verifies status overview cards (Total, Running, Paused, Errors) show numbers
- [ ] If pipelines exist: test clicks "..." on a pipeline card → dropdown shows Pause/Resume/Reset
- [ ] If pipelines exist: test clicks "Pause" → pipeline status changes to paused
- [ ] If no pipelines: test verifies empty state with "Create Pipeline" button
- [ ] Test verifies "Create Pipeline" button is visible and clickable
- [ ] `cd tooling/e2e && bunx playwright test tests/dashboard/pipelines.spec.ts` passes

Mark each item [x] as you complete it. Only close when all are checked.

### US-009: Test analytics page — time range and charts [UI]
Cover the analytics time range selector and chart rendering.

**Acceptance Criteria:**
- [ ] Test file: `tests/dashboard/analytics.spec.ts`
- [ ] Test navigates to `/dashboard/analytics` and verifies page renders
- [ ] Test verifies summary cards render (Total Events, Event Types, Unique Entities)
- [ ] Test clicks each time range button (24h, 7d, 30d, 90d) and verifies active state changes
- [ ] Test verifies ingestion rate chart renders (Recharts SVG present)
- [ ] Test verifies event type distribution chart renders
- [ ] Test verifies top entity IDs chart renders
- [ ] `cd tooling/e2e && bunx playwright test tests/dashboard/analytics.spec.ts` passes

Mark each item [x] as you complete it. Only close when all are checked.

### US-010: Test demo zone — seeding, live fire, MCP showdown [UI]
Cover the demo zone's seeding flow and both view modes.

**Acceptance Criteria:**
- [ ] Test file: `tests/dashboard/demo-zone.spec.ts`
- [ ] Test navigates to `/dashboard/demo` and verifies page renders
- [ ] Test verifies view toggle shows "Live Fire" and "MCP Showdown" buttons
- [ ] Test clicks "MCP Showdown" → URL updates to `?view=mcp`, MCP content renders
- [ ] Test clicks "Live Fire" → URL updates back, Live Fire content renders
- [ ] If demo not seeded: test clicks "Start Demo" → seeding completes, live fire panels appear
- [ ] If demo seeded: test verifies Live Event Stream Panel renders with controls
- [ ] If demo seeded: test verifies Cost Calculator in MCP Showdown view is interactive (input fields respond)
- [ ] `cd tooling/e2e && bunx playwright test tests/dashboard/demo-zone.spec.ts` passes

Mark each item [x] as you complete it. Only close when all are checked.

### US-011: Test team page — invite, role change, remove [UI]
Full coverage of team member management.

**Acceptance Criteria:**
- [ ] Test file: `tests/dashboard/team.spec.ts`
- [ ] Test navigates to `/dashboard/team` and verifies member table renders
- [ ] Test verifies seat usage display is visible
- [ ] Test clicks "Invite Member" → modal opens with Email input, Role selector (Admin/Member/Viewer)
- [ ] Test fills in email, selects "Member" role, clicks "Send Invitation" → success state appears
- [ ] Test verifies modal auto-closes after success
- [ ] If non-owner members exist: test clicks role dropdown → verifies Admin/Member/Viewer options
- [ ] If non-owner members exist: test clicks "..." → "Remove member" → confirmation modal appears
- [ ] Test clicks "Cancel" in remove confirmation → modal closes
- [ ] `cd tooling/e2e && bunx playwright test tests/dashboard/team.spec.ts` passes

Mark each item [x] as you complete it. Only close when all are checked.

### US-012: Test event replay page [UI]
Cover the replay form, submission, and history list.

**Acceptance Criteria:**
- [ ] Test file: `tests/dashboard/replay.spec.ts`
- [ ] Test navigates to `/dashboard/tools/replay` and verifies page renders
- [ ] Test fills in From Date, From Time, To Date, To Time
- [ ] Test fills in optional Event Type Filter and Entity ID Filter
- [ ] Test fills in Target Projection
- [ ] Test clicks "Start Replay" → replay starts (appears in history list or error banner shows)
- [ ] Test verifies replay history list renders with status indicators
- [ ] If active replay: test verifies "Cancel" button appears and is clickable
- [ ] If completed replay: test verifies "Remove" button appears and is clickable
- [ ] `cd tooling/e2e && bunx playwright test tests/dashboard/replay.spec.ts` passes

Mark each item [x] as you complete it. Only close when all are checked.

### US-013: Test settings page — all 4 tabs [UI]
Cover Profile, Workspace, Security, and Notifications tabs.

**Acceptance Criteria:**
- [ ] Test file: `tests/dashboard/settings.spec.ts`
- [ ] Test navigates to `/dashboard/settings` and verifies page renders with tab navigation
- [ ] **Profile tab:** Test verifies Full Name input is editable, email input is disabled
- [ ] **Profile tab:** Test changes name, clicks "Save Changes" → success indicator appears
- [ ] **Profile tab:** Test reverts name back to original, saves again
- [ ] **Workspace tab:** Test clicks "Workspace" tab → Workspace Name and URL slug inputs visible
- [ ] **Workspace tab:** Test types in URL slug → verifies sanitization (lowercase, no spaces)
- [ ] **Workspace tab:** Test verifies Tenant ID is displayed (read-only)
- [ ] **Security tab:** Test clicks "Security" tab → connected accounts section visible
- [ ] **Security tab:** Test verifies "Delete Account" button exists
- [ ] **Notifications tab:** Test clicks "Notifications" tab → 5 toggle switches visible
- [ ] **Notifications tab:** Test toggles each switch and verifies state changes
- [ ] `cd tooling/e2e && bunx playwright test tests/dashboard/settings.spec.ts` passes

Mark each item [x] as you complete it. Only close when all are checked.

### US-014: Test audit log page — filters and pagination [UI]
Cover action filter pills and pagination.

**Acceptance Criteria:**
- [ ] Test file: `tests/dashboard/audit-log.spec.ts`
- [ ] Test navigates to `/dashboard/settings/audit-log` and verifies page renders
- [ ] Test verifies "All" filter pill is active by default
- [ ] Test clicks an action filter pill (e.g., "api_key.created") → list filters to that action type
- [ ] Test clicks "All" → list shows all actions again
- [ ] Test verifies pagination controls (Previous/Next buttons, page number)
- [ ] If multiple pages exist: test clicks "Next" → page number increments, entries change
- [ ] If multiple pages exist: test clicks "Previous" → page number decrements
- [ ] `cd tooling/e2e && bunx playwright test tests/dashboard/audit-log.spec.ts` passes

Mark each item [x] as you complete it. Only close when all are checked.

### US-015: Test billing page — plan toggle and actions [UI]
Cover billing toggle, plan cards, and subscription management.

**Acceptance Criteria:**
- [ ] Test file: `tests/dashboard/billing.spec.ts`
- [ ] Test navigates to `/dashboard/billing` and verifies page renders
- [ ] Test verifies usage charts render (Events, Queries)
- [ ] Test clicks "Yearly" toggle → plan card prices update to yearly pricing
- [ ] Test clicks "Monthly" toggle → prices revert to monthly
- [ ] Test verifies plan cards render with "Upgrade" or "Contact Sales" buttons
- [ ] If on paid plan: test verifies "Manage Subscription" button exists
- [ ] `cd tooling/e2e && bunx playwright test tests/dashboard/billing.spec.ts` passes

Mark each item [x] as you complete it. Only close when all are checked.

### US-016: Test feedback widget [UI]
Cover the floating feedback button and modal form.

**Acceptance Criteria:**
- [ ] Test file: `tests/dashboard/feedback-widget.spec.ts`
- [ ] Test verifies floating "+" button is visible (bottom-right corner)
- [ ] Test clicks "+" → feedback modal opens
- [ ] Test clicks each category button (Bug Report, Feature Request, Question) → active state changes
- [ ] Test fills in message textarea
- [ ] Test fills in optional email input
- [ ] Test clicks "Cancel" → modal closes
- [ ] Test reopens modal, fills form, clicks "Submit Feedback" → success state appears
- [ ] Test verifies modal auto-closes after success
- [ ] Test verifies Escape closes the modal
- [ ] `cd tooling/e2e && bunx playwright test tests/dashboard/feedback-widget.spec.ts` passes

Mark each item [x] as you complete it. Only close when all are checked.

### US-017: Test WebSocket live event streaming [Integration]
Verify the real-time event feed connects and receives events.

**Acceptance Criteria:**
- [ ] Test file: `tests/dashboard/websocket-streaming.spec.ts`
- [ ] Test navigates to `/dashboard/events` and verifies live event feed section renders
- [ ] Test verifies connection status indicator shows connected (Wifi icon) or simulation mode
- [ ] If connected: test waits for at least one live event to appear in the feed (max 10s timeout)
- [ ] If in simulation mode: test verifies simulated events appear at regular intervals
- [ ] Test clicks pause → verifies no new events appear for 3 seconds
- [ ] Test clicks play → verifies new events resume
- [ ] `cd tooling/e2e && bunx playwright test tests/dashboard/websocket-streaming.spec.ts` passes

Mark each item [x] as you complete it. Only close when all are checked.

### US-018: Test logout flow [UI]
Verify logging out works and redirects correctly.

**Acceptance Criteria:**
- [ ] Test file: `tests/dashboard/logout.spec.ts`
- [ ] Test clicks user avatar → user menu opens
- [ ] Test clicks "Log out" → session is destroyed, redirected to `/login`
- [ ] Test verifies navigating to `/dashboard` redirects back to `/login` (not authenticated)
- [ ] `cd tooling/e2e && bunx playwright test tests/dashboard/logout.spec.ts` passes

Mark each item [x] as you complete it. Only close when all are checked.

### US-019: Document all issues found and fix broken elements [Integration]
Create the issues report and fix every bug discovered during test writing.

**Acceptance Criteria:**
- [ ] `docs/e2e-issues.md` created with a table: Issue #, Page, Element, Description, Status (Fixed/Open), Fix Commit
- [ ] Every bug found during US-001 through US-018 is logged in this file
- [ ] Every logged bug with Status=Fixed has the corresponding code fix applied
- [ ] All previously-written E2E tests pass after fixes: `cd tooling/e2e && bunx playwright test` passes with 0 failures
- [ ] No regressions — pages that were working before still work

Mark each item [x] as you complete it. Only close when all are checked.

### US-020: Full suite green run and CI integration [Integration]
Verify the entire test suite passes end-to-end and integrates with CI.

**Acceptance Criteria:**
- [ ] `cd tooling/e2e && bunx playwright test` runs all spec files and passes with 0 failures
- [ ] `make ci` passes (includes any E2E-related CI steps if configured)
- [ ] Test execution completes within a reasonable time (under 5 minutes for full suite)
- [ ] Playwright HTML report generates in `tooling/e2e/playwright-report/`

Mark each item [x] as you complete it. Only close when all are checked.

## Functional Requirements
- FR-1: Every authenticated dashboard page must have at least one E2E test spec file
- FR-2: Every interactive element (button, input, toggle, dropdown, modal, drawer, link) must be exercised in at least one test
- FR-3: Tests must authenticate using a real test account (email/password), not mock auth
- FR-4: Session state must be reused across tests within a spec file via Playwright's `storageState` to avoid redundant logins
- FR-5: Tests must handle loading states gracefully (wait for elements, not arbitrary timeouts)
- FR-6: The "Export" functionality must verify a file download triggers (not just a button click)
- FR-7: Clipboard operations must verify the write succeeded (via button state change or Playwright clipboard API)
- FR-8: Tests must not leave test data behind (e.g., revoke created API keys, remove invited members)
- FR-9: All discovered bugs must be documented in `docs/e2e-issues.md` with page, element, and description

## Non-Goals (Out of Scope)
- Marketing/landing pages at `https://all-source.xyz/` (only the authenticated dashboard)
- Visual regression testing / screenshot comparison
- Multi-browser testing (Firefox, WebKit) — Chromium only
- Mobile viewport testing
- Performance/load testing
- Auth pages (login, signup, forgot password) — beyond the initial login needed for test setup
- Onboarding wizard (`/onboarding`) — not part of the main dashboard flow

## Technical Considerations
- Playwright `storageState` should be generated once in a global setup and reused by all tests
- Tests run against production (`https://all-source.xyz`) — no test data seeding beyond what the app provides
- API key tests create and revoke keys — ensure cleanup runs even on test failure (use `test.afterEach`)
- Team invite tests should use a throwaway email or verify invitation can be cancelled
- WebSocket tests need appropriate timeouts — production may have variable latency
- The test account must have sufficient permissions (owner or admin) to access all pages
- Clipboard tests require Playwright's `browserContext.grantPermissions(['clipboard-read', 'clipboard-write'])`

## Success Metrics
- 100% of dashboard pages covered by at least one spec file
- 0 test failures on full suite run
- Every interactive element exercised (80+ buttons, 25+ inputs, 6+ modals, all navigation)
- All discovered bugs documented and fixed
- Suite completes in under 5 minutes

## Open Questions
- What are the test account credentials? (Need to be set in `.env.test` before first run)
- Does the test account have owner-level permissions on its workspace?
- Is there rate limiting on production that could affect rapid test execution?
- Should we set up a dedicated test tenant to avoid polluting real data?