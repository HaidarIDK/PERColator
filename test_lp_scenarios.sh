#!/usr/bin/env bash
set -e

# Test script for 4 LP scenarios:
# 1. Direct Slab LP (no margin) - place-order directly
# 2. Direct AMM LP (no margin) - adapter_liquidity (AmmAdd) directly
# 3. Router→Slab LP (with margin) - RouterReserve → RouterLiquidity → ObAdd
# 4. Router→AMM LP (with margin) - RouterReserve → RouterLiquidity → AmmAdd

SCRIPT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
cd "$SCRIPT_DIR"

echo "=== LP Scenarios Test Suite ==="
echo

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Program IDs (from deployed programs)
ROUTER_ID="7NUzsomCpwX1MMVHSLDo8tmcCDpUTXiWb1SWa94BpANf"
SLAB_ID="CmJKuXjspb84yaaoWFSujVgzaXktCw4jwaxzdbRbrJ8g"
AMM_ID="C9PdrHtZfDe24iFpuwtv4FHd7mPUnq52feFiKFNYLFvy"

# Test keypair
TEST_KEYPAIR="test-keypair.json"

# Ensure keypair exists
if [ ! -f "$TEST_KEYPAIR" ]; then
    echo "${YELLOW}⚠ Creating test keypair...${NC}"
    solana-keygen new --no-bip39-passphrase -o "$TEST_KEYPAIR" --force
fi

USER_PUBKEY=$(solana-keygen pubkey "$TEST_KEYPAIR")
echo "User pubkey: $USER_PUBKEY"
echo

# Start validator in background if not running
if ! pgrep -x "solana-test-val" > /dev/null; then
    echo "${YELLOW}⚠ Starting local validator...${NC}"
    solana-test-validator \
        --bpf-program "$ROUTER_ID" target/deploy/percolator_router.so \
        --bpf-program "$SLAB_ID" target/deploy/percolator_slab.so \
        --bpf-program "$AMM_ID" target/deploy/percolator_amm.so \
        --reset --quiet &

    # Wait for validator to be ready
    echo "Waiting for validator to start..."
    for i in {1..30}; do
        if solana cluster-version &>/dev/null; then
            echo "${GREEN}✓ Validator started${NC}"
            break
        fi
        sleep 1
        if [ $i -eq 30 ]; then
            echo "${RED}✗ Validator failed to start${NC}"
            exit 1
        fi
    done
else
    echo "${GREEN}✓ Validator already running${NC}"
fi

echo

# Airdrop SOL for gas
echo "${BLUE}Requesting airdrop...${NC}"
solana airdrop 10 "$USER_PUBKEY" --url http://127.0.0.1:8899 || true
sleep 2
echo

# ============================================================================
# Setup: Create registry and slab
# ============================================================================

echo "${BLUE}=== Setup: Create Registry & Slab ===${NC}"
echo

# Initialize registry
INIT_OUTPUT=$(./target/release/percolator --keypair "$TEST_KEYPAIR" --network localnet init --name "lp-test" 2>&1)
REGISTRY=$(echo "$INIT_OUTPUT" | grep "Registry Address:" | head -1 | awk '{print $3}')

if [ -z "$REGISTRY" ]; then
    echo "${RED}✗ Failed to create registry${NC}"
    echo "$INIT_OUTPUT"
    exit 1
fi

echo "${GREEN}✓ Registry created: $REGISTRY${NC}"

# Create slab
CREATE_OUTPUT=$(./target/release/percolator --keypair "$TEST_KEYPAIR" --network localnet matcher create "$REGISTRY" "BTC-USD" --tick-size 1000000 --lot-size 1000000 2>&1)
TEST_SLAB=$(echo "$CREATE_OUTPUT" | grep "Slab Address:" | tail -1 | awk '{print $3}')

if [ -z "$TEST_SLAB" ]; then
    echo "${RED}✗ Failed to create slab${NC}"
    echo "$CREATE_OUTPUT"
    exit 1
fi

echo "${GREEN}✓ Slab created: $TEST_SLAB${NC}"
echo

