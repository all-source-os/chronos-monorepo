#!/usr/bin/env bash
# =============================================================================
# Chronos Data Flow Test
# =============================================================================
# Tests the full data flow through the Chronos Docker stack:
#   Core (port 3280) <- Query Service (port 3283) <- MCP (port 3904)
#
# Usage:
#   ./test-data-flow.sh              # Run all tests
#   ./test-data-flow.sh --state      # Show container states only
#   ./test-data-flow.sh --core       # Test Core only
#   ./test-data-flow.sh --query      # Test Query Service only
#   ./test-data-flow.sh --mcp        # Test MCP only
#   ./test-data-flow.sh --flow       # Test end-to-end data flow
#   ./test-data-flow.sh --json       # Output results as JSON
#
# Environment:
#   CORE_PORT      Core external port     (default: 3280)
#   QUERY_PORT     Query Service port     (default: 3283)
#   MCP_PORT       MCP server port        (default: 3904)
#   COMPOSE_FILE   Docker compose file    (auto-detected)
# =============================================================================

set -euo pipefail

# ---------------------------------------------------------------------------
# Configuration
# ---------------------------------------------------------------------------
CORE_PORT="${CORE_PORT:-3280}"
QUERY_PORT="${QUERY_PORT:-3283}"
MCP_PORT="${MCP_PORT:-3904}"
CORE_URL="http://localhost:${CORE_PORT}"
QUERY_URL="http://localhost:${QUERY_PORT}"
MCP_URL="http://localhost:${MCP_PORT}"
CURL_TIMEOUT=5
JSON_OUTPUT=false

# Compose file auto-detection
COMPOSE_FILE="${COMPOSE_FILE:-}"
if [[ -z "$COMPOSE_FILE" ]]; then
  for candidate in \
    "./docker-compose.chronos.yml" \
    "../docker-compose.chronos.yml" \
    "$HOME/Projects/alphaSigmaPro/wallet/docker/docker-compose.chronos.yml"; do
    if [[ -f "$candidate" ]]; then
      COMPOSE_FILE="$candidate"
      break
    fi
  done
fi

# ---------------------------------------------------------------------------
# Colors & formatting
# ---------------------------------------------------------------------------
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
BOLD='\033[1m'
DIM='\033[2m'
NC='\033[0m'

PASS=0
FAIL=0
SKIP=0
RESULTS=()

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------
header() {
  echo ""
  echo -e "${BOLD}${BLUE}=== $1 ===${NC}"
}

subheader() {
  echo -e "${CYAN}--- $1 ---${NC}"
}

check() {
  local name="$1"
  local status="$2"  # pass, fail, skip
  local detail="${3:-}"

  case "$status" in
    pass)
      echo -e "  ${GREEN}PASS${NC}  $name${DIM}${detail:+ ($detail)}${NC}"
      PASS=$((PASS + 1))
      RESULTS+=("{\"test\":\"$name\",\"status\":\"pass\",\"detail\":\"$detail\"}")
      ;;
    fail)
      echo -e "  ${RED}FAIL${NC}  $name${DIM}${detail:+ ($detail)}${NC}"
      FAIL=$((FAIL + 1))
      RESULTS+=("{\"test\":\"$name\",\"status\":\"fail\",\"detail\":\"$detail\"}")
      ;;
    skip)
      echo -e "  ${YELLOW}SKIP${NC}  $name${DIM}${detail:+ ($detail)}${NC}"
      SKIP=$((SKIP + 1))
      RESULTS+=("{\"test\":\"$name\",\"status\":\"skip\",\"detail\":\"$detail\"}")
      ;;
  esac
}

# curl wrappers: set global HTTP_CODE and HTTP_BODY
# Call directly (NOT inside $(...)) so globals propagate
HTTP_CODE=0
HTTP_BODY=""

http_get() {
  local url="$1"
  HTTP_CODE=0
  HTTP_BODY=""
  local response
  response=$(curl -s -w "\n%{http_code}" --max-time "$CURL_TIMEOUT" "$url" 2>/dev/null) || {
    HTTP_CODE=0
    HTTP_BODY=""
    return 1
  }
  HTTP_CODE=$(echo "$response" | tail -1)
  HTTP_BODY=$(echo "$response" | sed '$d')
  return 0
}

