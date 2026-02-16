#!/usr/bin/env bash
# =============================================================================
# Chronos Durability Test
# =============================================================================
# Proves that events survive container restarts — the core durability guarantee
# of WAL + Parquet. Writes events directly to Core, restarts the container,
# and verifies all events are still present.
#
# Usage:
#   ./test-durability.sh                        # Docker local (default)
#   ./test-durability.sh --target fly           # Fly.io deployment
#   ./test-durability.sh --target compose       # Docker compose (internal ports)
#   ./test-durability.sh --count 50             # Write 50 events (default: 10)
#   ./test-durability.sh --skip-restart         # Write + verify only, no restart
#   ./test-durability.sh --json                 # JSON output
#
# Targets:
#   docker  (default)  Core=localhost:3280  restart via docker restart
#   compose            Core=localhost:3900  restart via docker restart
#   fly                Core=allsource-core.fly.dev  restart via fly machines
# =============================================================================

set -euo pipefail

# ---------------------------------------------------------------------------
# Configuration
# ---------------------------------------------------------------------------
TARGET="docker"
EVENT_COUNT=10
SKIP_RESTART=false
JSON_OUTPUT=false
CURL_TIMEOUT=10
RECOVERY_TIMEOUT=60
SETTLE_DELAY=2

# Target-specific defaults (set after arg parsing)
CORE_URL=""
QS_URL=""
CONTAINER_NAME=""

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
# Target Configuration
# ---------------------------------------------------------------------------
configure_target() {
  case "$TARGET" in
    docker)
      CORE_URL="${CORE_URL:-http://localhost:3280}"
      QS_URL="${QS_URL:-http://localhost:3283}"
      CONTAINER_NAME="allsource-core"
      ;;
    compose)
      CORE_URL="${CORE_URL:-http://localhost:3900}"
      QS_URL="${QS_URL:-http://localhost:3902}"
      CONTAINER_NAME="allsource-core"
      ;;
    fly)
      CORE_URL="${CORE_URL:-https://allsource-core.fly.dev}"
      QS_URL="${QS_URL:-https://allsource-query.fly.dev}"
      CONTAINER_NAME=""
      ;;
    *)
      echo "Unknown target: $TARGET (use docker, compose, or fly)"
      exit 1
      ;;
  esac
}

# ---------------------------------------------------------------------------
# Phase 1: Pre-flight
# ---------------------------------------------------------------------------
test_preflight() {
  header "Pre-flight Checks"

  # Core reachable
  http_get "${CORE_URL}/health" || true
  if [[ "$HTTP_CODE" == "200" ]]; then
    local version
    version=$(json_field "$HTTP_BODY" "version")
    check "Core reachable" pass "${CORE_URL} (${version:-unknown})"
  else
    check "Core reachable" fail "HTTP ${HTTP_CODE} at ${CORE_URL}"
    echo -e "  ${RED}Cannot proceed without Core. Aborting.${NC}"
    if [[ "$JSON_OUTPUT" == "true" ]]; then print_json; fi
    print_summary || true
    exit 1
  fi

  # Restart tooling available (unless --skip-restart)
  if [[ "$SKIP_RESTART" == "false" ]]; then
    if [[ "$TARGET" == "fly" ]]; then
      if command -v fly &>/dev/null || command -v flyctl &>/dev/null; then
        check "fly CLI available" pass
      else
        check "fly CLI available" fail "install flyctl"
        echo -e "  ${RED}Cannot restart without fly CLI. Use --skip-restart or install flyctl.${NC}"
        if [[ "$JSON_OUTPUT" == "true" ]]; then print_json; fi
        print_summary || true
        exit 1
      fi
    else
      if command -v docker &>/dev/null; then
        check "docker CLI available" pass
      else
        check "docker CLI available" fail "install docker"
        echo -e "  ${RED}Cannot restart without docker CLI. Use --skip-restart or install docker.${NC}"
        if [[ "$JSON_OUTPUT" == "true" ]]; then print_json; fi
        print_summary || true
        exit 1
      fi

      # Verify container exists
      if docker inspect "$CONTAINER_NAME" &>/dev/null; then
        check "Container '${CONTAINER_NAME}' exists" pass
      else
        check "Container '${CONTAINER_NAME}' exists" fail "not found"
        echo -e "  ${RED}Cannot restart container. Use --skip-restart or check container name.${NC}"
        if [[ "$JSON_OUTPUT" == "true" ]]; then print_json; fi
        print_summary || true
        exit 1
      fi
    fi
  fi
}

# ---------------------------------------------------------------------------
# Phase 2: Baseline
# ---------------------------------------------------------------------------
BASELINE_TOTAL_EVENTS=0

test_baseline() {
  header "Baseline Stats"

  http_get "${CORE_URL}/api/v1/stats" || true
  if [[ "$HTTP_CODE" == "200" ]]; then
    BASELINE_TOTAL_EVENTS=$(json_field_raw "$HTTP_BODY" "total_events")
    BASELINE_TOTAL_EVENTS="${BASELINE_TOTAL_EVENTS:-0}"
    check "Baseline stats" pass "total_events=${BASELINE_TOTAL_EVENTS}"
  else
    check "Baseline stats" skip "HTTP ${HTTP_CODE}"
  fi
}

