#!/bin/bash

# AllSource Core - Advanced Security Demo Runner
# This script runs the comprehensive security features demonstration

set -e

echo "╔═══════════════════════════════════════════════════════════════╗"
echo "║      AllSource Core - Security Demo Launcher                 ║"
echo "╚═══════════════════════════════════════════════════════════════╝"
echo ""

# Colors
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${BLUE}🚀 Starting AllSource Core Security Demo...${NC}"
echo ""

# Check if cargo is available
if ! command -v cargo &> /dev/null; then
    echo -e "${YELLOW}⚠️  Cargo not found. Please install Rust: https://rustup.rs/${NC}"
    exit 1
fi

echo -e "${GREEN}✓${NC} Cargo found"
echo ""

# Optional: Install cargo-audit for security scanning demo
if ! command -v cargo-audit &> /dev/null; then
    echo -e "${YELLOW}ℹ️  cargo-audit not found (optional for Demo 5)${NC}"
    echo "   Install with: cargo install cargo-audit"
    echo ""
fi

# Build the example
echo -e "${BLUE}📦 Building demo application...${NC}"
cargo build --example advanced_security_demo

echo ""
echo -e "${GREEN}✓${NC} Build complete"
echo ""

# Run the demo
echo -e "${BLUE}🎬 Launching interactive demo...${NC}"
echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""

cargo run --example advanced_security_demo

echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""
echo -e "${GREEN}✓ Demo completed successfully!${NC}"
echo ""
echo "📚 Next steps:"
echo "   1. Review the demo source: examples/advanced_security_demo.rs"
echo "   2. Read comprehensive docs: SECURITY.md"
echo "   3. Run security tests: cargo test --lib security::"
echo "   4. Check examples README: examples/README.md"
echo ""
echo "🔒 AllSource Core - Enterprise-Grade Event Store with Advanced Security"
