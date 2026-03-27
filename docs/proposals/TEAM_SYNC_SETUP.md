# Team Sync Setup — Priority Assessment

**Goal:** Enable the team to share chronis tasks via the hosted AllSource instance, with the web dashboard for visualization.

**Date:** 2026-03-27

## Current State

| Component | Status | Version | Notes |
|-----------|--------|---------|-------|
| AllSource Core (Fly.io) | Running | **v0.13.0** (stale) | Needs deploy to v0.17.2 |
| Query Service (Fly.io) | Running | v0.14.8 | Needs deploy to v0.17.2 |
| Web Dashboard (Vercel) | Running | latest | Deployed on every push |
| Control Plane (Fly.io) | Auto-stop | v0.17.1 | Has OAuth + tenant management |
| Chronis CLI | Published | v0.6.2 | Has sync + Prime features |

## What's Needed — Rock / Sand / Water

### Rocks (must do, blocking)

| # | Task | Effort | Why |
|---|------|--------|-----|
| R1 | **Deploy Core v0.17.2 to Fly.io** | 15 min | Core is at v0.13.0 — missing Prime, MCP auth fix, all v0.17 features |
| R2 | **Deploy Query Service v0.17.2 to Fly.io** | 15 min | Needs to match Core version for API compat |
| R3 | **Create team tenant + API key on hosted Core** | 5 min | `cn sync` needs an API key to authenticate |
| R4 | **Configure chronis sync for each team member** | 5 min/person | `.chronis/config.toml` with remote_url + api_key |
| R5 | **Test end-to-end: create task → sync → verify on dashboard** | 15 min | Prove the flow works before sharing |

### Sand (should do, improves experience)

| # | Task | Effort | Why |
|---|------|--------|-----|
| S1 | **`cn init --remote` command** | 1 hour | Auto-configures sync during workspace init |
| S2 | **Web dashboard task view** | 2 hours | Show chronis tasks in the dashboard (query Core events) |
| S3 | **Document the team setup flow** | 30 min | Step-by-step guide for team onboarding |

### Water (nice to have)

| # | Task | Effort | Why |
|---|------|--------|-----|
| W1 | Real-time task updates via WebSocket | 4 hours | Dashboard auto-refreshes on sync |
| W2 | Per-user task filtering | 2 hours | Filter by `claimed_by` agent ID |
| W3 | Task graph visualization in dashboard | 4 hours | Visual dependency graph |

## Execution Plan (Rocks only — ~1 hour total)

### Step 1: Deploy Core v0.17.2

```bash
cd apps/core
fly deploy --config apps/core/fly.toml --dockerfile apps/core/Dockerfile
```

### Step 2: Deploy Query Service v0.17.2

```bash
cd apps/query-service
fly deploy
```

### Step 3: Create API key for team sync

```bash
# Use the bootstrap API key to create a team-specific key
curl -X POST https://allsource-core.fly.dev/api/v1/auth/api-keys \
  -H "Authorization: Bearer $ALLSOURCE_BOOTSTRAP_API_KEY" \
  -H "Content-Type: application/json" \
  -d '{"name": "chronis-team-sync", "tenant_id": "default"}'
```

### Step 4: Configure chronis for each team member

Each team member adds to `.chronis/config.toml`:

```toml
[sync]
remote_url = "https://allsource-core.fly.dev"
api_key = "<team-api-key>"
```

### Step 5: Test the flow

```bash
cn init
cn task create "Test sync" -p p1
cn sync --toon
# Verify on dashboard: https://www.all-source.xyz/dashboard/events
```

## Risks

- Core v0.13.0 → v0.17.2 is a big jump. WAL format is backward-compatible but projections will rebuild on restart. Existing data (if any) is preserved.
- The web dashboard doesn't have a dedicated chronis task view yet — tasks show as raw events in the events page.
