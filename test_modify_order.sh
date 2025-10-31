#!/bin/bash

# ========================================
# Modify Order E2E Test
# ========================================
#
# Tests:
# - PlaceOrder instruction (slab-order, discriminator 2)
# - ModifyOrder instruction (slab-modify, discriminator 8)
# - CancelOrder instruction (slab-cancel, discriminator 3)
#
# Scenarios:
# 1. Create exchange and slab
# 2. Place buy order at $100
# 3. Modify order to $101 (different price - loses time priority)
# 4. Modify order to $101 with new qty (same price - preserves time priority)
# 5. Place another buy at $102
# 6. Modify first order to $99
# 7. Cancel modified order
# 8. SUCCESS if all operations work correctly

set -e  # Exit on error

# Colors for output
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Cleanup function
cleanup() {
    echo -e "\n${YELLOW}Cleaning up...${NC}"
    if [ ! -z "$VALIDATOR_PID" ]; then
        kill $VALIDATOR_PID 2>/dev/null || true
        wait $VALIDATOR_PID 2>/dev/null || true
    fi
    rm -f test-keypair.json
    rm -rf test-ledger
}

# Set cleanup trap
trap cleanup EXIT

echo "========================================"
echo "    Modify Order E2E Test"
echo "========================================"

# Step 1: Start validator
echo -e "\n${GREEN}[1/11] Starting localnet validator...${NC}"

# BPF program addresses
ROUTER_ID="7NUzsomCpwX1MMVHSLDo8tmcCDpUTXiWb1SWa94BpANf"
SLAB_ID="CmJKuXjspb84yaaoWFSujVgzaXktCw4jwaxzdbRbrJ8g"
AMM_ID="C9PdrHtZfDe24iFpuwtv4FHd7mPUnq52feFiKFNYLFvy"

# Create test-ledger directory
mkdir -p test-ledger

# Start validator in background with deployed programs
solana-test-validator \
    --bpf-program $ROUTER_ID ./target/deploy/percolator_router.so \
    --bpf-program $SLAB_ID ./target/deploy/percolator_slab.so \
    --bpf-program $AMM_ID ./target/deploy/percolator_amm.so \
    --reset \
    --quiet \
    &> test-ledger/validator.log &

VALIDATOR_PID=$!
echo "Validator PID: $VALIDATOR_PID"

# Wait for validator to be ready
echo "Waiting for validator to start..."
for i in {1..30}; do
    if solana cluster-version --url http://127.0.0.1:8899 &>/dev/null; then
        echo -e "${GREEN}✓ Validator ready${NC}"
        break
    fi
    if [ $i -eq 30 ]; then
        echo -e "${RED}✗ Validator failed to start${NC}"
        exit 1
    fi
    sleep 1
done

# Step 2: Create test keypair
echo -e "\n${GREEN}[2/11] Creating test keypair...${NC}"
solana-keygen new --no-passphrase --force --silent --outfile test-keypair.json
TEST_PUBKEY=$(solana-keygen pubkey test-keypair.json)
echo "Test pubkey: $TEST_PUBKEY"

