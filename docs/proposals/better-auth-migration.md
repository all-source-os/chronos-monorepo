# Better Auth Migration Plan

## Context

The current auth system has three disconnected auth stores across services, and **email/password login doesn't work** despite the frontend having the forms for it. The Query Service uses Ueberauth (OAuth) + Guardian (JWT) but only supports Google/GitHub OAuth — `POST /api/auth/login` and `POST /api/auth/register` don't exist on the backend.

**Goal**: Replace the custom Ueberauth/Guardian auth in the Query Service with [Better Auth](https://www.better-auth.com/), running as a Next.js API route in the web app. This gives us:
- Working email/password auth (signup, login, forgot/reset password, email verification)
- OAuth (Google, GitHub) via Better Auth's social plugins
- A single auth source (Better Auth's Postgres tables) instead of three disconnected systems
- JWT plugin with JWKS endpoint so the Elixir Query Service can validate tokens without sharing secrets

## Architecture Change

```
BEFORE:
  Browser → Next.js (cookie proxy) → Query Service (Ueberauth + Guardian + Postgres)

AFTER:
  Browser → Next.js (Better Auth server + API routes) → Better Auth Postgres DB
                                                              ↓
  Query Service validates JWT via JWKS ← Better Auth JWT plugin exposes /.well-known/jwks.json
```

Better Auth runs inside the Next.js app (API route handler). It manages its own tables in the same Postgres DB the Query Service uses (or a separate one). The Query Service no longer handles auth — it only validates incoming JWTs by fetching the public keys from Better Auth's JWKS endpoint.

---

## Phase 1: Install Better Auth in Web App

**Files to create/modify:**
- `apps/web/package.json` — add `better-auth` dependency
- `apps/web/src/lib/auth.ts` — Better Auth server instance config
- `apps/web/src/lib/auth-client.ts` — Better Auth client instance
- `apps/web/src/app/api/auth/[...all]/route.ts` — catch-all API route handler

**Details:**
1. `bun add better-auth` in `apps/web/`
2. Configure Better Auth server with:
   - PostgreSQL adapter (using `DATABASE_URL` env var — same Postgres as Query Service or separate)
   - Email/password plugin enabled
   - Google + GitHub social providers (reuse existing OAuth client IDs/secrets)
   - JWT plugin with JWKS endpoint enabled (for Elixir validation)
   - Session config: cookie-based sessions for the web app
3. Create the catch-all route at `apps/web/src/app/api/auth/[...all]/route.ts` that delegates to Better Auth's `toNextJsHandler()`
4. Create client-side auth instance in `apps/web/src/lib/auth-client.ts` using `createAuthClient()`

## Phase 2: Database Setup

**Details:**
1. Run Better Auth's schema generation: `npx @better-auth/cli generate` to get the migration SQL
2. Better Auth creates these tables: `user`, `session`, `account`, `verification`
3. Write a data migration script to copy existing users from the Query Service's `users` table into Better Auth's `user` table, and create corresponding `account` records for their OAuth providers
4. The existing `users` table in the Query Service stays — we'll add a `better_auth_user_id` column or map by email

**Key consideration:** Better Auth's `user` table uses its own schema. Existing Query Service users need their OAuth `account` records created so they can still log in with Google/GitHub.

## Phase 3: Update Frontend Auth Pages

**Files to modify:**
- `apps/web/src/app/(auth)/login/page.tsx`
- `apps/web/src/app/(auth)/signup/page.tsx`
- `apps/web/src/app/(auth)/forgot-password/page.tsx`
- `apps/web/src/app/(auth)/reset-password/page.tsx`
- `apps/web/src/app/(auth)/verify-email/page.tsx`

**Details:**
1. Replace direct `fetch()` calls to `${apiUrl}/api/auth/login` with Better Auth client methods:
   - Login: `authClient.signIn.email({ email, password })`
   - Register: `authClient.signUp.email({ name, email, password })`
   - OAuth: `authClient.signIn.social({ provider: "google" })` / `authClient.signIn.social({ provider: "github" })`
   - Forgot password: `authClient.forgetPassword({ email })`
   - Reset password: `authClient.resetPassword({ token, newPassword })`
2. Remove `getApiUrl()` usage for auth — all auth goes through local Next.js routes now
3. OAuth buttons change from `window.location.href = '${apiUrl}/api/auth/google'` to `authClient.signIn.social({ provider: "google" })`

## Phase 4: Update Middleware & Session Handling

**Files to modify:**
- `apps/web/src/middleware.ts`
- `apps/web/src/app/api/auth/session/route.ts` — replace or remove
- `apps/web/src/app/api/auth/callback/route.ts` — remove (Better Auth handles callbacks)

**Details:**
1. Update middleware to check Better Auth session instead of `auth_token` cookie:
   - Use Better Auth's `auth.api.getSession()` with the request headers
   - Or check for Better Auth's session cookie (default: `better-auth.session_token`)
2. Remove the custom `/api/auth/callback/route.ts` — Better Auth's catch-all handles OAuth callbacks
3. Replace `/api/auth/session/route.ts` with a simpler version that calls Better Auth's session API

## Phase 5: Update API Client & Dashboard

**Files to modify:**
- `apps/web/src/lib/api/client.ts`
- `apps/web/src/components/dashboard/sidebar.tsx` (logout handler)
- Any component using `apiClient.getMe()`, `apiClient.login()`, etc.

**Details:**
1. Remove auth methods from `ApiClient` (getMe, login, register, verifyEmail, forgotPassword, resetPassword, resendVerification) — these are now handled by Better Auth client
2. Update `ApiClient.request()` to attach the Better Auth JWT as a Bearer token instead of relying on `credentials: "include"` to the Query Service:
   - Get JWT from Better Auth session: `const session = await authClient.getSession()`
   - Add `Authorization: Bearer ${session.token}` header to all Query Service requests
3. Update logout in sidebar to call `authClient.signOut()`

## Phase 6: Elixir Query Service — JWKS JWT Validation

**Files to modify:**
- `apps/query-service/mix.exs` — add `jose` dependency
- `apps/query-service/lib/query_service_ex_web/plugs/auth_pipeline.ex` — replace Guardian pipeline
- `apps/query-service/lib/query_service_ex/accounts/guardian.ex` — remove or keep for dev mode
- `apps/query-service/config/config.exs` — add JWKS URL config

**Details:**
1. Add `{:jose, "~> 1.11"}` to mix.exs deps (JOSE library for JWT/JWKS validation)
2. Create a new plug `BetterAuthJwt` or modify `AuthPipeline` to:
   - Fetch JWKS from Better Auth's `/.well-known/jwks.json` endpoint (with caching)
   - Extract Bearer token from Authorization header
   - Verify JWT signature using the JWKS public key
   - Extract user ID and email from JWT claims
   - Look up or create the user in Query Service's local `users` table (by email or better_auth_user_id)
   - Assign `current_user` to conn
3. Keep the dev mode bypass (`AUTH_DISABLED=true`) as-is
4. The Query Service's `users` table stays — it just gets synced on first authenticated request from each user (upsert by email)

**JWKS caching:** Cache the JWKS response in an ETS table or Agent, refresh every ~1 hour or on JWT validation failure.

## Phase 7: Tenant Association

**Files to modify:**
- `apps/query-service/lib/query_service_ex/accounts.ex` — add `find_or_create_from_better_auth/1`

**Details:**
1. When a JWT from Better Auth is validated and the user doesn't exist in the Query Service's `users` table:
   - Create the user record (email, name from JWT claims)
   - Auto-create a tenant (same logic as current `create_user_with_tenant/1`)
2. When the user already exists (matched by email):
   - Return existing user with tenant preloaded
3. This preserves existing tenant/billing associations for migrated users

## Phase 8: Cleanup

**Files to remove/modify:**
- Remove Ueberauth deps from `mix.exs`: `ueberauth`, `ueberauth_google`, `ueberauth_github`
- Remove Guardian dep from `mix.exs` (keep JOSE)
- Remove `apps/query-service/lib/query_service_ex/accounts/guardian.ex`
- Remove Ueberauth config from `config.exs`, `dev.exs`, `test.exs`, `runtime.exs`
- Remove Guardian config from all config files
- Remove or simplify `AuthController` — only keep `me` endpoint (now backed by JWKS validation)
- Remove OAuth routes from `router.ex` (`GET /api/auth/:provider`, `GET /api/auth/:provider/callback`)
- Remove `auth_error_handler.ex` (Guardian-specific)

## Phase 9: Environment Variables

**New env vars needed:**
- `BETTER_AUTH_SECRET` — secret for Better Auth (used for session signing)
- `BETTER_AUTH_URL` — base URL of the web app (for OAuth callbacks)
- `DATABASE_URL` — Better Auth needs direct Postgres access (may reuse Query Service's DB or separate)
- `GOOGLE_CLIENT_ID`, `GOOGLE_CLIENT_SECRET` — reuse existing
- `GITHUB_CLIENT_ID`, `GITHUB_CLIENT_SECRET` — reuse existing
- `JWKS_URL` — for Query Service config (e.g., `https://all-source.xyz/api/auth/.well-known/jwks.json`)

---

## Verification

1. **Email/password flow**: Sign up with email → verify email → login → see dashboard
2. **OAuth flow**: Click Google/GitHub → redirected → callback → session created → dashboard
3. **Query Service auth**: Authenticated requests to `/api/events`, `/api/query` etc. work with Better Auth JWT
4. **Existing users**: Users who signed up via Google/GitHub before migration can still log in
5. **Tenant association**: New users get auto-created tenant, existing users keep their tenant
6. **Dev mode**: `AUTH_DISABLED=true` still bypasses auth in the Query Service
7. Run existing e2e tests in `tooling/e2e/`

## Implementation Order

Phases 1-2 first (install + DB), then 3-5 (frontend), then 6-7 (Elixir backend), then 8-9 (cleanup). Each phase is independently deployable — the old auth can coexist with Better Auth during migration.
