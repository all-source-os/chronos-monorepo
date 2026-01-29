#!/usr/bin/env bash
# =============================================================================
# Chronos Container Testing Script
# World-class container build verification before pushing to GitHub
# =============================================================================

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color
BOLD='\033[1m'

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
TIMESTAMP=$(date +%Y%m%d_%H%M%S)
LOG_DIR="${REPO_ROOT}/.container-test-logs"
SUMMARY_FILE="${LOG_DIR}/summary_${TIMESTAMP}.md"

# Container definitions
declare -A CONTAINERS=(
    ["core"]="apps/core:chronos-core:3900"
    ["query-service"]="apps/query-service:chronos-query-service:4000"
    ["mcp-server"]="apps/mcp-server-elixir:chronos-mcp-server:4001"
    ["control-plane"]="apps/control-plane:chronos-control-plane:8080"
    ["web"]=".:chronos-web:3000"
)

# Web uses monorepo root as context with -f flag
WEB_DOCKERFILE="apps/web/Dockerfile"

# Test results
declare -A BUILD_RESULTS
declare -A SECURITY_RESULTS
declare -A SIZE_RESULTS
declare -A STARTUP_RESULTS

# =============================================================================
# Helper Functions
# =============================================================================

log() {
    echo -e "${BLUE}[$(date +'%H:%M:%S')]${NC} $*"
}

log_success() {
    echo -e "${GREEN}✓${NC} $*"
}

log_error() {
    echo -e "${RED}✗${NC} $*"
}

log_warning() {
    echo -e "${YELLOW}!${NC} $*"
}

log_section() {
    echo ""
    echo -e "${BOLD}${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo -e "${BOLD}${CYAN}  $*${NC}"
    echo -e "${BOLD}${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
}

ensure_log_dir() {
    mkdir -p "$LOG_DIR"
}

# =============================================================================
# Build Tests
# =============================================================================

test_build() {
    local name=$1
    local config=${CONTAINERS[$name]}
    local context=$(echo "$config" | cut -d: -f1)
    local image=$(echo "$config" | cut -d: -f2)
    local log_file="${LOG_DIR}/build_${name}_${TIMESTAMP}.log"

    log "Building ${BOLD}$name${NC}..."
    
    local build_cmd
    local build_context
    
    if [[ "$name" == "web" ]]; then
        # Web builds from monorepo root with -f flag
        build_context="$REPO_ROOT"
        build_cmd="docker build --no-cache -f $WEB_DOCKERFILE -t ${image}:test $build_context"
    else
        build_context="$REPO_ROOT/$context"
        build_cmd="docker build --no-cache -t ${image}:test $build_context"
    fi
    
    local start_time=$(date +%s)
    
    if $build_cmd > "$log_file" 2>&1; then
        local end_time=$(date +%s)
        local duration=$((end_time - start_time))
        BUILD_RESULTS[$name]="PASS:${duration}s"
        log_success "$name built successfully in ${duration}s"
        return 0
    else
        BUILD_RESULTS[$name]="FAIL"
        log_error "$name build failed. See: $log_file"
        return 1
    fi
}

# =============================================================================
# Security Scanning
# =============================================================================

test_security() {
    local name=$1
    local config=${CONTAINERS[$name]}
    local image=$(echo "$config" | cut -d: -f2)
    local log_file="${LOG_DIR}/security_${name}_${TIMESTAMP}.log"

    log "Scanning ${BOLD}$name${NC} for security issues..."

    # Check if trivy is available
    if ! command -v trivy &> /dev/null; then
        log_warning "Trivy not installed, skipping security scan for $name"
        SECURITY_RESULTS[$name]="SKIP:trivy not installed"
        return 0
    fi

    if trivy image --severity HIGH,CRITICAL --exit-code 0 "${image}:test" > "$log_file" 2>&1; then
        local high_count=$(grep -c "HIGH" "$log_file" 2>/dev/null || echo "0")
        local critical_count=$(grep -c "CRITICAL" "$log_file" 2>/dev/null || echo "0")
        
        if [[ "$critical_count" -gt 0 ]]; then
            SECURITY_RESULTS[$name]="WARN:${critical_count} critical, ${high_count} high"
            log_warning "$name has ${critical_count} critical and ${high_count} high vulnerabilities"
        elif [[ "$high_count" -gt 0 ]]; then
            SECURITY_RESULTS[$name]="WARN:${high_count} high"
            log_warning "$name has ${high_count} high severity vulnerabilities"
        else
            SECURITY_RESULTS[$name]="PASS"
            log_success "$name passed security scan"
        fi
        return 0
    else
        SECURITY_RESULTS[$name]="FAIL"
        log_error "$name security scan failed"
        return 1
    fi
}