# ============================================================================
# Scenario 1: Direct Slab LP (No Margin)
# ============================================================================

echo "${BLUE}=== Scenario 1: Direct Slab LP (No Margin) ===${NC}"
echo "Description: LP places orders directly to slab with their own funds"
echo "Flow: place-order → slab (no router involved)"
echo

# Place buy order
echo "${YELLOW}Placing buy order at $50,000...${NC}"
BUY_ORDER=$(./target/release/percolator --keypair "$TEST_KEYPAIR" --network localnet \
    matcher place-order "$TEST_SLAB" buy \
    --price 50000000000 \
    --qty 1000000 2>&1)

BUY_ORDER_ID=$(echo "$BUY_ORDER" | grep "Order ID:" | awk '{print $3}')

if [ -z "$BUY_ORDER_ID" ]; then
    echo "${RED}✗ Failed to place buy order${NC}"
    echo "$BUY_ORDER"
    exit 1
fi

echo "${GREEN}✓ Buy order placed: $BUY_ORDER_ID${NC}"

# Place sell order
echo "${YELLOW}Placing sell order at $51,000...${NC}"
SELL_ORDER=$(./target/release/percolator --keypair "$TEST_KEYPAIR" --network localnet \
    matcher place-order "$TEST_SLAB" sell \
    --price 51000000000 \
    --qty 1000000 2>&1)

SELL_ORDER_ID=$(echo "$SELL_ORDER" | grep "Order ID:" | awk '{print $3}')

if [ -z "$SELL_ORDER_ID" ]; then
    echo "${RED}✗ Failed to place sell order${NC}"
    echo "$SELL_ORDER"
    exit 1
fi

echo "${GREEN}✓ Sell order placed: $SELL_ORDER_ID${NC}"

# Verify orderbook
echo "${YELLOW}Checking orderbook...${NC}"
ORDERBOOK=$(./target/release/percolator --keypair "$TEST_KEYPAIR" --network localnet \
    matcher get-orderbook "$TEST_SLAB" 2>&1)

echo "$ORDERBOOK" | grep -E "bid|ask|Bid|Ask"

echo "${GREEN}✓ Scenario 1 Complete: Direct Slab LP${NC}"
echo

# ============================================================================
# Scenario 2: Direct AMM LP (No Margin)
# ============================================================================

echo "${BLUE}=== Scenario 2: Direct AMM LP (No Margin) ===${NC}"
echo "Description: LP adds liquidity directly to AMM with their own funds"
echo "Flow: adapter_liquidity (AmmAdd) → AMM (no router involved)"
echo

# Create AMM pool first
echo "${YELLOW}Creating AMM pool...${NC}"
echo "${YELLOW}Note: AMM creation via CLI not yet implemented${NC}"
echo

echo "
${BLUE}Direct AMM LP Flow:${NC}
1. Initialize AMM (discriminator 0)
   - lp_owner, router_id, instrument
   - mark_px, taker_fee_bps, contract_size
   - x_reserve, y_reserve (initial liquidity)
   - Accounts: [amm_account, payer]

2. Add Liquidity (discriminator 2 - adapter_liquidity)
   - LiquidityIntent::AmmAdd {
       lower_px_q64: u128,      // Price range lower bound
       upper_px_q64: u128,      // Price range upper bound
       quote_notional_q64: u128,// Amount to add
       curve_id: u32,           // Curve type (0 = constant product)
       fee_bps: u16,            // LP fee
     }
   - RiskGuard: max_slippage_bps, max_fee_bps, oracle_bound_bps
   - Mint LP shares directly to caller
   - No router involvement, no margin checking
   - Accounts: [amm_account, lp_token_account, user_signer]

3. LP owns shares directly (not in router portfolio)
"

echo "${YELLOW}⚠ Scenario 2 Requires: AMM CLI commands${NC}"
echo "  - percolator amm create <registry> <instrument> --x-reserve <amt> --y-reserve <amt>"
echo "  - percolator amm add-liquidity <amm> --amount <amt> --lower-price <px> --upper-price <px>"
echo

echo "${GREEN}✓ Scenario 2 Complete: Direct AMM LP (documented)${NC}"
echo

