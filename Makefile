.PHONY: help install dev build clean demo test lint check-versions \
        core control web mcp \
        docker-build docker-test docker-test-quick docker-clean docker-purge \
        docker-core docker-web docker-query docker-mcp docker-control \
        ci quality-gates quality-rust quality-go quality-elixir quality-elixir-full \
        validate-workflows validate-workflows-quick \
        elixir-test elixir-test-failed elixir-test-watch elixir-test-report

# Test output directory for failure reports
TEST_OUTPUT_DIR := .test-reports

# =============================================================================
# General Commands
# =============================================================================

help:
	@echo "Chronos - Available Commands"
	@echo "=============================="
	@echo ""
	@echo "Development:"
	@echo "  make install        - Install all dependencies"
	@echo "  make dev            - Run all services in development mode"
	@echo "  make build          - Build all services"
	@echo "  make clean          - Clean all build artifacts"
	@echo "  make demo           - Quick demo setup (install + dev)"
	@echo "  make test           - Run tests"
	@echo "  make lint           - Run linters across all services"
	@echo "  make check-versions - Check version consistency across services"
	@echo ""
	@echo "Quality Gates (CI pipeline locally):"
	@echo "  make ci             - Full CI pipeline (matches GitHub Actions exactly)"
	@echo "  make quality-gates  - Run ALL quality checks (Rust + Go + Elixir)"
	@echo "  make quality-rust   - Run Rust quality gates only"
	@echo "  make quality-go     - Run Go quality gates only"
	@echo "  make quality-elixir - Run Elixir quality gates only (quick)"
	@echo "  make quality-elixir-full - Full Elixir checks with Dialyzer"
	@echo ""
	@echo "Elixir Testing:"
	@echo "  make elixir-test         - Run all Elixir tests with detailed output"
	@echo "  make elixir-test-failed  - Re-run only failed tests"
	@echo "  make elixir-test-watch   - Run tests in watch mode"
	@echo "  make elixir-test-report  - Generate a markdown test report"
	@echo ""
	@echo "Workflow Validation:"
	@echo "  make validate-workflows       - Validate GitHub Actions with act"
	@echo "  make validate-workflows-quick - Quick syntax check (actionlint only)"
	@echo ""
	@echo "Individual Services:"
	@echo "  make core         - Run Rust event store only"
	@echo "  make control      - Run Go control plane only"
	@echo "  make web          - Run Next.js web UI only"
	@echo "  make mcp          - Run MCP server only"
	@echo ""
	@echo "Container Testing:"
	@echo "  make docker-test       - Full container test suite (all services)"
	@echo "  make docker-test-quick - Quick build-only test"
	@echo "  make docker-build      - Build all containers"
	@echo "  make docker-clean      - Remove test containers and images"
	@echo "  make docker-purge      - Remove ALL docker resources (nuclear option)"
	@echo ""
	@echo "Individual Container Builds:"
	@echo "  make docker-core       - Build core container"
	@echo "  make docker-web        - Build web container"
	@echo "  make docker-query      - Build query-service container"
	@echo "  make docker-mcp        - Build mcp-server container"
	@echo "  make docker-control    - Build control-plane container"

# =============================================================================
# Development Commands
# =============================================================================

install:
	@echo "📦 Installing dependencies..."
	bun install
	cd apps/control-plane && go mod download

dev:
	@echo "🚀 Starting all services..."
	@echo "Press Ctrl+C to stop all services"
	bun dev

build:
	@echo "🔨 Building all services..."
	bun build

clean:
	@echo "🧹 Cleaning build artifacts..."
	bun clean
	-cd apps/core && cargo clean
	-cd apps/control-plane && rm -rf bin
	-rm -rf .container-test-logs

demo: install
	@echo "🎪 Starting Chronos demo..."
	@echo "Dashboard will be available at http://localhost:3000"
	@echo ""
	bun dev

test:
	@echo "🧪 Running tests..."
	bun test

lint:
	@echo "🔍 Running linters..."
	bun run lint
	cd apps/core && cargo fmt --check && cargo clippy --all-targets -- -D warnings
	cd apps/control-plane && go fmt ./... && go vet ./...

check-versions:
	@echo "🔢 Checking version consistency..."
	./scripts/check-versions.sh

# =============================================================================
# Quality Gates (mirrors CI pipeline)
# =============================================================================

