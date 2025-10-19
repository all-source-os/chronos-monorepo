.PHONY: help install dev build clean demo test

help:
	@echo "AllSource - Available Commands"
	@echo "=============================="
	@echo "make install    - Install all dependencies"
	@echo "make dev        - Run all services in development mode"
	@echo "make build      - Build all services"
	@echo "make clean      - Clean all build artifacts"
	@echo "make demo       - Quick demo setup (install + dev)"
	@echo "make test       - Run tests"
	@echo ""
	@echo "Individual Services:"
	@echo "make core       - Run Rust event store only"
	@echo "make control    - Run Go control plane only"
	@echo "make web        - Run Next.js web UI only"
	@echo "make mcp        - Run MCP server only"

install:
	@echo "📦 Installing dependencies..."
	bun install
	cd services/control-plane && go mod download

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
	cd services/core && cargo clean
	cd services/control-plane && rm -rf bin

demo: install
	@echo "🎪 Starting AllSource demo..."
	@echo "Dashboard will be available at http://localhost:3000"
	@echo ""
	bun dev

test:
	@echo "🧪 Running tests..."
	bun test

# Individual service commands
core:
	@echo "⚡ Starting Rust event store on :8080"
	cd services/core && cargo run --release

control:
	@echo "🎯 Starting Go control plane on :8081"
	cd services/control-plane && go run main.go

web:
	@echo "🌐 Starting Next.js web UI on :3000"
	cd apps/web && bun dev

mcp:
	@echo "🤖 Starting MCP server"
	cd packages/mcp-server && bun dev
