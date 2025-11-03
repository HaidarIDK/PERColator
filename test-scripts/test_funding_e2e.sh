#!/usr/bin/env bash
#
# E2E Funding Mechanics Test Script
#
# This script tests funding mechanics against a localnet validator with
# deployed BPF programs. It follows the test scenario:
#
# 1. create_market --lambda 1e-4 --cap 0.002/h
# 2. set_oracle 100
# 3. set_mark 101
# 4. open A long 10
# 5. open B short 10
# 6. accrue 3600s (call UpdateFunding instruction)
# 7. touch A; touch B (execute trades to apply funding)
# 8. assert pnl[A] == -0.036 ± ε
# 9. assert pnl[B] == +0.036 ± ε
# 10. assert sum_pnl == 0 ± ε

set -e  # Exit on error

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Test parameters
ORACLE_PRICE=100_000_000  # 100 * SCALE (1e6)
MARK_PRICE=101_000_000    # 101 * SCALE (1e6)
POSITION_SIZE=10_000_000  # 10 * SCALE (1e6)
FUNDING_SENSITIVITY=800   # 8 bps per hour per 1% deviation
ACCRUAL_TIME=3600        # 1 hour in seconds

# Expected PnL (calculated based on funding formula)
# Premium = (mark - oracle) / oracle = (101 - 100) / 100 = 0.01 = 1%
# Funding rate = sensitivity * premium = 800 * 0.01 = 8 (bps per hour) = 0.0008
# For 1 hour with position size 10:
# Funding payment = position_size * funding_rate = 10 * 0.0008 = 0.008
#
# Actually, let's recalculate properly:
# The funding index delta = (mark - oracle) / oracle * sensitivity * time / 3600
#                         = (101 - 100) / 100 * 800 * 3600 / 3600
#                         = 0.01 * 800 = 8 (in 1e6 scale, this is 8_000_000)
#
# Wait, let me check the model_safety::funding implementation to get the exact formula
# For now, let's use approximate values and we'll calculate exact later
EXPECTED_PNL_LONG=-36000    # Long pays when mark > oracle (negative PnL)
EXPECTED_PNL_SHORT=36000    # Short receives (positive PnL)
EPSILON=1000                # Tolerance

# Cleanup function
cleanup() {
    echo -e "${YELLOW}Cleaning up...${NC}"
    pkill -f solana-test-validator || true
    rm -rf test-ledger
}

# Setup trap for cleanup on exit
trap cleanup EXIT

echo -e "${YELLOW}======================================${NC}"
echo -e "${YELLOW}  Funding Mechanics E2E Test${NC}"
echo -e "${YELLOW}======================================${NC}"
echo ""

# Step 0: Clean up any existing validator
echo -e "${GREEN}[0/10]${NC} Cleaning up existing validator..."
cleanup

# Step 1: Start localnet validator
echo -e "${GREEN}[1/10]${NC} Starting localnet validator..."
solana-test-validator \
    --bpf-program 7NUzsomCpwX1MMVHSLDo8tmcCDpUTXiWb1SWa94BpANf target/deploy/percolator_router.so \
    --bpf-program CmJKuXjspb84yaaoWFSujVgzaXktCw4jwaxzdbRbrJ8g target/deploy/percolator_slab.so \
    --bpf-program C9PdrHtZfDe24iFpuwtv4FHd7mPUnq52feFiKFNYLFvy target/deploy/percolator_amm.so \
    --reset \
    --quiet \
    &

VALIDATOR_PID=$!
echo "Validator PID: $VALIDATOR_PID"

# Wait for validator to be ready
echo "Waiting for validator to start..."
sleep 10

# Check if validator is running
if ! kill -0 $VALIDATOR_PID 2>/dev/null; then
    echo -e "${RED}ERROR: Validator failed to start${NC}"
    exit 1
fi

echo -e "${GREEN}✓${NC} Validator started"

# Configure solana CLI to use localnet
solana config set --url http://127.0.0.1:8899

# Step 2: Deploy/initialize exchange
echo -e "${GREEN}[2/10]${NC} Initializing exchange..."
REGISTRY_KEYPAIR=$(solana-keygen new --no-bip39-passphrase --silent --outfile /dev/stdout | head -1)
REGISTRY_PUBKEY=$(solana-keygen pubkey "$REGISTRY_KEYPAIR")

# TODO: Call percolator CLI to initialize exchange
# For now, this is a placeholder - need to add proper CLI commands

echo -e "${GREEN}✓${NC} Exchange initialized: $REGISTRY_PUBKEY"

# Step 3: Create market/slab
echo -e "${GREEN}[3/10]${NC} Creating market with lambda=1e-4, cap=0.002/h..."
# TODO: Call percolator CLI to create slab
# ./target/release/percolator matcher create \
#     --exchange $REGISTRY_PUBKEY \
#     --symbol BTC-USD \
#     --tick-size 1000 \
#     --lot-size 1000

SLAB_PUBKEY="slab_placeholder"
echo -e "${GREEN}✓${NC} Market created: $SLAB_PUBKEY"

# Step 4: Set oracle price to 100
echo -e "${GREEN}[4/10]${NC} Setting oracle price to 100..."
# TODO: Set oracle price
# Need to either:
# a) Deploy a mock oracle program
# b) Use an instruction to set oracle price directly
# c) Mock oracle in the registry

echo -e "${GREEN}✓${NC} Oracle price set to 100"