http_post() {
  local url="$1"
  local data="$2"
  local content_type="${3:-application/json}"
  HTTP_CODE=0
  HTTP_BODY=""
  local response
  response=$(curl -s -w "\n%{http_code}" --max-time "$CURL_TIMEOUT" \
    -X POST -H "Content-Type: $content_type" -d "$data" "$url" 2>/dev/null) || {
    HTTP_CODE=0
    HTTP_BODY=""
    return 1
  }
  HTTP_CODE=$(echo "$response" | tail -1)
  HTTP_BODY=$(echo "$response" | sed '$d')
  return 0
}

# Check if a port is reachable
port_open() {
  nc -z localhost "$1" 2>/dev/null
}

# Extract JSON field (lightweight, no jq dependency)
json_field() {
  local json="$1"
  local field="$2"
  echo "$json" | grep -o "\"$field\"[[:space:]]*:[[:space:]]*\"[^\"]*\"" | head -1 | sed 's/.*: *"\([^"]*\)".*/\1/'
}

json_field_raw() {
  local json="$1"
  local field="$2"
  echo "$json" | grep -o "\"$field\"[[:space:]]*:[[:space:]]*[^,}]*" | head -1 | sed 's/.*: *//'
}

# ---------------------------------------------------------------------------
# Container State Detection
# ---------------------------------------------------------------------------
CORE_UP=false
QUERY_UP=false
MCP_UP=false

show_container_state() {
  header "Container State"

  local core_state="down" query_state="down" mcp_state="down" pg_state="down"

  if port_open "$CORE_PORT"; then core_state="up"; fi
  if port_open "$QUERY_PORT"; then query_state="up"; fi
  if port_open "$MCP_PORT"; then mcp_state="up"; fi
  if port_open 3284; then pg_state="up"; fi

  echo -e "  Core           (port $CORE_PORT):  $(if [[ $core_state == "up" ]]; then echo -e "${GREEN}UP${NC}"; else echo -e "${RED}DOWN${NC}"; fi)"
  echo -e "  Query Service  (port $QUERY_PORT):  $(if [[ $query_state == "up" ]]; then echo -e "${GREEN}UP${NC}"; else echo -e "${RED}DOWN${NC}"; fi)"
  echo -e "  MCP Server     (port $MCP_PORT):  $(if [[ $mcp_state == "up" ]]; then echo -e "${GREEN}UP${NC}"; else echo -e "${RED}DOWN${NC}"; fi)"
  echo -e "  PostgreSQL     (port 3284):  $(if [[ $pg_state == "up" ]]; then echo -e "${GREEN}UP${NC}"; else echo -e "${DIM}not configured (standalone)${NC}"; fi)"

  if [[ -n "$COMPOSE_FILE" ]]; then
    echo -e "  ${DIM}Compose file: $COMPOSE_FILE${NC}"
  fi

  if [[ "$query_state" == "up" && "$pg_state" != "up" ]]; then
    echo -e "\n  ${BOLD}Mode: Standalone${NC} (no PostgreSQL)"
  elif [[ "$query_state" == "up" && "$pg_state" == "up" ]]; then
    echo -e "\n  ${BOLD}Mode: Full${NC} (with PostgreSQL)"
  fi

  CORE_UP=$([[ "$core_state" == "up" ]] && echo true || echo false)
  QUERY_UP=$([[ "$query_state" == "up" ]] && echo true || echo false)
  MCP_UP=$([[ "$mcp_state" == "up" ]] && echo true || echo false)
}

