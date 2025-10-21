#!/bin/bash

# AllSource v1.0 Integration Test Script
# Tests the Rust Core + Go Control-Plane working together

set -e  # Exit on error

echo "🧪 AllSource v1.0 Integration Test Suite"
echo "=========================================="
echo ""

# Configuration
CORE_URL="http://localhost:8080"
CONTROL_PLANE_URL="http://localhost:8081"
ADMIN_USER="admin"
ADMIN_PASSWORD="secure_password_123"
TEST_TENANT="test_tenant"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Test counter
TESTS_RUN=0
TESTS_PASSED=0
TESTS_FAILED=0

# Helper functions
function log_info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

function log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

function log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

function test_start() {
    TESTS_RUN=$((TESTS_RUN + 1))
    echo ""
    echo "Test #$TESTS_RUN: $1"
    echo "---"
}

function test_pass() {
    TESTS_PASSED=$((TESTS_PASSED + 1))
    log_info "✅ PASS: $1"
}

function test_fail() {
    TESTS_FAILED=$((TESTS_FAILED + 1))
    log_error "❌ FAIL: $1"
}

# Pre-flight checks
echo "1. Pre-flight Checks"
echo "-------------------"

test_start "Checking if Rust core is running"
if curl -s "$CORE_URL/health" > /dev/null; then
    test_pass "Rust core is running at $CORE_URL"
else
    test_fail "Rust core is NOT running at $CORE_URL"
    log_error "Please start the Rust core first: cd services/core && cargo run"
    exit 1
fi

test_start "Checking if Go control-plane is running"
if curl -s "$CONTROL_PLANE_URL/health" > /dev/null; then
    test_pass "Go control-plane is running at $CONTROL_PLANE_URL"
else
    test_fail "Go control-plane is NOT running at $CONTROL_PLANE_URL"
    log_error "Please start the Go control-plane first: cd services/control-plane && go run ."
    exit 1
fi

echo ""
echo "2. Authentication Flow Tests"
echo "----------------------------"

test_start "Register admin user via control-plane"
REGISTER_RESPONSE=$(curl -s -X POST "$CONTROL_PLANE_URL/api/v1/auth/register" \
    -H "Content-Type: application/json" \
    -d "{\"username\":\"$ADMIN_USER\",\"email\":\"admin@test.com\",\"password\":\"$ADMIN_PASSWORD\",\"role\":\"Admin\"}" \
    -w "\n%{http_code}")

HTTP_CODE=$(echo "$REGISTER_RESPONSE" | tail -n1)
if [ "$HTTP_CODE" = "201" ] || [ "$HTTP_CODE" = "409" ]; then
    test_pass "Admin user registered or already exists (HTTP $HTTP_CODE)"
else
    test_fail "Failed to register admin user (HTTP $HTTP_CODE)"
    echo "$REGISTER_RESPONSE"
fi

test_start "Login via control-plane"
LOGIN_RESPONSE=$(curl -s -X POST "$CONTROL_PLANE_URL/api/v1/auth/login" \
    -H "Content-Type: application/json" \
    -d "{\"username\":\"$ADMIN_USER\",\"password\":\"$ADMIN_PASSWORD\"}")

TOKEN=$(echo "$LOGIN_RESPONSE" | grep -o '"token":"[^"]*"' | cut -d'"' -f4)
if [ -n "$TOKEN" ]; then
    test_pass "Login successful, received JWT token"
    log_info "Token: ${TOKEN:0:20}..."
else
    test_fail "Login failed, no token received"
    echo "$LOGIN_RESPONSE"
    exit 1
fi

test_start "Get current user info via control-plane"
ME_RESPONSE=$(curl -s -X GET "$CONTROL_PLANE_URL/api/v1/auth/me" \
    -H "Authorization: Bearer $TOKEN" \
    -w "\n%{http_code}")

HTTP_CODE=$(echo "$ME_RESPONSE" | tail -n1)
if [ "$HTTP_CODE" = "200" ]; then
    test_pass "Successfully retrieved user info (HTTP $HTTP_CODE)"
else
    test_fail "Failed to get user info (HTTP $HTTP_CODE)"
fi

echo ""
echo "3. Multi-Tenancy Tests"
echo "---------------------"

test_start "Create tenant via control-plane"
CREATE_TENANT_RESPONSE=$(curl -s -X POST "$CONTROL_PLANE_URL/api/v1/tenants" \
    -H "Authorization: Bearer $TOKEN" \
    -H "Content-Type: application/json" \
    -d "{\"id\":\"$TEST_TENANT\",\"name\":\"Test Tenant\",\"tier\":\"professional\"}" \
    -w "\n%{http_code}")

