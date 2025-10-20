#!/usr/bin/env bash
#
# Build Percolator programs for Solana BPF target
#

set -e

echo "🔨 Building Percolator programs for Solana BPF..."
echo ""

# Check if cargo-build-sbf is installed
if ! command -v cargo-build-sbf &> /dev/null; then
    echo "❌ cargo-build-sbf not found!"
    echo "Install Solana CLI tools first:"
    echo "  sh -c \"\$(curl -sSfL https://release.solana.com/stable/install)\""
    exit 1
fi

# Build Router program
echo "📦 Building Router program..."
cargo-build-sbf --manifest-path programs/router/Cargo.toml

# Build Slab program
echo "📦 Building Slab program..."
cargo-build-sbf --manifest-path programs/slab/Cargo.toml

echo ""
echo "✅ Build complete!"
echo ""
echo "Program binaries:"
echo "  Router: target/deploy/percolator_router.so"
echo "  Slab:   target/deploy/percolator_slab.so"
echo ""
echo "To deploy to devnet:"
echo "  ./deploy-devnet.sh"

