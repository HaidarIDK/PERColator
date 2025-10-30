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

# Check if validator is running
if ! solana cluster-version --url http://127.0.0.1:8899 &>/dev/null; then
    echo -e "${YELLOW}Starting localnet validator...${NC}"
    solana-test-validator \
        --bpf-program Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS target/deploy/percolator_router.so \
        --bpf-program 7gUX8cKNEgSZ9Fg6X5BGDTKaK4qsaZLqvMadGkePmHjH target/deploy/percolator_slab.so \
        --reset \
        --quiet \
        &

    VALIDATOR_PID=$!
    echo "Validator PID: $VALIDATOR_PID"
    sleep 10
else
    echo -e "${GREEN}✓ Validator already running${NC}"
fi

# Configure solana to use localnet
solana config set --url http://127.0.0.1:8899 &>/dev/null

echo ""
echo -e "${YELLOW}Testing UpdateFunding command...${NC}"
echo ""

# Create a test slab pubkey (would be real slab in full E2E)
# For this demo, we'll use the slab program ID itself as a placeholder
TEST_SLAB="7gUX8cKNEgSZ9Fg6X5BGDTKaK4qsaZLqvMadGkePmHjH"
ORACLE_PRICE=100000000  # 100 * 1e6

echo "Slab: $TEST_SLAB"
echo "Oracle Price: $ORACLE_PRICE (= 100.0)"
echo ""

# Call update-funding command
./target/release/percolator matcher update-funding \
    "$TEST_SLAB" \
    --oracle-price "$ORACLE_PRICE"

echo ""
echo -e "${GREEN}======================================${NC}"
echo -e "${GREEN}  Test Complete ✓${NC}"
echo -e "${GREEN}======================================${NC}"
echo ""
echo -e "${YELLOW}Next Steps for Full E2E Test:${NC}"
echo "1. Create actual slab/market with init command"
echo "2. Set up oracle with price feed"
echo "3. Open long/short positions via router"
echo "4. Call update-funding after time delay"
echo "5. Execute trades to trigger funding application"
echo "6. Query portfolio PnL to verify funding payments"
echo ""
echo "See test_funding_e2e.sh for complete test plan"
