# Authentication Chain — Core, Query Service, and Clients

## Overview

```
Client (X-API-Key: ask_xxx)
        |
Query Service (Elixir, port 3902)
        |  validates key via Core /api/v1/auth/me
        |  caches result for 120s (ETS, SHA-256 key)
        |
Core (Rust, port 3900)
        |  AuthManager validates key hash in DashMap
        |  returns {tenant_id, role} from stored key
```

## How it works

### 1. Client sends request to Query Service

Clients authenticate with `X-API-Key` header:

```bash
curl -H "X-API-Key: ask_xxx..." https://query-service/api/v1/events?stream=orders
```

### 2. Query Service validates the key

The `AuthPipeline` plug (`auth_pipeline.ex`) tries methods in order:

1. `X-API-Key` header -> calls `ApiKeyCache.fetch/2`
2. `Authorization: Bearer <jwt>` -> validates JWT locally
3. Dev mode bypass (`AUTH_DISABLED=true`)

For API keys, `ApiKeyCache` checks its ETS cache (120s TTL, keyed by SHA-256
hash of the raw key). On cache miss, it calls `RustCoreClient.verify_api_key/1`.

### 3. RustCoreClient calls Core's `/api/v1/auth/me`

```elixir
# rust_core_client.ex — simplified
Tesla.get(client, "/api/v1/auth/me",
  headers: [{"authorization", raw_key}])
```

Core's auth middleware:
- Extracts the token from the `Authorization` header
- Detects it's an API key (starts with `ask_`)
- Calls `AuthManager.validate_api_key(&token)` which hash-matches against its
  in-memory DashMap (loaded from durable system WAL on startup)
- Returns `Claims {tenant_id, role}` -> synthesized into `UserInfo` by `/me`

### 4. Query Service resolves tenant

After `/me` returns `{tenant_id, role}`, Query Service:
- Fetches the full tenant record from Core (`GET /api/v1/tenants/:id`)
- Verifies subscription status (active/trialing)
- Caches the validated result in ETS
- Sets `conn.assigns` with `tenant_id`, `current_tenant`, `auth_method: :api_key`

## Environment variables

### Core

| Variable | Purpose |
|----------|---------|
| `ALLSOURCE_BOOTSTRAP_API_KEY` | Bootstrap a persistent API key on first boot (idempotent) |
| `ALLSOURCE_BOOTSTRAP_TENANT_ID` | Tenant ID for the bootstrap key |
| `ALLSOURCE_DEV_MODE` | `true`/`1` to bypass auth (never in production) |
| `ALLSOURCE_DATA_DIR` | Enables system WAL for durable key storage |
| `JWT_SECRET` | Shared secret for JWT validation |

### Query Service

| Variable | Purpose |
|----------|---------|
| `CORE_URL` | Core HTTP URL (e.g. `http://core:3900`) |
| `CORE_WS_URL` | Core WebSocket URL for streaming |
| `CORE_API_KEY` | API key for QS -> Core internal calls |
| `JWT_SECRET` | Same shared secret as Core |
| `AUTH_DISABLED` | `true` to bypass auth in dev (QS-side) |

## Production setup (recommended)

```bash
# Core
ALLSOURCE_BOOTSTRAP_API_KEY=ask_your-production-key
ALLSOURCE_BOOTSTRAP_TENANT_ID=your-tenant
ALLSOURCE_DATA_DIR=/data/events
ALLSOURCE_DEV_MODE=false
JWT_SECRET=your-shared-secret

# Query Service
CORE_URL=http://core:3900
CORE_API_KEY=ask_your-production-key
JWT_SECRET=your-shared-secret
```

On first boot, Core persists the bootstrap key as a `_system.auth.key_provisioned`
event in the system WAL. On subsequent restarts, the key is replayed from storage
automatically — the env var is only needed for the initial creation (but is safe
to leave set; it's idempotent).

## Dev mode behavior

When `ALLSOURCE_DEV_MODE=true`:

- **Auth on provisioning endpoints is skipped** — you can create keys without
  an existing key
- **If a valid API key is provided**, it is validated normally and `/me` returns
  the key's real `tenant_id` and `role`
- **If no token is provided**, a synthetic admin context is used
  (`tenant_id: "dev-tenant"`, `role: admin`)
- **Rate limiting is disabled**

This means dev mode is safe to use for initial key provisioning without breaking
the QS auth chain — as long as the provisioned keys are persisted (requires
`ALLSOURCE_DATA_DIR` to be set).

## Key formats

- **API keys**: `ask_` prefix, validated by hash lookup in Core's AuthManager
- **JWTs**: Bearer tokens, validated by shared `JWT_SECRET`
- **Header precedence**: `Authorization` header checked first, `X-API-Key` as
  fallback (legacy)
