#!/bin/bash

# Verification script to check if all crates compile and tests pass

set -e

echo "🔍 Synapsed Verification Script"
echo "================================"
echo ""

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Check if we're in the right directory
if [ ! -f "Cargo.toml" ]; then
    echo -e "${RED}Error: Not in synapsed root directory${NC}"
    exit 1
fi

echo "📁 Checking directory structure..."
REQUIRED_DIRS=(
    "crates/observability/synapsed-substrates"
    "crates/observability/synapsed-serventis"
    "crates/core/synapsed-core"
    "crates/core/synapsed-crypto"
    "crates/storage/synapsed-storage"
    "crates/network/synapsed-net"
    "crates/security/synapsed-identity"
)

for dir in "${REQUIRED_DIRS[@]}"; do
    if [ -d "$dir" ]; then
        echo -e "  ✅ $dir"
    else
        echo -e "  ${RED}❌ Missing: $dir${NC}"
    fi
done

echo ""
echo "🔧 Checking Cargo.toml files..."
find crates -name "Cargo.toml" -type f | head -10

echo ""
echo "🏗️ Attempting to build all crates..."
echo "This may take a while on first run..."

# Try to build each crate individually first to identify specific issues
CRATES=(
    "synapsed-substrates"
    "synapsed-serventis"
    "synapsed-core"
    "synapsed-crypto"
    "synapsed-storage"
    "synapsed-net"
    "synapsed-identity"
)

FAILED_CRATES=()
PASSED_CRATES=()

for crate in "${CRATES[@]}"; do
    echo ""
    echo "Building $crate..."
    if cargo build -p "$crate" 2>/dev/null; then
        echo -e "${GREEN}✅ $crate compiled successfully${NC}"
        PASSED_CRATES+=("$crate")
    else
        echo -e "${YELLOW}⚠️ $crate has compilation issues${NC}"
        FAILED_CRATES+=("$crate")
    fi
done

echo ""
echo "📊 Build Summary:"
echo "=================="
echo -e "${GREEN}Passed: ${#PASSED_CRATES[@]} crates${NC}"
for crate in "${PASSED_CRATES[@]}"; do
    echo -e "  ✅ $crate"
done

if [ ${#FAILED_CRATES[@]} -gt 0 ]; then
    echo -e "${YELLOW}Failed: ${#FAILED_CRATES[@]} crates${NC}"
    for crate in "${FAILED_CRATES[@]}"; do
        echo -e "  ⚠️ $crate"
    done
    
    echo ""
    echo "🔍 Getting detailed error for first failed crate..."
    if [ ${#FAILED_CRATES[@]} -gt 0 ]; then
        echo "Errors for ${FAILED_CRATES[0]}:"
        cargo build -p "${FAILED_CRATES[0]}" 2>&1 | head -50 || true
    fi
fi

echo ""
echo "📋 Next Steps:"
if [ ${#FAILED_CRATES[@]} -eq 0 ]; then
    echo "  1. ✅ All crates compile!"
    echo "  2. Run tests: cargo test --all"
    echo "  3. Run benchmarks: cargo bench --all"
    echo "  4. Check documentation: cargo doc --all --open"
else
    echo "  1. Fix compilation errors in failed crates"
    echo "  2. Update dependency paths in Cargo.toml files"
    echo "  3. Ensure all internal dependencies use workspace versions"
    echo "  4. Re-run this script to verify fixes"
fi

echo ""
echo "💡 Tips:"
echo "  - Check individual crate errors: cargo build -p <crate-name>"
echo "  - Fix path dependencies: Update 'path = ' in Cargo.toml files"
echo "  - Use workspace dependencies: dependency = { workspace = true }"