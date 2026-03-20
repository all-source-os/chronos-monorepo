# PRD: E2E Test Cleanup and Restructure

## Overview
The E2E test suite has 268+ failing tests across 5 browsers, with test artifacts creating repository clutter. The tests fail because the web server and Core service aren't properly started before test execution. This PRD covers cleaning up artifacts, restructuring tests to be comprehensive yet maintainable, reducing the browser matrix to Chromium-only, and ensuring reliable server startup through Playwright's webServer configuration.

## Goals
- Achieve 100% passing E2E tests on Chromium
- Consolidate fragmented tests into comprehensive, maintainable test files
- Ensure reliable automatic startup of both web app and Core service
- Clean repository by gitignoring test artifacts while keeping report structure
- Verify actual UI behavior with comprehensive interaction and data tests

## Quality Gates

These commands must pass for every user story:
- `bun test` - All E2E tests pass (run from `tooling/e2e`)

## User Stories

### US-001: Clean up test artifacts and configure gitignore
As a developer, I want test artifacts excluded from git so that the repository stays clean.

**Acceptance Criteria:**
- [ ] Delete all files in `tooling/e2e/playwright-report/data/`
- [ ] Delete all files in `tooling/e2e/test-results/` subdirectories
- [ ] Add `playwright-report/data/*` to `.gitignore`
- [ ] Add `test-results/**/*` to `.gitignore` (keep directory structure)
- [ ] Keep `playwright-report/index.html` structure but ignore generated data
- [ ] Verify `git status` shows clean test directories after running tests

### US-002: Configure Chromium-only browser matrix
As a developer, I want tests to run on Chromium only so that test execution is fast and focused.

**Acceptance Criteria:**
- [ ] Update `playwright.config.ts` to include only Chromium project
- [ ] Remove Firefox, WebKit, Mobile Chrome, and Mobile Safari configurations
- [ ] Verify tests run with single browser when executing `bun test`
- [ ] Update any browser-specific test logic to target Chromium only

### US-003: Configure reliable web app startup in Playwright
As a developer, I want the web app to start automatically before tests so that tests don't fail due to missing server.

**Acceptance Criteria:**
- [ ] Configure `webServer` in `playwright.config.ts` for web app
- [ ] Use `bun run start` as the start command
- [ ] Set appropriate `url` for health check (the base URL)
- [ ] Configure reasonable `timeout` for server startup
- [ ] Set `reuseExistingServer: true` for local development flexibility
- [ ] Verify tests pass when no server is pre-running

### US-004: Configure Core service startup in Playwright
As a developer, I want the Core service to start automatically before tests so that API-dependent tests work reliably.

**Acceptance Criteria:**
- [ ] Add Core service to `webServer` array in `playwright.config.ts`
- [ ] Determine correct build and start commands for Core (Rust binary)
- [ ] Configure health check endpoint/port for Core readiness
- [ ] Ensure Core starts before web app (correct dependency order)
- [ ] Tests fail fast with clear error if Core fails to start
- [ ] Verify API-dependent tests receive real Core responses

### US-005: Explore demo page structure and document test requirements
As a developer, I want to understand the actual demo page structure so that tests match the real UI.

**Acceptance Criteria:**
- [ ] Document the demo page route structure (single page vs multiple routes)
- [ ] List all demo sections and their UI elements
- [ ] Identify which sections require Core API data
- [ ] Document expected API endpoints and response shapes
- [ ] Create a test plan mapping sections to test coverage needs

### US-006: Consolidate and rewrite Metrics demo tests
As a developer, I want comprehensive metrics demo tests so that all metrics functionality is verified in one test file.

**Acceptance Criteria:**
- [ ] Create single `metrics.spec.ts` replacing fragmented metric tests
- [ ] Test page loads with all metric cards visible
- [ ] Test each metric displays correct label, value, and icon
- [ ] Test refresh button triggers data update
- [ ] Test loading states appear during refresh
- [ ] Test metric values update with new data from Core
- [ ] Test hover/interaction states on metric cards
- [ ] Verify data comes from Core API (not mocked)