quality-gates: check-versions quality-rust quality-go quality-elixir
	@echo ""
	@echo "✅ All quality gates passed!"

# Full CI pipeline - replicates exact GitHub Actions checks
ci: check-versions quality-rust quality-go quality-elixir-full
	@echo ""
	@echo "=============================================="
	@echo "✅ Full CI pipeline passed!"
	@echo "   Safe to push - this matches GitHub Actions"
	@echo "=============================================="

quality-rust:
	@echo ""
	@echo "🦀 Running Rust quality gates..."
	@echo "================================"
	@echo "→ Checking formatting..."
	cd apps/core && cargo +nightly fmt --check
	@echo "→ Checking Cargo.toml sorting..."
	cd apps/core && cargo +nightly sort --check
	@echo "→ Running Clippy..."
	cd apps/core && cargo +nightly clippy --locked --all-targets --all-features -- -D warnings
	@echo "→ Running tests..."
	cd apps/core && cargo +nightly test --locked --lib --all-features
	@echo "→ Building release..."
	cd apps/core && cargo +nightly build --locked --lib --release
	@echo "→ Checking documentation..."
	cd apps/core && RUSTDOCFLAGS="-D warnings" cargo +nightly doc --no-deps --document-private-items
	@echo "✅ Rust quality gates passed!"

quality-go:
	@echo ""
	@echo "🐹 Running Go quality gates..."
	@echo "=============================="
	@echo "→ Downloading dependencies..."
	cd apps/control-plane && go mod download
	@echo "→ Verifying dependencies..."
	cd apps/control-plane && go mod verify
	@echo "→ Checking formatting..."
	@cd apps/control-plane && if [ -n "$$(gofmt -l .)" ]; then \
		echo "Go code is not formatted:"; \
		gofmt -d .; \
		exit 1; \
	fi
	@echo "→ Running golangci-lint (includes staticcheck, gosec, etc.)..."
	@if command -v golangci-lint >/dev/null 2>&1; then \
		cd apps/control-plane && golangci-lint run --timeout=5m; \
	else \
		echo "⚠ golangci-lint not installed, falling back to go vet"; \
		echo "  Install with: brew install golangci-lint"; \
		cd apps/control-plane && go vet ./...; \
	fi
	@echo "→ Running tests..."
	cd apps/control-plane && go test -v -race -covermode=atomic ./...
	@echo "→ Building binary..."
	cd apps/control-plane && CGO_ENABLED=0 go build -ldflags="-s -w" -o control-plane .
	@echo "✅ Go quality gates passed!"

quality-elixir:
	@echo ""
	@echo "💧 Running Elixir quality gates..."
	@echo "=================================="
	@echo "→ Installing dependencies..."
	cd apps/query-service && mix deps.get
	@echo "→ Checking formatting..."
	cd apps/query-service && mix format --check-formatted
	@echo "→ Compiling with warnings as errors..."
	cd apps/query-service && mix compile --warnings-as-errors
	@echo "→ Running Credo..."
	-cd apps/query-service && mix credo --strict
	@echo "→ Running tests (testcontainers will start PostgreSQL automatically)..."
	cd apps/query-service && TESTCONTAINERS_RYUK_DISABLED=true mix test
	@echo "✅ Elixir quality gates passed!"

