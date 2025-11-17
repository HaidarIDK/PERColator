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
log "${BLUE}Checking for existing validators...${NC}"
if pgrep -f solana-test-validator > /dev/null 2>&1; then
    run_cmd "Killing existing test validators" killall -9 solana-test-validator || true
    sleep 2
else
    log "${GREEN}  ✓ No existing validators found${NC}"
fi

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

# Get wallet address for funding
WALLET_ADDRESS=$(solana address 2>/dev/null || echo "")
if [ -n "$WALLET_ADDRESS" ]; then
    log "${CYAN}  Wallet Address: ${WALLET_ADDRESS}${NC}"
fi

# Ensure we have enough SOL for deployment
log "${BLUE}Checking SOL balance for deployment...${NC}"
MIN_BALANCE=5000000000  # 5 SOL minimum for deployments
MIN_SOL_NEEDED=$((MIN_BALANCE / 1000000000))

while true; do
    BALANCE=$(solana balance --lamports 2>/dev/null | grep -oE '[0-9]+' | head -1 || echo "0")
    BALANCE_SOL=$((BALANCE / 1000000000))
    
    if [ "$BALANCE" -lt "$MIN_BALANCE" ]; then
        log "${YELLOW}  Current balance: ${BALANCE_SOL} SOL${NC}"
        log "${YELLOW}  Need at least: ${MIN_SOL_NEEDED} SOL for program deployment${NC}"
        if [ -n "$WALLET_ADDRESS" ]; then
            log "${CYAN}  ═══════════════════════════════════════════════════════════${NC}"
            log "${CYAN}  Please send at least ${MIN_SOL_NEEDED} SOL to:${NC}"
            log "${CYAN}  ${WALLET_ADDRESS}${NC}"
            log "${CYAN}  ═══════════════════════════════════════════════════════════${NC}"
        fi
        log "${BLUE}  Waiting for sufficient balance... (checking every 3 seconds)${NC}"
        log "${BLUE}  Press Ctrl+C to cancel${NC}"
        sleep 3
    else
        log "${GREEN}  ✓ Balance sufficient: ${BALANCE_SOL} SOL${NC}"
        break
    fi
done

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

# Step 6: Initialize Exchange (if not already initialized)
log ""
log "${YELLOW}═══ Step 6: Initialize Exchange${NC}"

# Show wallet address (already shown in Step 4, but show again for clarity)
if [ -z "$WALLET_ADDRESS" ]; then
    WALLET_ADDRESS=$(solana address 2>/dev/null || echo "")
fi
if [ -n "$WALLET_ADDRESS" ]; then
    log "${CYAN}  Wallet Address: ${WALLET_ADDRESS}${NC}"
fi

# Ensure we have enough SOL for registry initialization (needs ~0.3 SOL for rent)
log "${BLUE}Checking SOL balance for initialization...${NC}"
MIN_BALANCE_FOR_INIT=500000000  # 0.5 SOL minimum for registry rent
MIN_SOL_NEEDED=$((MIN_BALANCE_FOR_INIT / 1000000000))

while true; do
    BALANCE=$(solana balance --lamports 2>/dev/null | grep -oE '[0-9]+' | head -1 || echo "0")
    BALANCE_SOL=$((BALANCE / 1000000000))
    
    if [ "$BALANCE" -lt "$MIN_BALANCE_FOR_INIT" ]; then
        log "${YELLOW}  Current balance: ${BALANCE_SOL} SOL${NC}"
        log "${YELLOW}  Need at least: ${MIN_SOL_NEEDED} SOL${NC}"
        if [ -n "$WALLET_ADDRESS" ]; then
            log "${CYAN}  ═══════════════════════════════════════════════════════════${NC}"
            log "${CYAN}  Please send at least ${MIN_SOL_NEEDED} SOL to:${NC}"
            log "${CYAN}  ${WALLET_ADDRESS}${NC}"
            log "${CYAN}  ═══════════════════════════════════════════════════════════${NC}"
        fi
        log "${BLUE}  Waiting for sufficient balance... (checking every 3 seconds)${NC}"
        log "${BLUE}  Press Ctrl+C to cancel${NC}"
        sleep 3
    else
        log "${GREEN}  ✓ Balance sufficient: ${BALANCE_SOL} SOL${NC}"
        break
    fi
