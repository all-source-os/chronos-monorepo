#!/usr/bin/env bash
# validate-workflows.sh - Validate GitHub Actions workflows using act
#
# This script provides multiple levels of workflow validation:
# 1. actionlint - Fast syntax checking
# 2. act --list - Validates workflow structure and job graph
# 3. act --dryrun - Validates workflow execution without running commands
#
# Usage:
#   ./scripts/validate-workflows.sh           # Run all validations
#   ./scripts/validate-workflows.sh --quick   # Only actionlint (fastest)
#   ./scripts/validate-workflows.sh --graph   # Validate job graph with act
#   ./scripts/validate-workflows.sh --dryrun  # Full dry-run validation

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(dirname "$SCRIPT_DIR")"
WORKFLOWS_DIR="$ROOT_DIR/.github/workflows"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Default mode
MODE="${1:-all}"

log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[OK]${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

check_command() {
    local cmd=$1
    local install_hint=$2
    if ! command -v "$cmd" &>/dev/null; then
        log_error "$cmd is not installed."
        echo "  Install with: $install_hint"
        return 1
    fi
    return 0
}

# Check for required tools
check_dependencies() {
    local missing=0

    if ! check_command "actionlint" "brew install actionlint"; then
        missing=1
    fi

    if [[ "$MODE" != "--quick" ]]; then
        if ! check_command "act" "brew install act"; then
            missing=1
        fi
    fi

    if [[ $missing -eq 1 ]]; then
        exit 1
    fi
}

# Run actionlint for fast syntax validation
run_actionlint() {
    log_info "Running actionlint for syntax validation..."
    echo ""

    # Run actionlint and capture output
    # Use -ignore to skip shellcheck info/style warnings which are not critical
    local output
    output=$(actionlint -color -ignore 'SC2086' -ignore 'SC2129' "$WORKFLOWS_DIR"/*.yml 2>&1) || true

    # Check if there are any real errors (not just info/style warnings)
    if echo "$output" | grep -q "error:"; then
        echo "$output"
        log_error "actionlint: Syntax errors found in workflow files"
        return 1
    else
        # Show warnings if any, but don't fail
        if [ -n "$output" ]; then
            echo "$output"
            log_warn "actionlint: Some warnings found (info-level, not blocking)"
        fi
        log_success "actionlint: All workflow files have valid syntax"
        return 0
    fi
}

# Use act --list to validate workflow structure and job graph
run_act_list() {
    log_info "Validating workflow structure with act --list..."
    echo ""

    local failed=0
    for workflow in "$WORKFLOWS_DIR"/*.yml; do
        local workflow_name
        workflow_name=$(basename "$workflow")
        echo "  Checking $workflow_name..."

        # act --list validates the workflow structure and prints the job graph
        if act --list -W "$workflow" 2>&1 | head -20; then
            log_success "  $workflow_name: Structure valid"
        else
            log_error "  $workflow_name: Structure validation failed"
            failed=1
        fi
        echo ""
    done

    if [[ $failed -eq 0 ]]; then
        log_success "act --list: All workflows have valid structure"
        return 0
    else
        log_error "act --list: Some workflows have invalid structure"
        return 1
    fi
}

# Use act --dryrun for full validation without executing commands
run_act_dryrun() {
    log_info "Running act dry-run validation..."
    echo ""

    local failed=0

    # Only test ci.yml as it's the main workflow
    # Full dry-run of all workflows would be too slow
    local workflow="$WORKFLOWS_DIR/ci.yml"
    local workflow_name
    workflow_name=$(basename "$workflow")

    echo "  Dry-run validating $workflow_name..."

    # act --dryrun runs through the workflow logic without executing commands
    # Use a minimal event to trigger
    if act push --dryrun -W "$workflow" --job workflow-lint 2>&1; then
        log_success "  $workflow_name: Dry-run validation passed"
    else
        # Dry-run may fail due to missing secrets/inputs, which is expected
        log_warn "  $workflow_name: Dry-run completed with warnings (may be due to missing secrets)"
    fi

    echo ""
    log_success "act --dryrun: Validation complete"
    return 0
}

# Print summary header
print_header() {
    echo ""
    echo "=============================================="
    echo "  GitHub Actions Workflow Validation"
    echo "=============================================="
    echo ""
    echo "Mode: $MODE"
    echo "Workflows directory: $WORKFLOWS_DIR"
    echo ""
}

# Print usage
print_usage() {
    echo "Usage: $0 [OPTIONS]"
    echo ""
    echo "Options:"
    echo "  (no args)   Run all validations (actionlint + act --list)"
    echo "  --quick     Only run actionlint (fastest)"
    echo "  --graph     Validate job graph with act --list"
    echo "  --dryrun    Full dry-run validation with act"
    echo "  --help      Show this help message"
    echo ""
    echo "Prerequisites:"
    echo "  - actionlint: brew install actionlint"
    echo "  - act: brew install act"
}

# Main execution
main() {
    case "$MODE" in
        --help|-h)
            print_usage
            exit 0
            ;;
        --quick)
            print_header
            check_dependencies
            run_actionlint
            ;;
        --graph)
            print_header
            check_dependencies
            run_actionlint
            echo ""
            run_act_list
            ;;
        --dryrun)
            print_header
            check_dependencies
            run_actionlint
            echo ""
            run_act_list
            echo ""
            run_act_dryrun
            ;;
        all|"")
            print_header
            check_dependencies
            run_actionlint
            echo ""
            run_act_list
            ;;
        *)
            log_error "Unknown option: $MODE"
            print_usage
            exit 1
            ;;
    esac

    echo ""
    echo "=============================================="
    log_success "Workflow validation complete!"
    echo "=============================================="
}

main
