# Agent Authentication & Registration Design

## Status: Proposal
## Date: 2026-03-27

## Problem

AI agents need to programmatically create accounts and authenticate with AllSource. The current flow requires 4 steps across 3 services — too much friction for autonomous agents.

## What Exists Today

| Method | How | Good for Agents? |
|--------|-----|-------------------|
| **API Keys** | `POST /api/api-keys` → `X-API-Key` header | Yes, but requires an authenticated user first |
| **Demo flow** | `POST /api/auth/demo/start` → temp credentials | Quick start, but ephemeral |
| **Email signup** | Auth Service `POST /api/auth/sign-up/email` | Possible, but designed for humans |
| **Dev token** | `GET /api/auth/dev-token` (AUTH_DISABLED=true) | Dev only, not production |
| **Core bootstrap key** | `ALLSOURCE_BOOTSTRAP_API_KEY` env var | Operator-only, not self-service |

## The Gap

An AI agent today would need to:
1. Call Auth Service to create an account (email/password signup)
2. Get a JWT back
3. Have a tenant created via Control Plane
4. Use that JWT to create an API key
5. Use the API key going forward

That's 4 steps across 3 services.

## Options

### Option A: Agent Registration Endpoint (Recommended)

A single endpoint that does everything in one call:

```
POST /api/v1/agents/register
{
  "agent_name": "my-claude-code-agent",
  "agent_type": "mcp" | "sdk" | "cli",
  "callback_url": "https://..." (optional, for webhooks)
}

→ 201 Created
{
  "tenant_id": "tnt_...",
  "api_key": "ask_...",
  "tier": "free",
  "quotas": { "events": 10000, "queries": 1000 },
  "core_url": "https://core.all-source.xyz",
  "query_url": "https://query.all-source.xyz"
}
```

Lives in the **Control Plane** (tenant/account authority). Internally:
1. Creates a tenant with `agent_type` metadata
2. Creates an API key scoped to that tenant
3. Returns everything the agent needs to start working

No email, no password, no OAuth — just a name and you're in.

### Option B: Enhanced Demo Flow

Extend `POST /api/auth/demo/start` to return a **persistent** API key instead of ephemeral credentials. Less work but muddies the demo concept.

### Option C: MCP-Native Auth (OAuth 2.1 Device Flow)

RFC 8628 Device Authorization Grant:
1. Agent requests device code from Auth Service
2. User approves in browser (one-time)
3. Agent polls for token
4. Gets long-lived API key

How GitHub CLI, Docker CLI, etc. handle it. Best UX for agents with a human operator.

### Option D: Pre-provisioned Keys (Simplest)

Users create API keys in the dashboard, paste them into agent config. Already implemented.

## Recommendation

**Start with D (already done) + A (agent registration endpoint).** Add C (device flow) later when MCP auth spec stabilizes.

## Payments

See: x402 protocol investigation (separate section below or linked doc).