done

log "${BLUE}Initializing exchange registry...${NC}"

# Get the actual payer address that the CLI will use
# The CLI uses ~/.config/solana/id.json by default, but we need to check what it actually uses
# Run a quick dry-run to get the payer address, or check the keypair file
CLI_KEYPAIR_PATH="${HOME}/.config/solana/id.json"
if [ -f "$CLI_KEYPAIR_PATH" ]; then
    # Try to extract pubkey from keypair file (if it's a JSON array)
    # This is a simple approach - the actual payer will be shown in init output
    PAYER_ADDRESS=$(solana address --keypair "$CLI_KEYPAIR_PATH" 2>/dev/null || echo "")
else
    # Fallback to default solana address
    PAYER_ADDRESS=$(solana address 2>/dev/null || echo "")
fi

# If we couldn't get it, we'll extract it from init output
# But first, try to get it from a test run
if [ -z "$PAYER_ADDRESS" ]; then
    log "${BLUE}  Determining payer address from CLI...${NC}"
    # Run init with --help or a dry run to see payer, but that won't work
    # Instead, we'll extract it from the actual init output
    PAYER_ADDRESS=""
fi

# We'll check balance after we see the actual payer in init output
# For now, try airdrop to the default keypair if on localnet
if [ -n "$PAYER_ADDRESS" ]; then
    log "${CYAN}  Checking payer account: ${PAYER_ADDRESS}${NC}"
    PAYER_BALANCE=$(solana balance "$PAYER_ADDRESS" --lamports 2>/dev/null | grep -oE '[0-9]+' | head -1 || echo "0")
    PAYER_BALANCE_SOL=$((PAYER_BALANCE / 1000000000))
    log "${BLUE}  Payer balance: ${PAYER_BALANCE_SOL} SOL${NC}"
    
    # For localnet, ensure sufficient funding
    # Use transfer from funded wallet if available, otherwise airdrop
    if [ "$(solana config get 2>/dev/null | grep -i 'url.*localnet\|url.*127.0.0.1' || echo '')" != "" ]; then
        log "${BLUE}  Ensuring payer account has sufficient funds (localnet)...${NC}"
        
        # Calculate how much SOL we need (at least 10 SOL for all tests)
        DESIRED_BALANCE=10000000000  # 10 SOL
        
        if [ "$PAYER_BALANCE" -lt "$DESIRED_BALANCE" ]; then
            NEEDED=$((DESIRED_BALANCE - PAYER_BALANCE))
            NEEDED_SOL=$((NEEDED / 1000000000 + 1))
            
            # Check if we have a funded wallet to transfer from
            if [ -n "$WALLET_ADDRESS" ] && [ "$WALLET_ADDRESS" != "$PAYER_ADDRESS" ]; then
                WALLET_BALANCE=$(solana balance "$WALLET_ADDRESS" --lamports 2>/dev/null | grep -oE '[0-9]+' | head -1 || echo "0")
                
                # If wallet has enough, use transfer; otherwise use airdrop
                if [ "$WALLET_BALANCE" -gt "$((NEEDED + 1000000000))" ]; then
                    log "${BLUE}  Transferring ${NEEDED_SOL} SOL from funded wallet to payer...${NC}"
                    if solana transfer --allow-unfunded-recipient "$PAYER_ADDRESS" "$NEEDED_SOL" --from "$WALLET_ADDRESS" >> "$LOG_FILE" 2>&1; then
                        sleep 2
                        PAYER_BALANCE=$(solana balance "$PAYER_ADDRESS" --lamports 2>/dev/null | grep -oE '[0-9]+' | head -1 || echo "0")
                        PAYER_BALANCE_SOL=$((PAYER_BALANCE / 1000000000))
                        log "${GREEN}  ✓ Payer funded via transfer: ${PAYER_BALANCE_SOL} SOL${NC}"
                    else
                        log "${YELLOW}  Transfer failed, using airdrop...${NC}"
                        # Fall back to airdrop
                        while [ "$NEEDED_SOL" -gt 0 ]; do
                            AIRDROP_AMOUNT=$((NEEDED_SOL < 5 ? NEEDED_SOL : 5))
                            solana airdrop "$AIRDROP_AMOUNT" "$PAYER_ADDRESS" >> "$LOG_FILE" 2>&1 || true
                            sleep 2
                            NEEDED_SOL=$((NEEDED_SOL - AIRDROP_AMOUNT))
                        done
                    fi
                else
                    log "${BLUE}  Wallet doesn't have enough, using airdrop...${NC}"
                    # Do multiple airdrops if needed (max 5 SOL each)
                    while [ "$NEEDED_SOL" -gt 0 ]; do
                        AIRDROP_AMOUNT=$((NEEDED_SOL < 5 ? NEEDED_SOL : 5))
                        solana airdrop "$AIRDROP_AMOUNT" "$PAYER_ADDRESS" >> "$LOG_FILE" 2>&1 || true
                        sleep 2
                        NEEDED_SOL=$((NEEDED_SOL - AIRDROP_AMOUNT))
                    done
                fi
            else
                log "${BLUE}  Using airdrop to fund payer...${NC}"
                # Do multiple airdrops if needed (max 5 SOL each)
                while [ "$NEEDED_SOL" -gt 0 ]; do
                    AIRDROP_AMOUNT=$((NEEDED_SOL < 5 ? NEEDED_SOL : 5))
                    solana airdrop "$AIRDROP_AMOUNT" "$PAYER_ADDRESS" >> "$LOG_FILE" 2>&1 || true
                    sleep 2
                    NEEDED_SOL=$((NEEDED_SOL - AIRDROP_AMOUNT))
                done
            fi
            
            # Verify final balance
            PAYER_BALANCE=$(solana balance "$PAYER_ADDRESS" --lamports 2>/dev/null | grep -oE '[0-9]+' | head -1 || echo "0")
            PAYER_BALANCE_SOL=$((PAYER_BALANCE / 1000000000))
            log "${GREEN}  ✓ Payer balance after funding: ${PAYER_BALANCE_SOL} SOL${NC}"
        else
            log "${GREEN}  ✓ Payer balance sufficient: ${PAYER_BALANCE_SOL} SOL${NC}"
        fi
    fi
