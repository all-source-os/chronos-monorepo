# PRD: All-Source Demo Zone + MCP Showdown

## Overview

A new "Demo Zone" tab in the AllSource web dashboard that makes users say "holy crap, I need this" in under 30 seconds. Two views — **Live Fire** (real-time event streaming, vector search, speed comparisons) and **MCP Showdown** (side-by-side raw vs MCP-routed workloads with cost/latency metrics). Followed by a **Build Your Own** onboarding wizard that converts demo excitement into SDK adoption.

All data is live — queries hit the user's real Core instance via the Query Service. Empty databases auto-seed via a Core API endpoint. Comparison benchmarks (Kafka, Redis, etc.) are served from a Query Service config endpoint so marketing can update without redeploying.

## Goals

- Make the value proposition viscerally obvious within 30 seconds of landing on Demo Zone
- Show real throughput, real latency, real vector search — not mocked numbers
- Demonstrate MCP's token/cost savings with a live side-by-side comparison
- Convert demo engagement into SDK adoption via an onboarding wizard
- 80% of new users spend >1 min in Demo Zone
- 30% click "Build Your Own" after demo

## Quality Gates

### Epic-Level (run once on epic completion)
General codebase checks that run ONCE when all stories are done:
- `make ci` — full monorepo CI pipeline

### Story-Level (checked per story)
- **UI stories:** Verify in browser using dev-browser skill
- **UI stories:** Playwright e2e test for that story (tests in `tooling/e2e/`)
- **Backend stories:** Verify endpoint returns expected response via curl
- **API stories:** Verify via Query Service proxy (reads go through QS, writes to Core)

## User Stories

### US-001: Add demo seed endpoint to Core [Backend]
**Description:** As a new user with an empty database, I want demo data auto-seeded so I can experience the Demo Zone immediately.

**Acceptance Criteria:**
- [ ] `POST /api/v1/demo/seed` endpoint added to Core
- [ ] Seeds 1,000 events with diverse event types (logs, metrics, user actions)
- [ ] Seeds vector embeddings for seeded events
- [ ] Endpoint is idempotent — calling twice doesn't duplicate data
- [ ] Returns `{"seeded": true, "event_count": 1000}` on success
- [ ] Verify endpoint via curl: `curl -X POST http://localhost:3900/api/v1/demo/seed`

Mark each item [x] as you complete it. Only close when all are checked.

### US-002: Add demo seed proxy to Query Service [Backend]
**Description:** As a frontend developer, I want to trigger demo seeding through the Query Service so the web app doesn't talk directly to Core.

**Acceptance Criteria:**
- [ ] `POST /api/v1/demo/seed` proxied through Query Service to Core
- [ ] No auth required for seed endpoint (demo-friendly)
- [ ] Returns Core's response unchanged
- [ ] Verify via curl: `curl -X POST http://localhost:3902/api/v1/demo/seed`

Mark each item [x] as you complete it. Only close when all are checked.

### US-003: Add benchmark config endpoint to Query Service [Backend]
**Description:** As a marketing team member, I want comparison benchmarks (Kafka throughput, Redis latency, etc.) served from a config endpoint so I can update numbers without code changes.

**Acceptance Criteria:**
- [ ] `GET /api/v1/config/benchmarks` endpoint added to Query Service
- [ ] Returns JSON with competitor metrics: `{"kafka_throughput": 50000, "redis_vector_setup": "bolt-on", ...}`
- [ ] Config stored in a file (`config/benchmarks.json`) that can be updated without recompile
- [ ] Includes AllSource metrics alongside competitor metrics for easy comparison
- [ ] Verify via curl: `curl http://localhost:3902/api/v1/config/benchmarks`

Mark each item [x] as you complete it. Only close when all are checked.

### US-004: Set up Playwright e2e in tooling/e2e [Integration]
**Description:** As a developer, I need a Playwright test harness so UI stories can have automated e2e tests.

**Acceptance Criteria:**
- [ ] `tooling/e2e/` directory created with Playwright config
- [ ] `package.json` with Playwright dependency and test scripts
- [ ] `playwright.config.ts` pointing at `http://localhost:3000` (Next.js dev server)
- [ ] One smoke test: navigate to `/` and verify page loads
- [ ] `bun test` in `tooling/e2e/` runs and passes
- [ ] Added to `make ci` target

Mark each item [x] as you complete it. Only close when all are checked.

### US-005: Demo Zone page shell + navigation [UI]
**Description:** As a user, I want a "Demo Zone" tab in the dashboard navigation so I can access the demo experience.

