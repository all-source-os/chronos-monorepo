# AllSource Soft Launch Checklist

**Target:** ProductHunt launch Day 7-10
**Last updated:** 2026-03-01

---

## Phase 0: Blockers (Do First)

| Task | Status |
|------|--------|
| Register Google OAuth App | ○ |
| Register GitHub OAuth App | ○ |
| Create fly.toml configs | ○ |
| Deploy all services to Fly.io | ○ |

**Commands:**
```bash
# Google OAuth: https://console.cloud.google.com/apis/credentials
# Callback: https://all-source.xyz/api/auth/google/callback

# GitHub OAuth: https://github.com/settings/developers
# Callback: https://all-source.xyz/api/auth/github/callback

# Fly.io setup
fly apps create allsource-core
fly apps create allsource-control-plane
fly apps create allsource-query-service
fly apps create allsource-web

# Set secrets — Core
fly secrets set ALLSOURCE_JWT_SECRET=$(openssl rand -hex 32) --app allsource-core

# Set secrets — Control Plane
fly secrets set JWT_SECRET=$(openssl rand -hex 32) --app allsource-control-plane
fly secrets set CORE_URL=http://allsource-core.internal:3900 --app allsource-control-plane
fly secrets set FRONTEND_URL=https://all-source.xyz --app allsource-control-plane

# Set secrets — Query Service (stateless, no database needed)
fly secrets set SECRET_KEY_BASE=$(mix phx.gen.secret) --app allsource-query-service
fly secrets set CORE_URL=http://allsource-core.internal:3900 --app allsource-query-service
fly secrets set CORE_WS_URL=ws://allsource-core.internal:3900/api/v1/events/stream --app allsource-query-service
fly secrets set PHX_HOST=all-source.xyz --app allsource-query-service
fly secrets set GOOGLE_CLIENT_ID=xxx GOOGLE_CLIENT_SECRET=xxx --app allsource-query-service
fly secrets set GITHUB_CLIENT_ID=xxx GITHUB_CLIENT_SECRET=xxx --app allsource-query-service

# Deploy (order matters: Core first, then CP, then QS, then Web)
fly deploy --app allsource-core
fly deploy --app allsource-control-plane
fly deploy --app allsource-query-service
fly deploy --app allsource-web
```

---

## Phase 1: Day 1-2 (Soft Launch)

| Task | Status |
|------|--------|
| Verify OAuth login end-to-end | ○ |
| Add "Early Access" banner | ○ |
| Post X.com launch thread | ○ |
| Monitor signups, fix issues | ○ |

**X.com Thread:** See `docs/launch/MARKETING_MATERIALS.md`

---

## Phase 2: Day 3-5 (Backend Integration)

| Task | Status |
|------|--------|
| Connect dashboard to real API | ○ |
| Connect event explorer to real data | ○ |
| Connect WebSocket live feed | ○ |
| Test LemonSqueezy checkout | ○ |
| E2E test: signup → event → explorer | ○ |

---

## Phase 3: Day 7-10 (ProductHunt)

| Task | Status |
|------|--------|
| Record 60s demo video | ○ |
| Create hero screenshot | ○ |
| Create feature GIFs (3-5) | ○ |
| Draft ProductHunt listing | ○ |
| Schedule launch (Tue-Thu) | ○ |
| Write maker comment | ○ |

**ProductHunt Listing:** See `docs/launch/MARKETING_MATERIALS.md`

---

## Phase 4: Day 14+ (Post-Launch)

| Task | Status |
|------|--------|
| Collect user testimonials | ○ |
| Draft Show HN post | ○ |

---

## Environment Variables (Fly.io Secrets)

```bash
# OAuth (required for launch) - set on allsource-query-service
fly secrets set GOOGLE_CLIENT_ID=xxx --app allsource-query-service
fly secrets set GOOGLE_CLIENT_SECRET=xxx --app allsource-query-service
fly secrets set GITHUB_CLIENT_ID=xxx --app allsource-query-service
fly secrets set GITHUB_CLIENT_SECRET=xxx --app allsource-query-service

# App secrets — Query Service is stateless, no DATABASE_URL needed
fly secrets set SECRET_KEY_BASE=$(mix phx.gen.secret) --app allsource-query-service
fly secrets set PHX_HOST=all-source.xyz --app allsource-query-service

# Billing (for Day 3-5)
fly secrets set LEMON_SQUEEZY_API_KEY=xxx --app allsource-query-service
fly secrets set LEMON_SQUEEZY_STORE_ID=xxx --app allsource-query-service
fly secrets set LEMON_SQUEEZY_WEBHOOK_SECRET=xxx --app allsource-query-service

# Internal service URLs (use Fly.io internal DNS)
fly secrets set CORE_URL=http://allsource-core.internal:3900 --app allsource-query-service
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
| `docs/deployment/DOCKER.md` | Docker image reference |
| `docs/launch/SOFT_LAUNCH_CHECKLIST.md` | This file |