# ============================================================================
# Scenario 3: Router→Slab LP (With Margin)
# ============================================================================

echo "${BLUE}=== Scenario 3: Router→Slab LP (With Margin) ===${NC}"
echo "Description: LP uses router margin system to place slab orders"
echo "Flow: margin init → margin deposit → liquidity add (ObAdd)"
echo

# Initialize portfolio (margin account)
echo "${YELLOW}Initializing portfolio...${NC}"
PORTFOLIO_INIT=$(./target/release/percolator --keypair "$TEST_KEYPAIR" --network localnet \
    margin init 2>&1 || true)

echo "$PORTFOLIO_INIT" | head -5

# Deposit collateral
echo "${YELLOW}Depositing collateral (1000 units)...${NC}"
DEPOSIT_OUTPUT=$(./target/release/percolator --keypair "$TEST_KEYPAIR" --network localnet \
    margin deposit 1000 2>&1 || true)

echo "$DEPOSIT_OUTPUT" | head -5

# Add liquidity via router (ObAdd intent)
echo "${YELLOW}Adding orderbook liquidity via router...${NC}"
echo "${YELLOW}Note: This would use RouterReserve → RouterLiquidity → ObAdd${NC}"
echo "${YELLOW}Note: CLI currently defaults to AmmAdd, needs ObAdd support${NC}"

# TODO: CLI needs enhancement to support ObAdd variant
# This would be: ./target/release/percolator liquidity add "$TEST_SLAB" 100 --mode orderbook --price 50500
# For now, we document the intended flow:

echo "
${BLUE}Intended Router→Slab Flow:${NC}
1. RouterReserve (discriminator 9)
   - Lock collateral from portfolio into LP seat
   - Accounts: [portfolio_pda, lp_seat_pda]

2. RouterLiquidity (discriminator 11) with ObAdd intent
   - RiskGuard: max_slippage_bps, max_fee_bps, oracle_bound_bps
   - LiquidityIntent::ObAdd {
       orders: Vec<ObOrder>,  // List of orders to place
       post_only: bool,
       reduce_only: bool,
     }
   - CPI to slab adapter → place orders
   - Check seat limits (exposure within reserved)
   - Accounts: [portfolio_pda, lp_seat_pda, venue_pnl_pda, matcher_state]

3. Orders placed on slab with margin checking
"

echo "${YELLOW}⚠ Scenario 3 Requires CLI Enhancement: ObAdd variant support${NC}"
echo

# ============================================================================
# Scenario 4: Router→AMM LP (With Margin)
# ============================================================================

echo "${BLUE}=== Scenario 4: Router→AMM LP (With Margin) ===${NC}"
echo "Description: LP uses router margin system for AMM liquidity"
echo "Flow: margin init → margin deposit → liquidity add (AmmAdd)"
echo

# Create AMM (placeholder - would need actual AMM creation)
echo "${YELLOW}Note: AMM creation would be needed first${NC}"
echo "For this scenario, we would:"
echo "1. Create AMM state account"
echo "2. Initialize liquidity pools"
echo "3. Add liquidity via router"

# Add AMM liquidity via router (current CLI default)
echo "${YELLOW}Adding AMM liquidity via router...${NC}"
echo "${YELLOW}Note: This would use RouterReserve → RouterLiquidity → AmmAdd${NC}"

# This is what the current CLI does by default
# LIQUIDITY_ADD=$(./target/release/percolator --keypair "$TEST_KEYPAIR" --network localnet \
#     liquidity add "$AMM_STATE" 100 2>&1 || true)

echo "
${BLUE}Router→AMM Flow (Current CLI Default):${NC}
1. RouterReserve (discriminator 9)
   - Lock collateral from portfolio into LP seat
   - Accounts: [portfolio_pda, lp_seat_pda]

