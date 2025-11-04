#!/bin/bash

# Chronos x402 Hackathon - Pre-Setup Script
# Run this script before the hackathon to verify everything is ready

set -e  # Exit on any error

echo "🚀 Chronos x402 Pre-Hackathon Setup"
echo "===================================="
echo ""

# Colors for output
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Track overall status
ERRORS=0

# Function to check if command exists
command_exists() {
    command -v "$1" >/dev/null 2>&1
}

# Function to print status
print_status() {
    if [ $1 -eq 0 ]; then
        echo -e "${GREEN}✅ $2${NC}"
    else
        echo -e "${RED}❌ $2${NC}"
        ((ERRORS++))
    fi
}

echo "📋 Checking prerequisites..."
echo ""

# Check Bun
if command_exists bun; then
    print_status 0 "Bun is installed ($(bun --version))"
else
    print_status 1 "Bun is not installed. Install from https://bun.sh"
fi

# Check Solana CLI
if command_exists solana; then
    print_status 0 "Solana CLI is installed ($(solana --version | head -n1))"
else
    print_status 1 "Solana CLI not installed. Install from https://docs.solana.com/cli/install-solana-cli-tools"
fi

# Check Fly CLI
if command_exists fly; then
    print_status 0 "Fly CLI is installed ($(fly version | head -n1))"
else
    echo -e "${YELLOW}⚠️  Fly CLI not installed (optional for deployment)${NC}"
fi

# Check Vercel CLI
if command_exists vercel; then
    print_status 0 "Vercel CLI is installed"
else
    echo -e "${YELLOW}⚠️  Vercel CLI not installed. Run: npm i -g vercel${NC}"
fi

echo ""
echo "🔍 Checking environment variables..."
echo ""

# Check for required env vars
if [ -f ".env.local" ]; then
    print_status 0 ".env.local file exists"

    # Check for specific vars
    if grep -q "CHRONOS_URL" .env.local; then
        print_status 0 "CHRONOS_URL is set"
    else
        print_status 1 "CHRONOS_URL not found in .env.local"
    fi

    if grep -q "SOLANA_WALLET" .env.local; then
        print_status 0 "SOLANA_WALLET is set"
    else
        print_status 1 "SOLANA_WALLET not found in .env.local"
    fi

    if grep -q "OPENAI_API_KEY" .env.local; then
        print_status 0 "OPENAI_API_KEY is set"
    else
        print_status 1 "OPENAI_API_KEY not found in .env.local"
    fi
else
    print_status 1 ".env.local file not found. Create it from .env.example"
fi

echo ""
echo "🔌 Checking connections..."
echo ""

# Check Chronos deployment
if [ ! -z "$CHRONOS_URL" ]; then
    CHRONOS_URL_VAR=$CHRONOS_URL
elif [ -f ".env.local" ]; then
    CHRONOS_URL_VAR=$(grep CHRONOS_URL .env.local | cut -d '=' -f2)
fi

if [ ! -z "$CHRONOS_URL_VAR" ]; then
    echo "Testing Chronos at $CHRONOS_URL_VAR..."
    if curl -s -f "$CHRONOS_URL_VAR/health" > /dev/null; then
        print_status 0 "Chronos health endpoint responding"
    else
        print_status 1 "Chronos health endpoint not responding"
    fi
else
    echo -e "${YELLOW}⚠️  CHRONOS_URL not set, skipping connection test${NC}"
fi

# Check Solana devnet
echo "Testing Solana devnet connection..."
if solana cluster-version --url devnet > /dev/null 2>&1; then
    print_status 0 "Solana devnet is accessible"
else
    print_status 1 "Cannot connect to Solana devnet"
fi

# Check Solana wallet balance
if command_exists solana; then
    echo "Checking Solana wallet balance..."
    BALANCE=$(solana balance --url devnet 2>/dev/null || echo "0")
    if [ "$BALANCE" != "0" ] && [ "$BALANCE" != "0 SOL" ]; then
        print_status 0 "Wallet has balance: $BALANCE"
    else
        print_status 1 "Wallet has no balance. Run: solana airdrop 2 --url devnet"
    fi
fi

echo ""
echo "📦 Checking project structure..."
echo ""

# Check for key directories
[ -d "packages/x402-solana-sdk" ] && print_status 0 "SDK package directory exists" || print_status 1 "SDK package directory missing"
[ -d "apps/x402-demo" ] && print_status 0 "Demo app directory exists" || print_status 1 "Demo app directory missing"
[ -d "apps/x402-dashboard" ] && print_status 0 "Dashboard app directory exists" || print_status 1 "Dashboard app directory missing"
[ -d "docs/x402" ] && print_status 0 "Documentation directory exists" || print_status 1 "Documentation directory missing"

echo ""
echo "🔨 Checking build system..."
echo ""

# Try building (if packages exist)
if [ -d "packages/x402-solana-sdk" ]; then
    echo "Testing SDK build..."
    cd packages/x402-solana-sdk
    if [ -f "package.json" ]; then
        if bun install > /dev/null 2>&1 && bun run build > /dev/null 2>&1; then
            print_status 0 "SDK builds successfully"
        else
            print_status 1 "SDK build failed"
        fi
    else
        echo -e "${YELLOW}⚠️  SDK not initialized yet${NC}"
    fi
    cd ../..
fi

echo ""
echo "================================"
echo "📊 Setup Summary"
echo "================================"
echo ""

if [ $ERRORS -eq 0 ]; then
    echo -e "${GREEN}🎉 All checks passed! You're ready for the hackathon!${NC}"
    echo ""
    echo "Next steps:"
    echo "1. Review the checklist: docs/x402/HACKATHON_CHECKLIST.md"
    echo "2. Print the quick reference: docs/x402/QUICK_REFERENCE.md"
    echo "3. Start building! 🚀"
else
    echo -e "${RED}⚠️  Found $ERRORS issue(s) that need attention${NC}"
    echo ""
    echo "Please fix the issues above before starting the hackathon."
    echo "See HACKATHON_CHECKLIST.md for detailed setup instructions."
    exit 1
fi

echo ""
echo "🎯 Quick Start Commands:"
echo "  cd packages/x402-solana-sdk && bun run dev"
echo "  cd apps/x402-demo && bun run dev"
echo "  cd apps/x402-dashboard && bun run dev"
echo ""

exit 0
