#!/usr/bin/env bash
#
# Simplified Funding Update Test
#
# This demonstrates the UpdateFunding CLI command against localnet.
# For a full E2E test with positions and PnL verification, see test_funding_e2e.sh

set -e

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

echo -e "${YELLOW}======================================${NC}"
echo -e "${YELLOW}  Funding Update CLI Test${NC}"
echo -e "${YELLOW}======================================${NC}"
echo ""

# Check if CLI binary exists
if [ ! -f "target/release/percolator" ]; then
    echo -e "${RED}ERROR: CLI binary not found${NC}"
    echo "Please run: cargo build --release -p percolator-cli"
    exit 1
fi

# Correct program IDs
ROUTER_ID="7NUzsomCpwX1MMVHSLDo8tmcCDpUTXiWb1SWa94BpANf"
SLAB_ID="CmJKuXjspb84yaaoWFSujVgzaXktCw4jwaxzdbRbrJ8g"
AMM_ID="C9PdrHtZfDe24iFpuwtv4FHd7mPUnq52feFiKFNYLFvy"

# Check if validator is running
if ! solana cluster-version --url http://127.0.0.1:8899 &>/dev/null; then
    echo -e "${YELLOW}Starting localnet validator...${NC}"
    mkdir -p test-ledger
    solana-test-validator \
        --bpf-program $ROUTER_ID target/deploy/percolator_router.so \
        --bpf-program $SLAB_ID target/deploy/percolator_slab.so \
        --bpf-program $AMM_ID target/deploy/percolator_amm.so \
        --reset \
        --quiet \
        &> test-ledger/validator.log &

    VALIDATOR_PID=$!
    echo "Validator PID: $VALIDATOR_PID"

    # Wait for validator to be ready
    for i in {1..30}; do
        if solana cluster-version --url http://127.0.0.1:8899 &>/dev/null; then
            echo -e "${GREEN}✓ Validator ready${NC}"
            break
        fi
        [ $i -eq 30 ] && { echo -e "${RED}✗ Validator failed to start${NC}"; exit 1; }
        sleep 1
    done
else
    echo -e "${GREEN}✓ Validator already running${NC}"
fi

# Configure solana to use localnet
solana config set --url http://127.0.0.1:8899 &>/dev/null

echo ""
echo -e "${YELLOW}Creating test keypair and slab...${NC}"
echo ""

# Create test keypair
solana-keygen new --no-passphrase --force --silent --outfile test-keypair.json
TEST_PUBKEY=$(solana-keygen pubkey test-keypair.json)
echo "Test pubkey: $TEST_PUBKEY"

# Airdrop SOL
solana airdrop 10 $TEST_PUBKEY --url http://127.0.0.1:8899 > /dev/null
echo "Airdropped 10 SOL"

# Initialize exchange
INIT_OUTPUT=$(./target/release/percolator \
    --keypair test-keypair.json \
    --network localnet \
    init --name "funding-test" 2>&1)

REGISTRY=$(echo "$INIT_OUTPUT" | grep "Registry Address:" | head -1 | awk '{print $3}')
echo "Registry: $REGISTRY"

# Create slab
CREATE_OUTPUT=$(./target/release/percolator \
    --keypair test-keypair.json \
    --network localnet \
    matcher create \
    $REGISTRY \
    "BTC-USD" \
    --tick-size 1000000 \
    --lot-size 1000000 2>&1)

TEST_SLAB=$(echo "$CREATE_OUTPUT" | grep "Slab Address:" | tail -1 | awk '{print $3}')
echo "Slab: $TEST_SLAB"

echo ""
echo -e "${YELLOW}Testing UpdateFunding command...${NC}"
echo ""

ORACLE_PRICE=100000000  # 100 * 1e6
echo "Oracle Price: $ORACLE_PRICE (= 100.0)"
echo ""

# Call update-funding command
./target/release/percolator \
    --keypair test-keypair.json \
    --network localnet \
    matcher update-funding \
    "$TEST_SLAB" \
    --oracle-price "$ORACLE_PRICE"

echo ""
echo -e "${GREEN}======================================${NC}"
echo -e "${GREEN}  Test Complete ✓${NC}"
echo -e "${GREEN}======================================${NC}"
echo ""
echo "UpdateFunding command verified successfully!"
echo "Slab: $TEST_SLAB"
echo ""

# Cleanup
if [ ! -z "$VALIDATOR_PID" ]; then
    echo "Cleaning up validator..."
    kill $VALIDATOR_PID 2>/dev/null || true
fi
rm -f test-keypair.json
rm -rf test-ledger
