#!/usr/bin/env bash
# =============================================================================
# Local Quality Gate - Run before pushing to catch CI failures locally
# =============================================================================
# This script replicates the CI checks so you don't have to wait for GitHub Actions
#
# Usage: ./scripts/quality-gate.sh [--quick]
#   --quick: Skip slow checks (full test suite, release build)
# =============================================================================

set -euo pipefail

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

QUICK_MODE=false
FAILED=0

for arg in "$@"; do
    case $arg in
        --quick|-q)
            QUICK_MODE=true
            shift
            ;;
    esac
done

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

cd "$REPO_ROOT"

log() { echo -e "${BLUE}==>${NC} $1"; }
success() { echo -e "${GREEN}✓${NC} $1"; }
fail() { echo -e "${RED}✗${NC} $1"; FAILED=1; }
warn() { echo -e "${YELLOW}⚠${NC} $1"; }

run_check() {
    local name="$1"
    local cmd="$2"

    log "Running: $name"
    if eval "$cmd" > /tmp/quality-gate-output.txt 2>&1; then
        success "$name"
    else
        fail "$name"
        echo "  Output:"
        sed 's/^/    /' /tmp/quality-gate-output.txt | tail -20
    fi
}

echo ""
echo "╔═══════════════════════════════════════════════════════════════╗"
echo "║              Chronos Local Quality Gate                       ║"
echo "╚═══════════════════════════════════════════════════════════════╝"
echo ""

# =============================================================================
# Version Consistency
# =============================================================================
log "Checking version consistency..."
run_check "Version consistency" "./scripts/check-versions.sh"

# =============================================================================
# Rust Checks
# =============================================================================
echo ""
log "=== Rust Quality Gates ==="

cd "$REPO_ROOT/apps/core"

run_check "Rust formatting" "cargo fmt --check"

if command -v cargo-sort &> /dev/null; then
    run_check "Cargo.toml sorting" "cargo sort --check ."
else
    warn "cargo-sort not installed, skipping (install with: cargo install cargo-sort)"
fi

run_check "Clippy lints" "cargo clippy --all-targets --all-features -- -D warnings"

if [ "$QUICK_MODE" = false ]; then
    run_check "Rust tests" "cargo test --all-features"
    run_check "Release build" "cargo build --release"
    run_check "Doc generation" "cargo doc --no-deps"
else
    run_check "Rust tests (lib only)" "cargo test --lib"
fi

# =============================================================================
# Go Checks
# =============================================================================
echo ""
log "=== Go Quality Gates ==="

cd "$REPO_ROOT/apps/control-plane"

run_check "Go formatting" "test -z \"\$(gofmt -l .)\""
run_check "Go vet" "go vet ./..."

if [ "$QUICK_MODE" = false ]; then
    run_check "Go tests" "go test ./..."
    run_check "Go build" "go build -o /dev/null ."
fi

# =============================================================================
# Elixir Checks (if mix is available)
# =============================================================================
if command -v mix &> /dev/null; then
    echo ""
    log "=== Elixir Quality Gates ==="

    cd "$REPO_ROOT/apps/query-service"

    if [ -f "mix.exs" ]; then
        run_check "Elixir formatting" "mix format --check-formatted"
        run_check "Elixir compile warnings" "mix compile --warnings-as-errors"

        if [ "$QUICK_MODE" = false ]; then
            run_check "Elixir tests" "mix test"
            run_check "Credo analysis" "mix credo --strict"
        fi
    fi
fi

# =============================================================================
# Security Checks
# =============================================================================
if [ "$QUICK_MODE" = false ]; then
    echo ""
    log "=== Security Checks ==="

    cd "$REPO_ROOT/apps/core"
    if command -v cargo-audit &> /dev/null; then
        run_check "Cargo audit" "cargo audit"
    else
        warn "cargo-audit not installed (install with: cargo install cargo-audit)"
    fi

    cd "$REPO_ROOT/apps/control-plane"
    if command -v govulncheck &> /dev/null; then
        run_check "Go vulnerability check" "govulncheck ./..."
    else
        warn "govulncheck not installed (install with: go install golang.org/x/vuln/cmd/govulncheck@latest)"
    fi
fi

# =============================================================================
# Summary
# =============================================================================
echo ""
echo "=============================================="
echo "           QUALITY GATE SUMMARY"
echo "=============================================="

if [ $FAILED -eq 0 ]; then
    echo -e "${GREEN}✓ All checks passed! Safe to push.${NC}"
    exit 0
else
    echo -e "${RED}✗ Some checks failed. Fix issues before pushing.${NC}"
    exit 1
fi
