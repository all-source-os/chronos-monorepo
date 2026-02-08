#!/bin/bash
# Elixir Test Report Generator
# Runs tests and generates a detailed failure report

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
REPORT_DIR="$REPO_ROOT/.test-reports"
TIMESTAMP=$(date +%Y%m%d_%H%M%S)
REPORT_FILE="$REPORT_DIR/test-report-$TIMESTAMP.md"

# Colors for terminal output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

mkdir -p "$REPORT_DIR"

echo "# Elixir Test Report" > "$REPORT_FILE"
echo "" >> "$REPORT_FILE"
echo "Generated: $(date '+%Y-%m-%d %H:%M:%S')" >> "$REPORT_FILE"
echo "" >> "$REPORT_FILE"

overall_success=true

run_tests() {
    local service_name=$1
    local service_dir=$2
    local log_file="$REPORT_DIR/${service_name}-test.log"

    echo -e "${YELLOW}Running $service_name tests...${NC}"

    cd "$service_dir"

    # Run tests and capture output
    if [ "$service_name" == "query-service" ]; then
        TESTCONTAINERS_RYUK_DISABLED=true mix test --trace > "$log_file" 2>&1 || true
    else
        mix test --trace > "$log_file" 2>&1 || true
    fi

    # Extract results
    local test_summary=$(grep -E "^[0-9]+ (test|doctest)" "$log_file" | tail -1)
    local failures=$(echo "$test_summary" | grep -oE "[0-9]+ failure" | grep -oE "[0-9]+" || echo "0")

    echo "## $service_name" >> "$REPORT_FILE"
    echo "" >> "$REPORT_FILE"

    if [ "$failures" == "0" ]; then
        echo -e "${GREEN}✅ $service_name: $test_summary${NC}"
        echo "✅ **PASSED**" >> "$REPORT_FILE"
        echo "" >> "$REPORT_FILE"
        echo "$test_summary" >> "$REPORT_FILE"
    else
        echo -e "${RED}❌ $service_name: $test_summary${NC}"
        echo "❌ **FAILED**" >> "$REPORT_FILE"
        echo "" >> "$REPORT_FILE"
        echo "$test_summary" >> "$REPORT_FILE"
        echo "" >> "$REPORT_FILE"
        echo "### Failures" >> "$REPORT_FILE"
        echo "" >> "$REPORT_FILE"
        echo '```' >> "$REPORT_FILE"
        # Extract failure details - look for numbered test failures
        grep -A 30 "^\s*[0-9]*)" "$log_file" >> "$REPORT_FILE" 2>/dev/null || true
        echo '```' >> "$REPORT_FILE"
        overall_success=false
    fi
    echo "" >> "$REPORT_FILE"
    echo "Full log: \`$log_file\`" >> "$REPORT_FILE"
    echo "" >> "$REPORT_FILE"
    echo "---" >> "$REPORT_FILE"
    echo "" >> "$REPORT_FILE"
}

# Run tests for each Elixir service
run_tests "query-service" "$REPO_ROOT/apps/query-service"
run_tests "mcp-server" "$REPO_ROOT/apps/mcp-server-elixir"

# Summary
echo "## Summary" >> "$REPORT_FILE"
echo "" >> "$REPORT_FILE"

if $overall_success; then
    echo "✅ **All tests passed!**" >> "$REPORT_FILE"
    echo ""
    echo -e "${GREEN}✅ All Elixir tests passed!${NC}"
    echo ""
    echo "Report saved to: $REPORT_FILE"
    exit 0
else
    echo "❌ **Some tests failed. See details above.**" >> "$REPORT_FILE"
    echo "" >> "$REPORT_FILE"
    echo "### Quick Re-run Failed Tests" >> "$REPORT_FILE"
    echo "" >> "$REPORT_FILE"
    echo '```bash' >> "$REPORT_FILE"
    echo "make elixir-test-failed" >> "$REPORT_FILE"
    echo '```' >> "$REPORT_FILE"
    echo ""
    echo -e "${RED}❌ Some tests failed!${NC}"
    echo ""
    echo -e "📋 Report saved to: ${YELLOW}$REPORT_FILE${NC}"
    echo ""
    echo "To re-run failed tests:"
    echo "  make elixir-test-failed"
    echo ""
    echo "To view report:"
    echo "  cat $REPORT_FILE"
    exit 1
fi
