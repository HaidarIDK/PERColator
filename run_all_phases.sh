#!/bin/bash
#
# Comprehensive Test Runner for Percolator Protocol
# Sets up environment, deploys programs, and runs all 8 test phases
#

set -e  # Exit on error

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

# Log file
LOG_FILE="/tmp/percolator_test_$(date +%Y%m%d_%H%M%S).log"

echo -e "${CYAN}═══════════════════════════════════════════════════════════════${NC}"
echo -e "${CYAN}  Percolator Protocol - Complete Test Suite Runner${NC}"
echo -e "${CYAN}═══════════════════════════════════════════════════════════════${NC}"
echo ""
echo -e "${BLUE}Log file: ${LOG_FILE}${NC}"
echo ""

# Function to log with timestamp
log() {
    echo -e "$1" | tee -a "$LOG_FILE"
}

# Function to run command with logging
run_cmd() {
    local desc="$1"
    shift
    log "${YELLOW}▶ ${desc}...${NC}"
    if "$@" >> "$LOG_FILE" 2>&1; then
        log "${GREEN}  ✓ Success${NC}"
        return 0
    else
        log "${RED}  ✗ Failed${NC}"
        return 1
    fi
}

# Step 1: Clean up any existing validator
log "${YELLOW}═══ Step 1: Cleanup${NC}"
run_cmd "Killing existing test validators" killall -9 solana-test-validator || true
sleep 2

# Step 2: Build programs
log ""
log "${YELLOW}═══ Step 2: Building Programs${NC}"
log "${BLUE}This may take several minutes...${NC}"
if ! run_cmd "Building Solana programs" cargo build-sbf; then
    log "${RED}Build failed! Check ${LOG_FILE} for details${NC}"
    exit 1
fi

# Step 3: Start test validator
log ""
log "${YELLOW}═══ Step 3: Starting Test Validator${NC}"
run_cmd "Starting solana-test-validator" sh -c "solana-test-validator --reset --quiet > /tmp/validator.log 2>&1 &"
log "${BLUE}Waiting for validator to be ready...${NC}"
sleep 8

# Verify validator is running
if ! solana ping --count 2 >> "$LOG_FILE" 2>&1; then
    log "${RED}Validator failed to start! Check /tmp/validator.log${NC}"
    exit 1
fi
log "${GREEN}  ✓ Validator is ready${NC}"

# Step 4: Deploy programs
log ""
log "${YELLOW}═══ Step 4: Deploying Programs${NC}"

# Deploy each program explicitly to avoid loop issues
PROGRAMS_DIR="target/deploy"
if [ ! -d "$PROGRAMS_DIR" ]; then
    log "${RED}Programs directory not found: $PROGRAMS_DIR${NC}"
    exit 1
fi

DEPLOYED=0
FAILED=0

# Deploy percolator_amm
if [ -f "$PROGRAMS_DIR/percolator_amm.so" ]; then
    log "${BLUE}  Deploying percolator_amm...${NC}"
    if solana program deploy "$PROGRAMS_DIR/percolator_amm.so" >> "$LOG_FILE" 2>&1; then
        PROGRAM_ID=$(solana-keygen pubkey "$PROGRAMS_DIR/percolator_amm-keypair.json" 2>/dev/null || echo "unknown")
        log "${GREEN}    ✓ Deployed percolator_amm (${PROGRAM_ID})${NC}"
        DEPLOYED=$((DEPLOYED + 1))
    else
        log "${RED}    ✗ Failed to deploy percolator_amm${NC}"
        FAILED=$((FAILED + 1))
    fi
fi

# Deploy percolator_oracle
if [ -f "$PROGRAMS_DIR/percolator_oracle.so" ]; then
    log "${BLUE}  Deploying percolator_oracle...${NC}"
    if solana program deploy "$PROGRAMS_DIR/percolator_oracle.so" >> "$LOG_FILE" 2>&1; then
        PROGRAM_ID=$(solana-keygen pubkey "$PROGRAMS_DIR/percolator_oracle-keypair.json" 2>/dev/null || echo "unknown")
        log "${GREEN}    ✓ Deployed percolator_oracle (${PROGRAM_ID})${NC}"
        DEPLOYED=$((DEPLOYED + 1))
    else
        log "${RED}    ✗ Failed to deploy percolator_oracle${NC}"
        FAILED=$((FAILED + 1))
    fi
fi