**Acceptance Criteria:**
- [ ] New route: `/demo` in `apps/web/src/app/demo/page.tsx`
- [ ] "Demo Zone" tab added to main navigation (sidebar or top nav, match existing pattern)
- [ ] Page renders with two view toggle buttons: "Live Fire" (default) and "MCP Showdown"
- [ ] Toggle state managed in URL params (`?view=live-fire` / `?view=mcp-showdown`)
- [ ] Empty state: "Start Demo" button that triggers seed endpoint if DB is empty
- [ ] Verify in browser using dev-browser skill
- [ ] Playwright test: navigate to `/demo`, verify toggle buttons render

Mark each item [x] as you complete it. Only close when all are checked.

### US-006: Live Event Stream panel [UI]
**Description:** As a user, I want to see a real-time feed of events streaming into my database so I can feel the speed.

**Acceptance Criteria:**
- [ ] Left panel (40% width) in Live Fire view
- [ ] Connects to Core WebSocket endpoint via Query Service for real-time events
- [ ] Events appear as append-only feed with 1-2 second bursts
- [ ] Color-coded rows: green = ingested, blue = vector-indexed, yellow = keyword hit
- [ ] Overlay stats bar: "Now: Xk/sec | Peak: Yk/sec | Latency: Zms" — live updating
- [ ] Hover tooltip on any event row shows full JSON + timestamp
- [ ] Auto-scrolls to latest, with scroll-lock toggle
- [ ] Verify in browser using dev-browser skill
- [ ] Playwright test: navigate to `/demo`, verify event stream panel renders and receives events

Mark each item [x] as you complete it. Only close when all are checked.

### US-007: Replay last 10s button [UI]
**Description:** As a user, I want to rewind the event stream by 10 seconds to see time-travel queries in action.

**Acceptance Criteria:**
- [ ] "Replay last 10s" button in the event stream panel header
- [ ] On click: queries Core via QS with time-range filter (now - 10s to now)
- [ ] Replays returned events in the stream panel at accelerated speed (visual rewind effect)
- [ ] Highlights replayed events with a distinct border/glow to differentiate from live
- [ ] Button disabled during replay, re-enables when replay completes
- [ ] Verify in browser using dev-browser skill
- [ ] Playwright test: click replay button, verify events re-render

Mark each item [x] as you complete it. Only close when all are checked.

### US-008: Vector Query Playground panel [UI]
**Description:** As a user, I want to type natural-language queries and see vector search results with latency so I understand the power of built-in vector search.

**Acceptance Criteria:**
- [ ] Middle panel (30% width) in Live Fire view
- [ ] Text input with placeholder: "Search like 'cat video' or 'error 500'"
- [ ] On submit: queries Core vector search endpoint via QS
- [ ] Auto-suggest dropdown: top 3 vector/keyword combos from their data (debounced, 300ms)
- [ ] Results: top 10 matches displayed as cards, ranked by similarity + time
- [ ] Each result shows: event summary, similarity score, timestamp
- [ ] "Why?" popover on each result: shows vector embedding snippet / match explanation
- [ ] Latency badge next to results: "38ms" (actual measured latency)
- [ ] Verify in browser using dev-browser skill
- [ ] Playwright test: type a query, verify results render with latency badge

Mark each item [x] as you complete it. Only close when all are checked.

### US-009: Speed + Simplicity Dashboard panel [UI]
**Description:** As a user evaluating AllSource, I want to see at-a-glance comparisons against Kafka, Redis, and multi-service architectures.

**Acceptance Criteria:**
- [ ] Right panel (30% width) in Live Fire view
- [ ] Fetches comparison data from QS benchmark config endpoint (US-003)
- [ ] Card 1 — "Throughput": animated bar chart, AllSource vs Kafka, numbers from config
- [ ] Card 2 — "Vector Search: Built-in, 0 setup": comparison text vs Redis (from config)
- [ ] Card 3 — "No Glue: 1 service vs 3": simple diagram showing AllSource icon vs Kafka + Pinecone + ETL icons
- [ ] Bars animate on first render (count-up animation)
- [ ] Responsive: cards stack vertically on smaller screens
- [ ] Verify in browser using dev-browser skill
- [ ] Playwright test: verify three comparison cards render with data from config endpoint

Mark each item [x] as you complete it. Only close when all are checked.

### US-010: MCP Showdown split-screen layout [UI]
**Description:** As a user, I want a side-by-side view comparing Raw Mode vs MCP Mode so I can see the difference visually.

**Acceptance Criteria:**
- [ ] Full-width view when "MCP Showdown" toggle is active
- [ ] 50/50 horizontal split: left = "Raw Mode", right = "MCP Mode"
- [ ] Each side has a header label and distinct color accent (e.g., orange for raw, green for MCP)
- [ ] Shared "Start Test" button at top that triggers 10k events/sec ingest on both sides
- [ ] Both sides show: tokens used, latency, cost per million events — live updating
- [ ] Verify in browser using dev-browser skill
- [ ] Playwright test: toggle to MCP Showdown, verify split-screen renders

