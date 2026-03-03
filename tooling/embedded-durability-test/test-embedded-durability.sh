#!/usr/bin/env bash
# =============================================================================
# Chronos Embedded Durability Test
# =============================================================================
# Tests the allsource-core Rust crate's durability guarantees directly.
# No Docker, no HTTP — exercises EventStore and EmbeddedCore code paths.
#
# Specifically targets the #84 bug pattern:
#   WAL recovery → silent no-op checkpoint → WAL truncated → data loss
#
# Usage:
#   ./test-embedded-durability.sh              # Run all Rust durability tests
#   ./test-embedded-durability.sh --quick      # Only the #84-critical tests
#   ./test-embedded-durability.sh --embedded   # Only EmbeddedCore API tests
#   ./test-embedded-durability.sh --store      # Only EventStore integration tests
#   ./test-embedded-durability.sh --chaos      # Only chaos/resilience tests
#   ./test-embedded-durability.sh --status     # Only durability_status() API tests
#   ./test-embedded-durability.sh --json       # Machine-readable output
#
# Requires:
#   - EmbeddedCore::durability_status() (added for #84 fix)
#     Returns: memory_events, wal_entries, wal_bytes, wal_sequence,
#              parquet_files, parquet_bytes, parquet_pending_batch,
#              durable (bool), warnings (vec)
#
# Environment:
#   CORE_DIR    Path to apps/core (auto-detected)
# =============================================================================

set -euo pipefail

# ---------------------------------------------------------------------------
# Configuration
# ---------------------------------------------------------------------------
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
CORE_DIR="${CORE_DIR:-$REPO_ROOT/apps/core}"
JSON_OUTPUT=false

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
  local status="$2"
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

# Run a cargo test and check exit code
run_test() {
  local test_name="$1"
  local display_name="$2"
  local test_file="${3:-}"
  local features="${4:-}"

  local cmd="cargo test"
  if [[ -n "$test_file" ]]; then
    cmd="$cmd --test $test_file"
  fi
  if [[ -n "$features" ]]; then
    cmd="$cmd --features $features"
  fi
  cmd="$cmd $test_name -- --nocapture"

  local output
  local exit_code=0
  output=$(cd "$CORE_DIR" && eval "$cmd" 2>&1) || exit_code=$?

  if [[ $exit_code -eq 0 ]]; then
    # Check if the test actually ran (not just "0 tests filtered out")
    if echo "$output" | grep -q "test result: ok. 0 passed"; then
      check "$display_name" skip "test not found or filtered out"
    else
      local test_count
      test_count=$(echo "$output" | grep -o "test result: ok\. [0-9]* passed" | grep -o "[0-9]*" | head -1)
      check "$display_name" pass "${test_count:-?} test(s)"
    fi
  else
    local failure_line
    failure_line=$(echo "$output" | grep "^---- .* ----" | head -1 | sed 's/---- //;s/ ----//')
    if [[ -z "$failure_line" ]]; then
      failure_line=$(echo "$output" | grep "FAILED\|panicked\|error\[" | head -1 | cut -c1-80)
    fi
    check "$display_name" fail "${failure_line:-exit code $exit_code}"

    # Show last few lines of output for diagnosis
    echo -e "    ${DIM}Last output lines:${NC}"
    echo "$output" | tail -10 | while read -r line; do
      echo -e "    ${DIM}$line${NC}"
    done
  fi
}

# ---------------------------------------------------------------------------
# Prerequisites
# ---------------------------------------------------------------------------
check_prerequisites() {
  header "Prerequisites"

  if [[ -d "$CORE_DIR" ]]; then
    check "Core directory" pass "$CORE_DIR"
  else
    check "Core directory" fail "not found at $CORE_DIR"
    exit 1
  fi

  if [[ -f "$CORE_DIR/Cargo.toml" ]]; then
    check "Cargo.toml" pass "found"
  else
    check "Cargo.toml" fail "not found"
    exit 1
  fi

  if command -v cargo &>/dev/null; then
    local rust_version
    rust_version=$(rustc --version 2>/dev/null | head -1)
    check "Rust toolchain" pass "$rust_version"
  else
    check "Rust toolchain" fail "cargo not found"
    exit 1
  fi

  # Check for test files
  for test_file in integration_tests embedded_core_api chaos_resilience_tests; do
    if [[ -f "$CORE_DIR/tests/${test_file}.rs" ]]; then
      check "Test file: ${test_file}.rs" pass "exists"
    else
      check "Test file: ${test_file}.rs" skip "not found"
    fi
  done

  # Quick compile check
  subheader "Compile Check"
  local compile_output
  local compile_exit=0
  compile_output=$(cd "$CORE_DIR" && cargo check 2>&1) || compile_exit=$?
  if [[ $compile_exit -eq 0 ]]; then
    check "cargo check" pass "compiles"
  else
    check "cargo check" fail "compilation errors"
    echo "$compile_output" | grep "^error" | head -5 | while read -r line; do
      echo -e "    ${DIM}$line${NC}"
    done
  fi
}

