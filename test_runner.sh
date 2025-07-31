#!/bin/bash
# LSP Bridge Test Runner with LSP Server Detection
# This script runs all tests and conditionally runs LSP integration tests
# if the required language servers are installed.

set -e

echo "🧪 LSP Bridge Test Runner"
echo "========================"
echo ""

# Colors for output
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

# Function to check if a command exists
command_exists() {
    command -v "$1" >/dev/null 2>&1
}

# Check for LSP servers
echo "Checking for LSP servers..."
RUST_ANALYZER_AVAILABLE=false
TYPESCRIPT_LSP_AVAILABLE=false

if command_exists rust-analyzer; then
    echo -e "${GREEN}✓${NC} rust-analyzer found"
    RUST_ANALYZER_AVAILABLE=true
else
    echo -e "${YELLOW}⚠${NC} rust-analyzer not found - some tests will be skipped"
fi

if command_exists typescript-language-server; then
    echo -e "${GREEN}✓${NC} typescript-language-server found"
    TYPESCRIPT_LSP_AVAILABLE=true
else
    echo -e "${YELLOW}⚠${NC} typescript-language-server not found - some tests will be skipped"
fi

echo ""

# Run standard tests
echo "Running standard tests..."
echo "========================"
cargo test --lib || { echo -e "${RED}Unit tests failed${NC}"; exit 1; }
echo -e "${GREEN}✓${NC} Unit tests passed"
echo ""

# Run integration tests
echo "Running integration tests..."
echo "==========================="
cargo test --test integration || { echo -e "${RED}Integration tests failed${NC}"; exit 1; }
echo -e "${GREEN}✓${NC} Integration tests passed"
echo ""

# Run multi-repo tests
echo "Running multi-repo tests..."
echo "=========================="
cargo test --test multi_repo_test || { echo -e "${RED}Multi-repo tests failed${NC}"; exit 1; }
echo -e "${GREEN}✓${NC} Multi-repo tests passed"
echo ""

# Run LSP integration tests if servers are available
if [ "$RUST_ANALYZER_AVAILABLE" = true ] || [ "$TYPESCRIPT_LSP_AVAILABLE" = true ]; then
    echo "Running LSP integration tests..."
    echo "==============================="
    
    if [ "$RUST_ANALYZER_AVAILABLE" = true ]; then
        echo "Running rust-analyzer tests..."
        cargo test --test integration test_rust_analyzer -- --ignored || {
            echo -e "${YELLOW}⚠${NC} rust-analyzer tests failed (non-critical)"
        }
    fi
    
    if [ "$TYPESCRIPT_LSP_AVAILABLE" = true ]; then
        echo "Running typescript-language-server tests..."
        cargo test --test integration test_typescript_lsp -- --ignored || {
            echo -e "${YELLOW}⚠${NC} typescript-language-server tests failed (non-critical)"
        }
    fi
else
    echo -e "${YELLOW}⚠${NC} Skipping LSP integration tests - no LSP servers found"
fi

echo ""
echo "Test Summary"
echo "============"
echo -e "${GREEN}✓${NC} All required tests passed"

if [ "$RUST_ANALYZER_AVAILABLE" = false ] || [ "$TYPESCRIPT_LSP_AVAILABLE" = false ]; then
    echo ""
    echo "To run all tests, install missing LSP servers:"
    if [ "$RUST_ANALYZER_AVAILABLE" = false ]; then
        echo "  - rust-analyzer: cargo install rust-analyzer"
    fi
    if [ "$TYPESCRIPT_LSP_AVAILABLE" = false ]; then
        echo "  - typescript-language-server: npm install -g typescript-language-server typescript"
    fi
fi