# ---------------------------------------------------------------------------
# Phase 3: Write Events
# ---------------------------------------------------------------------------
TEST_ENTITY=""

test_write() {
  header "Write Events"

  TEST_ENTITY="durability-test-$(date +%s)"
  echo -e "  ${DIM}entity_id: ${TEST_ENTITY}${NC}"
  echo -e "  ${DIM}count: ${EVENT_COUNT}${NC}"

  local success=0
  local failed=0

  for i in $(seq 1 "$EVENT_COUNT"); do
    local body
    body="{\"entity_id\":\"${TEST_ENTITY}\",\"event_type\":\"durability_test\",\"payload\":{\"sequence\":${i},\"timestamp\":\"$(date -u +%Y-%m-%dT%H:%M:%SZ)\"}}"

    http_post "${CORE_URL}/api/v1/events" "$body" || true
    if [[ "$HTTP_CODE" == "200" ]]; then
      success=$((success + 1))
    else
      failed=$((failed + 1))
    fi
  done

  if [[ "$success" -eq "$EVENT_COUNT" ]]; then
    check "Write ${EVENT_COUNT} events" pass "all succeeded"
  elif [[ "$success" -gt 0 ]]; then
    check "Write ${EVENT_COUNT} events" fail "${success}/${EVENT_COUNT} succeeded"
  else
    check "Write ${EVENT_COUNT} events" fail "all failed (HTTP ${HTTP_CODE})"
    echo -e "  ${RED}Cannot proceed without written events. Aborting.${NC}"
    if [[ "$JSON_OUTPUT" == "true" ]]; then print_json; fi
    print_summary || true
    exit 1
  fi
}

# ---------------------------------------------------------------------------
# Phase 4: Verify Write
# ---------------------------------------------------------------------------
test_verify_write() {
  header "Verify Write"

  sleep 0.5

  http_get "${CORE_URL}/api/v1/events/query?entity_id=${TEST_ENTITY}" || true
  if [[ "$HTTP_CODE" == "200" ]]; then
    local count
    count=$(json_field_raw "$HTTP_BODY" "count")
    count="${count:-0}"
    if [[ "$count" -eq "$EVENT_COUNT" ]]; then
      check "Query returns all events" pass "count=${count}"
    else
      check "Query returns all events" fail "expected=${EVENT_COUNT} got=${count}"
    fi
  else
    check "Query returns all events" fail "HTTP ${HTTP_CODE}"
  fi
}

# ---------------------------------------------------------------------------
# Phase 5: Storage Check (informational)
# ---------------------------------------------------------------------------
test_storage_check() {
  header "Storage Investigation (informational)"

  # WAL status
  http_get "${CORE_URL}/api/v1/ops/wal/status" || true
  if [[ "$HTTP_CODE" == "200" ]]; then
    local wal_size segment_count
    wal_size=$(json_field_raw "$HTTP_BODY" "total_size")
    segment_count=$(json_field_raw "$HTTP_BODY" "segment_count")
    check "WAL status" pass "size=${wal_size:-?} segments=${segment_count:-?}"
  else
    check "WAL status" skip "endpoint not available (HTTP ${HTTP_CODE})"
  fi

  # Storage stats
  http_get "${CORE_URL}/api/v1/ops/storage/stats" || true
  if [[ "$HTTP_CODE" == "200" ]]; then
    local parquet_files
    parquet_files=$(json_field_raw "$HTTP_BODY" "parquet_files")
    check "Storage stats" pass "parquet_files=${parquet_files:-?}"
  else
    check "Storage stats" skip "endpoint not available (HTTP ${HTTP_CODE})"
  fi

  # Deep health
  http_get "${CORE_URL}/api/v1/ops/health/deep" || true
  if [[ "$HTTP_CODE" == "200" ]]; then
    local overall
    overall=$(json_field "$HTTP_BODY" "status")
    check "Deep health" pass "${overall:-ok}"
  else
    check "Deep health" skip "endpoint not available (HTTP ${HTTP_CODE})"
  fi
}

