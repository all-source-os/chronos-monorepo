# Unified Auth — OAuth Bridge, Teams, Agent/Human Roles

**Status:** Proposal
**Date:** 2026-03-28
**Supersedes:** AGENT_AUTH_DESIGN.md (which mixed auth, x402 payments, and wallet provisioning — archived)

## Problem

Three disconnected identity systems:

| System | Actors | Storage | Tokens |
|--------|--------|---------|--------|
| Control Plane OAuth | Dashboard humans | PostgreSQL | httpOnly cookie |
| Core auth | API/sync users | In-memory (lost on restart) | JWT + API keys |
| Agent registration | CLI agents | Core in-memory | API keys |

A human who logs into the dashboard via Google OAuth **cannot** get an API key for `cn sync`. An agent with a Core API key **cannot** see the dashboard. Teams don't exist — there's no way to share a tenant between two people.

## Three Concerns (scoped)

This proposal addresses ONLY:

1. **OAuth callback provisions Core user + API key** — so dashboard users can sync
2. **Teams as tenants with invite flow** — so multiple people share a workspace
3. **Human vs agent roles** — so agents have scoped permissions

x402 payments, wallet provisioning, and MCP auth are out of scope (separate proposals).

---

## 1. OAuth Bridge — Dashboard User Gets API Key

### Current Flow

```
User → Google OAuth → Control Plane → sets cookie → Dashboard works
                                                   → Core? No connection.
```

### Proposed Flow

```
User → Google OAuth → Control Plane callback:
  1. Create/find user in CP database (existing)
  2. NEW: Provision Core user + API key for user's tenant
     POST core:3900/api/v1/auth/register { username, password: random, tenant_id }
     POST core:3900/api/v1/auth/api-keys { name: "dashboard-{user_id}" }
  3. Store Core API key in CP user record
  4. Set cookie (existing)
  5. Return API key in login response (new field)
```

### Implementation

**Control Plane** (`apps/control-plane/`):

```go
// In OAuth callback handler, after creating CP user:
func (h *AuthHandler) provisionCoreAccess(user *User, tenant *Tenant) error {
    // Register user in Core (idempotent — skip if exists)
    coreUser, err := h.coreClient.Register(user.Email, randomPassword(), tenant.ID)
    if err != nil && !isAlreadyExists(err) {
        return err
    }

    // Create API key scoped to tenant
    apiKey, err := h.coreClient.CreateAPIKey(coreUser.Token, "sync-"+user.ID)
    if err != nil {
        return err
    }

    // Store in CP database for retrieval
    user.CoreAPIKey = apiKey.Key
    return h.userRepo.Update(user)
}
```

**Dashboard** (`apps/web/`):

New API endpoint or addition to `/api/auth/session`:
```json
{
  "user": { "id": "...", "email": "..." },
  "core_api_key": "ask_...",
  "tenant_id": "tenant-..."
}
```

Dashboard settings page shows the API key with a copy button. User pastes it into `.chronis/config.toml`.

### Effort: Sand (4h)

- 2h: Control Plane OAuth callback + Core client
- 1h: Dashboard API key display
- 1h: Tests

---

## 2. Teams as Tenants

### Model

A **team** IS a **tenant**. One tenant = one shared event namespace. Team members all have the same `tenant_id` in their JWTs and API keys.

```
Team "acme-eng" (tenant_id: "tenant-acme-eng")
├── Alice (admin, human) — created team, can invite
├── Bob (member, human) — invited by Alice
├── agent-ci (agent) — API key created by Alice
└── agent-claude (agent) — API key created by Bob
```

### Invite Flow

```
Alice → Dashboard → Team Settings → Invite bob@acme.com
  1. CP creates pending invite record
  2. Email sent with invite link
  3. Bob clicks link → OAuth → CP callback:
     - Creates Bob's CP user
     - Assigns Bob to Alice's tenant (not a new one)
     - Provisions Core user + API key in that tenant
  4. Bob can now sync to the same tenant as Alice
```

### Implementation

**Control Plane** new endpoints:

```
POST /api/v1/teams/invite     { email, role }
GET  /api/v1/teams/invite/:token   (accept invite)
GET  /api/v1/teams/members
PUT  /api/v1/teams/members/:id     { role }
DELETE /api/v1/teams/members/:id
```

**Database** (CP PostgreSQL):

```sql
CREATE TABLE team_invites (
  id UUID PRIMARY KEY,
  tenant_id TEXT NOT NULL,
  email TEXT NOT NULL,
  role TEXT DEFAULT 'member',
  invited_by UUID REFERENCES users(id),
  token TEXT UNIQUE NOT NULL,
  accepted_at TIMESTAMPTZ,
  created_at TIMESTAMPTZ DEFAULT NOW()
);
```

### Registration Change

Current: self-registration creates `tenant-{username}` (isolated).
New: if user accepts an invite, they join the existing tenant instead of creating a new one.

### Effort: Rock (1-2 days)

- 4h: Invite endpoints + DB schema
- 4h: OAuth callback team assignment
- 2h: Dashboard team settings page
- 2h: Tests

---

## 3. Human vs Agent Roles

### Role Model

Two actor types, three permission levels:

| Role | Actor | Can login to dashboard | Can use API keys | Can invite members | Can manage billing |
|------|-------|----------------------|------------------|-------------------|-------------------|
| **admin** | Human | Yes | Yes | Yes | Yes |
| **member** | Human | Yes | Yes | No | No |
| **agent** | Machine | No | Yes (scoped) | No | No |

### Agent Scoping

Agents get API keys with restricted permissions:

```json
{
  "key": "ask_...",
  "name": "ci-pipeline",
  "role": "agent",
  "permissions": ["events:write", "events:read"],
  "tenant_id": "tenant-acme-eng"
}
```

Agents CANNOT:
- Create other API keys
- Invite team members
- Access billing
- Register new users

### Implementation

**Core** — API key creation accepts a `role` field:

```rust
pub struct CreateApiKeyRequest {
    pub name: String,
    pub role: Option<String>,  // "admin", "member", "agent" (default: "agent")
}
```

**Core** — Permission check on sensitive endpoints:

```rust
// In auth middleware, after validating token:
if claims.role == "agent" && is_admin_only_path(path) {
    return Err(AuthError::Forbidden);
}
```

Admin-only paths: `/api/v1/auth/register`, `/api/v1/auth/api-keys` (create), `/api/v1/tenants/*`.

### Effort: Sand (3h)

- 1h: Role field on API keys + JWT claims
- 1h: Permission enforcement middleware
- 1h: Tests

---

## Priority Order

| # | What | Blocks | Estimate |
|---|------|--------|----------|
| 1 | OAuth bridge (API key on login) | Team members syncing | Sand (4h) |
| 2 | Teams/invites | Multi-person collaboration | Rock (1-2d) |
| 3 | Agent role scoping | Security hardening | Sand (3h) |

Total: ~3 days. Unblocks team sync + dashboard access for all team members.

## What This Does NOT Cover

- x402 crypto payments (separate proposal, not needed for team sync)
- MCP auth protocol (wait for spec to stabilize)
- Wallet provisioning (only needed if x402 is adopted)
- SSO/SAML (enterprise feature, post-1.0)