Mark each item [x] as you complete it. Only close when all are checked.

### US-011: MCP Showdown live metrics + progress bars [UI]
**Description:** As a user watching the showdown, I want dual progress bars showing how raw mode spikes under load while MCP flatlines.

**Acceptance Criteria:**
- [ ] Dual animated progress/line charts: one per side (raw vs MCP)
- [ ] Raw side shows latency spikes under load (real data from Core direct)
- [ ] MCP side shows flat latency (real data from MCP-routed queries)
- [ ] Metrics displayed per side: tokens used (e.g., 2.5k vs 250), latency (120ms vs 18ms), cost
- [ ] Numbers update in real-time during the test run
- [ ] Test completes after a defined duration (30s) with summary stats
- [ ] Verify in browser using dev-browser skill
- [ ] Playwright test: start test, verify metrics update in both panels

Mark each item [x] as you complete it. Only close when all are checked.

### US-012: Speed Race overlay [UI]
**Description:** As a user, I want a dramatic race animation comparing query speed between raw and MCP modes.

**Acceptance Criteria:**
- [ ] "Speed Race" button integrated in the MCP Showdown view
- [ ] On click: fires identical query ("Find 'cat videos' last week") on both paths
- [ ] Race animation: two horizontal bars fill from left to right — raw slower, MCP faster
- [ ] Bars show actual measured latency (e.g., raw 47ms, MCP 9ms)
- [ ] Completion label: "Tokens slashed. Speed 5x. No code tweaks."
- [ ] Can be re-run with different queries
- [ ] Verify in browser using dev-browser skill
- [ ] Playwright test: click speed race, verify both bars animate and show latency

Mark each item [x] as you complete it. Only close when all are checked.

### US-013: Cost Calculator widget [UI]
**Description:** As a user evaluating costs, I want a calculator that shows how much I'd save with MCP based on my expected volume.

**Acceptance Criteria:**
- [ ] Bottom section (20% height) of MCP Showdown view, full width
- [ ] Slider input: "Events/day" (range 1M–100M, logarithmic scale)
- [ ] Slider input: "Vectors/event" (range 1–10k)
- [ ] Output cards: "Raw: $X/month tokens" vs "MCP: $Y/month (Z% savings)"
- [ ] Cost formulas driven by benchmark config endpoint (US-003) — not hardcoded
- [ ] Explainer bullet points: why MCP saves (caching, pruning, batching)
- [ ] Updates instantly as sliders move (no submit button)
- [ ] Verify in browser using dev-browser skill
- [ ] Playwright test: move sliders, verify cost outputs update

Mark each item [x] as you complete it. Only close when all are checked.

### US-014: Onboarding Wizard — page shell + step navigation [UI]
**Description:** As a user who just saw the demo and clicked "Build Your Own", I want a guided wizard that walks me through setting up AllSource.

**Acceptance Criteria:**
- [ ] New route: `/onboarding` in `apps/web/src/app/onboarding/page.tsx`
- [ ] "Build Your Own" CTA button in Demo Zone links to `/onboarding`
- [ ] Step indicator at top (Step 1 of 4, Step 2 of 4, etc.)
- [ ] Steps: 1) Choose SDK, 2) Install, 3) Send First Event, 4) Query It Back
- [ ] Back/Next navigation between steps
- [ ] Progress persists in URL params (`?step=2`)
- [ ] Verify in browser using dev-browser skill
- [ ] Playwright test: navigate to `/onboarding`, verify step indicator and navigation

Mark each item [x] as you complete it. Only close when all are checked.

### US-015: Onboarding Wizard — SDK selection + install instructions [UI]
**Description:** As a developer, I want to choose my language and see copy-paste install commands.

**Acceptance Criteria:**
- [ ] Step 1: language selector cards (Rust, Go, TypeScript, Python) with icons
- [ ] Selecting a language advances to Step 2 and persists choice in state
- [ ] Step 2: install command for selected SDK (e.g., `npm install @allsource/client`)
- [ ] Commands include registry config (pointing to `registry.all-source.xyz`)
- [ ] Copy-to-clipboard button on each command block
- [ ] Verify in browser using dev-browser skill
- [ ] Playwright test: select TypeScript, verify install command renders with correct package name

Mark each item [x] as you complete it. Only close when all are checked.

### US-016: Onboarding Wizard — send first event + query it back [UI]
**Description:** As a developer, I want to see working code snippets for sending an event and querying it, so I can copy-paste into my project.

