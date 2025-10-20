#!/usr/bin/env bash
#
# Deploy Percolator programs to Solana devnet
#

set -e

echo "🚀 Deploying Percolator programs to devnet..."
echo ""

# Check if solana CLI is installed
if ! command -v solana &> /dev/null; then
    echo "❌ Solana CLI not found!"
    echo "Install it with:"
    echo '  sh -c "$(curl -sSfL https://release.solana.com/stable/install)"'
    exit 1
fi

# Configure for devnet
echo "⚙️  Configuring Solana CLI for devnet..."
solana config set --url https://api.devnet.solana.com

# Check wallet balance
echo ""
echo "💰 Checking wallet balance..."
BALANCE=$(solana balance --lamports)
MIN_BALANCE=10000000000  # 10 SOL

if [ "$BALANCE" -lt "$MIN_BALANCE" ]; then
    echo "⚠️  Low balance detected: $BALANCE lamports"
    echo "📥 Requesting airdrop..."
    solana airdrop 2
    sleep 2
fi

# Build programs first
echo ""
echo "🔨 Building programs..."
./build-bpf.sh

# Deploy Router program
echo ""
echo "📤 Deploying Router program..."
ROUTER_PROGRAM_ID=$(solana program deploy target/deploy/percolator_router.so --output json | jq -r '.programId')
echo "✅ Router deployed: $ROUTER_PROGRAM_ID"

# Deploy Slab program
echo ""
echo "📤 Deploying Slab program..."
SLAB_PROGRAM_ID=$(solana program deploy target/deploy/percolator_slab.so --output json | jq -r '.programId')
echo "✅ Slab deployed: $SLAB_PROGRAM_ID"

# Save program IDs
echo ""
echo "💾 Saving program IDs to .env..."
cat > api/.env.devnet << EOF
# Percolator Devnet Program IDs
# Generated: $(date)

ROUTER_PROGRAM_ID=$ROUTER_PROGRAM_ID
SLAB_PROGRAM_ID=$SLAB_PROGRAM_ID
SOLANA_RPC_URL=https://api.devnet.solana.com
SOLANA_NETWORK=devnet
EOF

echo ""
echo "🎉 Deployment complete!"
echo ""
echo "Program IDs saved to api/.env.devnet"
echo "Verify on Solscan:"
echo "  Router: https://solscan.io/account/${ROUTER_PROGRAM_ID}?cluster=devnet"
echo "  Slab:   https://solscan.io/account/${SLAB_PROGRAM_ID}?cluster=devnet"
echo ""
echo "Next steps:"
echo "1. Copy api/.env.devnet to api/.env"
echo "2. Start API server: cd api && npm run dev"
echo "3. Test with: curl http://localhost:3000/api/health"

