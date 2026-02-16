# OAuth & Secrets Configuration

How to configure Google and GitHub OAuth secrets for the AllSource stack.

---

## Required Secrets

The Query Service reads these from environment variables at runtime (`config/runtime.exs:58-63`):

| Secret | Env Var | Source |
|--------|---------|--------|
| Google OAuth | `GOOGLE_CLIENT_ID` | [Google Cloud Console](https://console.cloud.google.com/apis/credentials) |
| Google OAuth | `GOOGLE_CLIENT_SECRET` | Same |
| GitHub OAuth | `GITHUB_CLIENT_ID` | [GitHub Developer Settings](https://github.com/settings/developers) |
| GitHub OAuth | `GITHUB_CLIENT_SECRET` | Same |
| JWT signing | `JWT_SECRET` | Generate with `mix phx.gen.secret` |
| Phoenix secret | `SECRET_KEY_BASE` | Generate with `mix phx.gen.secret` |
| Frontend URL | `FRONTEND_URL` | e.g. `https://all-source.xyz` |

OAuth is optional — if `GOOGLE_CLIENT_ID` or `GITHUB_CLIENT_ID` are not set, the corresponding provider returns `:unconfigured` and the login redirect fails gracefully.

---

## Local Development

Add to a `.env` file (already gitignored) or export in your shell:

```bash
export GOOGLE_CLIENT_ID="your-id.apps.googleusercontent.com"
export GOOGLE_CLIENT_SECRET="GOCSPX-..."
export GITHUB_CLIENT_ID="Ov23li..."
export GITHUB_CLIENT_SECRET="..."
export JWT_SECRET="$(mix phx.gen.secret)"
export SECRET_KEY_BASE="$(mix phx.gen.secret)"
export FRONTEND_URL="http://localhost:3000"
```

---

## Fly.io Production

As noted in `apps/query-service/fly.toml:4`, use `flyctl secrets set`:

```bash
flyctl secrets set \
  GOOGLE_CLIENT_ID="your-id.apps.googleusercontent.com" \
  GOOGLE_CLIENT_SECRET="GOCSPX-..." \
  GITHUB_CLIENT_ID="Ov23li..." \
  GITHUB_CLIENT_SECRET="..." \
  JWT_SECRET="$(mix phx.gen.secret)" \
  SECRET_KEY_BASE="$(mix phx.gen.secret)" \
  FRONTEND_URL="https://all-source.xyz" \
  --app allsource-query
```

Fly.io secrets are encrypted at rest and injected as env vars at runtime. They never appear in `fly.toml` or git.

---

## Docker Compose (Local Stack)

Pass secrets via `docker-compose.override.yml` (gitignored) or a `.env` file:

```yaml
# docker-compose.override.yml
services:
  allsource-query-service:
    environment:
      - GOOGLE_CLIENT_ID=${GOOGLE_CLIENT_ID}
      - GOOGLE_CLIENT_SECRET=${GOOGLE_CLIENT_SECRET}
      - GITHUB_CLIENT_ID=${GITHUB_CLIENT_ID}
      - GITHUB_CLIENT_SECRET=${GITHUB_CLIENT_SECRET}
      - JWT_SECRET=${JWT_SECRET}
      - SECRET_KEY_BASE=${SECRET_KEY_BASE}
      - FRONTEND_URL=http://localhost:3000
```

Then either export the vars in your shell or create a `.env` file in the repo root (gitignored).

---

## OAuth Callback URLs

When registering OAuth apps with Google and GitHub, set the authorized redirect URIs:

### Google Cloud Console

- **Production**: `https://allsource-query.fly.dev/api/auth/google/callback`
- **Development**: `http://localhost:3902/api/auth/google/callback`

### GitHub Developer Settings

- **Production**: `https://allsource-query.fly.dev/api/auth/github/callback`
- **Development**: `http://localhost:3902/api/auth/github/callback`

The callback URL base is controlled by `OAUTH_CALLBACK_BASE_URL` env var, falling back to `PHX_HOST` (see `oauth_controller.ex:378`).

---

## How the OAuth Flow Works

```
Browser                Query Service (3902)         Control Plane (3901)      Provider
  |                         |                            |                      |
  |-- GET /api/auth/google ->                            |                      |
  |                         |-- redirect ---------------------------------------+
  |                         |                            |                      |
  |<-- redirect to Google --+                            |                      |
  |                         |                            |                      |
  |-- callback?code=... --->|                            |                      |
  |                         |-- exchange code -----------+--------------------->|
  |                         |<-- access_token -----------+---------------------+
  |                         |-- fetch user info ---------+--------------------->|
  |                         |<-- email, name, id --------+---------------------+
  |                         |-- POST /api/v1/auth/oauth ->                      |
  |                         |<-- JWT token + new_user ---+                      |
  |<-- redirect to frontend/callback?token=JWT           |                      |
```

1. `GET /api/auth/:provider` redirects to provider authorization page
2. Provider redirects back to `/api/auth/:provider/callback?code=...`
3. Query Service exchanges code for access token with provider
4. Query Service fetches user info (email, name) from provider API
5. Query Service calls Control Plane `POST /api/v1/auth/oauth` to find/create user
6. Control Plane returns JWT (or Query Service signs one locally as fallback)
7. Redirect to frontend with JWT in URL

---

## Related Files

- OAuth controller: `apps/query-service/lib/query_service_ex_web/controllers/oauth_controller.ex`
- Runtime config: `apps/query-service/config/runtime.exs` (lines 58-63)
- Control Plane handler: `apps/control-plane/auth.go` (line 211, `OAuthHandler`)
- Fly.io config: `apps/query-service/fly.toml`
