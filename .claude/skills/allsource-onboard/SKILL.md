---
name: allsource-onboard
description: Create an AllSource account and API key for chronis sync. Registers on the hosted Core, creates an API key, and configures .chronis/config.toml in the current project. Triggers on "onboard allsource", "setup sync", "get api key", "connect to allsource", "cn sync setup".
---

# AllSource Onboard — Account + Sync Setup

Creates an account on the hosted AllSource Core, generates an API key, and configures the current project's `.chronis/config.toml` for `cn sync`.

## Prerequisites

- `chronis` CLI installed (`cargo install chronis`)
- `.chronis/` workspace initialized (`cn init`)

## Procedure

### 1. Register an account

```bash
CORE_URL="https://allsource-core.fly.dev"

# Generate a unique username from the git user or project name
USERNAME=$(git config user.name | tr ' ' '-' | tr '[:upper:]' '[:lower:]' || echo "user-$(date +%s)")
EMAIL=$(git config user.email || echo "${USERNAME}@allsource.local")
PASSWORD=$(openssl rand -hex 16)

# Register
REGISTER_RESPONSE=$(curl -s -X POST "$CORE_URL/api/v1/auth/register" \
  -H "Content-Type: application/json" \
  -d "{\"username\":\"$USERNAME\",\"password\":\"$PASSWORD\",\"email\":\"$EMAIL\"}")

echo "$REGISTER_RESPONSE"
```

If registration returns a `user_id`, proceed. If it returns an error like `"username already exists"`, the user already has an account — skip to login.

### 2. Login to get a JWT

```bash
TOKEN=$(curl -s -X POST "$CORE_URL/api/v1/auth/login" \
  -H "Content-Type: application/json" \
  -d "{\"username\":\"$USERNAME\",\"password\":\"$PASSWORD\"}" | python3 -c "import sys,json; print(json.load(sys.stdin)['token'])")
```

### 3. Create an API key for sync

```bash
PROJECT_NAME=$(basename "$(git rev-parse --show-toplevel 2>/dev/null || pwd)")

API_KEY_RESPONSE=$(curl -s -X POST "$CORE_URL/api/v1/auth/api-keys" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d "{\"name\":\"chronis-sync-${PROJECT_NAME}\"}")

API_KEY=$(echo "$API_KEY_RESPONSE" | python3 -c "import sys,json; print(json.load(sys.stdin)['key'])")
echo "API Key: $API_KEY"
```

### 4. Configure chronis sync

Write the sync config to `.chronis/config.toml`:

```bash
INSTANCE_ID=$(basename "$(git rev-parse --show-toplevel 2>/dev/null || pwd)")

cat > .chronis/config.toml << EOF
mode = "embedded"
instance_id = "$INSTANCE_ID"

[sync]
remote_url = "$CORE_URL"
api_key = "$API_KEY"
EOF
```

### 5. Verify sync works

```bash
cn sync --toon
```

Expected output: `Already up to date.` or `Pushed N events to remote.`

### 6. Report to user

Tell the user:
- Account created: `$USERNAME` / `$EMAIL`
- API key name: `chronis-sync-$PROJECT_NAME`
- Sync configured in `.chronis/config.toml`
- Run `cn sync` to push/pull tasks

**IMPORTANT:** The API key is stored in `.chronis/config.toml`. This file should be in `.gitignore` (chronis init creates one). Never commit API keys to git.

## Sharing with team members

Each team member runs this same skill in their project. They can either:
1. Create their own account (run this skill)
2. Use a shared team API key (copy from a teammate's config)

For shared keys, just copy the `[sync]` section from an existing `.chronis/config.toml`:
```toml
[sync]
remote_url = "https://allsource-core.fly.dev"
api_key = "ask_..."
```

## Troubleshooting

| Error | Cause | Fix |
|-------|-------|-----|
| `username already exists` | Account exists | Login with existing credentials |
| `401 Unauthorized` on sync | API key invalid or expired | Re-create API key (step 3) |
| `cannot reach remote Core` | Core down or URL wrong | Check `curl $CORE_URL/health` |
| `Rate limit exceeded` | Too many sync events | Wait 60s and retry |
| Archived beads reappear after sync | Chronis < 0.6.2 | `cargo install chronis` to update |
