#!/usr/bin/env bash
#
# Percolator E2E Test Runner
#
# This script runs the comprehensive E2E test suite against real BPF programs.
# It orchestrates validator startup, program deployment, and test execution.
#

set -euo pipefail

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[✓]${NC} $1"
}

log_error() {
    echo -e "${RED}[✗]${NC} $1"
}

log_section() {
    echo ""
    echo -e "${YELLOW}========================================${NC}"
    echo -e "${YELLOW}$1${NC}"
    echo -e "${YELLOW}========================================${NC}"
}

# Configuration
NETWORK="${NETWORK:-localnet}"
SKIP_BUILD="${SKIP_BUILD:-0}"
SKIP_VALIDATOR="${SKIP_VALIDATOR:-0}"
VALIDATOR_STARTUP_WAIT="${VALIDATOR_STARTUP_WAIT:-10}"

cd "$PROJECT_ROOT"

log_section "PERCOLATOR E2E TEST SUITE"
log_info "Project Root: $PROJECT_ROOT"
log_info "Network: $NETWORK"
echo ""

# Build programs and CLI if not skipped
if [[ "$SKIP_BUILD" != "1" ]]; then
    log_section "Building Programs and CLI"

    log_info "Building BPF programs..."
    if cargo build-sbf --manifest-path programs/router/Cargo.toml \
            --sbf-out-dir ./target/deploy 2>&1 | grep -q "Finished"; then
        log_success "Router program built"
    else
        log_error "Failed to build router program"
        exit 1
    fi

    if cargo build-sbf --manifest-path programs/slab/Cargo.toml \
            --sbf-out-dir ./target/deploy 2>&1 | grep -q "Finished"; then
        log_success "Slab program built"
    else
        log_error "Failed to build slab program"
        exit 1
    fi

    log_info "Building CLI binary..."
    if cargo build --release -p percolator-cli 2>&1 | grep -q "Finished"; then
        log_success "CLI built successfully"
    else
        log_error "Failed to build CLI"
        exit 1
    fi
else
    log_info "Skipping build (SKIP_BUILD=1)"
fi

# Start validator if testing on localnet and not skipped
if [[ "$NETWORK" == "localnet" ]] && [[ "$SKIP_VALIDATOR" != "1" ]]; then
    log_section "Starting Solana Test Validator"

    # Kill existing validator
    pkill -f solana-test-validator || true
    sleep 1

    # Clean ledger
    rm -rf test-ledger

    # Start validator in background with BPF programs
    log_info "Starting validator with BPF programs..."
    solana-test-validator --reset --quiet \
        --bpf-program 7NUzsomCpwX1MMVHSLDo8tmcCDpUTXiWb1SWa94BpANf "$PROJECT_ROOT/target/deploy/percolator_router.so" \
        --bpf-program CmJKuXjspb84yaaoWFSujVgzaXktCw4jwaxzdbRbrJ8g "$PROJECT_ROOT/target/deploy/percolator_slab.so" \
        > /tmp/percolator_validator.log 2>&1 &
    VALIDATOR_PID=$!

    log_info "Waiting ${VALIDATOR_STARTUP_WAIT}s for validator to start..."
    sleep "$VALIDATOR_STARTUP_WAIT"

    # Verify validator is running
    if solana cluster-version > /dev/null 2>&1; then
        log_success "Validator running (PID: $VALIDATOR_PID)"
    else
        log_error "Validator failed to start"
        cat /tmp/percolator_validator.log
        exit 1
    fi
else
    log_info "Skipping validator startup (SKIP_VALIDATOR=1 or network != localnet)"
fi

# Run comprehensive test suite
log_section "Running Comprehensive Test Scenarios"
echo ""

if [[ -x "$SCRIPT_DIR/test_scenarios.sh" ]]; then
    if NETWORK="$NETWORK" "$SCRIPT_DIR/test_scenarios.sh"; then
        TEST_EXIT_CODE=0
        log_success "All test scenarios passed!"
    else
        TEST_EXIT_CODE=$?
        log_error "Some test scenarios failed"
    fi
else
    log_error "Test scenarios script not found or not executable"
    log_info "Expected: $SCRIPT_DIR/test_scenarios.sh"
    TEST_EXIT_CODE=1
fi

# Cleanup
if [[ "$NETWORK" == "localnet" ]] && [[ "$SKIP_VALIDATOR" != "1" ]]; then
    log_section "Cleanup"
    log_info "Stopping validator..."
    kill $VALIDATOR_PID 2>/dev/null || true
    log_success "Validator stopped"
fi

log_section "TEST SUITE COMPLETE"
exit $TEST_EXIT_CODE
