# Chronos Launch Checklist (Manual Tasks)

**Target:** Minimum viable "users can sign up and explore"
**Estimation:** Rock (days) / Sand (hours) / Water (minutes)

---

## Phase 1: Accounts & Credentials

### Google OAuth App
- **Estimate:** Sand (30 min)
- **Requires:** Google Cloud account with billing enabled

```
[ ] Go to https://console.cloud.google.com/apis/credentials
[ ] Create Project "Chronos Production" (if needed)
[ ] Configure OAuth consent screen
    [ ] User Type: External
    [ ] App name: Chronos
    [ ] Support email: your email
    [ ] Authorized domains: allsource.dev
[ ] Create OAuth 2.0 Client ID
    [ ] Application type: Web application
    [ ] Name: Chronos Web
    [ ] Authorized redirect URIs:
        - https://chronos.allsource.dev/api/auth/google/callback
        - http://localhost:3902/api/auth/google/callback (for testing)
[ ] Copy Client ID: _______________________
[ ] Copy Client Secret: _______________________
```

### GitHub OAuth App
- **Estimate:** Sand (20 min)
- **Requires:** GitHub account

```
[ ] Go to https://github.com/settings/developers
[ ] Click "New OAuth App"
[ ] Fill in:
    [ ] Application name: Chronos
    [ ] Homepage URL: https://chronos.allsource.dev
    [ ] Authorization callback URL: https://chronos.allsource.dev/api/auth/github/callback
[ ] Click "Register application"
[ ] Copy Client ID: _______________________
[ ] Click "Generate a new client secret"
[ ] Copy Client Secret: _______________________
```

### LemonSqueezy (Optional - for billing)
- **Estimate:** Sand (30 min)
- **Requires:** LemonSqueezy account

```
[ ] Go to https://app.lemonsqueezy.com/settings/api
[ ] Create API key with permissions: read, write
[ ] Copy API Key: _______________________
[ ] Go to Settings > Stores
[ ] Copy Store ID: _______________________
[ ] Go to Settings > Webhooks
[ ] Create webhook for subscription events
[ ] Copy Webhook Secret: _______________________
```

---

## Phase 2: Fly.io Setup

### Create Fly.io Account & Apps
- **Estimate:** Sand (15 min)
- **Requires:** Fly.io account with payment method

```
[ ] Install flyctl: curl -L https://fly.io/install.sh | sh
[ ] Login: fly auth login
[ ] Create apps:
    [ ] fly apps create chronos-core
    [ ] fly apps create chronos-query-service
    [ ] fly apps create chronos-web
[ ] Note your app names: _______________________
```

### Create Postgres Database
- **Estimate:** Water (10 min)

```
[ ] Create Postgres cluster:
    fly postgres create --name chronos-db --region iad --vm-size shared-cpu-1x --initial-cluster-size 1

[ ] Attach to query service:
    fly postgres attach chronos-db --app chronos-query-service

[ ] Note: DATABASE_URL is now automatically set as a secret
```

### Set Secrets
- **Estimate:** Sand (15 min)

```
[ ] Generate secret key:
    mix phx.gen.secret
    # Copy output: _______________________

[ ] Set query-service secrets:
    fly secrets set \
      SECRET_KEY_BASE="<generated-secret>" \
      PHX_HOST="chronos-query-service.fly.dev" \
      GOOGLE_CLIENT_ID="<from-step-1>" \
      GOOGLE_CLIENT_SECRET="<from-step-1>" \
      GITHUB_CLIENT_ID="<from-step-1>" \
      GITHUB_CLIENT_SECRET="<from-step-1>" \
      RUST_CORE_URL="http://chronos-core.internal:3900" \
      CORE_WS_URL="ws://chronos-core.internal:3900/api/v1/events/stream" \
      --app chronos-query-service

[ ] Set web secrets:
    fly secrets set \
      NEXT_PUBLIC_API_URL="https://chronos-query-service.fly.dev" \
      --app chronos-web
```

---

## Phase 3: Deploy

### Deploy Services
- **Estimate:** Sand (30 min per service, can parallelize)
- **Note:** fly.toml configs already created in each app directory

