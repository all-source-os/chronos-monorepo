# AllSource Launch Checklist

**Last reviewed:** 2026-04-14
**Current versions:** Core v0.18.2 · Query Service v0.18.2 · Web v0.18.2
**Active branches:** `main` — x402 + agent registration are already merged. The `feat/agent-auth-x402` branch is historical.

Single source of truth for what's left to launch. Supersedes the 6 archived docs under `docs/launch/archived/`. Pair with `docs/launch/CHRONIS_CLOUD_LAUNCH_PLAN.md` for the chronis cloud sync launch specifically.

Status legend: `[ ]` open · `[~]` in progress · `[x]` done · `[-]` dropped/deferred

---

## Phase A — Infrastructure & Credentials

- [x] Fly apps exist and are running: `allsource-core`, `allsource-query`, `allsource-control-plane`, `allsource-web`, `allsource-prime`, `allsource-auth`, `allsource-registry` (all `started`, health checks passing, region `iad`). Note: Query Service app is named `allsource-query`, not `allsource-query-service`
- [ ] Register Google OAuth app → save `GOOGLE_CLIENT_ID` / `GOOGLE_CLIENT_SECRET`
  - Callback: `https://all-source.xyz/api/auth/google/callback`
- [ ] Register GitHub OAuth app → save `GITHUB_CLIENT_ID` / `GITHUB_CLIENT_SECRET`
  - Callback: `https://all-source.xyz/api/auth/github/callback`
- [ ] LemonSqueezy: API key, store ID, webhook secret (needed for paid-tier upgrade flow)
- [ ] Coinbase CDP server wallet on Base (mainnet for launch, Sepolia for staging) — for x402 payouts
- [ ] Generate `SECRET_KEY_BASE` (`mix phx.gen.secret`) and `ALLSOURCE_JWT_SECRET` (`openssl rand -hex 32`) — only if not already set as Fly secrets
- [ ] Custom domain: point `all-source.xyz` at Fly (web app) + DNS for `api.all-source.xyz` → query service

## Phase B — Deploy core stack

All services already deployed and healthy on Fly (iad). Current deployments are **stale vs v0.18.2** — redeploy before launch to pick up the latest event-store, web, and x402 changes.

- [x] `allsource-core` — deployed Mar 29, healthy 1/1 · **stale (~17d)**, redeploy to pick up v0.18.2
- [x] `allsource-query` — deployed Mar 26, healthy 2/2 · **stale (~20d)**, redeploy to v0.18.2
- [x] `allsource-control-plane` — deployed Apr 14, healthy 1/1 · **redeploy required** to ship today's `GET /x402/routes` discovery endpoint (`bd8e97d`)
- [x] `allsource-web` — deployed Mar 7, healthy 1/1 (+ 1 stopped spare) · **redeploy required** for new `/use-cases`, `/compare/eventstoredb`, and live-metrics fix (`bd8e97d`)
- [x] `allsource-prime` — deployed Mar 23, healthy 1/1 · stale, optional redeploy
- [x] `allsource-auth` — deployed Mar 24, 2 machines healthy · stale, optional redeploy
- [x] `allsource-registry` — deployed Mar 1, healthy 1/1 · stale, optional redeploy
- [ ] **Redeploy `allsource-control-plane`** to ship x402 routes discovery
- [ ] **Redeploy `allsource-web`** to ship new marketing pages
- [ ] Redeploy `allsource-core` + `allsource-query` to v0.18.2 for version parity
- [ ] Verify autoscale min=1 on Core (cold starts break chronis sync UX)
- [ ] Fly alerts on Core/Query `/health` failures

### Required secrets (reference)

**allsource-core**: `ALLSOURCE_JWT_SECRET`

**allsource-query-service**: `SECRET_KEY_BASE`, `PHX_HOST=all-source.xyz`, `CORE_URL=http://allsource-core.internal:3900`, `CORE_WS_URL=ws://allsource-core.internal:3900/api/v1/events/stream`, `GOOGLE_CLIENT_*`, `GITHUB_CLIENT_*`, `LEMON_SQUEEZY_API_KEY`, `LEMON_SQUEEZY_STORE_ID`, `LEMON_SQUEEZY_WEBHOOK_SECRET`

