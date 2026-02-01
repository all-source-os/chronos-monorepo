.PHONY: help install dev build clean demo test lint check-versions \
        core control web mcp \
        docker-build docker-test docker-test-quick docker-clean docker-purge \
        docker-core docker-web docker-query docker-mcp docker-control \
        quality-gates quality-rust quality-go quality-elixir \
        validate-workflows validate-workflows-quick

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
	@echo "  make quality-gates  - Run ALL quality checks (Rust + Go + Elixir)"
	@echo "  make quality-rust   - Run Rust quality gates only"
	@echo "  make quality-go     - Run Go quality gates only"
	@echo "  make quality-elixir - Run Elixir quality gates only"
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
	@echo "→ Running go vet..."
	cd apps/control-plane && go vet ./...
	@echo "→ Running staticcheck..."
	cd apps/control-plane && staticcheck ./...
	@echo "→ Running golangci-lint..."
	cd apps/control-plane && golangci-lint run --timeout=5m
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
	cd apps/query-service && mix test
	@echo "✅ Elixir quality gates passed!"

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