# =============================================================================
# Image Size Analysis
# =============================================================================

test_image_size() {
    local name=$1
    local config=${CONTAINERS[$name]}
    local image=$(echo "$config" | cut -d: -f2)

    log "Checking ${BOLD}$name${NC} image size..."

    local size=$(docker images --format "{{.Size}}" "${image}:test" 2>/dev/null)
    
    if [[ -n "$size" ]]; then
        SIZE_RESULTS[$name]="$size"
        
        # Parse size for threshold checking
        local size_mb=0
        if [[ "$size" == *GB* ]]; then
            size_mb=$(echo "$size" | sed 's/GB//' | awk '{printf "%.0f", $1 * 1024}')
        elif [[ "$size" == *MB* ]]; then
            size_mb=$(echo "$size" | sed 's/MB//' | awk '{printf "%.0f", $1}')
        fi
        
        # Warn if image is over 500MB
        if [[ "$size_mb" -gt 500 ]]; then
            log_warning "$name image is large: $size (consider optimizing)"
        else
            log_success "$name image size: $size"
        fi
        return 0
    else
        SIZE_RESULTS[$name]="UNKNOWN"
        log_error "Could not determine $name image size"
        return 1
    fi
}

# =============================================================================
# Container Startup Test
# =============================================================================

test_startup() {
    local name=$1
    local config=${CONTAINERS[$name]}
    local image=$(echo "$config" | cut -d: -f2)
    local port=$(echo "$config" | cut -d: -f3)
    local container_name="test-${name}-${TIMESTAMP}"

    log "Testing ${BOLD}$name${NC} container startup..."

    # Start container
    local container_id
    container_id=$(docker run -d --name "$container_name" -p "${port}:${port}" "${image}:test" 2>/dev/null)
    
    if [[ -z "$container_id" ]]; then
        STARTUP_RESULTS[$name]="FAIL:could not start"
        log_error "$name container failed to start"
        return 1
    fi

    # Wait for container to be healthy or running
    local max_wait=30
    local waited=0
    local healthy=false
    
    while [[ $waited -lt $max_wait ]]; do
        local status=$(docker inspect --format='{{.State.Status}}' "$container_name" 2>/dev/null)
        local health=$(docker inspect --format='{{if .State.Health}}{{.State.Health.Status}}{{else}}no-healthcheck{{end}}' "$container_name" 2>/dev/null)
        
        if [[ "$status" == "exited" ]]; then
            local exit_code=$(docker inspect --format='{{.State.ExitCode}}' "$container_name" 2>/dev/null)
            STARTUP_RESULTS[$name]="FAIL:exited with code $exit_code"
            log_error "$name container exited with code $exit_code"
            docker logs "$container_name" >> "${LOG_DIR}/startup_${name}_${TIMESTAMP}.log" 2>&1
            docker rm -f "$container_name" > /dev/null 2>&1
            return 1
        fi
        
        if [[ "$health" == "healthy" ]] || [[ "$health" == "no-healthcheck" && "$status" == "running" ]]; then
            healthy=true
            break
        fi
        
        sleep 1
        ((waited++))
    done

    # Cleanup
    docker rm -f "$container_name" > /dev/null 2>&1

    if $healthy; then
        STARTUP_RESULTS[$name]="PASS:started in ${waited}s"
        log_success "$name container started successfully in ${waited}s"
        return 0
    else
        STARTUP_RESULTS[$name]="FAIL:timeout after ${max_wait}s"
        log_error "$name container failed to become healthy within ${max_wait}s"
        return 1
    fi
}

# =============================================================================
# Dockerfile Best Practices Check
# =============================================================================

