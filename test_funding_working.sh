#!/usr/bin/env bash
#
# Working E2E Funding Test Script
#
# This script:
# 1. Starts localnet validator with deployed programs
# 2. Creates a test keypair
# 3. Initializes exchange (registry)
# 4. Creates a slab (market)
# 5. Updates funding rate on the slab
# 6. Verifies the transaction succeeds

set -e  # Exit on error

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;36m'
NC='\033[0m' # No Color

echo -e "${YELLOW}========================================${NC}"
echo -e "${YELLOW}  Funding Mechanics E2E Test${NC}"
echo -e "${YELLOW}========================================${NC}"
echo ""

# Cleanup function
cleanup() {
    echo -e "${BLUE}Cleaning up...${NC}"
    pkill -f solana-test-validator || true
    rm -rf test-ledger
    rm -f test-keypair.json
}

# Setup trap for cleanup on exit
trap cleanup EXIT

# Step 0: Check dependencies
echo -e "${GREEN}[0/7]${NC} Checking dependencies..."

if [ ! -f "target/release/percolator" ]; then
    echo -e "${RED}ERROR: CLI binary not found${NC}"
    echo "Please run: cargo build --release -p percolator-cli"
    exit 1
fi

if [ ! -f "target/deploy/percolator_router.so" ]; then
    echo -e "${RED}ERROR: Router program not found${NC}"
    echo "Please run: cargo build-sbf"
    exit 1
fi

echo -e "${GREEN}✓${NC} Dependencies OK"
echo ""

# Step 1: Create test keypair
echo -e "${GREEN}[1/7]${NC} Creating test keypair..."
solana-keygen new --no-bip39-passphrase --silent --outfile test-keypair.json
TEST_PUBKEY=$(solana-keygen pubkey test-keypair.json)
echo -e "${GREEN}✓${NC} Test keypair: $TEST_PUBKEY"
echo ""

# Step 2: Start localnet validator with deployed programs
echo -e "${GREEN}[2/7]${NC} Starting localnet validator with programs..."

# Program IDs from cli/src/config.rs
ROUTER_ID="7NUzsomCpwX1MMVHSLDo8tmcCDpUTXiWb1SWa94BpANf"
SLAB_ID="CmJKuXjspb84yaaoWFSujVgzaXktCw4jwaxzdbRbrJ8g"
AMM_ID="C9PdrHtZfDe24iFpuwtv4FHd7mPUnq52feFiKFNYLFvy"

solana-test-validator \
    --bpf-program $ROUTER_ID target/deploy/percolator_router.so \
    --bpf-program $SLAB_ID target/deploy/percolator_slab.so \
    --bpf-program $AMM_ID target/deploy/percolator_amm.so \
    --reset \
    --quiet \
    &

VALIDATOR_PID=$!
echo "Validator PID: $VALIDATOR_PID"

# Wait for validator to be ready
echo "Waiting for validator to start..."
sleep 8

# Check if validator is running
if ! kill -0 $VALIDATOR_PID 2>/dev/null; then
    echo -e "${RED}ERROR: Validator failed to start${NC}"
    exit 1
fi

echo -e "${GREEN}✓${NC} Validator started"
echo ""

# Step 3: Airdrop SOL to test keypair
echo -e "${GREEN}[3/7]${NC} Airdropping SOL to test keypair..."
solana airdrop 10 $TEST_PUBKEY --url http://127.0.0.1:8899 > /dev/null 2>&1
BALANCE=$(solana balance $TEST_PUBKEY --url http://127.0.0.1:8899 | cut -d' ' -f1)
echo -e "${GREEN}✓${NC} Balance: $BALANCE SOL"
echo ""

# Step 4: Initialize exchange (create registry)
echo -e "${GREEN}[4/7]${NC} Initializing exchange..."
INIT_OUTPUT=$(./target/release/percolator \
    --keypair test-keypair.json \
    --network localnet \
    init --name "test-exchange" 2>&1)

echo "$INIT_OUTPUT"

# Extract registry address (take first occurrence only)
REGISTRY=$(echo "$INIT_OUTPUT" | grep "Registry Address:" | head -1 | awk '{print $3}')

if [ -z "$REGISTRY" ]; then
    echo -e "${RED}ERROR: Failed to extract registry address${NC}"
    echo "$INIT_OUTPUT"
    exit 1
fi

echo -e "${GREEN}✓${NC} Registry created: $REGISTRY"
echo ""

# Step 5: Create slab (market)
echo -e "${GREEN}[5/7]${NC} Creating slab..."
CREATE_OUTPUT=$(./target/release/percolator \
    --keypair test-keypair.json \
    --network localnet \
    matcher create \
    $REGISTRY \
    "BTC-USD" \
    --tick-size 1000 \
    --lot-size 1000 2>&1)

echo "$CREATE_OUTPUT"

# Extract slab address (take first occurrence only)
SLAB=$(echo "$CREATE_OUTPUT" | grep "Slab Address:" | head -1 | awk '{print $3}')

if [ -z "$SLAB" ]; then
    echo -e "${RED}ERROR: Failed to extract slab address${NC}"
    echo "$CREATE_OUTPUT"
    exit 1
fi

echo -e "${GREEN}✓${NC} Slab created: $SLAB"
echo ""

# Step 6: Wait minimum time (65 seconds for funding update)
echo -e "${GREEN}[6/7]${NC} Waiting 65 seconds for minimum funding interval..."
echo -e "${BLUE}This is required by the UpdateFunding instruction (min 60s)${NC}"
for i in {65..1}; do
    echo -ne "\rTime remaining: $i seconds  "
    sleep 1
done
echo ""
echo -e "${GREEN}✓${NC} Wait complete"
echo ""

# Step 7: Update funding rate
echo -e "${GREEN}[7/7]${NC} Updating funding rate..."
ORACLE_PRICE=100000000  # 100 * 1e6

# Temporarily disable exit on error for this command
set +e
UPDATE_OUTPUT=$(./target/release/percolator \
    --keypair test-keypair.json \
    --network localnet \
    matcher update-funding \
    $SLAB \
    --oracle-price $ORACLE_PRICE 2>&1)
UPDATE_EXIT_CODE=$?
set -e

echo "$UPDATE_OUTPUT"
echo ""
echo "Exit code: $UPDATE_EXIT_CODE"

# Check if update succeeded
if echo "$UPDATE_OUTPUT" | grep -q "✓ Funding updated!"; then
    echo ""
    echo -e "${GREEN}========================================${NC}"
    echo -e "${GREEN}  ✓ ALL TESTS PASSED ✓${NC}"
    echo -e "${GREEN}========================================${NC}"
    echo ""
    echo -e "${BLUE}Summary:${NC}"
    echo -e "  Registry: $REGISTRY"
    echo -e "  Slab: $SLAB"
    echo -e "  Oracle Price: 100.0"
    echo -e "  UpdateFunding: ${GREEN}SUCCESS${NC}"
    echo ""
    exit 0
else
    echo ""
    echo -e "${RED}========================================${NC}"
    echo -e "${RED}  ✗ TEST FAILED ✗${NC}"
    echo -e "${RED}========================================${NC}"
    echo ""
    echo "Update funding output:"
    echo "$UPDATE_OUTPUT"
    exit 1
fi
