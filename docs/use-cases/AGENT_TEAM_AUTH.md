# Use Case: AI Agents Syncing Tasks to an AllSource Team Tenant

This document describes how AI agents (e.g., `ralph-tui`, custom LLM workers) authenticate
against AllSource Core and sync `chronis` task events into a shared team workspace.

---

## Overview

AllSource issues two kinds of API credentials:

| Credential | Format | Issued by | Used by |
|---|---|---|---|
| **CP JWT** | `eyJ...` (signed JWT) | Control Plane at login | Dashboard browser sessions, human API clients |
| **Core API key** | `ask_...` (opaque token) | Core `/api/v1/auth/api-keys` | `chronis` CLI, AI agents, automation scripts |

Human team members receive a CP JWT on OAuth login. That JWT embeds a `core_api_key` field so
the dashboard can proxy event data transparently.

AI agents never go through OAuth. They use **dedicated `ask_...` keys** provisioned per agent
by a team Admin or Owner, scoped to the team's Core tenant.

---

## End-to-End Flow

### 1. Human login (automatic provisioning)

```
Browser → OAuth provider → Control Plane callback
  → provisionCoreAPIKey(tenantID, userID)
      → check Core config store: user:{userID}:core_api_key
      → if missing: POST /api/v1/auth/api-keys (ServiceAccount, team tenant)
      → cache key in Core config
  → embed core_api_key in CP JWT
  → return JWT to browser (auth_token cookie)
```

The dashboard reads `core_api_key` from the session response (`GET /api/auth/session`) and
can use it for direct Core queries when needed.

### 2. Agent key provisioning (manual, one-time per agent)

A team Admin or Owner creates a named key for each agent from **Settings → Team → Agent Keys**:

```
Dashboard → POST /api/team/agent-keys  { "name": "ralph-tui-prod" }
  → Control Plane CreateAgentKeyHandler
      → POST /api/v1/auth/api-keys on Core (ServiceAccount role, team tenant)
      → store AgentKeyMeta { name, key_id, created_at } in Core config
        key: team:{tenantID}:agent_keys
      → return { name, key, tenant_id, created_at }  ← key shown ONCE, never stored
```

The raw key is returned **exactly once**. Copy it immediately.

### 3. Agent configuration

Add the key to the agent's `chronis` config:

```toml
# .chronis/config.toml
mode = "remote"

[sync]
remote_url = "https://core.allsource.io"
api_key    = "ask_<your-key-here>"
```

Or via the CLI:

```bash
cn init --remote https://core.allsource.io --api-key ask_<your-key-here>
```

From this point the agent writes and reads task events directly against Core with no
dashboard involvement.

### 4. Agent runtime

```
ralph-tui / custom agent
  → reads api_key from .chronis/config.toml
  → POST https://core.allsource.io/api/v1/events
      Authorization: Bearer ask_<key>
  → Core validates key: ServiceAccount role, resolves tenant from key record
  → event written to tenant's event stream
```

All agents sharing the same team tenant write to the same event stream. Human team members
can query that stream from the dashboard in real time.

---

## Permissions Matrix

| Operation | CP JWT (human) | ask_... (agent) |
|---|---|---|
| Ingest events (`POST /api/v1/events`) | via proxy | direct |
| Query events (`GET /api/v1/events/query`) | via proxy | direct |
| Manage schemas | via proxy | direct |
| List/manage team members | CP only | blocked |
| Create/revoke agent keys | CP only | blocked |
| Access Core config store | Admin JWT only | blocked |

Agent keys (`ask_...`) have `ServiceAccount` role: Read + Write on event data only. They
cannot create tenants, manage users, or access the config store.

---

## Key Lifecycle

### Listing keys

```
GET /api/team/agent-keys
→ returns [ { name, key_id, created_at }, ... ]
```

Key IDs are shown truncated in the dashboard. The raw `ask_...` value is never retrievable
after initial creation.

### Revoking a key

```
DELETE /api/team/agent-keys/:name
→ Control Plane RevokeAgentKeyHandler
    → DELETE /api/v1/auth/api-keys/:key_id on Core
    → removes AgentKeyMeta from Core config
```

After revocation, any agent using that key will receive `401 Unauthorized`. Update the
agent's `.chronis/config.toml` with a new key provisioned from the dashboard.

### Rotation workflow

1. In the dashboard, create a new key with a new name (e.g., `ralph-tui-prod-2`)
2. Update `.chronis/config.toml` on the agent with the new key
3. Verify the agent syncs successfully
4. Revoke the old key from the dashboard

---

## Multiple Agents, One Tenant

All agent keys provisioned under a team tenant write to the **same** Core event stream.
This is intentional: the team tenant is the shared workspace.

```
ralph-tui (agent key A) ──┐
custom-llm (agent key B) ─┤──→ team tenant event stream ──→ dashboard
ci-bot (agent key C) ──────┘
```

If you need per-agent isolation, create a separate team tenant for that agent.

---

## Security Notes

- **Keys are never stored server-side in plaintext.** Only `key_id`, `name`, and `created_at`
  are persisted in Core's config store. The raw `ask_...` value exists only during creation.
- **One key per agent.** Name keys descriptively (e.g., `ralph-tui-staging`,
  `deploy-bot-prod`) so you know which key to revoke if an agent is compromised.
- **Revoke immediately** if a key is leaked. Rotation is safe and fast (see above).
- **Keys are scoped to the team tenant.** A key cannot be used to access another team's
  data even if the `tenant_id` is guessed.

---

## Dashboard UI

**Settings → Team → Agent Keys**

- **New Key** button: opens a name input; press Enter or click Create
- Key value shown once in a green callout with copy button and ready-to-paste config snippet
- Key table: name, truncated key ID, creation date, revoke button
- Empty state guides admins to create their first key