# ---------------------------------------------------------------------------
# Core Tests
# ---------------------------------------------------------------------------
test_core() {
  header "Core (${CORE_URL})"

  if [[ "$CORE_UP" != "true" ]]; then
    check "Core reachable" skip "port $CORE_PORT not open"
    return
  fi

  subheader "Health"
  http_get "${CORE_URL}/health" || true
  if [[ "$HTTP_CODE" == "200" ]]; then
    local status version
    status=$(json_field "$HTTP_BODY" "status")
    version=$(json_field "$HTTP_BODY" "version")
    check "GET /health" pass "${status:-ok}, ${version:-unknown}"
  else
    check "GET /health" fail "HTTP $HTTP_CODE"
  fi

  http_get "${CORE_URL}/api/v1/stats" || true
  if [[ "$HTTP_CODE" == "200" ]]; then
    local total_events
    total_events=$(json_field_raw "$HTTP_BODY" "total_events")
    check "GET /api/v1/stats" pass "total_events=${total_events:-0}"
  else
    check "GET /api/v1/stats" fail "HTTP $HTTP_CODE"
  fi

  subheader "API Endpoints"
  for endpoint in \
    "/api/v1/events/query" \
    "/api/v1/projections" \
    "/api/v1/schemas" \
    "/api/v1/snapshots" \
    "/api/v1/streams" \
    "/api/v1/event-types"; do
    http_get "${CORE_URL}${endpoint}" || true
    if [[ "$HTTP_CODE" == "200" ]]; then
      check "GET ${endpoint}" pass "HTTP 200"
    else
      check "GET ${endpoint}" fail "HTTP $HTTP_CODE"
    fi
  done
}

# ---------------------------------------------------------------------------
# Query Service Tests
# ---------------------------------------------------------------------------
test_query_service() {
  header "Query Service (${QUERY_URL})"

  if [[ "$QUERY_UP" != "true" ]]; then
    check "Query Service reachable" skip "port $QUERY_PORT not open"
    return
  fi

  subheader "Health"
  http_get "${QUERY_URL}/api/health" || true
  if [[ "$HTTP_CODE" == "200" ]]; then
    local status
    status=$(json_field "$HTTP_BODY" "status")
    check "GET /api/health" pass "$status"
  elif [[ "$HTTP_CODE" == "503" ]]; then
    local status
    status=$(json_field "$HTTP_BODY" "status")
    check "GET /api/health" fail "degraded ($status)"
  else
    check "GET /api/health" fail "HTTP $HTTP_CODE"
  fi

  http_get "${QUERY_URL}/api/health/live" || true
  if [[ "$HTTP_CODE" == "200" ]]; then
    check "GET /api/health/live" pass "alive"
  else
    check "GET /api/health/live" fail "HTTP $HTTP_CODE"
  fi

  http_get "${QUERY_URL}/api/health/ready" || true
  if [[ "$HTTP_CODE" == "200" ]]; then
    check "GET /api/health/ready" pass "ready"
  elif [[ "$HTTP_CODE" == "503" ]]; then
    check "GET /api/health/ready" fail "not ready"
  else
    check "GET /api/health/ready" fail "HTTP $HTTP_CODE"
  fi

  subheader "Tenant"
  http_get "${QUERY_URL}/api/tenant" || true
  if [[ "$HTTP_CODE" == "200" ]]; then
    local tier
    tier=$(json_field "$HTTP_BODY" "subscription_tier")
    check "GET /api/tenant" pass "tier=${tier:-unknown}"
  else
    check "GET /api/tenant" fail "HTTP $HTTP_CODE"
  fi

  http_get "${QUERY_URL}/api/tenant/usage" || true
  if [[ "$HTTP_CODE" == "200" ]]; then
    check "GET /api/tenant/usage" pass "HTTP 200"
  else
    check "GET /api/tenant/usage" fail "HTTP $HTTP_CODE"
  fi

  subheader "Event Proxy"
  http_get "${QUERY_URL}/api/events" || true
  if [[ "$HTTP_CODE" == "200" ]]; then
    local count
    count=$(json_field_raw "$HTTP_BODY" "count")
    check "GET /api/events" pass "count=${count:-0}"
  else
    check "GET /api/events" fail "HTTP $HTTP_CODE"
  fi

  http_get "${QUERY_URL}/api/streams" || true
  if [[ "$HTTP_CODE" == "200" ]]; then
    check "GET /api/streams" pass "HTTP 200"
  else
    check "GET /api/streams" fail "HTTP $HTTP_CODE"
  fi

  http_get "${QUERY_URL}/api/event-types" || true
  if [[ "$HTTP_CODE" == "200" ]]; then
    check "GET /api/event-types" pass "HTTP 200"
  else
    check "GET /api/event-types" fail "HTTP $HTTP_CODE"
  fi

  http_get "${QUERY_URL}/api/openapi" || true
  if [[ "$HTTP_CODE" == "200" ]]; then
    check "GET /api/openapi" pass "spec available"
  else
    check "GET /api/openapi" fail "HTTP $HTTP_CODE"
  fi
}

