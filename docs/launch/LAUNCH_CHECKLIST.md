# AllSource Launch Checklist

**Last reviewed:** 2026-04-14
**Current versions:** Core v0.18.1 · Query Service v0.18.1 · Web v0.18.1
**Active branches:** `main` (general), `feat/agent-auth-x402` (x402 + agent registration)

Single source of truth for what's left to launch. Supersedes the 6 archived docs under `docs/launch/archived/`. Pair with `docs/launch/CHRONIS_CLOUD_LAUNCH_PLAN.md` for the chronis cloud sync launch specifically.

Status legend: `[ ]` open · `[~]` in progress · `[x]` done · `[-]` dropped/deferred

---

## Phase A — Infrastructure & Credentials (blocker)

- [ ] Create Fly apps (verify `fly apps list`): `allsource-core`, `allsource-query-service`, `allsource-web`, `allsource-control-plane`
- [ ] Register Google OAuth app → save `GOOGLE_CLIENT_ID` / `GOOGLE_CLIENT_SECRET`
  - Callback: `https://all-source.xyz/api/auth/google/callback`
- [ ] Register GitHub OAuth app → save `GITHUB_CLIENT_ID` / `GITHUB_CLIENT_SECRET`
  - Callback: `https://all-source.xyz/api/auth/github/callback`
- [ ] LemonSqueezy: API key, store ID, webhook secret (needed for paid-tier upgrade flow)
- [ ] Coinbase CDP server wallet on Base (mainnet for launch, Sepolia for staging) — for x402 payouts
- [ ] Generate `SECRET_KEY_BASE` (`mix phx.gen.secret`) and `ALLSOURCE_JWT_SECRET` (`openssl rand -hex 32`)
- [ ] Custom domain: point `all-source.xyz` at Fly (web app) + DNS for `api.all-source.xyz` → query-service

## Phase B — Deploy core stack (blocker)

- [ ] `fly deploy -a allsource-core` from `apps/core/` — verify `/health` 200, WAL replay clean in logs
- [ ] `fly deploy -a allsource-query-service` from `apps/query-service/` — secrets below set first
- [ ] `fly deploy -a allsource-control-plane` from `apps/control-plane/` — from `feat/agent-auth-x402` branch
- [ ] `fly deploy -c apps/web/fly.toml --dockerfile apps/web/Dockerfile` from repo root
- [ ] Autoscale min=1 on Core (cold starts break chronis sync UX)
- [ ] Fly alerts on Core/QS `/health` failures

### Required secrets (reference)

**allsource-core**: `ALLSOURCE_JWT_SECRET`

**allsource-query-service**: `SECRET_KEY_BASE`, `PHX_HOST=all-source.xyz`, `CORE_URL=http://allsource-core.internal:3900`, `CORE_WS_URL=ws://allsource-core.internal:3900/api/v1/events/stream`, `GOOGLE_CLIENT_*`, `GITHUB_CLIENT_*`, `LEMON_SQUEEZY_API_KEY`, `LEMON_SQUEEZY_STORE_ID`, `LEMON_SQUEEZY_WEBHOOK_SECRET`

**allsource-control-plane**: `JWT_SECRET`, `CORE_URL=http://allsource-core.internal:3900`, `QUERY_SERVICE_URL=http://allsource-query-service.internal:3902`, `FRONTEND_URL=https://all-source.xyz`, `X402_ENABLED=true`, `X402_PRICING_CONFIG=/app/config/x402-pricing.json`, `X402_RECIPIENT_ADDRESS`, `X402_FACILITATOR_URL=https://x402.coinbase.com`, `CDP_API_KEY_NAME`, `CDP_API_KEY_PRIVATE_KEY`

**allsource-web**: `NEXT_PUBLIC_API_URL=https://api.all-source.xyz`

## Phase C — x402 & agent auth (blocker for agent use case)

- [ ] Land `feat/agent-auth-x402` → `main` once smoke-tested in staging
- [ ] Commit `apps/control-plane/config/x402-pricing.json` (routes: `POST /api/v1/events` $0.0001, `GET /api/v1/events/query` $0.001; free tier 10K/1K)
- [ ] Verify `GET /x402/routes` lists priced routes in production
- [ ] Verify 402 flow: unauth request to priced route → `402 Payment Required` with instructions
- [ ] Register test agent: `POST /api/v1/agents/register` → api_key returned with quotas + core_url + query_url
- [ ] Staging auto-pay test: blow past free tier on Base Sepolia, confirm `/api/v1/agents/me/payments` shows a settled payment

## Phase D — Tenant bootstrap & chronis sync

