#!/bin/bash
# Test runner script for synapsed-identity

set -e

echo "=================================="
echo "Synapsed Identity Test Suite"
echo "=================================="
echo ""

# Colors for output
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Parse command line arguments
RUN_UNIT=true
RUN_INTEGRATION=true
RUN_BENCHMARK=false
RUN_SECURITY=false
VERBOSE=false

while [[ $# -gt 0 ]]; do
    case $1 in
        --unit-only)
            RUN_INTEGRATION=false
            shift
            ;;
        --integration-only)
            RUN_UNIT=false
            shift
            ;;
        --benchmark)
            RUN_BENCHMARK=true
            shift
            ;;
        --security)
            RUN_SECURITY=true
            shift
            ;;
        --verbose)
            VERBOSE=true
            shift
            ;;
        --all)
            RUN_BENCHMARK=true
            RUN_SECURITY=true
            shift
            ;;
        *)
            echo "Unknown option: $1"
            echo "Usage: $0 [--unit-only] [--integration-only] [--benchmark] [--security] [--verbose] [--all]"
            exit 1
            ;;
    esac
done

# Set environment variables
export RUST_BACKTRACE=1
export RUST_LOG=debug

if [ "$VERBOSE" = true ]; then
    export TEST_VERBOSE=1
fi

if [ "$RUN_BENCHMARK" = true ]; then
    export TEST_BENCHMARK=1
fi

if [ "$RUN_SECURITY" = true ]; then
    export TEST_SECURITY=1
fi

# Function to run tests and capture results
run_tests() {
    local test_type=$1
    local test_args=$2
    
    echo -e "${YELLOW}Running $test_type tests...${NC}"
    
    if cargo test $test_args --color=always 2>&1 | tee test-output.log; then
        echo -e "${GREEN}✓ $test_type tests passed${NC}"
        return 0
    else
        echo -e "${RED}✗ $test_type tests failed${NC}"
        return 1
    fi
}

# Track overall success
OVERALL_SUCCESS=true

# Run unit tests
if [ "$RUN_UNIT" = true ]; then
    if ! run_tests "Unit" "--lib"; then
        OVERALL_SUCCESS=false
    fi
    echo ""
fi

# Run integration tests
if [ "$RUN_INTEGRATION" = true ]; then
    if ! run_tests "Integration" "--test '*' -- --test-threads=1"; then
        OVERALL_SUCCESS=false
    fi
    echo ""
fi

# Run benchmarks if requested
if [ "$RUN_BENCHMARK" = true ]; then
    echo -e "${YELLOW}Running benchmarks...${NC}"
    if cargo bench --no-run 2>&1 | tee -a test-output.log; then
        echo -e "${GREEN}✓ Benchmarks compiled successfully${NC}"
    else
        echo -e "${RED}✗ Benchmark compilation failed${NC}"
        OVERALL_SUCCESS=false
    fi
    echo ""
fi

# Generate code coverage report
if command -v cargo-tarpaulin &> /dev/null; then
    echo -e "${YELLOW}Generating code coverage report...${NC}"
    cargo tarpaulin --out Html --output-dir coverage/ || true
    echo -e "${GREEN}Coverage report generated in coverage/tarpaulin-report.html${NC}"
    echo ""
fi

# Run clippy for additional checks
echo -e "${YELLOW}Running clippy checks...${NC}"
if cargo clippy --all-targets --all-features -- -D warnings 2>&1 | tee -a test-output.log; then
    echo -e "${GREEN}✓ Clippy checks passed${NC}"
else
    echo -e "${RED}✗ Clippy checks failed${NC}"
    OVERALL_SUCCESS=false
fi
echo ""

# Summary
echo "=================================="
echo "Test Summary"
echo "=================================="

if [ "$OVERALL_SUCCESS" = true ]; then
    echo -e "${GREEN}✓ All tests passed successfully!${NC}"
    
    # Count test cases
    UNIT_COUNT=$(grep -c "test result: ok" test-output.log 2>/dev/null || echo "0")
    echo ""
    echo "Test Statistics:"
    echo "- Total tests run: $UNIT_COUNT"
    
    # Show coverage if available
    if [ -f coverage/tarpaulin-report.html ]; then
        COVERAGE=$(grep -oP 'Total coverage: \K[0-9.]+%' coverage/tarpaulin-report.html 2>/dev/null || echo "N/A")
        echo "- Code coverage: $COVERAGE"
    fi
    
    exit 0
else
    echo -e "${RED}✗ Some tests failed. Check the output above for details.${NC}"
    exit 1
fi