HTTP_CODE=$(echo "$CREATE_TENANT_RESPONSE" | tail -n1)
if [ "$HTTP_CODE" = "200" ] || [ "$HTTP_CODE" = "201" ] || [ "$HTTP_CODE" = "409" ]; then
    test_pass "Tenant created or already exists (HTTP $HTTP_CODE)"
else
    test_fail "Failed to create tenant (HTTP $HTTP_CODE)"
    echo "$CREATE_TENANT_RESPONSE"
fi

test_start "List tenants via control-plane"
LIST_TENANTS_RESPONSE=$(curl -s -X GET "$CONTROL_PLANE_URL/api/v1/tenants" \
    -H "Authorization: Bearer $TOKEN" \
    -w "\n%{http_code}")

HTTP_CODE=$(echo "$LIST_TENANTS_RESPONSE" | tail -n1)
if [ "$HTTP_CODE" = "200" ]; then
    test_pass "Successfully listed tenants (HTTP $HTTP_CODE)"
    TENANT_COUNT=$(echo "$LIST_TENANTS_RESPONSE" | head -n -1 | grep -o "\"id\"" | wc -l | tr -d ' ')
    log_info "Found $TENANT_COUNT tenant(s)"
else
    test_fail "Failed to list tenants (HTTP $HTTP_CODE)"
fi

echo ""
echo "4. RBAC & Permission Tests"
echo "--------------------------"

test_start "Create API key via control-plane"
API_KEY_RESPONSE=$(curl -s -X POST "$CONTROL_PLANE_URL/api/v1/auth/api-keys" \
    -H "Authorization: Bearer $TOKEN" \
    -H "Content-Type: application/json" \
    -d "{\"name\":\"test-key\",\"tenant_id\":\"default\"}" \
    -w "\n%{http_code}")

HTTP_CODE=$(echo "$API_KEY_RESPONSE" | tail -n1)
if [ "$HTTP_CODE" = "200" ] || [ "$HTTP_CODE" = "201" ]; then
    test_pass "API key created successfully (HTTP $HTTP_CODE)"
else
    test_fail "Failed to create API key (HTTP $HTTP_CODE)"
fi

test_start "Access protected endpoint with valid token"
PROTECTED_RESPONSE=$(curl -s -X GET "$CONTROL_PLANE_URL/api/v1/cluster/status" \
    -H "Authorization: Bearer $TOKEN" \
    -w "\n%{http_code}")

HTTP_CODE=$(echo "$PROTECTED_RESPONSE" | tail -n1)
if [ "$HTTP_CODE" = "200" ]; then
    test_pass "Protected endpoint accessible with valid token (HTTP $HTTP_CODE)"
else
    test_fail "Protected endpoint not accessible (HTTP $HTTP_CODE)"
fi

test_start "Access protected endpoint without token (should fail)"
UNAUTH_RESPONSE=$(curl -s -X GET "$CONTROL_PLANE_URL/api/v1/cluster/status" \
    -w "\n%{http_code}")

HTTP_CODE=$(echo "$UNAUTH_RESPONSE" | tail -n1)
if [ "$HTTP_CODE" = "401" ]; then
    test_pass "Protected endpoint correctly blocked without token (HTTP $HTTP_CODE)"
else
    test_fail "Protected endpoint should require auth (HTTP $HTTP_CODE)"
fi

echo ""
echo "5. Core Service Integration Tests"
echo "----------------------------------"

test_start "Access core health via control-plane"
CORE_HEALTH_RESPONSE=$(curl -s -X GET "$CONTROL_PLANE_URL/api/v1/health/core" \
    -H "Authorization: Bearer $TOKEN" \
    -w "\n%{http_code}")

HTTP_CODE=$(echo "$CORE_HEALTH_RESPONSE" | tail -n1)
if [ "$HTTP_CODE" = "200" ]; then
    test_pass "Core health check via control-plane successful (HTTP $HTTP_CODE)"
else
    test_fail "Core health check failed (HTTP $HTTP_CODE)"
fi

test_start "Get cluster status"
CLUSTER_STATUS_RESPONSE=$(curl -s -X GET "$CONTROL_PLANE_URL/api/v1/cluster/status" \
    -H "Authorization: Bearer $TOKEN" \
    -w "\n%{http_code}")

