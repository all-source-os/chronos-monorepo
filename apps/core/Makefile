.PHONY: help lint lint-sort format format-sort test build clean check ci

# Default target
help:
	@echo "AllSource Core - Quality Gates & Development Commands"
	@echo ""
	@echo "Quality Gates (CI/CD):"
	@echo "  make check         - Run all quality gates (format, lint, sort, test)"
	@echo "  make ci            - Full CI pipeline (check + build)"
	@echo ""
	@echo "Linting:"
	@echo "  make lint          - Check code formatting and run clippy"
	@echo "  make lint-sort     - Check if Cargo.toml is sorted"
	@echo ""
	@echo "Formatting:"
	@echo "  make format        - Auto-format code with rustfmt"
	@echo "  make format-sort   - Auto-sort Cargo.toml dependencies"
	@echo ""
	@echo "Testing & Building:"
	@echo "  make test          - Run all tests"
	@echo "  make build         - Build the project"
	@echo "  make clean         - Clean build artifacts"

# Quality gate: Format check
lint:
	@echo "→ Checking code formatting..."
	@cargo fmt --check
	@echo "→ Running clippy..."
	@cargo clippy --all-targets --all-features -- -D warnings

# Quality gate: Cargo.toml sorting check
lint-sort:
	@echo "→ Checking Cargo.toml sort order..."
	@cargo sort --check

# Auto-format code
format:
	@echo "→ Formatting code..."
	@cargo fmt

# Auto-sort Cargo.toml
format-sort:
	@echo "→ Sorting Cargo.toml..."
	@cargo sort -w

# Run tests
test:
	@echo "→ Running tests..."
	@cargo test --lib

# Build project
build:
	@echo "→ Building project..."
	@cargo build --lib

# Clean build artifacts
clean:
	@echo "→ Cleaning build artifacts..."
	@cargo clean

# Combined quality check (recommended before commit)
check: lint lint-sort test
	@echo "✅ All quality gates passed!"

# Full CI pipeline
ci: check build
	@echo "✅ CI pipeline completed successfully!"