fi

# Before initializing, ensure the CLI payer is funded on this validator
# CRITICAL: When validator restarts, accounts don't exist. Must fund on THIS validator instance.
if [ -n "$PAYER_ADDRESS" ] && [ -n "$WALLET_ADDRESS" ]; then
    log "${BLUE}  Ensuring CLI payer exists and is funded on THIS validator instance...${NC}"
    
    # Use transfer from funded wallet instead of airdrops (avoids rate limits)
    log "${BLUE}  Transferring 10 SOL from wallet to CLI payer...${NC}"
    
    if solana transfer "$PAYER_ADDRESS" 10 --from "$HOME/.config/solana/id.json" --allow-unfunded-recipient >> "$LOG_FILE" 2>&1; then
        log "${GREEN}  ✓ Transfer successful${NC}"
    else
        log "${YELLOW}  ⚠ Transfer failed, trying single airdrop as fallback...${NC}"
        solana airdrop 10 "$PAYER_ADDRESS" >> "$LOG_FILE" 2>&1 || true
        sleep 2
    fi
    
    # Verify it worked
    FINAL_BALANCE=$(solana balance "$PAYER_ADDRESS" --lamports 2>/dev/null | grep -oE '[0-9]+' | head -1 || echo "0")
    FINAL_BALANCE_SOL=$((FINAL_BALANCE / 1000000000))
    
    if [ "$FINAL_BALANCE" -gt 0 ]; then
        log "${GREEN}  ✓ CLI payer funded on fresh validator: ${FINAL_BALANCE_SOL} SOL${NC}"
    else
        log "${RED}  ✗ Failed to fund CLI payer${NC}"
        log "${YELLOW}  Registry initialization will likely fail${NC}"
    fi
