#!/bin/bash

# Build and deploy Percolator programs to devnet

set -e  # Exit on error

echo "🔧 Setting up environment..."
export PATH="$HOME/.local/share/solana/install/active_release/bin:$HOME/.cargo/bin:$PATH"

echo "📍 Current directory: $(pwd)"
echo "🔍 Checking cargo-build-sbf..."
which cargo-build-sbf || { echo "❌ cargo-build-sbf not found!"; exit 1; }

echo ""
echo "🔨 Building Slab program..."
cargo-build-sbf --manifest-path programs/slab/Cargo.toml

echo ""
echo "🔨 Building Router program..."  
cargo-build-sbf --manifest-path programs/router/Cargo.toml

echo ""
echo "📦 Checking built programs..."
ls -lh target/deploy/*.so 2>&1 || echo "⚠️ No .so files in target/deploy/"
find target/sbf-solana-solana -name '*.so' 2>&1 | head -10

echo ""
echo "🚀 Deploying to devnet..."
echo "💰 Current balance:"
solana balance

echo ""
echo "📤 Deploying Slab program..."
SLAB_PROGRAM_ID=$(solana program deploy target/deploy/percolator_slab.so --output json | jq -r '.programId')
echo "✅ Slab Program ID: $SLAB_PROGRAM_ID"

echo ""
echo "📤 Deploying Router program..."
ROUTER_PROGRAM_ID=$(solana program deploy target/deploy/percolator_router.so --output json | jq -r '.programId')
echo "✅ Router Program ID: $ROUTER_PROGRAM_ID"

echo ""
echo "🎉 Deployment complete!"
echo ""
echo "Update these in your backend:"
echo "SLAB_PROGRAM_ID=$SLAB_PROGRAM_ID"
echo "ROUTER_PROGRAM_ID=$ROUTER_PROGRAM_ID"
echo ""
echo "View on Solana Explorer:"
echo "Slab:   https://explorer.solana.com/address/$SLAB_PROGRAM_ID?cluster=devnet"
echo "Router: https://explorer.solana.com/address/$ROUTER_PROGRAM_ID?cluster=devnet"