# ---------------------------------------------------------------------------
# Phase 6: Restart
# ---------------------------------------------------------------------------
test_restart() {
  if [[ "$SKIP_RESTART" == "true" ]]; then
    header "Restart (skipped)"
    check "Container restart" skip "--skip-restart flag set"
    return
  fi

  header "Restart"

  echo -e "  ${DIM}Restarting Core...${NC}"

  local restart_ok=false
  if [[ "$TARGET" == "fly" ]]; then
    local fly_cmd
    fly_cmd=$(command -v flyctl 2>/dev/null || command -v fly 2>/dev/null)
    if $fly_cmd machines restart --app allsource-core 2>/dev/null; then
      restart_ok=true
    fi
  else
    if docker restart "$CONTAINER_NAME" &>/dev/null; then
      restart_ok=true
    fi
  fi

  if [[ "$restart_ok" == "true" ]]; then
    check "Restart issued" pass "$TARGET"
  else
    check "Restart issued" fail "command failed"
    return
  fi

  # Phase 7: Recovery — poll health until Core is back
  subheader "Recovery"
  local start_time elapsed
  start_time=$(date +%s)
  local recovered=false

  while true; do
    elapsed=$(( $(date +%s) - start_time ))
    if [[ "$elapsed" -ge "$RECOVERY_TIMEOUT" ]]; then
      break
    fi

    http_get "${CORE_URL}/health" || true
    if [[ "$HTTP_CODE" == "200" ]]; then
      recovered=true
      break
    fi

    sleep 2
  done

  if [[ "$recovered" == "true" ]]; then
    check "Core recovered" pass "${elapsed}s"
    echo -e "  ${DIM}Waiting ${SETTLE_DELAY}s for WAL replay to settle...${NC}"
    sleep "$SETTLE_DELAY"
  else
    check "Core recovered" fail "timeout after ${RECOVERY_TIMEOUT}s"
    echo -e "  ${RED}Core did not come back. Cannot verify durability.${NC}"
  fi
}

# ---------------------------------------------------------------------------
# Phase 8: Verify Durability (the money test)
# ---------------------------------------------------------------------------
test_verify_durability() {
  if [[ "$SKIP_RESTART" == "true" ]]; then
    return
  fi

  header "Verify Durability"

  http_get "${CORE_URL}/api/v1/events/query?entity_id=${TEST_ENTITY}" || true
  if [[ "$HTTP_CODE" == "200" ]]; then
    local count
    count=$(json_field_raw "$HTTP_BODY" "count")
    count="${count:-0}"
    if [[ "$count" -eq "$EVENT_COUNT" ]]; then
      check "Events survived restart" pass "count=${count}/${EVENT_COUNT}"
    elif [[ "$count" -gt 0 ]]; then
      check "Events survived restart" fail "partial: ${count}/${EVENT_COUNT} recovered"
    else
      check "Events survived restart" fail "ALL EVENTS LOST (0/${EVENT_COUNT})"
    fi
  else
    check "Events survived restart" fail "HTTP ${HTTP_CODE}"
  fi
}

# ---------------------------------------------------------------------------
# Phase 9: Post-restart Investigation
# ---------------------------------------------------------------------------
test_investigation() {
  if [[ "$SKIP_RESTART" == "true" ]]; then
    return
  fi

  header "Post-Restart Investigation"

  # Stats comparison
  http_get "${CORE_URL}/api/v1/stats" || true
  if [[ "$HTTP_CODE" == "200" ]]; then
    local post_total
    post_total=$(json_field_raw "$HTTP_BODY" "total_events")
    post_total="${post_total:-0}"
    check "Post-restart stats" pass "total_events=${post_total} (was ${BASELINE_TOTAL_EVENTS})"
  else
    check "Post-restart stats" skip "HTTP ${HTTP_CODE}"
  fi

  # WAL status after restart
  http_get "${CORE_URL}/api/v1/ops/wal/status" || true
  if [[ "$HTTP_CODE" == "200" ]]; then
    local wal_size segment_count
    wal_size=$(json_field_raw "$HTTP_BODY" "total_size")
    segment_count=$(json_field_raw "$HTTP_BODY" "segment_count")
    check "Post-restart WAL" pass "size=${wal_size:-?} segments=${segment_count:-?}"
  else
    check "Post-restart WAL" skip "endpoint not available"
  fi

  # Deep health after restart
  http_get "${CORE_URL}/api/v1/ops/health/deep" || true
  if [[ "$HTTP_CODE" == "200" ]]; then
    local overall
    overall=$(json_field "$HTTP_BODY" "status")
    check "Post-restart deep health" pass "${overall:-ok}"
  else
    check "Post-restart deep health" skip "endpoint not available"
  fi
}

# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------
main() {
  while [[ $# -gt 0 ]]; do
    case "$1" in
      --target)
        TARGET="$2"
        shift
        ;;
      --count)
        EVENT_COUNT="$2"
        shift
        ;;
      --skip-restart)
        SKIP_RESTART=true
        ;;
      --json)
        JSON_OUTPUT=true
        ;;
      --help|-h)
        echo "Usage: $0 [--target docker|compose|fly] [--count N] [--skip-restart] [--json]"
        exit 0
        ;;
      *)
        echo "Unknown option: $1"
        exit 1
        ;;
    esac
    shift
  done

  configure_target

  echo -e "${BOLD}Chronos Durability Test${NC}"
  echo -e "${DIM}$(date -u +%Y-%m-%dT%H:%M:%SZ)${NC}"
  echo -e "${DIM}Target: ${TARGET}  Core: ${CORE_URL}  Events: ${EVENT_COUNT}  Restart: $([[ "$SKIP_RESTART" == "true" ]] && echo "skip" || echo "yes")${NC}"

  test_preflight
  test_baseline
  test_write
  test_verify_write
  test_storage_check
  test_restart
  test_verify_durability
  test_investigation

  if [[ "$JSON_OUTPUT" == "true" ]]; then
    print_json
  fi

  print_summary
}

main "$@"
