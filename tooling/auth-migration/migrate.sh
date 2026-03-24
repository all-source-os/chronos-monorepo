#!/usr/bin/env bash
# Migrate existing users from Control Plane's Core-backed store to Auth Service.
#
# Reads user events from AllSource Core, creates equivalent auth-user events
# via the Auth Service's demo/provisioning endpoints.
#
# Usage: ./tooling/auth-migration/migrate.sh [options]
#   --core-url URL      AllSource Core URL (default: http://localhost:3900)
#   --auth-url URL      Auth Service URL (default: http://localhost:3903)
#   --api-key KEY       AllSource API key
#   --dry-run           Print what would be migrated without actually doing it

set -euo pipefail

CORE_URL="${CORE_URL:-http://localhost:3900}"
AUTH_URL="${AUTH_URL:-http://localhost:3903}"
API_KEY="${ALLSOURCE_API_KEY:-}"
DRY_RUN=false

while [[ $# -gt 0 ]]; do
  case $1 in
    --core-url) CORE_URL="$2"; shift 2 ;;
    --auth-url) AUTH_URL="$2"; shift 2 ;;
    --api-key) API_KEY="$2"; shift 2 ;;
    --dry-run) DRY_RUN=true; shift ;;
    *) echo "Unknown option: $1"; exit 1 ;;
  esac
done

echo "=== Auth Migration ==="
echo "Core: $CORE_URL"
echo "Auth: $AUTH_URL"
echo "Dry run: $DRY_RUN"
echo ""

# Auth headers
AUTH_HEADER=""
if [ -n "$API_KEY" ]; then
  AUTH_HEADER="-H 'Authorization: Bearer $API_KEY'"
fi

# 1. Fetch existing users from Control Plane's event format
echo "Fetching existing users from Core..."
USERS=$(curl -sf "$CORE_URL/api/v1/events/query?event_type=UserCreated&limit=1000" \
  ${AUTH_HEADER:+-H "Authorization: Bearer $API_KEY"} || echo '{"events":[]}')

USER_COUNT=$(echo "$USERS" | python3 -c "import sys,json; print(len(json.load(sys.stdin).get('events',[])))" 2>/dev/null || echo "0")
echo "Found $USER_COUNT user events"

if [ "$USER_COUNT" = "0" ]; then
  echo "No users to migrate. Checking for auth-user events..."
  AUTH_USERS=$(curl -sf "$CORE_URL/api/v1/events/query?event_type=UserCreated&entity_id_prefix=auth-user&limit=1000" \
    ${AUTH_HEADER:+-H "Authorization: Bearer $API_KEY"} || echo '{"events":[]}')
  AUTH_COUNT=$(echo "$AUTH_USERS" | python3 -c "import sys,json; print(len(json.load(sys.stdin).get('events',[])))" 2>/dev/null || echo "0")
  echo "Found $AUTH_COUNT auth-user events (already migrated or using new format)"
  echo ""
  echo "Nothing to migrate."
  exit 0
fi

# 2. Process each user
MIGRATED=0
SKIPPED=0
ERRORS=0

echo "$USERS" | python3 -c "
import sys, json
events = json.load(sys.stdin).get('events', [])
for e in events:
    p = e.get('payload', {})
    email = p.get('email', '')
    name = p.get('name', '')
    entity_id = e.get('entity_id', '')
    print(f'{email}|{name}|{entity_id}')
" 2>/dev/null | while IFS='|' read -r email name entity_id; do
  if [ -z "$email" ]; then
    SKIPPED=$((SKIPPED + 1))
    continue
  fi

  echo -n "  Migrating: $email ($name)... "

  if [ "$DRY_RUN" = true ]; then
    echo "DRY RUN (would create auth-user)"
    MIGRATED=$((MIGRATED + 1))
    continue
  fi

  # Create user via Auth Service sign-up endpoint
  RESULT=$(curl -sf -X POST "$AUTH_URL/api/auth/sign-up/email" \
    -H "Content-Type: application/json" \
    -d "{\"email\": \"$email\", \"name\": \"$name\", \"password\": \"migrated-$(uuidgen | tr '[:upper:]' '[:lower:]')\"}" \
    2>&1 || echo '{"error": "request failed"}')

  if echo "$RESULT" | grep -q '"error"'; then
    if echo "$RESULT" | grep -q "already exists"; then
      echo "SKIPPED (already exists)"
      SKIPPED=$((SKIPPED + 1))
    else
      echo "ERROR: $RESULT"
      ERRORS=$((ERRORS + 1))
    fi
  else
    echo "OK"
    MIGRATED=$((MIGRATED + 1))
  fi
done

echo ""
echo "=== Migration Complete ==="
echo "Migrated: $MIGRATED"
echo "Skipped: $SKIPPED"
echo "Errors: $ERRORS"
