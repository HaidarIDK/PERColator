#!/bin/bash
set -e

# Source Rust environment
source /home/haidarwtf/.cargo/env

# Add Solana to PATH
export PATH="/home/haidarwtf/.local/share/solana/install/active_release/bin:$PATH"

# Navigate to project
cd /mnt/c/Users/7haid/OneDrive/Desktop/percolator-v2

echo "=== Building Percolator Programs with Agave ==="
echo ""
cargo-build-sbf
echo ""
echo "=== Build complete! ==="
echo ""
ls -lh target/deploy/*.so