# ---------------------------------------------------------------------------
# MCP Tests
# ---------------------------------------------------------------------------
test_mcp() {
  header "MCP Server (${MCP_URL})"

  if [[ "$MCP_UP" != "true" ]]; then
    check "MCP Server reachable" skip "port $MCP_PORT not open"
    return
  fi

  http_get "${MCP_URL}/health" || true
  if [[ "$HTTP_CODE" == "200" ]]; then
    check "GET /health" pass "HTTP 200"
  else
    check "GET /health" fail "HTTP $HTTP_CODE"
  fi
}

# ---------------------------------------------------------------------------
# End-to-End Data Flow Test
# ---------------------------------------------------------------------------
test_data_flow() {
  header "End-to-End Data Flow"

  if [[ "$CORE_UP" != "true" ]]; then
    check "Data flow" skip "Core is down"
    return
  fi
  if [[ "$QUERY_UP" != "true" ]]; then
    check "Data flow via Query Service" skip "Query Service is down -- testing Core directly"
  fi

  local test_entity="dataflow-test-$(date +%s)"
  local test_type="data_flow_test"

  # Step 1: Create event
  subheader "Write Path"
  local create_body
  create_body="{\"entity_id\":\"${test_entity}\",\"event_type\":\"${test_type}\",\"payload\":{\"test\":true,\"timestamp\":\"$(date -u +%Y-%m-%dT%H:%M:%SZ)\"}}"

  local write_ok=false
  if [[ "$QUERY_UP" == "true" ]]; then
    http_post "${QUERY_URL}/api/events" "$create_body" || true
    if [[ "$HTTP_CODE" == "201" ]]; then
      check "Create event via Query Service" pass "HTTP 201"
      write_ok=true
    else
      check "Create event via Query Service" fail "HTTP $HTTP_CODE"
      # Fall back to Core direct
      http_post "${CORE_URL}/api/v1/events" "$create_body" || true
      if [[ "$HTTP_CODE" == "200" ]]; then
        check "Create event via Core (fallback)" pass "HTTP 200"
        write_ok=true
      else
        check "Create event via Core (fallback)" fail "HTTP $HTTP_CODE"
      fi
    fi
  else
    http_post "${CORE_URL}/api/v1/events" "$create_body" || true
    if [[ "$HTTP_CODE" == "200" ]]; then
      check "Create event via Core" pass "HTTP 200"
      write_ok=true
    else
      check "Create event via Core" fail "HTTP $HTTP_CODE"
    fi
  fi

  if [[ "$write_ok" != "true" ]]; then
    return
  fi

  sleep 0.5

  # Step 2: Read from Core
  subheader "Read Path"
  http_get "${CORE_URL}/api/v1/events/query?entity_id=${test_entity}" || true
  if [[ "$HTTP_CODE" == "200" ]]; then
    local count
    count=$(json_field_raw "$HTTP_BODY" "count")
    if [[ "${count:-0}" -ge 1 ]]; then
      check "Read event from Core" pass "count=${count}"
    else
      check "Read event from Core" fail "event not found (count=${count:-0})"
    fi
  else
    check "Read event from Core" fail "HTTP $HTTP_CODE"
  fi

  # Step 3: Read via Query Service
  if [[ "$QUERY_UP" == "true" ]]; then
    http_get "${QUERY_URL}/api/events?entity_id=${test_entity}" || true
    if [[ "$HTTP_CODE" == "200" ]]; then
      local count
      count=$(json_field_raw "$HTTP_BODY" "count")
      if [[ "${count:-0}" -ge 1 ]]; then
        check "Read event via Query Service" pass "count=${count}"
      else
        check "Read event via Query Service" fail "event not found (count=${count:-0})"
      fi
    else
      check "Read event via Query Service" fail "HTTP $HTTP_CODE"
    fi
  fi

  # Step 4: Verify entity in streams
  if [[ "$QUERY_UP" == "true" ]]; then
    http_get "${QUERY_URL}/api/streams" || true
    if [[ "$HTTP_CODE" == "200" ]]; then
      if echo "$HTTP_BODY" | grep -q "$test_entity"; then
        check "Entity in streams list" pass "found"
      else
        check "Entity in streams list" fail "not found in response"
      fi
    else
      check "Entity in streams list" fail "HTTP $HTTP_CODE"
    fi
  fi

  # Step 5: Verify event type in types
  if [[ "$QUERY_UP" == "true" ]]; then
    http_get "${QUERY_URL}/api/event-types" || true
    if [[ "$HTTP_CODE" == "200" ]]; then
      if echo "$HTTP_BODY" | grep -q "$test_type"; then
        check "Event type in types list" pass "found"
      else
        check "Event type in types list" fail "not found in response"
      fi
    else
      check "Event type in types list" fail "HTTP $HTTP_CODE"
    fi
  fi

  # Step 6: Query via DSL
  if [[ "$QUERY_UP" == "true" ]]; then
    subheader "Query DSL"
    local query_body="{\"from\":\"events\",\"where\":{\"op\":\"eq\",\"field\":\"entity_id\",\"value\":\"${test_entity}\"},\"limit\":10}"
    http_post "${QUERY_URL}/api/query" "$query_body" || true
    if [[ "$HTTP_CODE" == "200" ]]; then
      local count
      count=$(json_field_raw "$HTTP_BODY" "count")
      check "Query DSL (entity filter)" pass "count=${count:-0}"
    else
      check "Query DSL (entity filter)" fail "HTTP $HTTP_CODE"
    fi
  fi
}

