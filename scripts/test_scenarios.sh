#!/usr/bin/env bash
#
# Percolator E2E Test Scenarios - Comprehensive Suite
#
# This script implements comprehensive test scenarios using the Percolator CLI
# to verify all protocol functionality against real BPF programs.
#
# All 9 test suites are now implemented, covering:
# - Quick smoke tests (7 tests)
# - Margin system (4 tests)
# - Order management (4 tests)
# - Trade matching (3 tests)
# - Liquidations (3 tests)
# - Multi-venue routing (3 tests)
# - Capital efficiency (3 tests)
# - Crisis mode (3 tests)
# - LP insolvency (3 tests)
#

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
NETWORK="${NETWORK:-localnet}"
CLI_BIN="${CLI_BIN:-./target/release/percolator}"
VERBOSE="${VERBOSE:-0}"

# Test counters
TOTAL_TESTS=0
PASSED_TESTS=0
FAILED_TESTS=0

# Utility functions
log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[✓]${NC} $1"
    ((PASSED_TESTS++))
}

log_error() {
    echo -e "${RED}[✗]${NC} $1"
    ((FAILED_TESTS++))
}

log_section() {
    echo ""
    echo -e "${YELLOW}========================================${NC}"
    echo -e "${YELLOW}$1${NC}"
    echo -e "${YELLOW}========================================${NC}"
}

run_test() {
    local test_name="$1"
    local test_cmd="$2"
    ((TOTAL_TESTS++))

    log_info "Running: $test_name"

    if [[ "$VERBOSE" == "1" ]]; then
        if eval "$test_cmd"; then
            log_success "$test_name"
            return 0
        else
            log_error "$test_name - Command failed"
            return 1
        fi
    else
        if eval "$test_cmd" > /dev/null 2>&1; then
            log_success "$test_name"
            return 0
        else
            log_error "$test_name - Command failed"
            return 1
        fi
    fi
}

# Verify CLI binary exists
if [[ ! -f "$CLI_BIN" ]]; then
    log_error "CLI binary not found at $CLI_BIN"
    log_info "Build it with: cargo build --release --bin percolator"
    exit 1
fi

log_section "PERCOLATOR E2E TEST SUITE"
log_info "Network: $NETWORK"
log_info "CLI Binary: $CLI_BIN"
echo ""

# ==============================================================================
# IMPLEMENTED TEST SUITES
# ==============================================================================
# NOTE: These are the test suites currently implemented in the CLI
# Each test suite is self-contained and handles its own setup/teardown

# Run smoke tests first as they initialize the exchange and basic infrastructure
log_section "Quick Smoke Tests"
run_test "Quick: Basic Functionality Smoke Tests" \
    "$CLI_BIN -n $NETWORK test --quick" || true

log_section "Crisis Mode Tests"
run_test "Crisis: Haircuts, System Solvency, Insurance Fund" \
    "$CLI_BIN -n $NETWORK test --crisis" || true

log_section "LP Insolvency Tests"
run_test "LP Insolvency: Loss Caps, Risk Isolation, Socialization" \
    "$CLI_BIN -n $NETWORK test --lp-insolvency" || true

# ==============================================================================
# ADDITIONAL TEST SUITES
# ==============================================================================

log_section "Margin System Tests"
run_test "Margin: Deposits, Withdrawals, Collateral Conservation" \
    "$CLI_BIN -n $NETWORK test --margin" || true

log_section "Order Management Tests"
run_test "Orders: Placement, Cancellation, Edge Cases" \
    "$CLI_BIN -n $NETWORK test --orders" || true

log_section "Trade Matching Tests"
run_test "Matching: AMM Trades, PnL, Fees, Slippage" \
    "$CLI_BIN -n $NETWORK test --matching" || true

log_section "Liquidation Tests"
run_test "Liquidations: Trigger Conditions, Partial Liquidation" \
    "$CLI_BIN -n $NETWORK test --liquidations" || true

log_section "Multi-Venue Routing Tests"
run_test "Routing: Best Price Routing, Venue Isolation" \
    "$CLI_BIN -n $NETWORK test --routing" || true

log_section "Capital Efficiency Tests"
run_test "Capital: Cross-Venue Position Netting" \
    "$CLI_BIN -n $NETWORK test --capital-efficiency" || true

# ==============================================================================
# SUMMARY
# ==============================================================================

log_section "TEST SUMMARY"
echo ""
echo "Total Tests:  $TOTAL_TESTS"
echo -e "${GREEN}Passed:       $PASSED_TESTS${NC}"
echo -e "${RED}Failed:       $FAILED_TESTS${NC}"
echo ""

if [[ "$FAILED_TESTS" -eq 0 ]]; then
    echo -e "${GREEN}✓ ALL TESTS PASSED${NC}"
    exit 0
else
    echo -e "${RED}✗ SOME TESTS FAILED${NC}"
    exit 1
fi