# ---------------------------------------------------------------------------
# Issue #84 Critical Tests
# ---------------------------------------------------------------------------
test_issue84_critical() {
  header "Issue #84 Critical — WAL Recovery Checkpoint"
  echo -e "  ${DIM}These tests catch the silent-no-op checkpoint + WAL truncation bug${NC}"

  subheader "WAL Recovery Without Prior Flush (EventStore level)"
  run_test "test_wal_durability_and_recovery" \
    "WAL durability: drop without flush → reopen → events survive" \
    "integration_tests"

  subheader "Parquet Persistence + Recovery (EventStore level)"
  run_test "test_parquet_persistence_and_recovery" \
    "Parquet recovery: flush → drop → reopen → events survive" \
    "integration_tests"

  subheader "EmbeddedCore: Events Survive Restart via WAL"
  run_test "events_survive_store_restart_via_wal" \
    "EmbeddedCore: shutdown → reopen → events survive" \
    "embedded_core_api" "embedded"

  subheader "EmbeddedCore: Events Survive Drop Without Shutdown"
  run_test "events_survive_drop_without_shutdown" \
    "EmbeddedCore: drop (no shutdown) → reopen → events survive" \
    "embedded_core_api" "embedded"
}

# ---------------------------------------------------------------------------
# Durability Status API Tests (#84 regression)
# ---------------------------------------------------------------------------
test_durability_status() {
  header "Durability Status API (issue #84 regression)"
  echo -e "  ${DIM}Tests EmbeddedCore::durability_status() invariants${NC}"
  echo -e "  ${DIM}This API exposes the exact state that caused #84${NC}"

  subheader "durability_status after ingest (pre-flush)"
  run_test "durability_status_after_ingest" \
    "After ingest: memory_events > 0, wal_entries > 0, durable=true" \
    "embedded_core_api" "embedded"

  subheader "durability_status after flush"
  run_test "durability_status_after_flush" \
    "After flush: parquet_files > 0, parquet_pending_batch == 0" \
    "embedded_core_api" "embedded"

  subheader "durability_status after recovery (the #84 path)"
  run_test "durability_status_after_recovery" \
    "After WAL recovery: memory == wal, parquet_pending_batch == memory, durable=true" \
    "embedded_core_api" "embedded"

  subheader "durability_status detects dangerous state"
  run_test "durability_status_warns_on_memory_only" \
    "Warns when events in memory but not in WAL or Parquet" \
    "embedded_core_api" "embedded"

  subheader "durability_status: drop without shutdown → reopen → status consistent"
  run_test "durability_status_survives_unclean_restart" \
    "After unclean restart: recovered events counted, durable=true, no warnings" \
    "embedded_core_api" "embedded"
}

# ---------------------------------------------------------------------------
# Full EventStore Integration Tests
# ---------------------------------------------------------------------------
test_store_integration() {
  header "EventStore Integration Tests"

  run_test "" \
    "All integration tests" \
    "integration_tests"
}

# ---------------------------------------------------------------------------
# Full EmbeddedCore API Tests
# ---------------------------------------------------------------------------
test_embedded_api() {
  header "EmbeddedCore API Tests"

  run_test "" \
    "All embedded API tests" \
    "embedded_core_api" "embedded"
}

