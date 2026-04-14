---
name: fly-status
description: Check the status of all AllSource services on Fly.io. Shows running state, health checks, deployed versions, recent crashes/errors, and staleness vs latest git tag. Triggers on "fly status", "check fly", "deployment status", "are services running", "check production", "fly report", "fly health".
---

# Fly.io Status Report

Generate a comprehensive status report for all AllSource services deployed on Fly.io.

## Apps to Check

All AllSource **backend** apps on Fly. The web frontend is on **Vercel** at `https://all-source.xyz` — it is NOT on Fly and should never be added here.

| App Name | Service | Expected State |
|----------|---------|----------------|
| allsource-core | Rust event store | always running |
| allsource-query | Elixir query service | always running |
| allsource-control-plane | Go control plane | auto-stop (may be stopped when idle) |
| allsource-auth | Rust auth service | always running |
| allsource-prime | Rust Prime MCP server | always running |
| allsource-registry | Rust package registry | always running |

If an `allsource-web` app ever appears in `fly apps list`, flag it immediately — it was destroyed on 2026-04-15 and should not be recreated. The frontend lives on Vercel.

Also check `alphasigmapro-*` apps if they exist.

## Procedure

### 1. List all apps

```bash
fly apps list 2>&1 | grep -v "Warning"
```

### 2. Machine status for each app

For each app, run:

```bash
fly machines list -a <APP_NAME> 2>&1 | grep -v "Warning"
```

Capture: state, checks, region, image, last updated.

### 3. Recent errors/crashes

For each app, check logs for crashes:

```bash
fly logs -a <APP_NAME> --no-tail 2>&1 | grep -iE "error|crash|panic|SIGKILL|OOM|exit|restart|unhealthy" | tail -10
```

**Important:** `fly logs` can be slow. Set a 15-second timeout per app. If it hangs, skip and note "logs unavailable".

**Distinguish normal from abnormal:**
- `exit_code=0, requested_stop=true` → auto-stop (normal for control-plane)
- `exit_code=0, oom_killed=false` → clean shutdown (normal)
- `exit_code!=0` or `oom_killed=true` → **crash, investigate**
- `SIGKILL` → OOM or Fly forced kill — **investigate**

### 4. Check machine events for stopped apps

If a machine is stopped, check why:

```bash
fly machines status <MACHINE_ID> -a <APP_NAME> 2>&1 | grep -E "State|exit|oom|Event"
```

### 5. Version staleness

Compare deployed images to the latest git tag:

```bash
# Latest tag
git describe --tags --abbrev=0

# For each app, extract deployment timestamp from fly status
fly status -a <APP_NAME> 2>&1 | grep "Image"
```

### 6. GitHub Actions status

Check if the latest release workflow succeeded (Docker images built):

```bash
gh run list --workflow=release.yml --limit 1 2>&1
gh run list --workflow=docker-build.yml --limit 1 2>&1
```

## Output Format

Present results as a markdown table:

```
## Fly.io Deployment Report

### Service Status

| App | Status | Health | Last Deploy | Git Tag | Stale? |
|-----|--------|--------|-------------|---------|--------|
| allsource-core | Running | 1/1 | Mar 6 | v0.17.1 | YES (18d) |
| ... | ... | ... | ... | ... | ... |

### Crashes / Errors

[List any non-zero exits, OOM kills, or error-level log entries]
Or: "No crashes detected. All services healthy."

### Action Items

[List anything that needs attention: stale deployments, stopped services that shouldn't be, failed health checks]
```

## Common Issues

| Symptom | Cause | Fix |
|---------|-------|-----|
| Control plane stopped | Auto-stop on idle | Normal — starts on next request |
| Health check warning on stopped machine | Machine not running | Normal if auto-stop |
| Image very old (>7 days) | No recent deploy | Deploy latest: `fly deploy -a <app>` |
| OOM killed | Memory limit too low | Scale up: `fly scale memory 1024 -a <app>` |
| Exit code 137 | SIGKILL (OOM or timeout) | Check memory usage, increase limits |

## Deploy Commands (for reference only — do NOT auto-deploy)

```bash
# Core (needs monorepo root context)
fly deploy --config apps/core/fly.toml --dockerfile apps/core/Dockerfile

# Query Service
cd apps/query-service && fly deploy

# Control Plane
cd apps/control-plane && fly deploy

# Web — NOT ON FLY. Frontend is on Vercel at https://all-source.xyz.
# Do not run `fly deploy` for web. Redeploy via Vercel (git push to main, or `vercel --prod`).

# Prime (needs monorepo root context)
fly deploy --config apps/prime-mcp/fly.toml --dockerfile apps/prime-mcp/Dockerfile

# Registry
cd apps/registry && fly deploy
```

**NEVER deploy without the user explicitly asking.** This skill is read-only — report status only.