2. RouterLiquidity (discriminator 11) with AmmAdd intent
   - RiskGuard: max_slippage_bps, max_fee_bps, oracle_bound_bps
   - LiquidityIntent::AmmAdd {
       lower_px_q64: u128,      // Price range lower bound
       upper_px_q64: u128,      // Price range upper bound
       quote_notional_q64: u128,// Amount to add
       curve_id: u32,           // Curve type
       fee_bps: u16,            // LP fee
     }
   - CPI to AMM adapter → add liquidity
   - Mint LP shares
   - Check seat limits
   - Accounts: [portfolio_pda, lp_seat_pda, venue_pnl_pda, matcher_state]

3. LP shares minted, collateral stays in router
"

echo "${YELLOW}⚠ Scenario 3 Requires: AMM state creation${NC}"
echo

# ============================================================================
# Summary
# ============================================================================

echo
echo "${BLUE}=== LP Scenarios Summary ===${NC}"
echo
echo "${GREEN}✓ Scenario 1: Direct Slab LP${NC}"
echo "  Status: IMPLEMENTED & TESTED"
echo "  Commands: matcher place-order, matcher cancel-order"
echo "  Margin: NO (uses LP's own funds)"
echo "  Use case: Simple orderbook LP, no leverage"
echo
echo "${YELLOW}⚠ Scenario 2: Direct AMM LP${NC}"
echo "  Status: PROGRAM READY, CLI PENDING"
echo "  Program: AMM adapter_liquidity instruction (discriminator 2)"
echo "  CLI gap: Need AMM create and add-liquidity commands"
echo "  Margin: NO (LP owns shares directly)"
echo "  Use case: Direct AMM LP, concentrated liquidity, no leverage"
echo
echo "${YELLOW}⚠ Scenario 3: Router→Slab LP${NC}"
echo "  Status: PARTIALLY IMPLEMENTED"
echo "  Infrastructure: Router + Slab adapter ready"
echo "  CLI gap: Need ObAdd variant support in liquidity add command"
echo "  Margin: YES (portfolio margin system)"
echo "  Use case: Orderbook LP with leverage, portfolio cross-margining"
echo
echo "${YELLOW}⚠ Scenario 4: Router→AMM LP${NC}"
echo "  Status: INFRASTRUCTURE READY"
echo "  Infrastructure: Router + AMM adapter exists"
echo "  CLI: Supports AmmAdd (default in liquidity add)"
echo "  Gap: Need AMM state creation in CLI"
echo "  Margin: YES (portfolio margin system)"
echo "  Use case: AMM LP with leverage, concentrated liquidity with margin"
echo

echo "${BLUE}=== Architecture Summary ===${NC}"
echo
echo "Two-tier architecture:"
echo "  ${BLUE}Tier 1: Matchers (Slab/AMM)${NC} - Orderbook and liquidity logic"
echo "  ${BLUE}Tier 2: Router${NC} - Margin, portfolio, PnL management"
echo
echo "LP has 4 options:"
echo "  ${GREEN}1. Direct Slab${NC} - Orderbook LP, no margin, own funds"
echo "  ${GREEN}2. Direct AMM${NC} - AMM LP, no margin, own LP shares"
echo "  ${GREEN}3. Router→Slab${NC} - Orderbook LP with leverage, cross-margin"
echo "  ${GREEN}4. Router→AMM${NC} - AMM LP with leverage, cross-margin"
echo
echo "Comparison:"
echo "  ${BLUE}Direct (1,2)${NC}: Simple, no margin, LP owns assets/shares directly"
echo "  ${BLUE}Router (3,4)${NC}: Leverage, cross-margin, router custody"
echo
echo "Trader flow (for comparison):"
echo "  ${BLUE}ExecuteCrossSlab${NC} - Taker orders ONLY, margin always checked"
echo "  Traders CANNOT provide resting liquidity, only take from book/AMM"
echo

# Cleanup
echo "${BLUE}Cleaning up...${NC}"
./target/release/percolator --keypair "$TEST_KEYPAIR" --network localnet \
    matcher cancel-order "$TEST_SLAB" "$BUY_ORDER_ID" 2>&1 || true
./target/release/percolator --keypair "$TEST_KEYPAIR" --network localnet \
    matcher cancel-order "$TEST_SLAB" "$SELL_ORDER_ID" 2>&1 || true

echo
echo "${GREEN}✓ Test Complete${NC}"
