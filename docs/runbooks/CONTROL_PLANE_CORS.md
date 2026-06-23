# Runbook — Control Plane CORS

**Service:** `apps/control-plane` (Go/Gin, Fly app `allsource-control-plane`).
**Owner:** platform.
**TL;DR:** the Control Plane grants **credentialed** CORS only to an explicit allowlist of frontend origins. To let a new browser app (e.g. the admin panel) call the API with cookies, add its origin to `ALLOWED_FRONTEND_URLS` and redeploy. Never reflect arbitrary origins with credentials.

---

## 1. The policy (what the code does)

`corsMiddleware` (`apps/control-plane/main.go`) decides per request, by `Origin` header:

| Request `Origin` | `Access-Control-Allow-Origin` | `Access-Control-Allow-Credentials` | Effect |
|---|---|---|---|
| In the allowlist | echoes that exact origin | `true` + `Vary: Origin` | cookies / `Authorization` usable cross-site (web app, admin panel) |
| Not in the allowlist (or absent) | `*` | *(not set)* | public, **non-credentialed** reads only; browser refuses to send cookies or expose a credentialed response |

The allowlist is `allowedCORSOrigins()` (`apps/control-plane/oauth.go`), derived from the **same** frontend allowlist used for OAuth open-redirect protection (`getAllowedFrontendURLs()`): `FRONTEND_URL` plus comma-separated `ALLOWED_FRONTEND_URLS`, each reduced to `scheme://host`. One source of truth → adding an origin once enables both CORS and OAuth redirect for it.

### Why it's an allowlist, not a reflector (the security bug this fixed)

The previous middleware echoed **any** `Origin` with `Access-Control-Allow-Credentials: true`. That is a credential-theft / CSRF vector: any website a logged-in admin visited could make authenticated requests to the Control Plane **and read the responses** in the victim's browser, because the browser saw `Allow-Origin: <attacker>` + `Allow-Credentials: true`. The fix gates the credentialed grant on the allowlist; everything else gets only the non-credentialed `*`. Public unauthenticated endpoints still work from anywhere; nothing can make a **credentialed** request unless it is explicitly allowed.

---

## 2. Config

| Env | Meaning | Prod value (`fly.toml`) |
|---|---|---|
| `FRONTEND_URL` | the primary frontend; always allowed | `https://www.all-source.xyz` |
| `ALLOWED_FRONTEND_URLS` | additional allowed origins, comma-separated, **no trailing slash** | `https://all-source.xyz,https://admin.all-source.xyz` |

Local dev: with neither set, `FRONTEND_URL` defaults to `http://localhost:3000`. To run the admin app (`apps/admin`, default port 3001) against a local Control Plane, export `ALLOWED_FRONTEND_URLS=http://localhost:3001` before starting it.

---

## 3. How to allow a new browser origin

1. Add the exact origin (scheme + host, no path, no trailing slash) to `ALLOWED_FRONTEND_URLS` in `apps/control-plane/fly.toml` (version-controlled, preferred) **or** as a secret:
   ```
   fly secrets set ALLOWED_FRONTEND_URLS="https://all-source.xyz,https://admin.all-source.xyz,https://NEW.origin" --app allsource-control-plane
   ```
   (A `fly.toml` `[env]` edit ships on the next `fly deploy`; `fly secrets set` triggers its own restart.)
2. Redeploy the Control Plane so the new value is read at boot:
   ```
   fly deploy apps/control-plane --app allsource-control-plane
   ```
   (CORS allowlist is built once in `setupMiddleware` at startup — an env change needs a restart/redeploy, not just a config push.)
3. Verify (§5).

> Preview deploys: Vercel preview URLs are per-build `*.vercel.app` and are **not** allowlisted (we never trust all of `vercel.app`). Test previews against a non-prod Control Plane, or give the app a stable custom domain and allowlist that.

---

## 4. The admin panel (`admin.all-source.xyz`) — full cross-origin wiring

The admin app (`apps/admin`, Vercel project `allsource-admin`) calls the Control Plane **client-side with `credentials: "include"`** and authenticates via an `admin_token` httpOnly cookie. For login to work cross-origin, all three must hold:

1. **Custom domain** `admin.all-source.xyz` on the `allsource-admin` Vercel project — so it shares the `.all-source.xyz` parent with `api.all-source.xyz` and the auth cookie is sendable. (A `*.vercel.app` host cannot share cookies with the Fly API.)
2. **CORS** — `admin.all-source.xyz` in `ALLOWED_FRONTEND_URLS` (this runbook, §3). ✅ shipped in `fly.toml`.
3. **OAuth** — the Google/GitHub OAuth apps must list the admin domain in their authorized redirect URIs / origins (external console; owner action).

Until (1) and (3) land, the admin app loads but **login will not complete**. See [FLEET_HEALTH_RECOVERY.md](./FLEET_HEALTH_RECOVERY.md) for the surrounding feature.

---

## 5. Verify

```
# Allowlisted origin → echoes origin + credentials:
curl -sI -H "Origin: https://admin.all-source.xyz" https://api.all-source.xyz/api/v1/billing/catalog \
  | grep -i 'access-control-allow-'
# expect: Access-Control-Allow-Origin: https://admin.all-source.xyz
#         Access-Control-Allow-Credentials: true

# Unknown origin → "*", NO credentials header:
curl -sI -H "Origin: https://evil.example.com" https://api.all-source.xyz/api/v1/billing/catalog \
  | grep -i 'access-control-allow-'
# expect: Access-Control-Allow-Origin: *      (and NO Access-Control-Allow-Credentials)
```

Automated coverage: `apps/control-plane/cors_test.go` (`go test -run CORS .`) asserts allowlisted-gets-credentials, unknown-never-gets-credentials, preflight-204, and env-driven allowlist.

---

## 6. Troubleshooting

| Symptom | Cause | Fix |
|---|---|---|
| Browser console: *"No 'Access-Control-Allow-Origin'… credentials mode 'include'"* | the app's origin is not in the allowlist | add it to `ALLOWED_FRONTEND_URLS` + redeploy (§3) |
| Login redirects but session never sticks | cookie can't be shared cross-site | the app must be on a `.all-source.xyz` subdomain (§4 step 1), not `*.vercel.app` |
| `curl` shows `Allow-Origin: *` for an origin you *did* add | env not reloaded | redeploy/restart — the allowlist is built at boot (§3 step 2) |
| OAuth returns "redirect not allowed" | origin missing from `getAllowedFrontendURLs()` | same `ALLOWED_FRONTEND_URLS` covers both CORS and OAuth redirect — add it once |

---

## 7. Invariants

- **Never** pair `Access-Control-Allow-Credentials: true` with a reflected/arbitrary or `*` origin. Credentialed = allowlisted exact origin only.
- CORS and OAuth-redirect share one allowlist (`getAllowedFrontendURLs`). Don't fork them.
- Adding an origin is config + redeploy, not a code change.
