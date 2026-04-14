# Chronis Cloud Sync — Launch Plan (x402 + Agent Auth)

Manual setup steps to take chronis from embedded mode to syncing against a hosted AllSource Core, with x402 micropayments and agent registration enabled.

**Branch:** `feat/agent-auth-x402` (x402 + agent endpoints live here; not yet on `main`)
**Target versions:** Core v0.17.2, Query Service v0.17.2, Control Plane latest on branch
**Estimated time:** ~2 hours end-to-end, assuming Fly/Coinbase accounts already exist

---

## Phase 0 — Prerequisites (do these once)

1. **Accounts**
   - Fly.io account with payment method attached (`fly auth login`)
   - Coinbase Developer Platform (CDP) account → create a **server wallet** on Base mainnet (or Base Sepolia for staging). Save the wallet address and API key.
   - GitHub access to `all-source-os/chronos-monorepo`

2. **Local tooling**
   - `flyctl` installed and authenticated
   - `bun` (per repo convention — never npm/pnpm/yarn)
   - Rust toolchain (for building chronis locally if needed)

3. **Branch check out**
   ```bash
   git fetch origin
   git checkout feat/agent-auth-x402
   git pull
   ```
   Confirm these paths exist:
   - `apps/control-plane/internal/infrastructure/x402/`
   - `apps/control-plane/` (has `/api/v1/agents/register`)
   - `apps/core/fly.toml`

---

## Phase 1 — Deploy Core v0.17.2 (15 min)

Hosted app: `allsource-core` on Fly (region `iad`). Persistent volume `allsource_data` already attached; WAL + Parquet survive deploys.

```bash
cd apps/core
fly deploy --config fly.toml --dockerfile Dockerfile
fly status -a allsource-core
curl https://allsource-core.fly.dev/health
```