# ---------------------------------------------------------------------------
# Summary
# ---------------------------------------------------------------------------
print_summary() {
  echo ""
  echo -e "${BOLD}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
  local total=$((PASS + FAIL + SKIP))
  echo -e "  ${GREEN}${PASS} passed${NC}  ${RED}${FAIL} failed${NC}  ${YELLOW}${SKIP} skipped${NC}  ${DIM}(${total} total)${NC}"
  echo -e "${BOLD}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"

  if [[ "$FAIL" -gt 0 ]]; then
    return 1
  fi
  return 0
}

print_json() {
  echo "{"
  echo "  \"summary\": {\"pass\": $PASS, \"fail\": $FAIL, \"skip\": $SKIP},"
  echo "  \"results\": ["
  local first=true
  for r in "${RESULTS[@]}"; do
    if [[ "$first" == "true" ]]; then
      first=false
    else
      echo ","
    fi
    printf "    %s" "$r"
  done
  echo ""
  echo "  ]"
  echo "}"
}

# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------
main() {
  local run_state=true
  local run_core=true
  local run_query=true
  local run_mcp=true
  local run_flow=true

  while [[ $# -gt 0 ]]; do
    case "$1" in
      --state)  run_core=false; run_query=false; run_mcp=false; run_flow=false ;;
      --core)   run_state=false; run_query=false; run_mcp=false; run_flow=false ;;
      --query)  run_state=false; run_core=false;  run_mcp=false; run_flow=false ;;
      --mcp)    run_state=false; run_core=false;  run_query=false; run_flow=false ;;
      --flow)   run_state=false; run_core=false;  run_query=false; run_mcp=false ;;
      --json)   JSON_OUTPUT=true ;;
      --help|-h)
        echo "Usage: $0 [--state|--core|--query|--mcp|--flow] [--json]"
        exit 0
        ;;
      *) echo "Unknown option: $1"; exit 1 ;;
    esac
    shift
  done

  echo -e "${BOLD}Chronos Data Flow Test${NC}"
  echo -e "${DIM}$(date -u +%Y-%m-%dT%H:%M:%SZ)${NC}"

  show_container_state

  if [[ "$run_core" == "true" ]]; then test_core; fi
  if [[ "$run_query" == "true" ]]; then test_query_service; fi
  if [[ "$run_mcp" == "true" ]]; then test_mcp; fi
  if [[ "$run_flow" == "true" ]]; then test_data_flow; fi

  if [[ "$JSON_OUTPUT" == "true" ]]; then
    print_json
  fi

  print_summary
}

main "$@"