# ---------------------------------------------------------------------------
# Chaos / Resilience Tests
# ---------------------------------------------------------------------------
test_chaos() {
  header "Chaos & Resilience Tests"

  run_test "" \
    "All chaos resilience tests" \
    "chaos_resilience_tests"

  # Also run the stress WAL recovery tests if available
  subheader "Stress WAL Recovery"
  run_test "stress_tenant_wal_recovery" \
    "Stress: tenant WAL recovery after concurrent writes" \
    "event_sourced_stress_tests"

  run_test "stress_config_wal_recovery" \
    "Stress: config WAL recovery after concurrent writes" \
    "event_sourced_stress_tests"

  run_test "stress_audit_wal_recovery" \
    "Stress: audit WAL recovery after concurrent writes" \
    "event_sourced_stress_tests"
}

# ---------------------------------------------------------------------------
# WAL Unit Tests
# ---------------------------------------------------------------------------
test_wal_unit() {
  header "WAL Unit Tests"

  subheader "Core WAL operations"
  run_test "wal::" \
    "All WAL unit tests" \
    "" ""
}

# ---------------------------------------------------------------------------
# Summary
# ---------------------------------------------------------------------------
print_summary() {
  echo ""
  echo -e "${BOLD}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
  local total=$((PASS + FAIL + SKIP))
  echo -e "  ${GREEN}${PASS} passed${NC}  ${RED}${FAIL} failed${NC}  ${YELLOW}${SKIP} skipped${NC}  ${DIM}(${total} total)${NC}"

  if [[ "$FAIL" -gt 0 ]]; then
    echo ""
    echo -e "  ${RED}${BOLD}DURABILITY TESTS FAILED${NC}"
    echo -e "  ${DIM}If #84-critical tests failed, the WAL recovery checkpoint bug may be present.${NC}"
    echo -e "  ${DIM}Check store.rs recovery path: are WAL events added to current_batch before flush?${NC}"
  else
    echo ""
    echo -e "  ${GREEN}${BOLD}ALL DURABILITY TESTS PASSED${NC}"
  fi

  echo -e "${BOLD}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"

  if [[ "$FAIL" -gt 0 ]]; then
    return 1
  fi
  return 0
}

print_json() {
  echo "{"
  echo "  \"suite\": \"embedded-durability\","
  echo "  \"core_dir\": \"$CORE_DIR\","
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
  local run_quick=false
  local run_store=false
  local run_embedded=false
  local run_chaos=false
  local run_status=false
  local run_all=true

  while [[ $# -gt 0 ]]; do
    case "$1" in
      --quick)    run_quick=true; run_all=false ;;
      --store)    run_store=true; run_all=false ;;
      --embedded) run_embedded=true; run_all=false ;;
      --chaos)    run_chaos=true; run_all=false ;;
      --status)   run_status=true; run_all=false ;;
      --json)     JSON_OUTPUT=true ;;
      --help|-h)
        echo "Usage: $0 [--quick|--store|--embedded|--chaos|--status] [--json]"
        echo ""
        echo "  --quick      Only #84-critical tests (WAL recovery checkpoint)"
        echo "  --store      Only EventStore integration tests"
        echo "  --embedded   Only EmbeddedCore API tests"
        echo "  --chaos      Only chaos/resilience + stress tests"
        echo "  --status     Only durability_status() API tests (#84 regression)"
        echo "  --json       Machine-readable JSON output"
        exit 0
        ;;
      *) echo "Unknown option: $1"; exit 1 ;;
    esac
    shift
  done

  echo -e "${BOLD}Chronos Embedded Durability Test${NC}"
  echo -e "${DIM}$(date -u +%Y-%m-%dT%H:%M:%SZ)${NC}"
  echo -e "${DIM}Tests allsource-core Rust crate durability directly${NC}"

  check_prerequisites

  if [[ "$run_quick" == "true" ]]; then
    test_issue84_critical
  elif [[ "$run_store" == "true" ]]; then
    test_store_integration
  elif [[ "$run_embedded" == "true" ]]; then
    test_embedded_api
  elif [[ "$run_chaos" == "true" ]]; then
    test_chaos
  elif [[ "$run_status" == "true" ]]; then
    test_durability_status
  elif [[ "$run_all" == "true" ]]; then
    test_issue84_critical
    test_durability_status
    test_wal_unit
    test_store_integration
    test_embedded_api
    test_chaos
  fi

  if [[ "$JSON_OUTPUT" == "true" ]]; then
    print_json
  fi

  print_summary
}

main "$@"