### US-007: Consolidate and rewrite Event Ingestion demo tests
As a developer, I want comprehensive event ingestion tests so that all event functionality is verified in one test file.

**Acceptance Criteria:**
- [ ] Create single `event-ingestion.spec.ts` replacing fragmented tests
- [ ] Test page loads with event generation UI visible
- [ ] Test generating different event types (e-commerce, IoT, etc.)
- [ ] Test event stream displays generated events
- [ ] Test event details are shown correctly
- [ ] Test multiple batch generation works
- [ ] Test loading states during event generation
- [ ] Verify events are sent to and retrieved from Core

### US-008: Consolidate and rewrite Query demo tests
As a developer, I want comprehensive query demo tests so that all query functionality is verified in one test file.

**Acceptance Criteria:**
- [ ] Create single `query.spec.ts` replacing fragmented tests
- [ ] Test page loads with query interface visible
- [ ] Test query parameter inputs are functional
- [ ] Test executing query by entity type
- [ ] Test executing query by event type
- [ ] Test executing time range query
- [ ] Test query results display correctly
- [ ] Test loading states during query execution
- [ ] Test multiple queries in sequence
- [ ] Verify queries execute against Core API

### US-009: Consolidate and rewrite Demo Page UI tests
As a developer, I want comprehensive demo page tests so that navigation and layout are verified in one test file.

**Acceptance Criteria:**
- [ ] Create single `demo-page.spec.ts` replacing fragmented tests
- [ ] Test demo page loads successfully
- [ ] Test all feature cards are displayed
- [ ] Test navigation between demo sections works
- [ ] Test clicking each card navigates to correct section
- [ ] Test responsive layout elements
- [ ] Test accessibility (button roles, labels)

### US-010: Write UI Components tests
As a developer, I want UI component tests so that shared components are verified.

**Acceptance Criteria:**
- [ ] Create single `ui-components.spec.ts` replacing fragmented tests
- [ ] Test button variants render correctly
- [ ] Test button sizes render correctly
- [ ] Test badge variants render correctly
- [ ] Test card components display titles and content
- [ ] Test interactive states (click, hover)
- [ ] Test components on UI test page if it exists

### US-011: Delete obsolete test files and clean up test structure
As a developer, I want a clean test directory so that only relevant, consolidated tests remain.

**Acceptance Criteria:**
- [ ] Remove all old fragmented test files after new tests are verified
- [ ] Ensure test file naming is consistent (`*.spec.ts`)
- [ ] Remove any unused test utilities or fixtures
- [ ] Update any test documentation if present
- [ ] Verify `bun test` runs only the new consolidated tests

## Functional Requirements
- FR-1: Playwright must start web app automatically using `bun run start`
- FR-2: Playwright must start Core service automatically before web app
- FR-3: Tests must fail immediately if Core service fails to start
- FR-4: All tests must run on Chromium browser only
- FR-5: Test artifacts (videos, screenshots) must be gitignored
- FR-6: Each demo section must have exactly one comprehensive test file
- FR-7: Tests must verify real API responses from Core, not mocked data
- FR-8: Tests must cover smoke, interaction, and data verification scenarios

## Non-Goals
- Multi-browser testing (Firefox, WebKit, mobile browsers)
- Mocking Core API responses
- Testing MCP server integration
- Performance or load testing
- Visual regression testing
- Tests for non-demo pages

## Technical Considerations
- Core is a Rust application - need to determine build/run commands
- Web app start command is `bun run start`
- Playwright webServer supports multiple servers with dependency ordering
- Health checks needed for both web app and Core before tests run
- Test timeout may need adjustment for Core compilation time
- Consider `reuseExistingServer: true` for faster local development cycles

## Success Metrics
- 100% of E2E tests pass on `bun test`
- Test execution completes in under 2 minutes for full suite
- Zero test artifacts committed to repository
- Test file count reduced from 20+ fragmented files to ~6 consolidated files
- Both web app and Core start automatically without manual intervention

## Open Questions
- What is the exact command to build and start Core service?
- What health check endpoint does Core expose?
- What port does Core run on?
- Are there any demo sections beyond metrics, events, queries, and UI components?
- Is there a test page at `/ui-test` or similar for component testing?