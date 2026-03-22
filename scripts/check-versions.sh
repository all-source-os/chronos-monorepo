#!/usr/bin/env bash
# =============================================================================
# Version Consistency Checker for AllSource Monorepo
# =============================================================================
# Ensures version declarations are consistent across:
# - Dockerfiles
# - CI workflows
# - Package manifests (Cargo.toml, go.mod, package.json, mix.exs)
#
# Usage: ./scripts/check-versions.sh [--json]
# =============================================================================

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Flags
JSON_OUTPUT=false
ERRORS=0
WARNINGS=0

# Parse arguments
for arg in "$@"; do
    case $arg in
        --json)
            JSON_OUTPUT=true
            shift
            ;;
    esac
done

# Get script directory and repo root
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

cd "$REPO_ROOT"

# JSON output accumulator
declare -a JSON_RESULTS=()

# =============================================================================
# Helper Functions
# =============================================================================

log() {
    if [ "$JSON_OUTPUT" = false ]; then
        echo -e "$1"
    fi
}

log_success() {
    if [ "$JSON_OUTPUT" = false ]; then
        echo -e "${GREEN}✓${NC} $1"
    fi
}

log_warning() {
    ((WARNINGS++)) || true
    if [ "$JSON_OUTPUT" = false ]; then
        echo -e "${YELLOW}⚠${NC} $1"
    fi
}

log_error() {
    ((ERRORS++)) || true
    if [ "$JSON_OUTPUT" = false ]; then
        echo -e "${RED}✗${NC} $1"
    fi
}

add_json_result() {
    local service="$1"
    local component="$2"
    local expected="$3"
    local actual="$4"
    local status="$5"
    local file="$6"
    
    JSON_RESULTS+=("{\"service\":\"$service\",\"component\":\"$component\",\"expected\":\"$expected\",\"actual\":\"$actual\",\"status\":\"$status\",\"file\":\"$file\"}")
}

extract_dockerfile_arg() {
    local file="$1"
    local arg_name="$2"
    
    if [ -f "$file" ]; then
        grep -E "^ARG ${arg_name}=" "$file" 2>/dev/null | sed "s/ARG ${arg_name}=//" | head -1 || echo ""
    else
        echo ""
    fi
}

# =============================================================================
# Version Extraction
# =============================================================================

get_rust_versions() {
    log "=== Rust Versions ==="
    
    # Cargo.toml MSRV
    local cargo_msrv=""
    if [ -f "apps/core/Cargo.toml" ]; then
        cargo_msrv=$(grep -E '^rust-version\s*=' apps/core/Cargo.toml | sed 's/.*"\(.*\)".*/\1/' || echo "")
    fi
    
    # Dockerfile
    local dockerfile_rust=$(extract_dockerfile_arg "apps/core/Dockerfile" "RUST_VERSION")
    
    # CI workflow - look for toolchain in rust section
    local ci_rust=""
    if [ -f ".github/workflows/ci.yml" ]; then
        ci_rust=$(grep -E 'toolchain:\s*"?[0-9]+\.[0-9]+' .github/workflows/ci.yml | head -1 | grep -oE '[0-9]+\.[0-9]+' | head -1 || echo "")
    fi
    
    log "  Cargo.toml MSRV:    ${cargo_msrv:-not set}"
    log "  Dockerfile:         ${dockerfile_rust:-not set}"
    log "  CI workflow:        ${ci_rust:-not set}"
    
    # Validate
    if [ -n "$dockerfile_rust" ] && [ -n "$ci_rust" ]; then
        if [ "$dockerfile_rust" != "$ci_rust" ]; then
            log_error "Rust version mismatch: Dockerfile=$dockerfile_rust, CI=$ci_rust"
            add_json_result "core" "rust" "$dockerfile_rust" "$ci_rust" "error" ".github/workflows/ci.yml"
        else
            log_success "Rust versions consistent: $dockerfile_rust"
            add_json_result "core" "rust" "$dockerfile_rust" "$ci_rust" "ok" ""
        fi
    fi
    
    # MSRV should be <= build version
    if [ -n "$cargo_msrv" ] && [ -n "$dockerfile_rust" ]; then
        local msrv_major=$(echo "$cargo_msrv" | cut -d. -f1)
        local msrv_minor=$(echo "$cargo_msrv" | cut -d. -f2)
        local build_major=$(echo "$dockerfile_rust" | cut -d. -f1)
        local build_minor=$(echo "$dockerfile_rust" | cut -d. -f2)
        
        if [ "$msrv_major" -gt "$build_major" ] || \
           ([ "$msrv_major" -eq "$build_major" ] && [ "$msrv_minor" -gt "$build_minor" ]); then
            log_error "MSRV ($cargo_msrv) is higher than build version ($dockerfile_rust)"
            add_json_result "core" "rust-msrv" "$cargo_msrv" "$dockerfile_rust" "error" "apps/core/Cargo.toml"
        else
            log_success "MSRV ($cargo_msrv) is compatible with build version ($dockerfile_rust)"
        fi
    fi
    log ""
}