**allsource-control-plane**: `JWT_SECRET`, `CORE_URL=http://allsource-core.internal:3900`, `QUERY_SERVICE_URL=http://allsource-query-service.internal:3902`, `FRONTEND_URL=https://all-source.xyz`, `X402_ENABLED=true`, `X402_PRICING_CONFIG=/app/config/x402-pricing.json`, `X402_RECIPIENT_ADDRESS`, `X402_FACILITATOR_URL=https://x402.coinbase.com`, `CDP_API_KEY_NAME`, `CDP_API_KEY_PRIVATE_KEY`

**allsource-web**: `NEXT_PUBLIC_API_URL=https://api.all-source.xyz`

## Phase C — x402 & agent auth

- [x] x402 + agent registration merged to main (`apps/control-plane/internal/infrastructure/x402/`)
- [x] Pricing config exists at `apps/control-plane/docs/x402-pricing.example.json` — copy to deploy path and point `X402_PRICING_CONFIG` at it
- [x] Agent registration handler `POST /api/v1/agents/register` (`main.go:330`, `agents.go:14-42`)
- [x] Agent payment history `GET /api/v1/agents/me/payments` (`main.go:335`)
- [x] `GET /x402/routes` discovery endpoint added (`handler.go` Routes handler, `main.go:341`)
- [ ] Verify 402 flow against deployed Control Plane: unauth request to priced route → `402 Payment Required`
- [ ] Staging auto-pay test: blow past free tier on Base Sepolia, confirm `/api/v1/agents/me/payments` shows a settled payment

## Phase D — Tenant bootstrap & chronis sync

- [ ] Bootstrap admin key via Fly secrets (Core reads `ALLSOURCE_BOOTSTRAP_TENANT` and `ALLSOURCE_BOOTSTRAP_API_KEY` at startup — there is **no** `bootstrap` subcommand): `fly secrets set ALLSOURCE_BOOTSTRAP_TENANT=default ALLSOURCE_BOOTSTRAP_API_KEY=$(openssl rand -hex 32) -a allsource-core`
- [ ] Store bootstrap key in 1Password (not git); rotate after first real tenant exists
- [ ] Create team tenant + per-user API keys via `POST /api/v1/tenants` and `POST /api/v1/auth/api-keys`
- [ ] Each team member: populate `.chronis/config.toml` (`mode = "remote"`, `remote_url`, `api_key`) — see `CHRONIS_CLOUD_LAUNCH_PLAN.md`
- [ ] End-to-end: Alice `cn add` → Bob `cn sync` → Bob `cn list` shows it

## Phase E — Backend data integration

- [x] **Unified auth shipped**: OAuth signup auto-provisions a Core API key via `provisionCoreAPIKey()` (`apps/control-plane/.../auth.go:394-400`, `439-469`); key included in JWT claims
- [x] **LemonSqueezy webhook → tenant plan**: `webhook_lemonsqueezy.go:85-124` processes subscription events; `handleSubscriptionUpdated` (line 146) updates tenant tier + quotas via `updateSubUC.Execute()`
- [x] Dashboard stats wired (`apps/web/src/hooks/use-dashboard-stats.ts` — Promise.all over `getTenantUsage`/`listProjections`/`getMetrics`)
- [x] Live event feed uses Phoenix Channel WebSocket (`live-event-feed.tsx:31` → `usePhoenixChannel("events:all", ...)`)
- [x] Event explorer fetches real data via `useEvents()` — no demo-data fallback path
- [x] `live-metrics.tsx` magic-number fallbacks dropped — metric fields are now nullable, UI renders `—` when real metrics are unavailable
- [ ] End-to-end smoke test the unified auth + billing path against deployed stack (code is in, behavior unverified in production)

## Phase F — Product polish (post-launch OK)

- [x] Settings page (`apps/web/src/app/dashboard/settings/page.tsx`: profile, security, notifications)
- [x] Projections/Pipelines page (`pipelines/page.tsx` — fetches via `apiClient.listProjections()`, status cards, pause/resume controls)
- [x] Privacy Policy page (`apps/web/src/app/(marketing)/privacy/page.tsx`)
- [x] Terms of Service page (`apps/web/src/app/(marketing)/terms/page.tsx`)
- [ ] SLO definition: latency p99, uptime %, error rate — pre-req for SLA monitoring
- [ ] SLA monitoring + alerting (PagerDuty/Slack) — was `REMAINING_TASKS.md` P0-003

