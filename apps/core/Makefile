.PHONY: help lint lint-sort format format-sort test build clean check ci quality-gates quality-gates-docker install-ci-toolchain

# CI Rust toolchain version (must match .github/workflows/ci.yml)
RUST_CI_VERSION := nightly

# Docker image for replicating CI environment
CI_DOCKER_IMAGE := rust:1.92

# Default target
help:
	@echo "AllSource Core - Development Commands"
	@echo ""
	@echo "Quality Gates (run before pushing):"
	@echo "  make quality-gates        - Run exact CI checks with Rust nightly"
	@echo "  make quality-gates-docker - Run tests in Docker (replicates CI x86_64 Linux)"
	@echo ""
	@echo "Quick Checks (uses current toolchain):"
	@echo "  make check         - Run format, lint, sort, and test"
	@echo "  make lint          - Check formatting and run clippy"
	@echo "  make lint-sort     - Check if Cargo.toml is sorted"
	@echo "  make test          - Run tests"
	@echo ""
	@echo "Formatting:"
	@echo "  make format        - Auto-format code with rustfmt"
	@echo "  make format-sort   - Auto-sort Cargo.toml dependencies"
	@echo ""
	@echo "Building:"
	@echo "  make build         - Build the project"
	@echo "  make clean         - Clean build artifacts"
	@echo ""
	@echo "Setup:"
	@echo "  make install-ci-toolchain - Install Rust nightly for quality-gates"

# Quality gate: Format check
lint:
	@echo "Checking code formatting..."
	@cargo fmt --check
	@echo "Running clippy..."
	@cargo clippy --all-targets --all-features -- -D warnings

# Quality gate: Cargo.toml sorting check
lint-sort:
	@echo "Checking Cargo.toml sort order..."
	@cargo sort --check

# Auto-format code
format:
	@echo "Formatting code..."
	@cargo fmt

# Auto-sort Cargo.toml
format-sort:
	@echo "Sorting Cargo.toml..."
	@cargo sort -w

# Run tests
test:
	@echo "Running tests..."
	@cargo test --lib

# Build project
build:
	@echo "Building project..."
	@cargo build --lib

# Clean build artifacts
clean:
	@echo "Cleaning build artifacts..."
	@cargo clean

# Combined quality check (uses current toolchain)
check: lint lint-sort test
	@echo "All checks passed!"

# Full CI pipeline (uses current toolchain)
ci: check build
	@echo "CI pipeline completed!"

# Exact CI quality gates using Rust nightly (matches GitHub Actions)
# Requires: make install-ci-toolchain
quality-gates:
	@echo "Running CI quality gates with Rust $(RUST_CI_VERSION)..."
	@echo ""
	@echo "[1/6] Checking code formatting..."
	@cargo +$(RUST_CI_VERSION) fmt --check
	@echo ""
	@echo "[2/6] Checking Cargo.toml sorting..."
	@cargo +$(RUST_CI_VERSION) sort --check
	@echo ""
	@echo "[3/6] Running clippy..."
	@cargo +$(RUST_CI_VERSION) clippy --locked --all-targets --all-features -- -D warnings
	@echo ""
	@echo "[4/6] Running tests..."
	@cargo +$(RUST_CI_VERSION) test --locked --lib --all-features
	@echo ""
	@echo "[5/6] Building release..."
	@cargo +$(RUST_CI_VERSION) build --locked --lib --release
	@echo ""
	@echo "[6/6] Checking documentation..."
	@RUSTDOCFLAGS="-D warnings" cargo +$(RUST_CI_VERSION) doc --no-deps --document-private-items
	@echo ""
	@echo "All quality gates passed!"

# Install CI toolchain
install-ci-toolchain:
	@echo "Installing Rust $(RUST_CI_VERSION) toolchain..."
	@rustup toolchain install $(RUST_CI_VERSION) --component rustfmt clippy
	@echo "Rust $(RUST_CI_VERSION) installed!"

# Run quality gates in Docker container (replicates GitHub Actions x86_64 Linux environment)
# This catches SIMD and platform-specific bugs that won't show on macOS ARM
quality-gates-docker:
	@echo "Running quality gates in Docker (x86_64 Linux)..."
	@echo "This replicates the GitHub Actions CI environment"
	@echo ""
	docker run --rm --platform linux/amd64 \
		-v "$$(pwd):/app" \
		-w /app \
		$(CI_DOCKER_IMAGE) \
		sh -c '\
			echo "[1/5] Installing cargo-sort..." && \
			cargo install cargo-sort --quiet && \
			echo "[2/5] Checking formatting..." && \
			cargo fmt --check && \
			echo "[3/5] Checking Cargo.toml sorting..." && \
			cargo sort --check && \
			echo "[4/5] Running clippy..." && \
			cargo clippy --all-targets --all-features -- -D warnings && \
			echo "[5/5] Running tests..." && \
			cargo test --all-features && \
			echo "" && \
			echo "All quality gates passed in Docker!"'
