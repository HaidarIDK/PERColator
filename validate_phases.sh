#!/bin/bash
#
# Quick validation script to check if run_all_phases.sh can run
#

set -e

echo "═══════════════════════════════════════════════════════════════"
echo "  Validating Test Phases Setup"
echo "═══════════════════════════════════════════════════════════════"
echo ""

# Check prerequisites
echo "Checking prerequisites..."

# Check cargo
if command -v cargo &> /dev/null; then
    echo "✓ Cargo found: $(cargo --version)"
else
    echo "✗ Cargo not found - install Rust first"
    exit 1
fi

# Check solana CLI
if command -v solana &> /dev/null; then
    echo "✓ Solana CLI found: $(solana --version)"
else
    echo "✗ Solana CLI not found - install Solana CLI first"
    exit 1
fi

# Check solana-test-validator
if command -v solana-test-validator &> /dev/null; then
    echo "✓ solana-test-validator found"
else
    echo "✗ solana-test-validator not found"
    exit 1
fi

# Check if we're in the right directory
if [ ! -f "Cargo.toml" ]; then
    echo "✗ Not in project root (Cargo.toml not found)"
    exit 1
fi
echo "✓ In project root"

# Check if run_all_phases.sh exists
if [ ! -f "run_all_phases.sh" ]; then
    echo "✗ run_all_phases.sh not found"
    exit 1
fi
echo "✓ run_all_phases.sh found"

# Check if CLI package exists
if [ ! -f "cli/Cargo.toml" ]; then
    echo "✗ CLI package not found"
    exit 1
fi
echo "✓ CLI package found"

# Check if test function exists in code
if grep -q "test_loss_socialization_integration\|test_kitchen_sink_e2e" cli/src/tests.rs; then
    echo "✓ Test phases found in cli/src/tests.rs"
else
    echo "✗ Test phases not found in cli/src/tests.rs"
    exit 1
fi

# Check if CLI command supports --crisis flag
if grep -q "crisis: test_crisis" cli/src/main.rs; then
    echo "✓ CLI supports --crisis flag"
else
    echo "✗ CLI does not support --crisis flag"
    exit 1
fi

echo ""
echo "═══════════════════════════════════════════════════════════════"
echo "  ✓ All prerequisites met! Ready to run tests."
echo "═══════════════════════════════════════════════════════════════"
echo ""
echo "To run all phases:"
echo "  ./run_all_phases.sh"
echo ""
echo "To keep validator running after tests:"
echo "  ./run_all_phases.sh --keep-validator"
echo ""