# Full Elixir CI checks (includes Dialyzer and MCP server) - matches GitHub Actions exactly
quality-elixir-full:
	@echo ""
	@echo "💧 Running full Elixir CI pipeline..."
	@echo "====================================="
	@mkdir -p $(TEST_OUTPUT_DIR)
	@echo ""
	@echo "--- Query Service ---"
	@echo "→ Installing dependencies..."
	cd apps/query-service && mix deps.get
	@echo "→ Checking formatting..."
	cd apps/query-service && mix format --check-formatted
	@echo "→ Checking unused dependencies..."
	cd apps/query-service && mix deps.unlock --check-unused
	@echo "→ Compiling with warnings as errors..."
	cd apps/query-service && mix compile --warnings-as-errors
	@echo "→ Running Credo..."
	cd apps/query-service && mix credo --strict
	@echo "→ Running Dialyzer..."
	cd apps/query-service && mkdir -p priv/plts && mix dialyzer
	@echo "→ Running tests..."
	@cd apps/query-service && TESTCONTAINERS_RYUK_DISABLED=true mix test 2>&1 | tee $(CURDIR)/$(TEST_OUTPUT_DIR)/query-service-ci.log; \
		test_result=$${PIPESTATUS[0]}; \
		if [ $$test_result -ne 0 ]; then \
			echo ""; \
			echo "❌ QUERY SERVICE TEST FAILURES:"; \
			echo "================================"; \
			grep -E "^\s+[0-9]+\)" $(CURDIR)/$(TEST_OUTPUT_DIR)/query-service-ci.log -A 20 2>/dev/null | head -100 || true; \
			echo ""; \
			echo "📋 Full log: $(CURDIR)/$(TEST_OUTPUT_DIR)/query-service-ci.log"; \
			echo "💡 Re-run failed tests: make elixir-test-failed"; \
			exit 1; \
		fi
	@echo "done (passed successfully)"
	@echo ""
	@echo "--- MCP Server ---"
	@echo "→ Installing dependencies..."
	cd apps/mcp-server-elixir && mix deps.get
	@echo "→ Checking formatting..."
	cd apps/mcp-server-elixir && mix format --check-formatted
	@echo "→ Checking unused dependencies..."
	cd apps/mcp-server-elixir && mix deps.unlock --check-unused
	@echo "→ Compiling with warnings as errors..."
	cd apps/mcp-server-elixir && mix compile --warnings-as-errors
	@echo "→ Running Credo..."
	cd apps/mcp-server-elixir && mix credo --strict
	@echo "→ Running Dialyzer..."
	cd apps/mcp-server-elixir && mkdir -p priv/plts && mix dialyzer
	@echo "→ Running tests..."
	@cd apps/mcp-server-elixir && mix test 2>&1 | tee $(CURDIR)/$(TEST_OUTPUT_DIR)/mcp-server-ci.log; \
		test_result=$${PIPESTATUS[0]}; \
		if [ $$test_result -ne 0 ]; then \
			echo ""; \
			echo "❌ MCP SERVER TEST FAILURES:"; \
			echo "============================"; \
			grep -E "^\s+[0-9]+\)" $(CURDIR)/$(TEST_OUTPUT_DIR)/mcp-server-ci.log -A 20 2>/dev/null | head -100 || true; \
			echo ""; \
			echo "📋 Full log: $(CURDIR)/$(TEST_OUTPUT_DIR)/mcp-server-ci.log"; \
			echo "💡 Re-run failed tests: make elixir-test-failed"; \
			exit 1; \
		fi
	@echo "✅ Full Elixir CI pipeline passed!"

# =============================================================================
# Individual Service Commands
# =============================================================================

core:
	@echo "⚡ Starting Rust event store on :3900"
	cd apps/core && cargo run --release

control:
	@echo "🎯 Starting Go control plane on :8080"
	cd apps/control-plane && go run main.go

web:
	@echo "🌐 Starting Next.js web UI on :3000"
	cd apps/web && bun dev

mcp:
	@echo "🤖 Starting MCP server"
	cd apps/mcp-server-elixir && mix phx.server

# =============================================================================
# Container Testing Commands
# =============================================================================

docker-test:
	@echo "🐳 Running full container test suite..."
	./scripts/container-test.sh

docker-test-quick:
	@echo "🐳 Running quick container build test..."
	./scripts/container-test.sh --quick

docker-build:
	@echo "🐳 Building all containers..."
	./scripts/container-test.sh --build-only

docker-clean:
	@echo "🧹 Cleaning test containers and images..."
	-docker rm -f query-service-test-db 2>/dev/null
	-docker rmi chronos-core:test 2>/dev/null
	-docker rmi chronos-web:test 2>/dev/null
	-docker rmi chronos-query-service:test 2>/dev/null
	-docker rmi chronos-mcp-server:test 2>/dev/null
	-docker rmi chronos-control-plane:test 2>/dev/null
	@echo "✅ Test images cleaned"

docker-purge:
	@echo "🧹 Purging ALL docker resources (containers, images, volumes, networks)..."
	-docker stop $$(docker ps -aq) 2>/dev/null
	-docker rm $$(docker ps -aq) 2>/dev/null
	-docker rmi $$(docker images -q) 2>/dev/null
	-docker volume prune -f 2>/dev/null
	-docker network prune -f 2>/dev/null
	-docker system prune -af 2>/dev/null
	@echo "✅ All docker resources purged"