**Acceptance Criteria:**
- [ ] Step 3: "Send First Event" — code snippet in selected language showing event creation
- [ ] Code snippet uses the correct SDK API (`client.create_event(...)`)
- [ ] "Run It" button that actually sends the event via the user's API key to their Core instance
- [ ] Success feedback: green checkmark, "Event created! ID: xyz"
- [ ] Step 4: "Query It Back" — code snippet showing how to query the event just created
- [ ] "Try It" button that runs the query and shows the result inline
- [ ] "Go to Dashboard" CTA at the end
- [ ] Verify in browser using dev-browser skill
- [ ] Playwright test: verify code snippets render for selected language, verify CTA links to dashboard

Mark each item [x] as you complete it. Only close when all are checked.

### US-017: Responsive layout + accessibility [UI]
**Description:** As a mobile user or keyboard-only user, I want the Demo Zone to be usable on all screen sizes and accessible.

**Acceptance Criteria:**
- [ ] Live Fire panels stack vertically on screens < 1024px wide
- [ ] MCP Showdown split stacks vertically on screens < 768px
- [ ] Cost calculator sliders work on touch devices
- [ ] All interactive elements have ARIA labels
- [ ] Keyboard navigation works for: view toggle, query playground, sliders, wizard steps
- [ ] Tab order is logical (left-to-right, top-to-bottom)
- [ ] Verify in browser using dev-browser skill (test at 375px and 1440px widths)
- [ ] Playwright test: verify layout changes at mobile breakpoint

Mark each item [x] as you complete it. Only close when all are checked.

### US-018: Feedback widget [UI]
**Description:** As a product team, I want in-app thumbs up/down feedback so we can measure demo effectiveness.

**Acceptance Criteria:**
- [ ] Floating feedback widget at bottom-right of Demo Zone: "Did this show the power?"
- [ ] Thumbs up / thumbs down buttons
- [ ] On click: sends feedback to QS endpoint (POST `/api/v1/feedback`)
- [ ] Shows "Thanks!" confirmation and hides widget for the session
- [ ] Feedback stored (can be a simple Core event with type "demo_feedback")
- [ ] Verify in browser using dev-browser skill
- [ ] Playwright test: click thumbs up, verify confirmation appears

Mark each item [x] as you complete it. Only close when all are checked.

## Functional Requirements

- FR-1: All demo data queries must go through the Query Service (reads via QS, never direct to Core from frontend)
- FR-2: WebSocket connections for real-time streaming must connect via Core's WebSocket endpoint
- FR-3: Demo Zone must auto-detect empty databases and offer one-click seeding
- FR-4: All latency numbers displayed must be real measured values, not simulated
- FR-5: MCP Showdown must route "Raw Mode" directly to Core and "MCP Mode" through the control plane
- FR-6: Cost calculator formulas must be configurable via the benchmark config endpoint
- FR-7: Comparison metrics (Kafka, Redis numbers) must be served from QS config, not hardcoded
- FR-8: Demo Zone must cap ingest at 50k events/sec to prevent abuse
- FR-9: All queries must complete in <100ms
- FR-10: Onboarding wizard code snippets must be accurate for all four SDK languages

## Non-Goals (Out of Scope)

- Authentication changes — Demo Zone uses existing auth
- Write operations from Demo Zone (except seeding and feedback) — read-only experience
- Custom demo scenarios or user-configurable demo parameters
- Video recording or sharing of demo sessions
- A/B testing framework for demo variants
- Multi-language i18n for demo text

## Technical Considerations

- **WebSocket**: Core exposes a WebSocket endpoint; QS may need to proxy or the frontend connects directly for streaming
- **Recharts**: Use for all bar charts, progress bars, and animated visualizations (already in Next.js ecosystem)
- **Tailwind**: Green/blue/yellow color scheme per the spec; extend existing Tailwind config
- **State management**: URL params for view toggle and wizard steps; React state for real-time metrics
- **Rate limiting**: Demo seed endpoint should be rate-limited (1 call per minute per tenant)
- **Existing patterns**: Follow `apps/web/src/app/` routing conventions and existing component patterns in `packages/ui/`

## Success Metrics

- 80% of new users spend >1 min in Demo Zone (tracked via page time analytics)
- 30% click "Build Your Own" after demo (tracked via CTA click events)
- Positive feedback ratio >70% on thumbs up/down widget
- <100ms p95 latency on all demo queries
- Zero demo failures from empty-DB edge case (seed endpoint works reliably)

## Open Questions

- Should the onboarding wizard include API key generation, or assume the user already has one from sign-up?
- What is the exact WebSocket endpoint path on Core for event streaming?
- Should the "Build Your Own" wizard support team onboarding (multiple developers) or just individual setup?
- Do we need analytics/telemetry beyond the feedback widget (e.g., Mixpanel, PostHog)?