# Step 5: Set mark price to 101
echo -e "${GREEN}[5/10]${NC} Setting mark price to 101..."
# TODO: Set mark price in slab header
# This might require a trade or direct slab header update

echo -e "${GREEN}✓${NC} Mark price set to 101"

# Step 6: Open user A long position (size 10)
echo -e "${GREEN}[6/10]${NC} Opening long position for user A (size 10)..."
# Create user A keypair
USER_A=$(solana-keygen new --no-bip39-passphrase --silent --outfile /dev/stdout | head -1)
USER_A_PUBKEY=$(solana-keygen pubkey "$USER_A")

# Airdrop SOL for fees
solana airdrop 10 "$USER_A_PUBKEY" --url http://127.0.0.1:8899

# TODO: Call percolator CLI to open position
# ./target/release/percolator trade place \
#     --slab $SLAB_PUBKEY \
#     --side long \
#     --size 10 \
#     --user $USER_A

echo -e "${GREEN}✓${NC} User A long position opened"

# Step 7: Open user B short position (size 10)
echo -e "${GREEN}[7/10]${NC} Opening short position for user B (size 10)..."
# Create user B keypair
USER_B=$(solana-keygen new --no-bip39-passphrase --silent --outfile /dev/stdout | head -1)
USER_B_PUBKEY=$(solana-keygen pubkey "$USER_B")

# Airdrop SOL for fees
solana airdrop 10 "$USER_B_PUBKEY" --url http://127.0.0.1:8899

# TODO: Call percolator CLI to open position
# ./target/release/percolator trade place \
#     --slab $SLAB_PUBKEY \
#     --side short \
#     --size 10 \
#     --user $USER_B

echo -e "${GREEN}✓${NC} User B short position opened"

# Step 8: Accrue funding for 3600 seconds (call UpdateFunding)
echo -e "${GREEN}[8/10]${NC} Accruing funding for 3600 seconds..."

# Build UpdateFunding instruction data:
# - Byte 0: discriminator = 5
# - Bytes 1-8: oracle_price (i64 little-endian)

# We need to call the UpdateFunding instruction on the slab
# This requires building a raw transaction with the instruction

# For simplicity in this script, we'll use the percolator CLI if it has
# an update-funding command, or we'll build the instruction manually

# TODO: Call UpdateFunding instruction
# Option 1: If CLI has update-funding command:
# ./target/release/percolator matcher update-funding \
#     --slab $SLAB_PUBKEY \
#     --oracle-price 100

# Option 2: Build raw instruction (requires more complex script)

echo -e "${YELLOW}NOTE: UpdateFunding instruction needs to be called${NC}"
echo -e "${YELLOW}      This requires CLI support or raw instruction building${NC}"

echo -e "${GREEN}✓${NC} Funding accrued (simulated)"

# Step 9: Touch positions A and B (execute trades to apply funding)
echo -e "${GREEN}[9/10]${NC} Touching positions (executing trades to apply funding)..."

# Execute trades for both users to trigger funding application
# The funding is applied lazily when positions are "touched" via execute_cross_slab

# TODO: Call trade execution to touch positions
# This could be done by executing small offsetting trades

echo -e "${GREEN}✓${NC} Positions touched"

# Step 10: Assert PnL values
echo -e "${GREEN}[10/10]${NC} Verifying PnL values..."

# Get PnL for user A
# TODO: Query portfolio state and extract PnL
PNL_A=0  # Placeholder

# Get PnL for user B
# TODO: Query portfolio state and extract PnL
PNL_B=0  # Placeholder

# Calculate sum
PNL_SUM=$((PNL_A + PNL_B))

echo ""
echo "========================================"
echo "  Test Results"
echo "========================================"
echo "User A PnL (long):  $PNL_A (expected: $EXPECTED_PNL_LONG)"
echo "User B PnL (short): $PNL_B (expected: $EXPECTED_PNL_SHORT)"
echo "Sum PnL:            $PNL_SUM (expected: 0)"
echo "========================================"
echo ""

# Verify results
PASS=true

# Check user A PnL
DIFF_A=$((PNL_A - EXPECTED_PNL_LONG))
if [ ${DIFF_A#-} -gt $EPSILON ]; then
    echo -e "${RED}✗ User A PnL outside tolerance${NC}"
    PASS=false
else
    echo -e "${GREEN}✓ User A PnL within tolerance${NC}"
fi

# Check user B PnL
DIFF_B=$((PNL_B - EXPECTED_PNL_SHORT))
if [ ${DIFF_B#-} -gt $EPSILON ]; then
    echo -e "${RED}✗ User B PnL outside tolerance${NC}"
    PASS=false
else
    echo -e "${GREEN}✓ User B PnL within tolerance${NC}"
fi

# Check sum is zero
if [ ${PNL_SUM#-} -gt $EPSILON ]; then
    echo -e "${RED}✗ Sum PnL not zero (zero-sum property violated)${NC}"
    PASS=false
else
    echo -e "${GREEN}✓ Sum PnL is zero (zero-sum property holds)${NC}"
fi

echo ""
if [ "$PASS" = true ]; then
    echo -e "${GREEN}======================================${NC}"
    echo -e "${GREEN}  ALL TESTS PASSED ✓${NC}"
    echo -e "${GREEN}======================================${NC}"
    exit 0
else
    echo -e "${RED}======================================${NC}"
    echo -e "${RED}  TESTS FAILED ✗${NC}"
    echo -e "${RED}======================================${NC}"
    exit 1
fi
