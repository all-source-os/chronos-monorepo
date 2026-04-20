#!/usr/bin/env bash
# verify-x402-flow.sh — Smoke test the x402 tier gate and payment flow
# against the deployed Control Plane at api.all-source.xyz.
#
# Run from any dev machine:
#   ./tooling/scripts/verify-x402-flow.sh
#
# What it tests:
#   1. /x402/routes is public (200, no auth)
#   2. Free-tier tenant on priced route → 403 tier_not_allowed
#   3. Free-tier tenant on unpriced route → 200 (no tier gate)
#   4. Unauthenticated on priced route → 401
#
# What it does NOT test (requires LemonSqueezy + CDP secrets):
#   - Pro-tier tenant on priced route → 402 Payment Required
#   - Auto-pay settlement with CDP wallet → 200
#   Those are blocked until Phase A credentials are set up.
#
# Usage:
#   ./tooling/scripts/verify-x402-flow.sh [BASE_URL]
#
# Default BASE_URL: https://api.all-source.xyz

set -euo pipefail

BASE_URL="${1:-https://api.all-source.xyz}"
PASS=0
FAIL=0
TESTS=()

pass() { PASS=$((PASS + 1)); TESTS+=("PASS: $1"); echo "  ✓ $1"; }
fail() { FAIL=$((FAIL + 1)); TESTS+=("FAIL: $1"); echo "  ✗ $1"; }

echo "=== x402 Flow Verification ==="
echo "Target: $BASE_URL"
echo ""

# --- Test 1: /x402/routes is public ---
echo "--- Test 1: x402 route discovery (public, no auth) ---"
STATUS=$(curl -sS -o /tmp/x402-routes.json -w "%{http_code}" "$BASE_URL/x402/routes")
if [ "$STATUS" = "200" ]; then
  if grep -q "agent-echo" /tmp/x402-routes.json 2>/dev/null; then
    pass "/x402/routes → 200 with agent-echo route listed"
  else
    fail "/x402/routes → 200 but agent-echo route missing from response"
  fi
else
  fail "/x402/routes → HTTP $STATUS (expected 200)"
fi

# --- Test 2: Create free-tier tenant ---
echo ""
echo "--- Test 2: Create free-tier test tenant ---"
ONBOARD_RESP=$(curl -sS -w "\n%{http_code}" -X POST "$BASE_URL/api/v1/onboard/start" \
  -H "content-type: application/json" \
  -d "{\"email\":\"x402-verify-$(date +%s)@test.all-source.xyz\",\"name\":\"x402 Verify\"}")
ONBOARD_STATUS=$(echo "$ONBOARD_RESP" | tail -1)
ONBOARD_BODY=$(echo "$ONBOARD_RESP" | sed '$d')

if [ "$ONBOARD_STATUS" = "201" ]; then
  API_KEY=$(echo "$ONBOARD_BODY" | python3 -c "import sys,json; print(json.load(sys.stdin)['api_key'])" 2>/dev/null || true)
  TIER=$(echo "$ONBOARD_BODY" | python3 -c "import sys,json; print(json.load(sys.stdin).get('tier',''))" 2>/dev/null || true)
  if [ -n "$API_KEY" ] && [ "$TIER" = "free" ]; then
    pass "Onboard → 201, free-tier tenant created with API key"
  else
    fail "Onboard → 201 but couldn't extract api_key or tier != free"
  fi
else
  fail "Onboard → HTTP $ONBOARD_STATUS (expected 201)"
  echo "  Response: $ONBOARD_BODY"
  echo ""
  echo "Cannot continue without a test tenant. Aborting."
  exit 1
fi

# --- Test 3: Free-tier on priced route → 403 ---
echo ""
echo "--- Test 3: Free-tier tenant on priced route (POST /api/v1/agent-echo) ---"
ECHO_RESP=$(curl -sS -w "\n%{http_code}" -X POST "$BASE_URL/api/v1/agent-echo" \
  -H "Authorization: Bearer $API_KEY" \
  -H "content-type: application/json" \
  -d '{"test":"tier-gate"}')
ECHO_STATUS=$(echo "$ECHO_RESP" | tail -1)
ECHO_BODY=$(echo "$ECHO_RESP" | sed '$d')

if [ "$ECHO_STATUS" = "403" ]; then
  if echo "$ECHO_BODY" | grep -q "tier_not_allowed" 2>/dev/null; then
    pass "Priced route → 403 tier_not_allowed (free-tier correctly blocked)"
  else
    fail "Priced route → 403 but body doesn't contain tier_not_allowed"
  fi
else
  fail "Priced route → HTTP $ECHO_STATUS (expected 403)"
  echo "  Body: $ECHO_BODY"
fi

# --- Test 4: Free-tier on unpriced route → 200 ---
echo ""
echo "--- Test 4: Free-tier tenant on unpriced route (GET /api/v1/health/core) ---"
HEALTH_STATUS=$(curl -sS -o /dev/null -w "%{http_code}" "$BASE_URL/api/v1/health/core" \
  -H "Authorization: Bearer $API_KEY")

if [ "$HEALTH_STATUS" = "200" ]; then
  pass "Unpriced route → 200 (tier gate did not fire)"
else
  fail "Unpriced route → HTTP $HEALTH_STATUS (expected 200)"
fi

# --- Test 5: Unauthenticated on priced route → 401 ---
echo ""
echo "--- Test 5: Unauthenticated request on priced route ---"
UNAUTH_STATUS=$(curl -sS -o /dev/null -w "%{http_code}" -X POST "$BASE_URL/api/v1/agent-echo" \
  -H "content-type: application/json" \
  -d '{"test":"no-auth"}')

if [ "$UNAUTH_STATUS" = "401" ]; then
  pass "Unauth priced route → 401 (auth middleware catches first)"
else
  fail "Unauth priced route → HTTP $UNAUTH_STATUS (expected 401)"
fi

# --- Summary ---
echo ""
echo "=== Summary ==="
TOTAL=$((PASS + FAIL))
for t in "${TESTS[@]}"; do echo "  $t"; done
echo ""
echo "  $PASS/$TOTAL passed"
if [ "$FAIL" -gt 0 ]; then
  echo "  $FAIL FAILED"
  exit 1
else
  echo "  All tests passed."
fi

# --- Not yet testable ---
echo ""
echo "=== Not yet testable (blocked on Phase A credentials) ==="
echo "  - Pro-tier tenant on priced route → 402 Payment Required"
echo "    (requires LemonSqueezy Pro variant + VARIANT_MAP)"
echo "  - Auto-pay settlement with CDP wallet → 200"
echo "    (requires CDP_API_KEY_NAME + CDP_API_KEY_PRIVATE_KEY)"
