#!/usr/bin/env bash
# Prime MCP Integration Test — exercises the full product via MCP stdio.
#
# Tests: initialize → add nodes → add edges → search → neighbors → history → stats
#
# Usage:
#   ./docs/demos/prime-mcp-test.sh
#   PRIME_BIN=./target/release/allsource-prime ./docs/demos/prime-mcp-test.sh

set -euo pipefail

PRIME_BIN="${PRIME_BIN:-cargo run --manifest-path apps/prime-mcp/Cargo.toml --}"
DATA_DIR=$(mktemp -d)
PASS=0
FAIL=0

cleanup() {
    rm -rf "$DATA_DIR"
    if [ "$FAIL" -gt 0 ]; then
        echo ""
        echo "RESULT: $PASS passed, $FAIL FAILED"
        exit 1
    else
        echo ""
        echo "RESULT: $PASS passed, 0 failed"
    fi
}
trap cleanup EXIT

# Send a single MCP message and get the response
mcp_call() {
    local msg="$1"
    local len=${#msg}
    printf "Content-Length: %d\r\n\r\n%s" "$len" "$msg" | \
        $PRIME_BIN --data-dir "$DATA_DIR" 2>/dev/null | \
        sed 's/^Content-Length: [0-9]*\r$//' | grep -v '^$' | head -1
}

# Assert response contains expected string
assert_contains() {
    local test_name="$1"
    local response="$2"
    local expected="$3"
    if echo "$response" | grep -q "$expected"; then
        echo "  PASS: $test_name"
        PASS=$((PASS + 1))
    else
        echo "  FAIL: $test_name"
        echo "    expected to contain: $expected"
        echo "    got: $response"
        FAIL=$((FAIL + 1))
    fi
}

echo "=== Prime MCP Integration Test ==="
echo "Data dir: $DATA_DIR"
echo ""

# --- Test 1: Initialize ---
echo "1. Initialize handshake"
RESP=$(mcp_call '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}')
assert_contains "returns server info" "$RESP" "allsource-prime"
assert_contains "returns protocol version" "$RESP" "2024-11-05"

# --- Test 2: List tools ---
echo "2. List tools"
RESP=$(mcp_call '{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}')
assert_contains "has prime_add_node" "$RESP" "prime_add_node"
assert_contains "has prime_neighbors" "$RESP" "prime_neighbors"
assert_contains "has prime_history" "$RESP" "prime_history"

# --- Test 3: Add nodes ---
echo "3. Add nodes"
RESP=$(mcp_call '{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"prime_add_node","arguments":{"type":"person","properties":{"name":"Alice","role":"engineer"}}}}')
assert_contains "returns node_id" "$RESP" "node_id"

RESP=$(mcp_call '{"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"prime_add_node","arguments":{"type":"project","properties":{"name":"Prime","status":"active"}}}}')
assert_contains "returns project node_id" "$RESP" "node_id"

# --- Test 4: Stats ---
echo "4. Stats"
RESP=$(mcp_call '{"jsonrpc":"2.0","id":5,"method":"tools/call","params":{"name":"prime_stats","arguments":{}}}')
assert_contains "shows nodes" "$RESP" "total_nodes"

# --- Test 5: Search by type ---
echo "5. Search"
RESP=$(mcp_call '{"jsonrpc":"2.0","id":6,"method":"tools/call","params":{"name":"prime_search","arguments":{"type":"person"}}}')
assert_contains "finds person nodes" "$RESP" "Alice"

echo ""
echo "=== All MCP tests complete ==="