test_dockerfile_lint() {
    local name=$1
    local config=${CONTAINERS[$name]}
    local context=$(echo "$config" | cut -d: -f1)
    local dockerfile
    
    if [[ "$name" == "web" ]]; then
        dockerfile="$REPO_ROOT/$WEB_DOCKERFILE"
    else
        dockerfile="$REPO_ROOT/$context/Dockerfile"
    fi

    log "Linting ${BOLD}$name${NC} Dockerfile..."

    # Check if hadolint is available
    if ! command -v hadolint &> /dev/null; then
        log_warning "Hadolint not installed, skipping Dockerfile lint for $name"
        return 0
    fi

    local log_file="${LOG_DIR}/lint_${name}_${TIMESTAMP}.log"
    
    if hadolint "$dockerfile" > "$log_file" 2>&1; then
        log_success "$name Dockerfile passed lint"
        return 0
    else
        local warnings=$(wc -l < "$log_file" | tr -d ' ')
        log_warning "$name Dockerfile has $warnings lint warnings"
        return 0  # Don't fail on lint warnings
    fi
}

# =============================================================================
# Generate Summary Report
# =============================================================================

generate_summary() {
    log_section "Generating Test Summary"
    
    cat > "$SUMMARY_FILE" << EOF
# Container Test Summary

**Generated:** $(date -u +"%Y-%m-%d %H:%M:%S UTC")
**Repository:** chronos-monorepo

## Build Results

| Container | Status | Duration |
|-----------|--------|----------|
EOF

    local all_passed=true
    
    for name in "${!CONTAINERS[@]}"; do
        local result=${BUILD_RESULTS[$name]:-"NOT_RUN"}
        local status="❓"
        local duration="-"
        
        if [[ "$result" == PASS:* ]]; then
            status="✅ PASS"
            duration="${result#PASS:}"
        elif [[ "$result" == "FAIL" ]]; then
            status="❌ FAIL"
            all_passed=false
        fi
        
        echo "| $name | $status | $duration |" >> "$SUMMARY_FILE"
    done

    cat >> "$SUMMARY_FILE" << EOF

## Image Sizes

| Container | Size |
|-----------|------|
EOF

    for name in "${!CONTAINERS[@]}"; do
        local size=${SIZE_RESULTS[$name]:-"N/A"}
        echo "| $name | $size |" >> "$SUMMARY_FILE"
    done

    cat >> "$SUMMARY_FILE" << EOF

## Startup Tests

| Container | Status |
|-----------|--------|
EOF

    for name in "${!CONTAINERS[@]}"; do
        local result=${STARTUP_RESULTS[$name]:-"NOT_RUN"}
        local status="❓"
        
        if [[ "$result" == PASS:* ]]; then
            status="✅ ${result#PASS:}"
        elif [[ "$result" == FAIL:* ]]; then
            status="❌ ${result#FAIL:}"
            all_passed=false
        fi
        
        echo "| $name | $status |" >> "$SUMMARY_FILE"
    done

    cat >> "$SUMMARY_FILE" << EOF

## Security Scans

| Container | Status |
|-----------|--------|
EOF

    for name in "${!CONTAINERS[@]}"; do
        local result=${SECURITY_RESULTS[$name]:-"NOT_RUN"}
        local status="❓"
        
        if [[ "$result" == "PASS" ]]; then
            status="✅ PASS"
        elif [[ "$result" == SKIP:* ]]; then
            status="⏭️ ${result#SKIP:}"
        elif [[ "$result" == WARN:* ]]; then
            status="⚠️ ${result#WARN:}"
        elif [[ "$result" == "FAIL" ]]; then
            status="❌ FAIL"
        fi
        
        echo "| $name | $status |" >> "$SUMMARY_FILE"
    done

    cat >> "$SUMMARY_FILE" << EOF

---

## Logs

All detailed logs are available in: \`${LOG_DIR}\`
EOF

    echo ""
    cat "$SUMMARY_FILE"
    echo ""
    log "Full summary saved to: $SUMMARY_FILE"
    
    if $all_passed; then
        return 0
    else
        return 1
    fi
}

# =============================================================================
# Cleanup
# =============================================================================

cleanup_test_images() {
    log "Cleaning up test images..."
    
    for name in "${!CONTAINERS[@]}"; do
        local config=${CONTAINERS[$name]}
        local image=$(echo "$config" | cut -d: -f2)
        docker rmi "${image}:test" > /dev/null 2>&1 || true
    done
    
    log_success "Cleanup complete"
}

# =============================================================================
# Main Execution
# =============================================================================

usage() {
    cat << EOF
Usage: $(basename "$0") [OPTIONS] [CONTAINERS...]

Options:
    -h, --help          Show this help message
    -b, --build-only    Only run build tests
    -s, --skip-security Skip security scanning
    -k, --skip-startup  Skip startup tests
    -c, --cleanup       Clean up test images after tests
    -q, --quick         Quick mode: build only, no startup/security
    -l, --list          List available containers

Containers:
    core            Rust event store
    query-service   Elixir query service
    mcp-server      Elixir MCP server
    control-plane   Go control plane
    web             Next.js web UI

Examples:
    $(basename "$0")                    # Test all containers
    $(basename "$0") core web           # Test only core and web
    $(basename "$0") -q                 # Quick build test only
    $(basename "$0") -c core            # Test core and cleanup
EOF
}

main() {
    local build_only=false
    local skip_security=false
    local skip_startup=false
    local do_cleanup=false
    local containers_to_test=()

    # Parse arguments
    while [[ $# -gt 0 ]]; do
        case $1 in
            -h|--help)
                usage
                exit 0
                ;;
            -b|--build-only)
                build_only=true
                shift
                ;;
            -s|--skip-security)
                skip_security=true
                shift
                ;;
            -k|--skip-startup)
                skip_startup=true
                shift
                ;;
            -c|--cleanup)
                do_cleanup=true
                shift
                ;;
            -q|--quick)
                build_only=true
                skip_security=true
                skip_startup=true
                shift
                ;;
            -l|--list)
                echo "Available containers:"
                for name in "${!CONTAINERS[@]}"; do
                    echo "  - $name"
                done
                exit 0
                ;;
            -*)
                echo "Unknown option: $1"
                usage
                exit 1
                ;;
            *)
                if [[ -n "${CONTAINERS[$1]:-}" ]]; then
                    containers_to_test+=("$1")
                else
                    echo "Unknown container: $1"
                    usage
                    exit 1
                fi
                shift
                ;;
        esac
    done

    # Default to all containers if none specified
    if [[ ${#containers_to_test[@]} -eq 0 ]]; then
        containers_to_test=("${!CONTAINERS[@]}")
    fi

    # Start testing
    log_section "Chronos Container Test Suite"
    log "Testing containers: ${containers_to_test[*]}"
    log "Repository root: $REPO_ROOT"
    
    ensure_log_dir
    
    local exit_code=0

    # Build tests
    log_section "Build Tests"
    for name in "${containers_to_test[@]}"; do
        if ! test_build "$name"; then
            exit_code=1
        fi
    done

    # Skip remaining tests if builds failed or build-only mode
    if [[ $exit_code -ne 0 ]]; then
        log_error "Build failures detected, skipping remaining tests"
        generate_summary || true
        exit $exit_code
    fi

    if ! $build_only; then
        # Image size analysis
        log_section "Image Size Analysis"
        for name in "${containers_to_test[@]}"; do
            test_image_size "$name"
        done

        # Dockerfile lint
        log_section "Dockerfile Linting"
        for name in "${containers_to_test[@]}"; do
            test_dockerfile_lint "$name"
        done

        # Startup tests
        if ! $skip_startup; then
            log_section "Startup Tests"
            for name in "${containers_to_test[@]}"; do
                if ! test_startup "$name"; then
                    exit_code=1
                fi
            done
        fi

        # Security scanning
        if ! $skip_security; then
            log_section "Security Scanning"
            for name in "${containers_to_test[@]}"; do
                test_security "$name"
            done
        fi
    fi

    # Generate summary
    generate_summary || exit_code=1

    # Cleanup if requested
    if $do_cleanup; then
        cleanup_test_images
    fi

    if [[ $exit_code -eq 0 ]]; then
        log_section "All Tests Passed! ✅"
    else
        log_section "Some Tests Failed! ❌"
    fi

    exit $exit_code
}

main "$@"