**Verification:** `/health` returns 200. Check `fly logs -a allsource-core` for a clean WAL recovery line and Parquet restore (no silent failures — #101 was the regression guard).

**Rollback:** `fly releases -a allsource-core` → `fly deploy --image <prior-image>`.

---

## Phase 2 — Deploy Query Service v0.17.2 (15 min)

```bash
cd apps/query-service
fly deploy
```

**Required env vars** (set via `fly secrets set -a <qs-app>`):
- `CORE_URL=https://allsource-core.fly.dev`
- `CORE_WS_URL=wss://allsource-core.fly.dev`
- `DATABASE_URL=postgres://...` (Postgres for users/tenants/API keys — **not** events)
- `SECRET_KEY_BASE=<phx.gen.secret output>`

**Verification:** `/health` returns 200; a test `GET /api/v1/events/query` with a valid API key returns `{"events":[],"count":0}` on an empty tenant.

---

## Phase 3 — Deploy Control Plane with x402 enabled (20 min)

Control Plane owns auth, agent registration, and the x402 middleware. Deploy the `feat/agent-auth-x402` build.

### 3a. Create x402 pricing config

Create `apps/control-plane/config/x402-pricing.json`:
```json
{
  "routes": [
    { "method": "POST", "path": "/api/v1/events",       "price_usd": "0.0001", "asset": "USDC", "network": "base" },
    { "method": "GET",  "path": "/api/v1/events/query", "price_usd": "0.001",  "asset": "USDC", "network": "base" }
  ],
  "free_tier": { "events_per_month": 10000, "queries_per_month": 1000 }
}
```

### 3b. Fly secrets for Control Plane

```bash
fly secrets set -a <control-plane-app> \
  X402_ENABLED=true \
  X402_PRICING_CONFIG=/app/config/x402-pricing.json \
  X402_RECIPIENT_ADDRESS=0xYOUR_CDP_WALLET_ADDRESS \
  X402_FACILITATOR_URL=https://x402.coinbase.com \
  CDP_API_KEY_NAME=<coinbase key name> \
  CDP_API_KEY_PRIVATE_KEY=<coinbase key secret> \
  CORE_URL=https://allsource-core.fly.dev \
  QUERY_SERVICE_URL=https://allsource-query-service.fly.dev \
  DATABASE_URL=postgres://...
```

### 3c. Deploy

```bash
cd apps/control-plane
fly deploy
```

**Verification:**
```bash
# x402 discovery — should list the two priced routes
curl https://<control-plane>.fly.dev/x402/routes

# Hitting a priced route without a payment header should return 402
curl -i https://<control-plane>.fly.dev/api/v1/events/query
# expect: HTTP/1.1 402 Payment Required
```

---

## Phase 4 — Bootstrap tenant + admin API key (5 min)

One-time: mint the first API key so you can provision everything else.

```bash
# SSH into Core machine
fly ssh console -a allsource-core

# Inside: run the bootstrap CLI (ships with Core v0.17.2)
allsource-core bootstrap \
  --tenant-id default \
  --email you@example.com \
  --output-key /tmp/bootstrap.key
cat /tmp/bootstrap.key
```

Save the resulting key as `ALLSOURCE_BOOTSTRAP_API_KEY` in your local shell. This is an admin key — do not commit it.

---

## Phase 5 — Register a team tenant + user API key (5 min)

From your laptop:

```bash
export CP=https://<control-plane>.fly.dev

# Create team tenant
curl -X POST $CP/api/v1/tenants \
  -H "Authorization: Bearer $ALLSOURCE_BOOTSTRAP_API_KEY" \
  -H "Content-Type: application/json" \
  -d '{"name": "my-team", "plan": "free"}'
# → { "tenant_id": "tnt_...", ... }

# Mint a per-user API key for chronis sync
curl -X POST $CP/api/v1/auth/api-keys \
  -H "Authorization: Bearer $ALLSOURCE_BOOTSTRAP_API_KEY" \
  -H "Content-Type: application/json" \
  -d '{"name": "chronis-alice", "tenant_id": "tnt_..."}'
# → { "api_key": "ask_...", ... }
```

Repeat the second call per team member. Keys are per-user so usage/billing attribution works.

---

## Phase 6 — Register an agent (optional, for MCP/SDK consumers) (5 min)

Agents use a single self-service endpoint — no email/password flow.

```bash
curl -X POST $CP/api/v1/agents/register \
  -H "Content-Type: application/json" \
  -d '{
    "agent_name": "my-mcp-agent",
    "agent_type": "mcp",
    "tenant_id": "tnt_...",
    "cdp_wallet_address": "0xAGENT_WALLET"
  }'
# → { "api_key": "ask_agent_...", "core_url": "...", "query_url": "...", "quotas": {...} }
```

The returned `api_key` is what an MCP server, SDK, or CLI agent uses as its `Authorization: Bearer` token. When the agent exceeds the free tier, x402 middleware intercepts the request, the agent's CDP wallet auto-pays via the facilitator, and the request proceeds. Agent payment history:

```bash
curl $CP/api/v1/agents/me/payments -H "Authorization: Bearer ask_agent_..."
```

---

## Phase 7 — Configure chronis on each machine (5 min / person)

Per team member, in their chronis project directory:

```bash
mkdir -p .chronis
cat > .chronis/config.toml <<'EOF'
mode = "remote"

[sync]
remote_url = "https://allsource-core.fly.dev"
api_key = "ask_..."   # from Phase 5 (user) or Phase 6 (agent)
EOF

# First sync: bootstraps local projection cache by replaying remote events
cn sync

# Sanity check
cn list
cn add "verify cloud sync works"
cn list   # new task should appear
```

Add `.chronis/config.toml` to `.gitignore` — it contains a secret.

**Second machine test:** on a teammate's laptop, run the same config (with their own api_key under the same tenant) and confirm `cn list` shows the task created above. This proves the round-trip.

---

## Phase 8 — Smoke tests (15 min)

1. **Core health:** `curl https://allsource-core.fly.dev/health` → 200
2. **Query Service health:** `curl https://<qs>.fly.dev/health` → 200
3. **Control Plane x402 discovery:** `curl $CP/x402/routes` → priced routes listed
4. **Free-tier request:** chronis `cn add` + `cn sync` succeeds without payment
5. **402 path:** hit `/api/v1/events/query` on Control Plane without auth → 402 with payment instructions
6. **Agent auto-pay (staging):** point an agent at Base Sepolia, blow past the free tier, verify `/agents/me/payments` shows a settled payment
7. **Dashboard:** open the Vercel web dashboard, sign in, confirm events from your tenant are visible (tenant isolation working — Gap 1 fix)

---

## Known gaps at launch (from `SELF_SERVICE_ONBOARDING.md`)

These are **not** blockers for a controlled team launch, but you should know them:

| Gap | Status | Impact |
|---|---|---|
| Tenant isolation on signup | Fixed (commit 7395a1a) | Fresh users only see their own events |
| Sync pagination | Fixed (commit 7395a1a) | First sync no longer 502s on large tenants |
| Unified auth (OAuth ↔ Core API key) | **Open** | Dashboard users can't self-mint Core API keys; you provision manually via Phase 5 |
| LemonSqueezy quota enforcement | **Open** | Free-tier → paid-tier upgrade doesn't lift x402 automatically; manual tenant plan bump for now |

---

## Operational checklist after launch

- [ ] Set Fly autoscale min=1 on Core (cold starts corrupt chronis sync UX)
- [ ] Configure Fly alerts on Core `/health` failures
- [ ] Back up the Postgres used by Query Service (users/tenants/keys — losing this is painful)
- [ ] Rotate `ALLSOURCE_BOOTSTRAP_API_KEY` and store in 1Password
- [ ] Document the Control Plane app name + region in `docs/current/` so the next operator isn't guessing
- [ ] Open PRs to land `feat/agent-auth-x402` → `main` once smoke tests pass in production

---

## Related docs

- `docs/proposals/TEAM_SYNC_SETUP.md` — original R1–R5 rocks this plan extends
- `docs/proposals/AGENT_AUTH_DESIGN.md` — why Option A (agent registration) was chosen
- `docs/proposals/prd-agent-auth-x402.md` — PRD for the x402 + agent auth work
- `docs/proposals/SELF_SERVICE_ONBOARDING.md` — gap analysis
- `docs/proposals/UNIFIED_AUTH_TEAMS.md` — Gap 3 resolution plan
- `apps/chronis/README.md` (lines 189–240) — chronis sync modes
- `apps/chronis/docs/BEST_PRACTICES.md` (lines 44–74) — embedded vs remote
