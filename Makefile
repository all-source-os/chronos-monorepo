.PHONY: help install dev build clean demo test lint check-versions \
        core control web mcp \
        docker-build docker-test docker-test-quick docker-clean \
        docker-core docker-web docker-query docker-mcp docker-control

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
	@echo "  make docker-clean      - Remove test images"
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
	@echo "🧹 Cleaning test containers..."
	-docker rmi chronos-core:test 2>/dev/null
	-docker rmi chronos-web:test 2>/dev/null
	-docker rmi chronos-query-service:test 2>/dev/null
	-docker rmi chronos-mcp-server:test 2>/dev/null
	-docker rmi chronos-control-plane:test 2>/dev/null
	@echo "✅ Test images cleaned"

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