## Phase G — Positioning & messaging (web app copy)

- [x] Hero subtitle "Time-travel your data" with benchmarks (`hero.tsx:150-152`, `186-191`: 469K events/sec, 11.9μs)
- [x] Benchmarks component on landing (`HeroStats()` in `hero.tsx`, count-up animation)
- [x] MCP tools showcase (`hero.tsx:189` "27 MCP tools"; mentioned in `features.tsx`)
- [x] "database" → "event store" audit: only one user-facing fix needed (`dashboard/demo/page.tsx` "your database" → "your event store"); rest of the copy already uses contrastive framing correctly
- [x] Use cases page (`apps/web/src/app/(marketing)/use-cases/page.tsx` — audit trails, event replay, AI agent memory, financial history)
- [x] AllSource vs EventStoreDB comparison page (`apps/web/src/app/(marketing)/compare/eventstoredb/page.tsx`)
- [ ] Pricing page review: current tiers (`config.ts:105-162`) are Developer (free, 100K events) / Team ($99 or $79 yearly, 10M events) / Enterprise. **Differs** from the Turso plan recommendation (Free 50K / Pro $29 / Team $79 / Scale $199). Decide which to keep

## Phase H — Launch marketing assets

- [ ] Record 60s demo video (OAuth login → dashboard → event explorer → time-travel query → CTA)
- [ ] Hero screenshot 1270×760 dark mode
- [ ] 3–5 feature GIFs: event explorer search, live stream, API key creation, onboarding
- [ ] Draft ProductHunt listing — see `archived/MARKETING_MATERIALS.md`
- [ ] Draft X.com launch thread — see `archived/MARKETING_MATERIALS.md`
- [ ] Draft Show HN post (after 2–3 testimonials collected)
- [ ] Line up 5–10 upvoters for ProductHunt launch day

## Phase I — Launch execution

- [ ] Day 1: Deploy + smoke test + OAuth verification + "Early Access" banner
- [ ] Day 2: X.com thread, monitor signups
- [ ] Day 3–5: Billing checkout tested end-to-end against LemonSqueezy
- [ ] Day 7–10: ProductHunt launch (Tue–Thu, schedule 12:01 AM PT, first 2h engagement)
- [ ] Day 14+: Collect testimonials, post Show HN
- [ ] Success metrics: 5 signups day 2 · 50 signups day 10 · demo video 100+ views

## Phase J — Deferred / nice-to-have

- [-] SALES-001 Export C4 diagrams to PNG/SVG (P3)
- [-] SALES-002 Generate performance charts via Python/plotly (P3)
- [-] SALES-003 Convert pitch deck to presentation format (P3, blocked by SALES-002)
- [-] Interactive time-travel query playground (P1 Turso plan, month 2)
- [-] Early adopter case studies (month 2+)

## Phase K — SDKs

SDKs already exist in `sdks/` (go, python-client, rust, typescript). Per memory: **only Rust crates publish to crates.io**. JS/Python/Go SDKs distribute via GitHub registry.

- [x] SDK READMEs reference `https://api.all-source.xyz` (TS fixed from `allsource-query.fly.dev`, Go and Python already correct)
- [x] TypeScript SDK README — Authentication section + quickstart + API reference + error handling
- [x] Python SDK README — Authentication section + sync/async quickstart + API reference
- [x] Go SDK README — Authentication section + quickstart + error handling + query options
- [ ] Sample apps (JS/Python/Go) — deferred, docs cover the essentials

---

## Known gaps at launch

| Gap | Impact | Workaround |
|---|---|---|
| Pricing tiers differ from Turso plan recommendation | Marketing/positioning question | Decide tier strategy before launch day |

---

## Doc map

- `LAUNCH_CHECKLIST.md` — this file, single checklist
- `CHRONIS_CLOUD_LAUNCH_PLAN.md` — detailed runbook for chronis → cloud sync with x402
- `archived/` — superseded planning docs (marketing copy drafts in `MARKETING_MATERIALS.md` still usable)