# Deploy percolator_router
if [ -f "$PROGRAMS_DIR/percolator_router.so" ]; then
    log "${BLUE}  Deploying percolator_router...${NC}"
    if solana program deploy "$PROGRAMS_DIR/percolator_router.so" >> "$LOG_FILE" 2>&1; then
        PROGRAM_ID=$(solana-keygen pubkey "$PROGRAMS_DIR/percolator_router-keypair.json" 2>/dev/null || echo "unknown")
        log "${GREEN}    ✓ Deployed percolator_router (${PROGRAM_ID})${NC}"
        DEPLOYED=$((DEPLOYED + 1))
    else
        log "${RED}    ✗ Failed to deploy percolator_router${NC}"
        FAILED=$((FAILED + 1))
    fi
fi

# Deploy percolator_slab
if [ -f "$PROGRAMS_DIR/percolator_slab.so" ]; then
    log "${BLUE}  Deploying percolator_slab...${NC}"
    if solana program deploy "$PROGRAMS_DIR/percolator_slab.so" >> "$LOG_FILE" 2>&1; then
        PROGRAM_ID=$(solana-keygen pubkey "$PROGRAMS_DIR/percolator_slab-keypair.json" 2>/dev/null || echo "unknown")
        log "${GREEN}    ✓ Deployed percolator_slab (${PROGRAM_ID})${NC}"
        DEPLOYED=$((DEPLOYED + 1))
    else
        log "${RED}    ✗ Failed to deploy percolator_slab${NC}"
        FAILED=$((FAILED + 1))
    fi
fi

log ""
log "${GREEN}Deployed: ${DEPLOYED} programs${NC}"
if [ "$FAILED" -gt 0 ]; then
    log "${RED}Failed: ${FAILED} programs${NC}"
fi

# Step 5: Build CLI
log ""
log "${YELLOW}═══ Step 5: Building CLI${NC}"
if ! run_cmd "Building percolator CLI" cargo build --release --package percolator-cli; then
    log "${RED}CLI build failed! Check ${LOG_FILE} for details${NC}"
    exit 1
fi

# Step 6: Run crisis tests (includes all 8 phases)
log ""
log "${CYAN}═══════════════════════════════════════════════════════════════${NC}"
log "${CYAN}  Running Crisis Tests (8-Phase Kitchen Sink E2E)${NC}"
log "${CYAN}═══════════════════════════════════════════════════════════════${NC}"
log ""

# Run the test and capture output
TEST_OUTPUT="/tmp/crisis_test_output_$(date +%Y%m%d_%H%M%S).log"
log "${BLUE}Running: cargo run --release --package percolator-cli --bin percolator -- test --crisis${NC}"
log "${BLUE}Test output: ${TEST_OUTPUT}${NC}"
log ""

if cargo run --release --package percolator-cli --bin percolator -- test --crisis 2>&1 | tee "$TEST_OUTPUT"; then
    log ""
    log "${GREEN}═══════════════════════════════════════════════════════════════${NC}"
    log "${GREEN}  ✓ Tests Completed Successfully!${NC}"
    log "${GREEN}═══════════════════════════════════════════════════════════════${NC}"
    EXIT_CODE=0
else
    log ""
    log "${RED}═══════════════════════════════════════════════════════════════${NC}"
    log "${RED}  ✗ Tests Failed!${NC}"
    log "${RED}═══════════════════════════════════════════════════════════════${NC}"
    EXIT_CODE=1
fi

# Summary
log ""
log "${YELLOW}═══ Test Summary${NC}"
log "  Full log: ${LOG_FILE}"
log "  Test output: ${TEST_OUTPUT}"
log "  Validator log: /tmp/validator.log"
log ""

# Extract key results from test output
if [ -f "$TEST_OUTPUT" ]; then
    log "${YELLOW}═══ Results${NC}"
    grep -E "^(✓|✗)" "$TEST_OUTPUT" | head -20 || true
    log ""
fi

log "${BLUE}To view detailed test output:${NC}"
log "  cat ${TEST_OUTPUT}"
log ""
log "${BLUE}To view full execution log:${NC}"
log "  cat ${LOG_FILE}"
log ""

# Cleanup option
if [ "$1" == "--keep-validator" ]; then
    log "${YELLOW}Keeping validator running (--keep-validator flag set)${NC}"
    log "${BLUE}To stop: killall solana-test-validator${NC}"
else
    log "${YELLOW}Stopping validator...${NC}"
    killall solana-test-validator 2>/dev/null || true
    log "${GREEN}  ✓ Cleanup complete${NC}"
fi

exit $EXIT_CODE
