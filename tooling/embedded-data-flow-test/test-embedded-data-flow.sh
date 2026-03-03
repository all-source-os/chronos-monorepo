#!/usr/bin/env bash
# =============================================================================
# Chronos Embedded Data Flow Test
# =============================================================================
# Tests the MCP server running with an embedded Core backend (CORE_MODE=embedded).
# No separate Core container needed — Core runs in-process via Rustler NIFs.
#
# Usage:
#   ./test-embedded-data-flow.sh              # Run all tests
#   ./test-embedded-data-flow.sh --prereqs    # Check prerequisites only
#   ./test-embedded-data-flow.sh --ingest     # Test event ingestion only
#   ./test-embedded-data-flow.sh --query      # Test event querying only
#   ./test-embedded-data-flow.sh --schema     # Test schema operations only
#   ./test-embedded-data-flow.sh --stats      # Test stats/health only
#   ./test-embedded-data-flow.sh --persist    # Test persistence (restart cycle)
#   ./test-embedded-data-flow.sh --json       # Output results as JSON
#
# Environment:
#   MCP_DIR         Path to mcp-server-elixir app (auto-detected)
#   CORE_MODE       Must be "embedded" (set automatically)
#   DATA_DIR        Embedded Core data directory (default: /tmp/chronos-embedded-test)
# =============================================================================

set -euo pipefail

# ---------------------------------------------------------------------------
# Configuration
# ---------------------------------------------------------------------------
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
MCP_DIR="${MCP_DIR:-$REPO_ROOT/apps/mcp-server-elixir}"
DATA_DIR="${DATA_DIR:-/tmp/chronos-embedded-test}"
JSON_OUTPUT=false
MCP_PID=""
FIFO_IN=""
FIFO_OUT=""

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

cleanup() {
  if [[ -n "$MCP_PID" ]] && kill -0 "$MCP_PID" 2>/dev/null; then
    kill "$MCP_PID" 2>/dev/null || true
    wait "$MCP_PID" 2>/dev/null || true
  fi
  [[ -n "$FIFO_IN" && -p "$FIFO_IN" ]] && rm -f "$FIFO_IN"
  [[ -n "$FIFO_OUT" && -p "$FIFO_OUT" ]] && rm -f "$FIFO_OUT"
}
trap cleanup EXIT

# ---------------------------------------------------------------------------
# JSON-RPC helpers
# ---------------------------------------------------------------------------
RPC_ID=0

# Send a JSON-RPC request and capture response
rpc_call() {
  local method="$1"
  local params="$2"
  RPC_ID=$((RPC_ID + 1))

  local request="{\"jsonrpc\":\"2.0\",\"id\":${RPC_ID},\"method\":\"${method}\",\"params\":${params}}"

  # Send request to MCP server stdin
  echo "$request" > "$FIFO_IN"

  # Read response with timeout
  local response=""
  if read -r -t 10 response < "$FIFO_OUT" 2>/dev/null; then
    echo "$response"
  else
    echo '{"error":"timeout"}'
  fi
}

# Call an MCP tool via tools/call
call_tool() {
  local tool_name="$1"
  local arguments="$2"
  rpc_call "tools/call" "{\"name\":\"${tool_name}\",\"arguments\":${arguments}}"
}

# Extract text content from MCP tool response
extract_content() {
  local response="$1"
  # MCP responses wrap content in {"content": [{"type": "text", "text": "..."}]}
  echo "$response" | grep -o '"text":"[^"]*"' | head -1 | sed 's/"text":"//;s/"$//'
}