# Individual container builds
docker-core:
	@echo "🐳 Building core container..."
	docker build -t chronos-core:test apps/core

docker-web:
	@echo "🐳 Building web container..."
	docker build -f apps/web/Dockerfile -t chronos-web:test .

docker-query:
	@echo "🐳 Building query-service container..."
	docker build -t chronos-query-service:test apps/query-service

docker-mcp:
	@echo "🐳 Building mcp-server container..."
	docker build -t chronos-mcp-server:test apps/mcp-server-elixir

docker-control:
	@echo "🐳 Building control-plane container..."
	docker build -t chronos-control-plane:test apps/control-plane

# =============================================================================
# Docker Compose Commands
# =============================================================================

up:
	@echo "🚀 Starting all services with Docker Compose..."
	docker compose up -d

down:
	@echo "🛑 Stopping all services..."
	docker compose down

logs:
	docker compose logs -f

ps:
	docker compose ps

# =============================================================================
# Workflow Validation Commands
# =============================================================================

validate-workflows:
	@echo "🔍 Validating GitHub Actions workflows with act..."
	./scripts/validate-workflows.sh

validate-workflows-quick:
	@echo "🔍 Quick workflow syntax validation (actionlint only)..."
	./scripts/validate-workflows.sh --quick

# =============================================================================
# Elixir Testing Commands
# =============================================================================

elixir-test:
	@echo ""
	@echo "🧪 Running Elixir tests with detailed output..."
	@echo "================================================"
	@mkdir -p $(TEST_OUTPUT_DIR)
	@echo "--- Query Service Tests ---"
	cd apps/query-service && TESTCONTAINERS_RYUK_DISABLED=true \
		mix test --trace 2>&1 | tee $(CURDIR)/$(TEST_OUTPUT_DIR)/query-service-test.log; \
		test_result=$$?; \
		if [ $$test_result -ne 0 ]; then \
			echo ""; \
			echo "❌ QUERY SERVICE TEST FAILURES:"; \
			echo "================================"; \
			grep -A 50 "^\s*[0-9]*)" $(CURDIR)/$(TEST_OUTPUT_DIR)/query-service-test.log 2>/dev/null || true; \
			echo ""; \
			echo "📋 Full log: $(CURDIR)/$(TEST_OUTPUT_DIR)/query-service-test.log"; \
		fi; \
		exit $$test_result
	@echo ""
	@echo "--- MCP Server Tests ---"
	cd apps/mcp-server-elixir && mix test --trace 2>&1 | tee $(CURDIR)/$(TEST_OUTPUT_DIR)/mcp-server-test.log; \
		test_result=$$?; \
		if [ $$test_result -ne 0 ]; then \
			echo ""; \
			echo "❌ MCP SERVER TEST FAILURES:"; \
			echo "============================"; \
			grep -A 50 "^\s*[0-9]*)" $(CURDIR)/$(TEST_OUTPUT_DIR)/mcp-server-test.log 2>/dev/null || true; \
			echo ""; \
			echo "📋 Full log: $(CURDIR)/$(TEST_OUTPUT_DIR)/mcp-server-test.log"; \
		fi; \
		exit $$test_result
	@echo ""
	@echo "✅ All Elixir tests passed!"
	@echo "📋 Test logs saved to $(TEST_OUTPUT_DIR)/"

elixir-test-failed:
	@echo ""
	@echo "🔄 Re-running failed Elixir tests..."
	@echo "====================================="
	@echo "--- Query Service Failed Tests ---"
	cd apps/query-service && TESTCONTAINERS_RYUK_DISABLED=true mix test --failed --trace || true
	@echo ""
	@echo "--- MCP Server Failed Tests ---"
	cd apps/mcp-server-elixir && mix test --failed --trace || true
	@echo ""
	@echo "💡 Tip: If no tests run, there are no recorded failures."
	@echo "   Run 'make elixir-test' first to record failures."

elixir-test-watch:
	@echo ""
	@echo "👀 Running Elixir tests in watch mode..."
	@echo "========================================="
	@echo "Press Ctrl+C to stop"
	cd apps/query-service && TESTCONTAINERS_RYUK_DISABLED=true mix test.watch

elixir-test-report:
	@./scripts/elixir-test-report.sh
