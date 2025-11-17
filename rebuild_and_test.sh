#!/bin/bash
# Force rebuild CLI with the new config.rs changes, then run tests

echo "════════════════════════════════════════════════════"
echo "  Rebuilding CLI with Program ID Fix"
echo "════════════════════════════════════════════════════"
echo ""

cd /mnt/c/Users/7haid/OneDrive/Desktop/percolator

# Force clean and rebuild CLI to pick up config.rs changes
echo "Cleaning old CLI binary..."
cargo clean --package percolator-cli

echo ""
echo "Rebuilding CLI (this will take ~30 seconds)..."
cargo build --release --package percolator-cli --bin percolator

if [ $? -eq 0 ]; then
    echo "✓ CLI rebuilt successfully!"
    echo ""
    echo "Now running full test suite..."
    echo ""
    ./run_all_phases.sh
else
    echo "✗ CLI rebuild failed!"
    exit 1
fi

