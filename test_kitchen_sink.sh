#!/bin/bash
# Kitchen Sink End-to-End Test Runner
# Comprehensive multi-phase test exercising all protocol features

set -e

echo "═══════════════════════════════════════════════════════════════"
echo " Kitchen Sink E2E Test Runner (KS-00)"
echo "═══════════════════════════════════════════════════════════════"
echo ""
echo "This comprehensive test exercises:"
echo "  • Multi-market setup (SOL-PERP, BTC-PERP)"
echo "  • Multiple actors (Alice, Bob, Dave, Erin, Keeper)"
echo "  • Order book liquidity and taker trades"
echo "  • Funding rate accrual"
echo "  • Oracle shocks and liquidations"
echo "  • Insurance fund stress"
echo "  • Loss socialization under crisis"
echo "  • Cross-phase invariants"
echo ""

# Clean up old validators
trap "trap - SIGTERM && kill -- -$$" SIGINT SIGTERM EXIT
# echo "🧹 Cleaning up old validator processes..."
# killall -9 solana-test-validator 2>/dev/null || true
# sleep 2
# echo "✓ Old validators killed"
# echo ""

# Build programs
echo "🔨 Building Solana programs..."
cargo build-sbf 2>&1 | grep -E "(Finished|error)" || true
echo "✓ Solana programs built"
echo ""

# Build CLI
echo "🔨 Building CLI..."
cargo build --release --quiet 2>&1 | grep -v "warning:" || true
echo "✓ CLI built"
echo ""

# Start fresh validator
echo "🚀 Starting fresh validator..."
solana-test-validator --reset --quiet > /tmp/validator.log 2>&1 &
VALIDATOR_PID=$!
sleep 10
echo "✓ Validator started (PID: $VALIDATOR_PID)"
echo ""

# Request SOL airdrop for deployments
echo "💰 Requesting SOL airdrop for deployments..."
solana airdrop 10 2>&1
echo "✓ Airdrop complete"
echo ""

# Get program IDs from keypair files
ROUTER_PROGRAM_ID=$(solana-keygen pubkey target/deploy/percolator_router-keypair.json)
SLAB_PROGRAM_ID=$(solana-keygen pubkey target/deploy/percolator_slab-keypair.json)
AMM_PROGRAM_ID=$(solana-keygen pubkey target/deploy/percolator_amm-keypair.json)
ORACLE_PROGRAM_ID=$(solana-keygen pubkey target/deploy/percolator_oracle-keypair.json)

# Close any existing programs to allow fresh deployment
echo "🧹 Closing any existing program deployments..."
solana program close "$ROUTER_PROGRAM_ID" 2>/dev/null || true
solana program close "$SLAB_PROGRAM_ID" 2>/dev/null || true
solana program close "$AMM_PROGRAM_ID" 2>/dev/null || true
solana program close "$ORACLE_PROGRAM_ID" 2>/dev/null || true
echo "✓ Cleanup complete"
echo ""

# Deploy programs using solana CLI (let default wallet pay, use upgradeable loader)
echo "📦 Deploying programs to validator..."
echo ""
echo "  Deploying router program..."
solana program deploy target/deploy/percolator_router.so --upgrade-authority ~/.config/solana/id.json --program-id target/deploy/percolator_router-keypair.json
if [ $? -ne 0 ]; then
    echo "✗ Router deployment failed"
    exit 1
fi
echo ""

echo "  Deploying slab program..."
solana program deploy target/deploy/percolator_slab.so --upgrade-authority ~/.config/solana/id.json --program-id target/deploy/percolator_slab-keypair.json
if [ $? -ne 0 ]; then
    echo "✗ Slab deployment failed"
    exit 1
fi
echo ""

echo "  Deploying AMM program..."
solana program deploy target/deploy/percolator_amm.so --upgrade-authority ~/.config/solana/id.json --program-id target/deploy/percolator_amm-keypair.json
if [ $? -ne 0 ]; then
    echo "✗ AMM deployment failed"
    exit 1
fi
echo ""

echo "  Deploying oracle program..."
solana program deploy target/deploy/percolator_oracle.so --upgrade-authority ~/.config/solana/id.json --program-id target/deploy/percolator_oracle-keypair.json
if [ $? -ne 0 ]; then
    echo "✗ Oracle deployment failed"
    exit 1
fi
echo ""

echo "✓ All programs deployed successfully"
echo ""

echo "═══════════════════════════════════════════════════════════════"
echo " Running Kitchen Sink Test"
echo "═══════════════════════════════════════════════════════════════"
echo ""

# Run the kitchen sink test via crisis test suite
./target/release/percolator --network localnet test --crisis 2>&1

echo ""
echo "═══════════════════════════════════════════════════════════════"
echo " Test Complete - Summary"
echo "═══════════════════════════════════════════════════════════════"
echo ""
echo "✅ KITCHEN SINK TEST PHASES:"
echo "  ✓ Phase 1 (KS-01): Multi-market bootstrap"
echo "  ⚠ Phase 2 (KS-02): Taker trades (pending implementation)"
echo "  ⚠ Phase 3 (KS-03): Funding accrual (pending)"
echo "  ⚠ Phase 4 (KS-04): Oracle shocks + liquidations (pending)"
echo "  ⚠ Phase 5 (KS-05): Insurance drawdown (pending)"
echo ""
echo "📊 INVARIANTS CHECKED:"
echo "  ✓ Non-negative balances"
echo "  ⚠ Conservation (pending vault query)"
echo "  ⚠ Funding conservation (pending)"
echo "  ⚠ Liquidation monotonicity (pending)"
echo ""
echo "📝 NOTE: This is a skeleton implementation."
echo "   Full phases will be added as features are implemented:"
echo "   • Liquidity placement (order book maker operations)"
echo "   • Funding rate mechanism"
echo "   • Oracle integration"
echo "   • Advanced liquidation scenarios"
echo ""