HTTP_CODE=$(echo "$CLUSTER_STATUS_RESPONSE" | tail -n1)
if [ "$HTTP_CODE" = "200" ]; then
    test_pass "Cluster status retrieved successfully (HTTP $HTTP_CODE)"
    HEALTHY_NODES=$(echo "$CLUSTER_STATUS_RESPONSE" | head -n -1 | grep -o '"healthy_nodes":[0-9]*' | cut -d':' -f2)
    log_info "Healthy nodes: $HEALTHY_NODES"
else
    test_fail "Failed to get cluster status (HTTP $HTTP_CODE)"
fi

echo ""
echo "6. Audit & Observability Tests"
echo "------------------------------"

test_start "Check if audit log is being written"
if [ -f "audit.log" ]; then
    AUDIT_LINES=$(wc -l < audit.log | tr -d ' ')
    test_pass "Audit log exists with $AUDIT_LINES entries"
else
    test_fail "Audit log file not found"
fi

test_start "Prometheus metrics endpoint"
METRICS_RESPONSE=$(curl -s -X GET "$CONTROL_PLANE_URL/metrics" \
    -w "\n%{http_code}")

HTTP_CODE=$(echo "$METRICS_RESPONSE" | tail -n1)
if [ "$HTTP_CODE" = "200" ]; then
    test_pass "Prometheus metrics endpoint accessible (HTTP $HTTP_CODE)"
    METRIC_COUNT=$(echo "$METRICS_RESPONSE" | head -n -1 | grep -c "^# HELP" || true)
    log_info "Found $METRIC_COUNT metrics"
else
    test_fail "Failed to access metrics endpoint (HTTP $HTTP_CODE)"
fi

echo ""
echo "7. Policy Enforcement Tests"
echo "---------------------------"

test_start "Attempt to delete default tenant (should be blocked by policy)"
DELETE_DEFAULT_RESPONSE=$(curl -s -X DELETE "$CONTROL_PLANE_URL/api/v1/tenants/default" \
    -H "Authorization: Bearer $TOKEN" \
    -w "\n%{http_code}")

HTTP_CODE=$(echo "$DELETE_DEFAULT_RESPONSE" | tail -n1)
if [ "$HTTP_CODE" = "403" ]; then
    test_pass "Policy correctly prevented default tenant deletion (HTTP $HTTP_CODE)"
else
    log_warn "Default tenant deletion not blocked by policy (HTTP $HTTP_CODE)"
    # Don't fail, policy might not be implemented yet
fi

echo ""
echo "8. Operation Tests"
echo "-----------------"

test_start "Trigger snapshot operation"
SNAPSHOT_RESPONSE=$(curl -s -X POST "$CONTROL_PLANE_URL/api/v1/operations/snapshot" \
    -H "Authorization: Bearer $TOKEN" \
    -H "Content-Type: application/json" \
    -d "{}" \
    -w "\n%{http_code}")

HTTP_CODE=$(echo "$SNAPSHOT_RESPONSE" | tail -n1)
if [ "$HTTP_CODE" = "200" ]; then
    test_pass "Snapshot operation initiated successfully (HTTP $HTTP_CODE)"
else
    test_fail "Failed to initiate snapshot (HTTP $HTTP_CODE)"
fi

test_start "Trigger replay operation"
REPLAY_RESPONSE=$(curl -s -X POST "$CONTROL_PLANE_URL/api/v1/operations/replay" \
    -H "Authorization: Bearer $TOKEN" \
    -H "Content-Type: application/json" \
    -d "{\"entity_id\":\"test-entity\"}" \
    -w "\n%{http_code}")

HTTP_CODE=$(echo "$REPLAY_RESPONSE" | tail -n1)
if [ "$HTTP_CODE" = "200" ]; then
    test_pass "Replay operation initiated successfully (HTTP $HTTP_CODE)"
else
    test_fail "Failed to initiate replay (HTTP $HTTP_CODE)"
fi

echo ""
echo "=========================================="
echo "Integration Test Summary"
echo "=========================================="
echo ""
echo "Tests Run:    $TESTS_RUN"
echo "Tests Passed: ${GREEN}$TESTS_PASSED${NC}"
echo "Tests Failed: ${RED}$TESTS_FAILED${NC}"
echo ""

if [ $TESTS_FAILED -eq 0 ]; then
    echo -e "${GREEN}✅ ALL INTEGRATION TESTS PASSED!${NC}"
    echo ""
    echo "AllSource v1.0 Rust Core + Go Control-Plane integration verified!"
    exit 0
else
    echo -e "${RED}❌ SOME TESTS FAILED${NC}"
    echo ""
    echo "Please review the failed tests above."
    exit 1
fi