- [ ] Bootstrap admin key via `fly ssh console -a allsource-core` → `allsource-core bootstrap --tenant-id default --email you@example.com`
- [ ] Store bootstrap key in 1Password (not git)
- [ ] Create team tenant + per-user API keys via `POST /api/v1/tenants` and `POST /api/v1/auth/api-keys`
- [ ] Each team member: populate `.chronis/config.toml` (`mode = "remote"`, `remote_url`, `api_key`) — see `CHRONIS_CLOUD_LAUNCH_PLAN.md`
- [ ] End-to-end: Alice `cn add` → Bob `cn sync` → Bob `cn list` shows it

## Phase E — Backend data integration (was Gap 3/4)

- [ ] **Unified auth**: OAuth signup → Control Plane auto-provisions Core API key, surfaces it in dashboard API Keys page (`docs/proposals/UNIFIED_AUTH_TEAMS.md`)
- [ ] **Billing enforcement**: LemonSqueezy webhook → tenant plan bump → x402 middleware bypass for paid tenants
- [ ] Dashboard stats cards fetch from `/api/v1/events/stats` instead of client-side mock (`apps/web/src/components/dashboard/`)
- [ ] Live event feed: replace simulated stream with WebSocket → `CORE_WS_URL` (Query Service already proxies)
- [ ] Event explorer: remove demo-data fallback, show empty state for new tenants

## Phase F — Product polish (post-launch OK)

- [ ] Settings page content (currently skeleton)
- [ ] Projections/Pipelines page content (currently minimal)
- [ ] Privacy Policy + Terms of Service pages
- [ ] SLO definition: latency p99, uptime %, error rate — pre-req for SLA monitoring
- [ ] SLA monitoring + alerting (PagerDuty/Slack) — `REMAINING_TASKS.md` P0-003

## Phase G — Positioning & messaging (web app copy)

From `TURSO_COMPETITIVE_LAUNCH_PLAN.md`. These are **web-app edits**, not infra:

- [ ] Update hero subtitle to "Time-travel your data" with benchmark stats (469K events/sec · 11.9μs · 27 MCP tools)
- [ ] Add benchmarks component on landing
- [ ] Add MCP tools showcase section
- [ ] Replace "database" language with "event store" across landing/marketing pages
- [ ] Use cases page: audit trails, event replay, AI agent memory, financial history
- [ ] AllSource vs **EventStoreDB** comparison page (do NOT build AllSource vs Turso)
- [ ] Pricing page review: Free 50K events, Pro $29, Team $79, Scale $199

## Phase H — Launch marketing assets

- [ ] Record 60s demo video (OAuth login → dashboard → event explorer → time-travel query → CTA)
- [ ] Hero screenshot 1270×760 dark mode
- [ ] 3–5 feature GIFs: event explorer search, live stream, API key creation, onboarding
- [ ] Draft ProductHunt listing (tagline, 3-paragraph description, 5 features) — see archived `MARKETING_MATERIALS.md`
- [ ] Draft X.com launch thread (6 posts) — see archived `MARKETING_MATERIALS.md`
- [ ] Draft Show HN post (after 2–3 testimonials collected)
- [ ] Line up 5–10 upvoters for ProductHunt launch day

## Phase I — Launch execution

- [ ] Day 1: Deploy + smoke test + OAuth verification + "Early Access" banner
- [ ] Day 2: X.com thread, monitor signups
- [ ] Day 3–5: Real data integration live, billing checkout tested
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

- [ ] JS SDK (`@allsource/client`) usage docs + sample app — called out as P0 in Turso plan
- [ ] Python SDK usage docs + sample app — P0 in Turso plan
- [ ] Go SDK usage docs — P1
- [ ] Verify all SDK READMEs reference `api.all-source.xyz` (not localhost)

---

## Known gaps at launch

These ship with the v1 launch and are tracked in Phase E / F:

| Gap | Impact | Workaround |
|---|---|---|
| Unified auth (OAuth ↔ Core API key) | Dashboard users can't self-mint Core API keys | Operators provision manually via Phase D |
| LemonSqueezy → quota enforcement | Paid upgrade doesn't auto-lift x402 gate | Manual tenant plan bump |
| Settings page | Skeleton only | Hide from nav until filled |
| Projections page | Minimal | Hide from nav until filled |

---

## Doc map

- `LAUNCH_CHECKLIST.md` — this file, single checklist
- `CHRONIS_CLOUD_LAUNCH_PLAN.md` — detailed runbook for chronis → cloud sync with x402
- `archived/` — superseded planning docs kept for historical context (marketing copy drafts still usable)
