# AllSource Soft Launch Checklist

**Target:** ProductHunt launch Day 7-10
**Tracking:** `br show allsource-monorepo-2ie`

---

## Phase 0: Blockers (Do First)

| Task | Bead ID | Status |
|------|---------|--------|
| Register Google OAuth App | 2ie.1 | ○ |
| Register GitHub OAuth App | 2ie.2 | ○ |
| Create fly.toml configs | 2ie.22 | ○ |
| Create Fly.io Postgres + migrations | 2ie.3 | ○ |
| Deploy all services to Fly.io | 2ie.4 | ○ |

**Commands:**
```bash
# Google OAuth: https://console.cloud.google.com/apis/credentials
# Callback: https://allsource.allsource.dev/api/auth/google/callback

# GitHub OAuth: https://github.com/settings/developers
# Callback: https://allsource.allsource.dev/api/auth/github/callback

# Fly.io setup
fly apps create allsource-core
fly apps create allsource-control-plane
fly apps create allsource-query-service
fly apps create allsource-web

# Create Postgres
fly postgres create --name allsource-db
fly postgres attach allsource-db --app allsource-query-service

# Set secrets (for each app)
fly secrets set GOOGLE_CLIENT_ID=xxx GOOGLE_CLIENT_SECRET=xxx --app allsource-query-service
fly secrets set GITHUB_CLIENT_ID=xxx GITHUB_CLIENT_SECRET=xxx --app allsource-query-service
fly secrets set SECRET_KEY_BASE=$(mix phx.gen.secret) --app allsource-query-service

# Deploy
fly deploy --app allsource-core
fly deploy --app allsource-query-service
fly deploy --app allsource-web

# Run migrations
fly ssh console -a allsource-query-service -C "/app/bin/query_service_ex eval 'QueryServiceEx.Release.migrate()'"
```

---

## Phase 1: Day 1-2 (Soft Launch)

| Task | Bead ID | Status |
|------|---------|--------|
| Verify OAuth login end-to-end | 2ie.5 | ○ |
| Add "Early Access" banner | 2ie.6 | ○ |
| Post X.com launch thread | 2ie.7 | ○ |
| Monitor signups, fix issues | 2ie.8 | ○ |

**X.com Thread:** See `docs/launch/MARKETING_MATERIALS.md`

---

## Phase 2: Day 3-5 (Backend Integration)

| Task | Bead ID | Status |
|------|---------|--------|
| Connect dashboard to real API | 2ie.9 | ○ |
| Connect event explorer to real data | 2ie.10 | ○ |
| Connect WebSocket live feed | 2ie.11 | ○ |
| Test LemonSqueezy checkout | 2ie.12 | ○ |
| E2E test: signup → event → explorer | 2ie.13 | ○ |

---

## Phase 3: Day 7-10 (ProductHunt)

| Task | Bead ID | Status |
|------|---------|--------|
| Record 60s demo video | 2ie.14 | ○ |
| Create hero screenshot | 2ie.15 | ○ |
| Create feature GIFs (3-5) | 2ie.16 | ○ |
| Draft ProductHunt listing | 2ie.17 | ○ |
| Schedule launch (Tue-Thu) | 2ie.18 | ○ |
| Write maker comment | 2ie.19 | ○ |

**ProductHunt Listing:** See `docs/launch/MARKETING_MATERIALS.md`

---

## Phase 4: Day 14+ (Post-Launch)

| Task | Bead ID | Status |
|------|---------|--------|
| Collect user testimonials | 2ie.20 | ○ |
| Draft Show HN post | 2ie.21 | ○ |

---

## Quick Commands

```bash
# View all soft launch tasks
br list --parent allsource-monorepo-2ie

# Start a task
br update allsource-monorepo-2ie.1 -s in_progress

# Complete a task
br close allsource-monorepo-2ie.1

# View ready tasks (unblocked)
br ready
```

---

## Environment Variables (Fly.io Secrets)

```bash
# OAuth (required for launch) - set on allsource-query-service
fly secrets set GOOGLE_CLIENT_ID=xxx --app allsource-query-service
fly secrets set GOOGLE_CLIENT_SECRET=xxx --app allsource-query-service
fly secrets set GITHUB_CLIENT_ID=xxx --app allsource-query-service
fly secrets set GITHUB_CLIENT_SECRET=xxx --app allsource-query-service

# Database - auto-set by fly postgres attach
# DATABASE_URL is automatically injected

# App secrets
fly secrets set SECRET_KEY_BASE=$(mix phx.gen.secret) --app allsource-query-service
fly secrets set PHX_HOST=allsource.allsource.dev --app allsource-query-service

# Billing (for Day 3-5)
fly secrets set LEMON_SQUEEZY_API_KEY=xxx --app allsource-query-service
fly secrets set LEMON_SQUEEZY_STORE_ID=xxx --app allsource-query-service
fly secrets set LEMON_SQUEEZY_WEBHOOK_SECRET=xxx --app allsource-query-service

# Internal service URLs (use Fly.io internal DNS)
fly secrets set RUST_CORE_URL=http://allsource-core.internal:3900 --app allsource-query-service
fly secrets set CORE_WS_URL=ws://allsource-core.internal:3900/api/v1/events/stream --app allsource-query-service
```

---

## Success Metrics

**Day 2:**
- [ ] OAuth login works (both Google and GitHub)
- [ ] At least 5 signups
- [ ] No critical errors in logs

**Day 5:**
- [ ] Real events created by users
- [ ] Events visible in explorer
- [ ] At least one billing checkout tested

**Day 10:**
- [ ] ProductHunt launched
- [ ] Demo video viewed 100+ times
- [ ] 50+ signups total

---

## Files Reference

| File | Purpose |
|------|---------|
| `docs/launch/LAUNCH_READINESS_ASSESSMENT.md` | Technical assessment |
| `docs/launch/MARKETING_MATERIALS.md` | X thread, PH listing, HN post |
| `docs/launch/SOFT_LAUNCH_CHECKLIST.md` | This file |
