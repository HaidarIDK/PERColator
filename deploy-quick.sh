#!/bin/bash
# Quick Deployment Script for Percolator DEX (Linux/WSL)

echo "🚀 Percolator DEX Deployment Script (ULTRA-CHEAP Mode!)"
echo "========================================================"
echo "Slab Size: ~60 KB | Rent: ~0.5 SOL | Savings: 72.5 SOL vs 10MB!"
echo ""

# Check prerequisites
echo "Checking prerequisites..."

if ! command -v solana &> /dev/null; then
    echo "❌ Solana CLI not found!"
    exit 1
fi

echo "✅ Solana CLI: $(solana --version)"

if ! command -v cargo &> /dev/null; then
    echo "❌ Cargo not found!"
    exit 1
fi

echo "✅ Cargo: $(cargo --version)"

# Check network
echo "📡 Network: $(solana config get | grep 'RPC URL')"

# Check balance
BALANCE=$(solana balance 2>&1)
echo "💰 Balance: $BALANCE"

if [[ $BALANCE =~ ^[0-9]+\.[0-9]+ ]]; then
    BAL_NUM=$(echo $BALANCE | awk '{print $1}')
    if (( $(echo "$BAL_NUM < 30" | bc -l) )); then
        echo "⚠️  Warning: You have less than 30 SOL."
        echo "Use web faucets:"
        echo "  - https://faucet.solana.com"
        echo "  - https://solfaucet.com"
        echo ""
        read -p "Continue anyway? (y/n) " -n 1 -r
        echo
        if [[ ! $REPLY =~ ^[Yy]$ ]]; then
            exit 0
        fi
    fi
fi

echo ""

# Build programs
echo "🔨 Building programs..."
echo ""

echo "Building Router..."
cargo build-sbf --manifest-path programs/router/Cargo.toml
if [ $? -ne 0 ]; then
    echo "❌ Router build failed!"
    exit 1
fi
echo "✅ Router built successfully"

echo ""
echo "Building Slab..."
cargo build-sbf --manifest-path programs/slab/Cargo.toml
if [ $? -ne 0 ]; then
    echo "❌ Slab build failed!"
    exit 1
fi
echo "✅ Slab built successfully"
echo ""

# Deploy programs
echo "🚀 Deploying programs to devnet..."
echo ""

echo "Deploying Router..."
ROUTER_OUTPUT=$(solana program deploy target/deploy/percolator_router.so 2>&1)
echo "$ROUTER_OUTPUT"

ROUTER_ID=$(echo "$ROUTER_OUTPUT" | grep -oP 'Program Id: \K\w+')
if [ -n "$ROUTER_ID" ]; then
    echo "✅ Router deployed!"
    echo "📝 Router Program ID: $ROUTER_ID"
    echo "ROUTER_PROGRAM_ID=$ROUTER_ID" > deployment-ids.txt
else
    echo "❌ Router deployment failed!"
    exit 1
fi

echo ""
echo "Deploying Slab..."
SLAB_OUTPUT=$(solana program deploy target/deploy/percolator_slab.so 2>&1)
echo "$SLAB_OUTPUT"

SLAB_ID=$(echo "$SLAB_OUTPUT" | grep -oP 'Program Id: \K\w+')
if [ -n "$SLAB_ID" ]; then
    echo "✅ Slab deployed!"
    echo "📝 Slab Program ID: $SLAB_ID"
    echo "SLAB_PROGRAM_ID=$SLAB_ID" >> deployment-ids.txt
else
    echo "❌ Slab deployment failed!"
    exit 1
fi

echo ""
echo "====================================="
echo "🎉 Deployment Complete!"
echo "====================================="
echo ""
echo "Program IDs saved to: deployment-ids.txt"
echo ""
echo "Next steps:"
echo "1. Create slab account (~60 KB, costs only 0.5 SOL rent!):"
echo "   solana-keygen new --outfile slab-account.json --no-bip39-passphrase"
echo "   SLAB_ADDR=\$(solana address -k slab-account.json)"
echo "   solana create-account \$SLAB_ADDR 60000 $SLAB_ID --from ~/.config/solana/id.json"
echo ""
echo "2. Update api/.env with your program IDs"
echo "3. Restart API: cd api && npm run dev"
echo ""
echo "Your Program IDs:"
echo "Router: $ROUTER_ID"
echo "Slab: $SLAB_ID"
echo ""
echo "💰 Rent Cost: 0.5 SOL per slab (~\$75 @ \$150/SOL)"
echo "🎉 Saved 72.5 SOL vs original 10MB design!"
echo ""
echo "✨ Ready to test with real users!"