get_go_versions() {
    log "=== Go Versions ==="
    
    # go.mod
    local gomod_version=""
    if [ -f "apps/control-plane/go.mod" ]; then
        gomod_version=$(grep -E '^go\s+' apps/control-plane/go.mod | awk '{print $2}' || echo "")
    fi
    
    # Dockerfile
    local dockerfile_go=$(extract_dockerfile_arg "apps/control-plane/Dockerfile" "GO_VERSION")
    
    # CI workflow - look for go-version
    local ci_go=""
    if [ -f ".github/workflows/ci.yml" ]; then
        ci_go=$(grep 'go-version:' .github/workflows/ci.yml | head -1 | sed 's/.*go-version:[[:space:]]*"\{0,1\}\([^"]*\)"\{0,1\}/\1/' | tr -d ' "' || echo "")
    fi
    
    log "  go.mod:             ${gomod_version:-not set}"
    log "  Dockerfile:         ${dockerfile_go:-not set}"
    log "  CI workflow:        ${ci_go:-not set}"
    
    # Validate all three match
    local versions_to_check=("$gomod_version" "$dockerfile_go" "$ci_go")
    local unique_versions=($(printf '%s\n' "${versions_to_check[@]}" | grep -v '^$' | sort -u))
    
    if [ ${#unique_versions[@]} -gt 1 ]; then
        log_error "Go version mismatch: go.mod=$gomod_version, Dockerfile=$dockerfile_go, CI=$ci_go"
        add_json_result "control-plane" "go" "$gomod_version" "$dockerfile_go|$ci_go" "error" "multiple"
    elif [ ${#unique_versions[@]} -eq 1 ]; then
        log_success "Go versions consistent: ${unique_versions[0]}"
        add_json_result "control-plane" "go" "${unique_versions[0]}" "${unique_versions[0]}" "ok" ""
    fi
    log ""
}

get_node_versions() {
    log "=== Node/Bun Versions ==="
    
    # package.json engines (if defined)
    local pkg_node=""
    if [ -f "package.json" ]; then
        pkg_node=$(grep -oP '"node":\s*"\K[^"]+' package.json 2>/dev/null || echo "")
    fi
    
    # Dockerfile
    local dockerfile_node=$(extract_dockerfile_arg "apps/web/Dockerfile" "NODE_VERSION")
    local dockerfile_bun=$(extract_dockerfile_arg "apps/web/Dockerfile" "BUN_VERSION")
    
    # CI workflow (uses setup-bun with 'latest')
    local ci_bun="latest"
    
    log "  package.json:       ${pkg_node:-not set}"
    log "  Dockerfile Node:    ${dockerfile_node:-not set}"
    log "  Dockerfile Bun:     ${dockerfile_bun:-not set}"
    log "  CI Bun:             ${ci_bun}"
    
    if [ -n "$dockerfile_node" ]; then
        log_success "Node version in Dockerfile: $dockerfile_node"
        add_json_result "web" "node" "$dockerfile_node" "$dockerfile_node" "ok" ""
    fi
    
    if [ -n "$dockerfile_bun" ]; then
        log_success "Bun version in Dockerfile: $dockerfile_bun"
        add_json_result "web" "bun" "$dockerfile_bun" "$ci_bun" "ok" ""
    fi
    log ""
}

get_elixir_versions() {
    log "=== Elixir/OTP Versions ==="
    
    for service in "query-service" "mcp-server-elixir"; do
        local service_path="apps/$service"
        
        if [ ! -d "$service_path" ]; then
            continue
        fi
        
        log "  --- $service ---"
        
        # mix.exs elixir version
        local mix_elixir=""
        if [ -f "$service_path/mix.exs" ]; then
            mix_elixir=$(grep -oP 'elixir:\s*"~>\s*\K[0-9.]+' "$service_path/mix.exs" 2>/dev/null || echo "")
        fi
        
        # Dockerfile
        local dockerfile_elixir=$(extract_dockerfile_arg "$service_path/Dockerfile" "ELIXIR_VERSION")
        local dockerfile_otp=$(extract_dockerfile_arg "$service_path/Dockerfile" "OTP_VERSION")
        
        # CI workflow
        local ci_elixir=""
        local ci_otp=""
        if [ -f ".github/workflows/ci.yml" ]; then
            ci_elixir=$(grep 'elixir-version:' .github/workflows/ci.yml | head -1 | sed 's/.*elixir-version:[[:space:]]*"\{0,1\}\([^"]*\)"\{0,1\}/\1/' | tr -d ' "' || echo "")
            ci_otp=$(grep 'otp-version:' .github/workflows/ci.yml | head -1 | sed 's/.*otp-version:[[:space:]]*"\{0,1\}\([^"]*\)"\{0,1\}/\1/' | tr -d ' "' || echo "")
        fi
        
        log "    mix.exs:          ${mix_elixir:-not set}"
        log "    Dockerfile:       Elixir ${dockerfile_elixir:-?}, OTP ${dockerfile_otp:-?}"
        log "    CI workflow:      Elixir ${ci_elixir:-?}, OTP ${ci_otp:-?}"
        
        # Validate Elixir versions
        if [ -n "$dockerfile_elixir" ] && [ -n "$ci_elixir" ]; then
            if [ "$dockerfile_elixir" != "$ci_elixir" ]; then
                log_error "$service: Elixir version mismatch: Dockerfile=$dockerfile_elixir, CI=$ci_elixir"
                add_json_result "$service" "elixir" "$dockerfile_elixir" "$ci_elixir" "error" ".github/workflows/ci.yml"
            else
                log_success "$service: Elixir versions consistent: $dockerfile_elixir"
                add_json_result "$service" "elixir" "$dockerfile_elixir" "$ci_elixir" "ok" ""
            fi
        fi
        
        # Validate OTP versions
        if [ -n "$dockerfile_otp" ] && [ -n "$ci_otp" ]; then
            if [ "$dockerfile_otp" != "$ci_otp" ]; then
                log_error "$service: OTP version mismatch: Dockerfile=$dockerfile_otp, CI=$ci_otp"
                add_json_result "$service" "otp" "$dockerfile_otp" "$ci_otp" "error" ".github/workflows/ci.yml"
            else
                log_success "$service: OTP versions consistent: $dockerfile_otp"
                add_json_result "$service" "otp" "$dockerfile_otp" "$ci_otp" "ok" ""
            fi
        fi
    done
    log ""
}

get_app_versions() {
    log "=== Application Versions ==="

    declare -A app_versions

    # Rust Core - Cargo.toml
    if [ -f "apps/core/Cargo.toml" ]; then
        local core_version=$(grep -E '^version\s*=' apps/core/Cargo.toml | head -1 | sed 's/.*"\(.*\)".*/\1/' || echo "")
        if [ -n "$core_version" ]; then
            app_versions["core"]=$core_version
            log "  Core (Cargo.toml):           $core_version"
        fi
    fi

    # Go Control Plane - main.go healthHandler
    if [ -f "apps/control-plane/main.go" ]; then
        local control_version=$(grep '"version":' apps/control-plane/main.go | head -1 | grep -oE '[0-9]+\.[0-9]+\.[0-9]+' || echo "")
        if [ -n "$control_version" ]; then
            app_versions["control-main"]=$control_version
            log "  Control (main.go):           $control_version"
        fi
    fi

    # Go Control Plane - main_v1.go Version const
    if [ -f "apps/control-plane/main_v1.go" ]; then
        local control_v1_version=$(grep 'Version\s*=' apps/control-plane/main_v1.go | grep -oE '[0-9]+\.[0-9]+\.[0-9]+' || echo "")
        if [ -n "$control_v1_version" ]; then
            app_versions["control-v1"]=$control_v1_version
            log "  Control (main_v1.go):        $control_v1_version"
        fi
    fi

    # Go Control Plane - tracing.go serviceVersion
    if [ -f "apps/control-plane/tracing.go" ]; then
        local control_tracing_version=$(grep 'serviceVersion\s*=' apps/control-plane/tracing.go | grep -oE '[0-9]+\.[0-9]+\.[0-9]+' || echo "")
        if [ -n "$control_tracing_version" ]; then
            app_versions["control-tracing"]=$control_tracing_version
            log "  Control (tracing.go):        $control_tracing_version"
        fi
    fi

    # Query Service - mix.exs
    if [ -f "apps/query-service/mix.exs" ]; then
        local query_version=$(grep 'version:' apps/query-service/mix.exs | head -1 | grep -oE '[0-9]+\.[0-9]+\.[0-9]+' || echo "")
        if [ -n "$query_version" ]; then
            app_versions["query-service"]=$query_version
            log "  Query Service (mix.exs):     $query_version"
        fi
    fi

    # MCP Server - mix.exs
    if [ -f "apps/mcp-server-elixir/mix.exs" ]; then
        local mcp_version=$(grep 'version:' apps/mcp-server-elixir/mix.exs | head -1 | grep -oE '[0-9]+\.[0-9]+\.[0-9]+' || echo "")
        if [ -n "$mcp_version" ]; then
            app_versions["mcp-server"]=$mcp_version
            log "  MCP Server (mix.exs):        $mcp_version"
        fi
    fi

    # Prime MCP Server - Cargo.toml
    if [ -f "apps/prime-mcp/Cargo.toml" ]; then
        local prime_version=$(grep -E '^version\s*=' apps/prime-mcp/Cargo.toml | head -1 | sed 's/.*"\(.*\)".*/\1/' || echo "")
        if [ -n "$prime_version" ]; then
            app_versions["prime-mcp"]=$prime_version
            log "  Prime MCP (Cargo.toml):      $prime_version"
        fi
    fi

    # K8s Core image
    if [ -f "deploy/k8s/core.yaml" ]; then
        local k8s_core_version=$(grep 'image:.*allsource.*core' deploy/k8s/core.yaml | grep -oE '[0-9]+\.[0-9]+\.[0-9]+' || echo "")
        if [ -n "$k8s_core_version" ]; then
            app_versions["k8s-core"]=$k8s_core_version
            log "  K8s Core (image tag):        $k8s_core_version"
        fi
    fi

    # K8s Query Service image
    if [ -f "deploy/k8s/query-service.yaml" ]; then
        local k8s_query_version=$(grep 'image:.*allsource.*query' deploy/k8s/query-service.yaml | grep -oE '[0-9]+\.[0-9]+\.[0-9]+' || echo "")
        if [ -n "$k8s_query_version" ]; then
            app_versions["k8s-query"]=$k8s_query_version
            log "  K8s Query (image tag):       $k8s_query_version"
        fi
    fi

    # Check consistency
    local unique_versions=($(printf '%s\n' "${app_versions[@]}" | sort -u))

    if [ ${#unique_versions[@]} -gt 1 ]; then
        log_error "Application version mismatch detected!"
        log_error "  Found versions: ${unique_versions[*]}"
        log_error "  Run 'make set-version VERSION=X.Y.Z' to fix"
        for service in "${!app_versions[@]}"; do
            add_json_result "$service" "app-version" "${unique_versions[0]}" "${app_versions[$service]}" "error" ""
        done
    elif [ ${#unique_versions[@]} -eq 1 ]; then
        log_success "All application versions consistent: ${unique_versions[0]}"
        add_json_result "all" "app-version" "${unique_versions[0]}" "${unique_versions[0]}" "ok" ""
    fi
    log ""
}

get_sdk_versions() {
    log "=== SDK Versions ==="

    declare -A sdk_versions

    # Rust SDK
    if [ -f "sdks/rust/Cargo.toml" ]; then
        local rust_sdk=$(grep -E '^version\s*=' sdks/rust/Cargo.toml | head -1 | sed 's/.*"\(.*\)".*/\1/' || echo "")
        if [ -n "$rust_sdk" ]; then
            sdk_versions["rust-sdk"]=$rust_sdk
            log "  Rust SDK (Cargo.toml):       $rust_sdk"
        fi
    fi

    # Go SDK
    if [ -f "sdks/go/version.go" ]; then
        local go_sdk=$(grep 'Version\s*=' sdks/go/version.go | grep -oE '[0-9]+\.[0-9]+\.[0-9]+' || echo "")
        if [ -n "$go_sdk" ]; then
            sdk_versions["go-sdk"]=$go_sdk
            log "  Go SDK (version.go):         $go_sdk"
        fi
    fi

    # TypeScript SDK
    if [ -f "sdks/typescript/package.json" ]; then
        local ts_sdk=$(grep '"version"' sdks/typescript/package.json | head -1 | grep -oE '[0-9]+\.[0-9]+\.[0-9]+' || echo "")
        if [ -n "$ts_sdk" ]; then
            sdk_versions["ts-sdk"]=$ts_sdk
            log "  TypeScript SDK (package.json): $ts_sdk"
        fi
    fi

    # Python SDK
    if [ -f "sdks/python-client/pyproject.toml" ]; then
        local py_sdk=$(grep -E '^version\s*=' sdks/python-client/pyproject.toml | head -1 | sed 's/.*"\(.*\)".*/\1/' || echo "")
        if [ -n "$py_sdk" ]; then
            sdk_versions["python-sdk"]=$py_sdk
            log "  Python SDK (pyproject.toml): $py_sdk"
        fi
    fi

    # Check SDK versions match each other
    local unique_sdk=($(printf '%s\n' "${sdk_versions[@]}" | sort -u))
    if [ ${#unique_sdk[@]} -gt 1 ]; then
        log_error "SDK version mismatch detected!"
        log_error "  Found versions: ${unique_sdk[*]}"
        log_error "  Run 'make set-version VERSION=X.Y.Z' to fix"
        for sdk in "${!sdk_versions[@]}"; do
            add_json_result "$sdk" "sdk-version" "${unique_sdk[0]}" "${sdk_versions[$sdk]}" "error" ""
        done
    elif [ ${#unique_sdk[@]} -eq 1 ]; then
        log_success "All SDK versions consistent: ${unique_sdk[0]}"
        add_json_result "all-sdks" "sdk-version" "${unique_sdk[0]}" "${unique_sdk[0]}" "ok" ""
    fi

    # Check SDK versions match app version (Core as reference)
    if [ ${#unique_sdk[@]} -eq 1 ] && [ -f "apps/core/Cargo.toml" ]; then
        local core_version=$(grep -E '^version\s*=' apps/core/Cargo.toml | head -1 | sed 's/.*"\(.*\)".*/\1/' || echo "")
        if [ -n "$core_version" ] && [ "${unique_sdk[0]}" != "$core_version" ]; then
            log_error "SDK version (${unique_sdk[0]}) does not match Core version ($core_version)"
            add_json_result "sdks-vs-core" "version-sync" "$core_version" "${unique_sdk[0]}" "error" ""
        fi
    fi
    log ""
}

get_alpine_versions() {
    log "=== Alpine Base Image Versions ==="
    
    declare -A alpine_versions
    
    for dockerfile in apps/*/Dockerfile; do
        if [ -f "$dockerfile" ]; then
            local service=$(dirname "$dockerfile" | xargs basename)
            local alpine=$(extract_dockerfile_arg "$dockerfile" "ALPINE_VERSION")
            
            if [ -n "$alpine" ]; then
                alpine_versions[$service]=$alpine
                log "  $service: $alpine"
            fi
        fi
    done
    
    # Check consistency
    local unique_alpine=($(printf '%s\n' "${alpine_versions[@]}" | sort -u))
    
    if [ ${#unique_alpine[@]} -gt 1 ]; then
        log_warning "Different Alpine versions across services: ${unique_alpine[*]}"
        add_json_result "all" "alpine" "${unique_alpine[0]}" "${unique_alpine[*]}" "warning" "Dockerfiles"
    elif [ ${#unique_alpine[@]} -eq 1 ]; then
        log_success "All services using Alpine ${unique_alpine[0]}"
        add_json_result "all" "alpine" "${unique_alpine[0]}" "${unique_alpine[0]}" "ok" ""
    fi
    log ""
}

# =============================================================================
# Summary
# =============================================================================

print_summary() {
    log "=============================================="
    log "              VERSION CHECK SUMMARY"
    log "=============================================="
    
    if [ $ERRORS -eq 0 ] && [ $WARNINGS -eq 0 ]; then
        log "${GREEN}✓ All version checks passed!${NC}"
    else
        if [ $ERRORS -gt 0 ]; then
            log "${RED}✗ $ERRORS error(s) found${NC}"
        fi
        if [ $WARNINGS -gt 0 ]; then
            log "${YELLOW}⚠ $WARNINGS warning(s) found${NC}"
        fi
    fi
    log ""
}

print_json_output() {
    echo "{"
    echo "  \"summary\": {"
    echo "    \"errors\": $ERRORS,"
    echo "    \"warnings\": $WARNINGS,"
    echo "    \"status\": \"$([ $ERRORS -eq 0 ] && echo 'pass' || echo 'fail')\""
    echo "  },"
    echo "  \"results\": ["
    local first=true
    for result in "${JSON_RESULTS[@]}"; do
        if [ "$first" = true ]; then
            first=false
        else
            echo ","
        fi
        echo "    $result"
    done
    echo "  ]"
    echo "}"
}

# =============================================================================
# Main
# =============================================================================

main() {
    if [ "$JSON_OUTPUT" = false ]; then
        log ""
        log "╔═══════════════════════════════════════════════════════════════╗"
        log "║       AllSource Monorepo Version Consistency Checker            ║"
        log "╚═══════════════════════════════════════════════════════════════╝"
        log ""
    fi
    
    get_app_versions
    get_sdk_versions
    get_rust_versions
    get_go_versions
    get_node_versions
    get_elixir_versions
    get_alpine_versions
    
    if [ "$JSON_OUTPUT" = true ]; then
        print_json_output
    else
        print_summary
    fi
    
    # Exit with error if there are issues
    if [ $ERRORS -gt 0 ]; then
        exit 1
    fi
    
    exit 0
}

main