fi

# Initialize exchange - this will skip if already exists
log "${BLUE}  Running init command (this may take 30-60 seconds)...${NC}"
log "${BLUE}  Building CLI if needed and initializing registry...${NC}"

# Use timeout to prevent hanging (90 seconds should be enough for build + init)
# Run command directly with timeout - simpler and more reliable
INIT_LOG="/tmp/init_output_$(date +%Y%m%d_%H%M%S).log"

# Run init command with timeout and capture output
# Use timeout command to prevent infinite hangs
if timeout 90 cargo run --release --package percolator-cli --bin percolator -- -n localnet init \
    --name "test-exchange" \
    --insurance-fund 1000000000 \
    --maintenance-margin 250 \
    --initial-margin 500 > "$INIT_LOG" 2>&1; then
    INIT_EXIT_CODE=0
else
    INIT_EXIT_CODE=$?
    # Check if it was a timeout
    if [ $INIT_EXIT_CODE -eq 124 ]; then
        log "${RED}  ✗ Init command timed out after 90 seconds${NC}"
    fi
fi

# Read the output
INIT_OUTPUT=$(cat "$INIT_LOG" 2>/dev/null || echo "")
echo "$INIT_OUTPUT" >> "$LOG_FILE"

# Extract the actual payer address from init output (CLI shows it)
ACTUAL_PAYER=$(echo "$INIT_OUTPUT" | grep "^Payer:" | awk '{print $2}' | head -1 || echo "")
if [ -n "$ACTUAL_PAYER" ] && [ "$ACTUAL_PAYER" != "$PAYER_ADDRESS" ]; then
    log "${YELLOW}  Note: CLI uses different payer: ${ACTUAL_PAYER}${NC}"
    PAYER_ADDRESS="$ACTUAL_PAYER"
    
    # Check and fund the actual payer if needed
    PAYER_BALANCE=$(solana balance "$PAYER_ADDRESS" --lamports 2>/dev/null | grep -oE '[0-9]+' | head -1 || echo "0")
    PAYER_BALANCE_SOL=$((PAYER_BALANCE / 1000000000))
    
    if [ "$PAYER_BALANCE" -lt "$MIN_BALANCE_FOR_INIT" ]; then
        log "${YELLOW}  Actual payer needs more SOL${NC}"
        if [ "$(solana config get 2>/dev/null | grep -i 'url.*localnet\|url.*127.0.0.1' || echo '')" != "" ]; then
            log "${BLUE}  Airdropping 2 SOL to actual payer account...${NC}"
            solana airdrop 2 "$PAYER_ADDRESS" 2>&1 | grep -v "Signature" >> "$LOG_FILE" || true
            sleep 3
            PAYER_BALANCE=$(solana balance "$PAYER_ADDRESS" --lamports 2>/dev/null | grep -oE '[0-9]+' | head -1 || echo "0")
            PAYER_BALANCE_SOL=$((PAYER_BALANCE / 1000000000))
            log "${BLUE}  Actual payer balance: ${PAYER_BALANCE_SOL} SOL${NC}"
        fi
        
        if [ "$PAYER_BALANCE" -lt "$MIN_BALANCE_FOR_INIT" ]; then
            log "${CYAN}  ═══════════════════════════════════════════════════════════${NC}"
            log "${CYAN}  Please send at least ${MIN_SOL_NEEDED} SOL to actual payer:${NC}"
            log "${CYAN}  ${PAYER_ADDRESS}${NC}"
            log "${CYAN}  ═══════════════════════════════════════════════════════════${NC}"
            log "${YELLOW}  Re-running init after funding...${NC}"
            
            # Re-run init after funding
            timeout 90 cargo run --release --package percolator-cli --bin percolator -- -n localnet init \
                --name "test-exchange" \
                --insurance-fund 1000000000 \
                --maintenance-margin 250 \
                --initial-margin 500 > "$INIT_LOG" 2>&1
            INIT_EXIT_CODE=$?
            INIT_OUTPUT=$(cat "$INIT_LOG" 2>/dev/null || echo "")
            echo "$INIT_OUTPUT" >> "$LOG_FILE"
        fi
    fi