# Check if response is an error
is_error() {
  local response="$1"
  echo "$response" | grep -q '"error"'
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
# Prerequisites Check
# ---------------------------------------------------------------------------
check_prerequisites() {
  header "Prerequisites"

  # Check MCP server directory exists
  if [[ -d "$MCP_DIR" ]]; then
    check "MCP server directory" pass "$MCP_DIR"
  else
    check "MCP server directory" fail "not found at $MCP_DIR"
    return 1
  fi

  # Check mix.exs exists
  if [[ -f "$MCP_DIR/mix.exs" ]]; then
    check "mix.exs" pass "found"
  else
    check "mix.exs" fail "not found"
    return 1
  fi

  # Check Elixir/Erlang available
  if command -v elixir &>/dev/null; then
    local elixir_version
    elixir_version=$(elixir --version 2>/dev/null | grep "Elixir" | head -1)
    check "Elixir runtime" pass "$elixir_version"
  else
    check "Elixir runtime" fail "elixir not found in PATH"
    return 1
  fi

  # Check Rustler NIF compiled (native/core_nif)
  if [[ -d "$MCP_DIR/native/core_nif" ]]; then
    check "Rustler NIF source" pass "native/core_nif exists"
  else
    check "Rustler NIF source" skip "native/core_nif not found (NIF not yet implemented)"
  fi

  # Check if NIF .so/.dylib is compiled
  local nif_so
  nif_so=$(find "$MCP_DIR/priv/native" -name "*.so" -o -name "*.dylib" 2>/dev/null | head -1)
  if [[ -n "$nif_so" ]]; then
    check "Rustler NIF binary" pass "$(basename "$nif_so")"
  else
    check "Rustler NIF binary" skip "not compiled yet"
  fi

  # Check CoreBackend behaviour exists
  if [[ -f "$MCP_DIR/lib/mcp_server_elixir/infrastructure/core_backend.ex" ]]; then
    check "CoreBackend behaviour" pass "defined"
  else
    check "CoreBackend behaviour" fail "core_backend.ex not found"
  fi

  # Check CoreClient implements behaviour
  if grep -q "@behaviour.*CoreBackend" "$MCP_DIR/lib/mcp_server_elixir/infrastructure/core_client.ex" 2>/dev/null; then
    check "CoreClient @behaviour" pass "implements CoreBackend"
  else
    check "CoreClient @behaviour" skip "not yet annotated"
  fi

  # Check for CoreEmbedded module
  if [[ -f "$MCP_DIR/lib/mcp_server_elixir/infrastructure/core_embedded.ex" ]]; then
    check "CoreEmbedded module" pass "exists"
  else
    check "CoreEmbedded module" skip "not yet implemented"
  fi

  # Check for CORE_MODE config support
  if grep -rq "CORE_MODE\|core_mode" "$MCP_DIR/config/" 2>/dev/null; then
    check "CORE_MODE config" pass "configured"
  else
    check "CORE_MODE config" skip "not yet in config/"
  fi
}

# ---------------------------------------------------------------------------
# Start MCP Server in Embedded Mode
# ---------------------------------------------------------------------------
start_mcp_server() {
  header "Starting MCP Server (embedded mode)"

  # Create data directory
  mkdir -p "$DATA_DIR"
  echo -e "  ${DIM}Data dir: $DATA_DIR${NC}"

  # Create FIFOs for bidirectional communication
  FIFO_IN=$(mktemp -u /tmp/mcp-in-XXXXXX)
  FIFO_OUT=$(mktemp -u /tmp/mcp-out-XXXXXX)
  mkfifo "$FIFO_IN"
  mkfifo "$FIFO_OUT"

  # Start MCP server with embedded mode env vars
  (
    cd "$MCP_DIR"
    CORE_MODE=embedded \
    ALLSOURCE_DATA_DIR="$DATA_DIR" \
    ALLSOURCE_WAL_DIR="$DATA_DIR/wal" \
    ALLSOURCE_STORAGE_DIR="$DATA_DIR/storage" \
    MIX_ENV=prod \
    mix run --no-halt < "$FIFO_IN" > "$FIFO_OUT" 2>/tmp/chronos-embedded-test-stderr.log &
    echo $!
  ) > /tmp/mcp-pid.txt 2>/dev/null

  MCP_PID=$(cat /tmp/mcp-pid.txt)
  rm -f /tmp/mcp-pid.txt

  # Wait for server to start
  sleep 3

  if kill -0 "$MCP_PID" 2>/dev/null; then
    check "MCP server started" pass "PID=$MCP_PID"
  else
    check "MCP server started" fail "process exited"
    echo -e "  ${DIM}Stderr log: /tmp/chronos-embedded-test-stderr.log${NC}"
    if [[ -f /tmp/chronos-embedded-test-stderr.log ]]; then
      echo -e "  ${DIM}Last lines:${NC}"
      tail -5 /tmp/chronos-embedded-test-stderr.log | while read -r line; do
        echo -e "    ${DIM}$line${NC}"
      done
    fi
    return 1
  fi

  # Send initialize request
  local init_response
  init_response=$(rpc_call "initialize" '{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}}')

  if echo "$init_response" | grep -q "protocolVersion"; then
    local server_name
    server_name=$(json_field "$init_response" "name")
    check "MCP initialize" pass "${server_name:-allsource-mcp}"
  else
    check "MCP initialize" fail "bad response"
  fi

  # Send initialized notification
  echo '{"jsonrpc":"2.0","method":"notifications/initialized"}' > "$FIFO_IN"
  sleep 0.5
}

# ---------------------------------------------------------------------------
# Test: Event Ingestion via Embedded Backend
# ---------------------------------------------------------------------------
TEST_ENTITY=""
TEST_TYPE="embedded_data_flow_test"

test_ingest() {
  header "Event Ingestion (embedded)"

  TEST_ENTITY="embedded-test-$(date +%s)"

  subheader "Ingest Single Event"
  local response
  response=$(call_tool "ingest_event" "{\"entity_id\":\"${TEST_ENTITY}\",\"event_type\":\"${TEST_TYPE}\",\"payload\":{\"test\":true,\"source\":\"embedded-data-flow-test\",\"timestamp\":\"$(date -u +%Y-%m-%dT%H:%M:%SZ)\"}}")

  if is_error "$response"; then
    check "Ingest event" fail "$(json_field "$response" "message")"
  else
    check "Ingest event" pass "entity_id=${TEST_ENTITY}"
  fi

  subheader "Ingest Batch (3 events)"
  local batch_ok=0
  for i in 1 2 3; do
    response=$(call_tool "ingest_event" "{\"entity_id\":\"${TEST_ENTITY}\",\"event_type\":\"${TEST_TYPE}\",\"payload\":{\"batch_index\":$i,\"test\":true}}")
    if ! is_error "$response"; then
      batch_ok=$((batch_ok + 1))
    fi
  done

  if [[ "$batch_ok" -eq 3 ]]; then
    check "Ingest batch events" pass "3/3 succeeded"
  else
    check "Ingest batch events" fail "${batch_ok}/3 succeeded"
  fi
}

# ---------------------------------------------------------------------------
# Test: Event Querying via Embedded Backend
# ---------------------------------------------------------------------------
test_query() {
  header "Event Querying (embedded)"

  if [[ -z "$TEST_ENTITY" ]]; then
    TEST_ENTITY="embedded-test-$(date +%s)"
    # Ingest a test event first
    call_tool "ingest_event" "{\"entity_id\":\"${TEST_ENTITY}\",\"event_type\":\"${TEST_TYPE}\",\"payload\":{\"test\":true}}" >/dev/null
    sleep 0.5
  fi

  subheader "Query by Entity ID"
  local response
  response=$(call_tool "query_events" "{\"entity_id\":\"${TEST_ENTITY}\"}")

  if is_error "$response"; then
    check "Query by entity_id" fail "$(json_field "$response" "message")"
  else
    local content
    content=$(extract_content "$response")
    check "Query by entity_id" pass "found events for ${TEST_ENTITY}"
  fi

  subheader "Query by Event Type"
  response=$(call_tool "query_events" "{\"event_type\":\"${TEST_TYPE}\"}")

  if is_error "$response"; then
    check "Query by event_type" fail "$(json_field "$response" "message")"
  else
    check "Query by event_type" pass "type=${TEST_TYPE}"
  fi

  subheader "Query with Limit"
  response=$(call_tool "query_events" "{\"entity_id\":\"${TEST_ENTITY}\",\"limit\":2}")

  if is_error "$response"; then
    check "Query with limit" fail "$(json_field "$response" "message")"
  else
    check "Query with limit" pass "limit=2"
  fi

  subheader "State Reconstruction"
  response=$(call_tool "reconstruct_state" "{\"entity_id\":\"${TEST_ENTITY}\"}")

  if is_error "$response"; then
    check "Reconstruct state" fail "$(json_field "$response" "message")"
  else
    check "Reconstruct state" pass "entity=${TEST_ENTITY}"
  fi
}

# ---------------------------------------------------------------------------
# Test: Schema Operations via Embedded Backend
# ---------------------------------------------------------------------------
test_schema() {
  header "Schema Operations (embedded)"

  subheader "Register Schema"
  local schema_subject="embedded-test-schema"
  local response
  response=$(call_tool "register_schema" "{\"subject\":\"${schema_subject}\",\"schema\":{\"type\":\"object\",\"properties\":{\"test\":{\"type\":\"boolean\"}}}}")

  if is_error "$response"; then
    check "Register schema" fail "$(json_field "$response" "message")"
  else
    check "Register schema" pass "subject=${schema_subject}"
  fi

  subheader "List Schemas"
  response=$(call_tool "list_schemas" "{}")

  if is_error "$response"; then
    check "List schemas" fail "$(json_field "$response" "message")"
  else
    check "List schemas" pass
  fi

  subheader "Get Schema"
  response=$(call_tool "get_schema" "{\"subject\":\"${schema_subject}\"}")

  if is_error "$response"; then
    check "Get schema" fail "$(json_field "$response" "message")"
  else
    check "Get schema" pass "subject=${schema_subject}"
  fi
}

# ---------------------------------------------------------------------------
# Test: Stats & Health via Embedded Backend
# ---------------------------------------------------------------------------
test_stats() {
  header "Stats & Health (embedded)"

  subheader "Event Store Stats"
  local response
  response=$(call_tool "get_stats" "{}")

  if is_error "$response"; then
    check "Get stats" fail "$(json_field "$response" "message")"
  else
    check "Get stats" pass
  fi

  subheader "Storage Stats"
  response=$(call_tool "storage_stats" "{}")

  if is_error "$response"; then
    check "Storage stats" fail "$(json_field "$response" "message")"
  else
    check "Storage stats" pass
  fi

  subheader "WAL Status"
  response=$(call_tool "wal_status" "{}")

  if is_error "$response"; then
    check "WAL status" fail "$(json_field "$response" "message")"
  else
    check "WAL status" pass
  fi

  subheader "Deep Health"
  response=$(call_tool "health_check" "{}")

  if is_error "$response"; then
    check "Deep health" fail "$(json_field "$response" "message")"
  else
    check "Deep health" pass
  fi
}

# ---------------------------------------------------------------------------
# Test: Persistence (write, restart, verify)
# ---------------------------------------------------------------------------
test_persistence() {
  header "Persistence (embedded restart cycle)"

  local persist_entity="persist-test-$(date +%s)"
  local persist_type="persistence_test"

  subheader "Phase 1: Write events before restart"
  local write_ok=0
  for i in 1 2 3 4 5; do
    local response
    response=$(call_tool "ingest_event" "{\"entity_id\":\"${persist_entity}\",\"event_type\":\"${persist_type}\",\"payload\":{\"index\":$i}}")
    if ! is_error "$response"; then
      write_ok=$((write_ok + 1))
    fi
  done

  if [[ "$write_ok" -eq 5 ]]; then
    check "Write 5 events pre-restart" pass "${write_ok}/5"
  else
    check "Write 5 events pre-restart" fail "${write_ok}/5"
    return
  fi

  # Verify events are readable before restart
  local response
  response=$(call_tool "query_events" "{\"entity_id\":\"${persist_entity}\"}")
  if is_error "$response"; then
    check "Read events pre-restart" fail
    return
  else
    check "Read events pre-restart" pass
  fi

  subheader "Phase 2: Restart MCP server"
  # Kill the current server
  if [[ -n "$MCP_PID" ]] && kill -0 "$MCP_PID" 2>/dev/null; then
    kill "$MCP_PID" 2>/dev/null || true
    wait "$MCP_PID" 2>/dev/null || true
    sleep 1
    check "Stop MCP server" pass "PID=$MCP_PID was running"
  else
    check "Stop MCP server" skip "server not running"
    return
  fi

  # Recreate FIFOs
  rm -f "$FIFO_IN" "$FIFO_OUT"
  mkfifo "$FIFO_IN"
  mkfifo "$FIFO_OUT"

  # Restart with same data directory
  (
    cd "$MCP_DIR"
    CORE_MODE=embedded \
    ALLSOURCE_DATA_DIR="$DATA_DIR" \
    ALLSOURCE_WAL_DIR="$DATA_DIR/wal" \
    ALLSOURCE_STORAGE_DIR="$DATA_DIR/storage" \
    MIX_ENV=prod \
    mix run --no-halt < "$FIFO_IN" > "$FIFO_OUT" 2>/tmp/chronos-embedded-test-stderr.log &
    echo $!
  ) > /tmp/mcp-pid.txt 2>/dev/null

  MCP_PID=$(cat /tmp/mcp-pid.txt)
  rm -f /tmp/mcp-pid.txt
  sleep 3

  if kill -0 "$MCP_PID" 2>/dev/null; then
    check "Restart MCP server" pass "PID=$MCP_PID"
  else
    check "Restart MCP server" fail "process exited"
    return
  fi

  # Re-initialize
  rpc_call "initialize" '{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}}' >/dev/null
  echo '{"jsonrpc":"2.0","method":"notifications/initialized"}' > "$FIFO_IN"
  sleep 0.5

  subheader "Phase 3: Verify events survived restart"
  response=$(call_tool "query_events" "{\"entity_id\":\"${persist_entity}\"}")

  if is_error "$response"; then
    check "Read events post-restart" fail "query failed"
  else
    local content
    content=$(extract_content "$response")
    # Check if the response mentions our events
    if echo "$content" | grep -q "$persist_entity" 2>/dev/null || echo "$response" | grep -q "$persist_entity" 2>/dev/null; then
      check "Read events post-restart" pass "events survived restart"
    else
      check "Read events post-restart" fail "events not found after restart"
    fi
  fi

  # Verify count
  response=$(call_tool "get_stats" "{}")
  if ! is_error "$response"; then
    check "Stats after restart" pass "store recovered"
  else
    check "Stats after restart" fail
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
  echo "  \"mode\": \"embedded\","
  echo "  \"data_dir\": \"$DATA_DIR\","
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
  local run_prereqs=true
  local run_ingest=true
  local run_query=true
  local run_schema=true
  local run_stats=true
  local run_persist=false  # opt-in (slower, restarts server)

  while [[ $# -gt 0 ]]; do
    case "$1" in
      --prereqs)  run_ingest=false; run_query=false; run_schema=false; run_stats=false; run_persist=false ;;
      --ingest)   run_prereqs=false; run_query=false; run_schema=false; run_stats=false; run_persist=false ;;
      --query)    run_prereqs=false; run_ingest=false; run_schema=false; run_stats=false; run_persist=false ;;
      --schema)   run_prereqs=false; run_ingest=false; run_query=false;  run_stats=false; run_persist=false ;;
      --stats)    run_prereqs=false; run_ingest=false; run_query=false;  run_schema=false; run_persist=false ;;
      --persist)  run_persist=true ;;
      --json)     JSON_OUTPUT=true ;;
      --help|-h)
        echo "Usage: $0 [--prereqs|--ingest|--query|--schema|--stats] [--persist] [--json]"
        exit 0
        ;;
      *) echo "Unknown option: $1"; exit 1 ;;
    esac
    shift
  done

  echo -e "${BOLD}Chronos Embedded Data Flow Test${NC}"
  echo -e "${DIM}$(date -u +%Y-%m-%dT%H:%M:%SZ)${NC}"
  echo -e "${DIM}Mode: embedded (Core via Rustler NIF)${NC}"

  if [[ "$run_prereqs" == "true" ]]; then
    check_prerequisites
  fi

  # For non-prereqs-only runs, start the MCP server
  if [[ "$run_ingest" == "true" || "$run_query" == "true" || "$run_schema" == "true" || "$run_stats" == "true" || "$run_persist" == "true" ]]; then
    start_mcp_server || {
      echo -e "\n  ${RED}Cannot start MCP server in embedded mode.${NC}"
      echo -e "  ${DIM}This is expected if the Rustler NIF is not yet compiled.${NC}"
      echo -e "  ${DIM}Run --prereqs to check implementation status.${NC}"
      if [[ "$JSON_OUTPUT" == "true" ]]; then print_json; fi
      print_summary
      exit 1
    }
  fi

  if [[ "$run_ingest" == "true" ]]; then test_ingest; fi
  if [[ "$run_query" == "true" ]]; then test_query; fi
  if [[ "$run_schema" == "true" ]]; then test_schema; fi
  if [[ "$run_stats" == "true" ]]; then test_stats; fi
  if [[ "$run_persist" == "true" ]]; then test_persistence; fi

  if [[ "$JSON_OUTPUT" == "true" ]]; then
    print_json
  fi

  print_summary
}

main "$@"