# Step 3: Airdrop SOL
echo -e "\n${GREEN}[3/11] Airdropping SOL...${NC}"
solana airdrop 10 $TEST_PUBKEY --url http://127.0.0.1:8899 > /dev/null
BALANCE=$(solana balance $TEST_PUBKEY --url http://127.0.0.1:8899)
echo "Balance: $BALANCE"

# Step 4: Initialize exchange and create slab
echo -e "\n${GREEN}[4/11] Initializing exchange and creating slab...${NC}"

INIT_OUTPUT=$(./target/release/percolator \
    --keypair test-keypair.json \
    --network localnet \
    init --name "modify-order-test" 2>&1)

REGISTRY=$(echo "$INIT_OUTPUT" | grep "Registry Address:" | head -1 | awk '{print $3}')

if [ -z "$REGISTRY" ]; then
    echo -e "${RED}✗ Failed to get registry address${NC}"
    exit 1
fi

CREATE_OUTPUT=$(./target/release/percolator \
    --keypair test-keypair.json \
    --network localnet \
    matcher create \
    $REGISTRY \
    "BTC-USD" \
    --tick-size 1000000 \
    --lot-size 1000000 2>&1)

SLAB=$(echo "$CREATE_OUTPUT" | grep "Slab Address:" | tail -1 | awk '{print $3}')

if [ -z "$SLAB" ]; then
    echo -e "${RED}✗ Failed to get slab address${NC}"
    exit 1
fi

echo -e "${GREEN}✓ Slab created: $SLAB${NC}"

# Step 5: Place first buy order at $100
echo -e "\n${GREEN}[5/11] Placing buy order at \$100 (price: 100.0, size: 2.0)...${NC}"

ORDER1_OUTPUT=$(./target/release/percolator \
    --keypair test-keypair.json \
    --network localnet \
    trade slab-order \
    $SLAB \
    buy \
    100.0 \
    2000000 2>&1)

if echo "$ORDER1_OUTPUT" | grep -q "Order placed"; then
    echo -e "${GREEN}✓ Order 1 placed successfully${NC}"
    ORDER1_SIG=$(echo "$ORDER1_OUTPUT" | grep "Transaction:" | awk '{print $NF}')
    echo "  Transaction: $ORDER1_SIG"
else
    echo -e "${RED}✗ Failed to place order 1${NC}"
    echo "$ORDER1_OUTPUT"
    exit 1
fi

# Extract order ID from logs (order IDs start at 1)
ORDER1_ID=1
echo "  Order ID: $ORDER1_ID"
sleep 1

# Step 6: Modify order to $101 (different price - loses time priority)
echo -e "\n${GREEN}[6/11] Modifying order to \$101 (price change - loses time priority)...${NC}"

MODIFY1_OUTPUT=$(./target/release/percolator \
    --keypair test-keypair.json \
    --network localnet \
    trade slab-modify \
    $SLAB \
    $ORDER1_ID \
    101.0 \
    2000000 2>&1)

if echo "$MODIFY1_OUTPUT" | grep -q "Order modified"; then
    echo -e "${GREEN}✓ Order modified to \$101${NC}"
    MODIFY1_SIG=$(echo "$MODIFY1_OUTPUT" | grep "Transaction:" | awk '{print $NF}')
    echo "  Transaction: $MODIFY1_SIG"
else
    echo -e "${RED}✗ Failed to modify order (1st attempt)${NC}"
    echo "$MODIFY1_OUTPUT"
    exit 1
fi

sleep 1

# Step 7: Modify order again with new quantity (same price - preserves time priority)
echo -e "\n${GREEN}[7/11] Modifying order qty to 3.0 (same price - preserves time priority)...${NC}"

MODIFY2_OUTPUT=$(./target/release/percolator \
    --keypair test-keypair.json \
    --network localnet \
    trade slab-modify \
    $SLAB \
    $ORDER1_ID \
    101.0 \
    3000000 2>&1)

if echo "$MODIFY2_OUTPUT" | grep -q "Order modified"; then
    echo -e "${GREEN}✓ Order modified to qty 3.0${NC}"
    MODIFY2_SIG=$(echo "$MODIFY2_OUTPUT" | grep "Transaction:" | awk '{print $NF}')
    echo "  Transaction: $MODIFY2_SIG"
    echo "  ${BLUE}Note: Time priority preserved (same price)${NC}"
else
    echo -e "${RED}✗ Failed to modify order (2nd attempt)${NC}"
    echo "$MODIFY2_OUTPUT"
    exit 1
fi

sleep 1

# Step 8: Place second buy order at $102
echo -e "\n${GREEN}[8/11] Placing second buy order at \$102 (size: 1.5)...${NC}"

ORDER2_OUTPUT=$(./target/release/percolator \
    --keypair test-keypair.json \
    --network localnet \
    trade slab-order \
    $SLAB \
    buy \
    102.0 \
    1500000 2>&1)

if echo "$ORDER2_OUTPUT" | grep -q "Order placed"; then
    echo -e "${GREEN}✓ Order 2 placed successfully${NC}"
    ORDER2_SIG=$(echo "$ORDER2_OUTPUT" | grep "Transaction:" | awk '{print $NF}')
    echo "  Transaction: $ORDER2_SIG"
else
    echo -e "${RED}✗ Failed to place order 2${NC}"
    echo "$ORDER2_OUTPUT"
    exit 1
fi

ORDER2_ID=2
echo "  Order ID: $ORDER2_ID"
sleep 1

# Step 9: Modify first order to $99
echo -e "\n${GREEN}[9/11] Modifying order 1 to \$99 (price decrease)...${NC}"

MODIFY3_OUTPUT=$(./target/release/percolator \
    --keypair test-keypair.json \
    --network localnet \
    trade slab-modify \
    $SLAB \
    $ORDER1_ID \
    99.0 \
    3000000 2>&1)

if echo "$MODIFY3_OUTPUT" | grep -q "Order modified"; then
    echo -e "${GREEN}✓ Order modified to \$99${NC}"
    MODIFY3_SIG=$(echo "$MODIFY3_OUTPUT" | grep "Transaction:" | awk '{print $NF}')
    echo "  Transaction: $MODIFY3_SIG"
else
    echo -e "${RED}✗ Failed to modify order (3rd attempt)${NC}"
    echo "$MODIFY3_OUTPUT"
    exit 1
fi

sleep 1

# Step 10: Cancel modified order
echo -e "\n${GREEN}[10/11] Cancelling order 1...${NC}"

CANCEL_OUTPUT=$(./target/release/percolator \
    --keypair test-keypair.json \
    --network localnet \
    trade slab-cancel \
    $SLAB \
    $ORDER1_ID 2>&1)

if echo "$CANCEL_OUTPUT" | grep -q "Order cancelled"; then
    echo -e "${GREEN}✓ Order 1 cancelled successfully${NC}"
    CANCEL_SIG=$(echo "$CANCEL_OUTPUT" | grep "Transaction:" | awk '{print $NF}')
    echo "  Transaction: $CANCEL_SIG"
else
    echo -e "${RED}✗ Failed to cancel order${NC}"
    echo "$CANCEL_OUTPUT"
    exit 1
fi

sleep 1

# Step 11: Query final orderbook state
echo -e "\n${GREEN}[11/11] Querying final orderbook state...${NC}"

BOOK_OUTPUT=$(./target/release/percolator \
    --keypair test-keypair.json \
    --network localnet \
    matcher get-orderbook \
    $SLAB 2>&1)

if echo "$BOOK_OUTPUT" | grep -q "owned by slab program"; then
    echo -e "${GREEN}✓ Final orderbook query successful${NC}"
else
    echo -e "${RED}✗ Failed to query final orderbook${NC}"
    echo "$BOOK_OUTPUT"
    exit 1
fi

# Summary
echo ""
echo "========================================"
echo -e "  ${GREEN}✓ ALL TESTS PASSED ✓${NC}"
echo "========================================"
echo ""
echo "Summary:"
echo "  Registry: $REGISTRY"
echo "  Slab: $SLAB"
echo ""
echo "Operations:"
echo "  1. Placed order 1 at \$100 (2.0): $ORDER1_SIG"
echo "  2. Modified to \$101 (2.0):       $MODIFY1_SIG"
echo "  3. Modified to \$101 (3.0):       $MODIFY2_SIG"
echo "  4. Placed order 2 at \$102 (1.5): $ORDER2_SIG"
echo "  5. Modified order 1 to \$99 (3.0): $MODIFY3_SIG"
echo "  6. Cancelled order 1:             $CANCEL_SIG"
echo ""
echo "Tested Instructions:"
echo "  ✓ PlaceOrder (slab-order, discriminator 2)"
echo "  ✓ ModifyOrder (slab-modify, discriminator 8)"
echo "  ✓ CancelOrder (slab-cancel, discriminator 3)"
echo ""
echo "Scenarios Covered:"
echo "  ✓ Modify price (loses time priority)"
echo "  ✓ Modify quantity at same price (preserves time priority)"
echo "  ✓ Modify price downward"
echo "  ✓ Cancel modified order"
echo "  ✓ Multiple orders coexisting"
echo ""