fi

# Check the log for success indicators
if echo "$INIT_OUTPUT" | grep -q "Registry account already exists\|Registry initialized\|Success!"; then
    log "${GREEN}  ✓ Exchange registry ready${NC}"
    # Extract registry address if shown
    REGISTRY=$(echo "$INIT_OUTPUT" | grep "Registry Address:" | head -1 | awk '{print $3}' || echo "")
    if [ -n "$REGISTRY" ]; then
        log "${BLUE}    Registry: ${REGISTRY}${NC}"
    fi
elif echo "$INIT_OUTPUT" | grep -q "Error.*debit.*account\|Failed to send transaction\|insufficient"; then
    log "${RED}  ✗ Initialization failed: Insufficient funds or transaction error${NC}"
    
    # Extract registry address for verification
    REGISTRY=$(echo "$INIT_OUTPUT" | grep "Registry Address:" | head -1 | awk '{print $3}' || echo "")
    if [ -n "$REGISTRY" ]; then
        log "${BLUE}    Expected registry: ${REGISTRY}${NC}"
        
        # Verify if registry exists on chain despite error
        log "${BLUE}    Checking if registry exists on chain...${NC}"
        if solana account "$REGISTRY" >> "$LOG_FILE" 2>&1; then
            log "${GREEN}  ✓ Registry account exists on chain (init may have succeeded)${NC}"
        else
            log "${YELLOW}  ⚠ Registry account does NOT exist - tests will retry initialization${NC}"
        fi
    fi
elif echo "$INIT_OUTPUT" | grep -q "Registry Address:"; then
    # Registry address shown but status unclear - verify
    REGISTRY=$(echo "$INIT_OUTPUT" | grep "Registry Address:" | head -1 | awk '{print $3}' || echo "")
    if [ -n "$REGISTRY" ]; then
        log "${BLUE}    Registry address: ${REGISTRY}${NC}"
        log "${BLUE}    Verifying registry on chain...${NC}"
        
        if solana account "$REGISTRY" >> "$LOG_FILE" 2>&1; then
            log "${GREEN}  ✓ Registry verified on chain${NC}"
        else
            log "${YELLOW}  ⚠ Registry not found on chain - tests will retry initialization${NC}"
        fi
    fi
else
    log "${YELLOW}  ℹ Initialization status unclear (exit code: ${INIT_EXIT_CODE})${NC}"
    log "${BLUE}  (Tests will handle initialization)${NC}"
fi

sleep 2

# Step 7: Run crisis tests (includes all 8 phases)
log ""
log "${CYAN}═══════════════════════════════════════════════════════════════${NC}"
log "${CYAN}  Running Crisis Tests (8-Phase Kitchen Sink E2E)${NC}"
log "${CYAN}═══════════════════════════════════════════════════════════════${NC}"
log ""

# Run the test and capture output
TEST_OUTPUT="/tmp/crisis_test_output_$(date +%Y%m%d_%H%M%S).log"
log "${BLUE}Running: cargo run --release --package percolator-cli --bin percolator -- -n localnet test --crisis${NC}"
log "${BLUE}Test output: ${TEST_OUTPUT}${NC}"
log ""

if cargo run --release --package percolator-cli --bin percolator -- -n localnet test --crisis 2>&1 | tee "$TEST_OUTPUT"; then
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
    if pgrep -f solana-test-validator > /dev/null 2>&1; then
        killall solana-test-validator 2>/dev/null || true
        log "${GREEN}  ✓ Validator stopped${NC}"
    else
        log "${GREEN}  ✓ No validator to stop${NC}"
    fi
    log "${GREEN}  ✓ Cleanup complete${NC}"
fi

exit $EXIT_CODE