```
[ ] Deploy core (from apps/core):
    cd apps/core
    fly deploy
    [ ] Verify: fly status --app chronos-core
    [ ] Verify health: curl https://chronos-core.fly.dev/health

[ ] Deploy query-service (from apps/query-service):
    cd apps/query-service
    fly deploy
    [ ] Verify: fly status --app chronos-query-service

[ ] Run migrations:
    fly ssh console -a chronos-query-service -C "/app/bin/query_service_ex eval 'QueryServiceEx.Release.migrate()'"
    [ ] Verify: fly logs -a chronos-query-service (check for migration success)

[ ] Deploy web (from monorepo root - Dockerfile needs monorepo context):
    fly deploy -c apps/web/fly.toml --dockerfile apps/web/Dockerfile
    [ ] Verify: fly status --app chronos-web
    [ ] Verify health: curl https://chronos-web.fly.dev/
```

---

## Phase 4: Verify

### Smoke Test
- **Estimate:** Water (15 min)

```
[ ] Open https://chronos-web.fly.dev (or your domain)
[ ] Landing page loads
[ ] Click "Get Started" or "Sign Up"
[ ] Click "Continue with Google"
    [ ] Google OAuth screen appears
    [ ] After auth, redirected to onboarding
[ ] Click "Continue with GitHub" (test logout first)
    [ ] GitHub OAuth screen appears
    [ ] After auth, redirected to onboarding
[ ] Complete onboarding flow
[ ] Dashboard loads with demo data
[ ] Navigate to Events page
[ ] Navigate to API Keys page
[ ] Navigate to Billing page
[ ] Logout works
```

---

## Summary

| Phase | Tasks | Estimate |
|-------|-------|----------|
| 1. Credentials | Google OAuth, GitHub OAuth | 1 hour |
| 2. Fly.io Setup | Apps, Postgres, Secrets | 45 min |
| 3. Deploy | Core, Query, Web + migrations | 1.5 hours |
| 4. Verify | Smoke test | 15 min |
| **Total** | | **~3.5 hours** |

---

## After Launch: Marketing (Day 7-14)

### Demo & Screenshots
- **Estimate:** Rock (2-3 hours)

```
[ ] Record 60-second demo video
    - Login with GitHub OAuth
    - Dashboard overview
    - Create event (if working), see in explorer
    - Time travel query demo
    - Call to action
[ ] Create hero screenshot (1270x760px, dark mode)
[ ] Create 3-5 feature GIFs:
    [ ] Event explorer search
    [ ] Live event stream
    [ ] API key creation
    [ ] Onboarding flow
```

### ProductHunt Launch
- **Estimate:** Sand (2 hours)

```
[ ] Draft listing (see docs/launch/MARKETING_MATERIALS.md)
    [ ] Tagline: "Open-source event sourcing with time-travel debugging"
    [ ] Description (3 paragraphs)
    [ ] Key features (5 items)
[ ] Write maker comment with personal story
[ ] Schedule for Tuesday-Thursday 12:01 AM PT
[ ] Prepare 5-10 supporters to upvote/comment
```

### X.com Launch Thread
- **Estimate:** Sand (1 hour)

```
[ ] Write thread (see docs/launch/MARKETING_MATERIALS.md for draft)
[ ] Attach demo GIF or screenshot
[ ] Post after ProductHunt goes live
[ ] Engage with replies for first 2 hours
```

### Show HN (Day 14+)
- **Estimate:** Sand (1 hour)

```
[ ] Collect 2-3 user testimonials first
[ ] Write post (see docs/launch/MARKETING_MATERIALS.md for draft)
[ ] Post on weekday morning (US time)
[ ] Be available to answer questions for 4+ hours
```

---

## After Launch: Polish (P2)

These improve the product but don't block launch:

```
[ ] Custom domain setup (chronos.allsource.dev → Fly.io)
[ ] LemonSqueezy billing integration test
[ ] Privacy Policy page
[ ] Terms of Service page
```

---

## Troubleshooting

**OAuth redirect error:**
- Check callback URLs match exactly (trailing slashes matter)
- Ensure secrets are set correctly: `fly secrets list --app chronos-query-service`

**Database connection error:**
- Verify attachment: `fly postgres list`
- Check DATABASE_URL is set: `fly secrets list --app chronos-query-service`

**Service can't reach another service:**
- Use `.internal` DNS: `http://chronos-core.internal:3900`
- Verify both apps are in same organization
