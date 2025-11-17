#!/bin/bash
# Quick re-run script for testing the fixes

cd /mnt/c/Users/7haid/OneDrive/Desktop/percolator

echo "================================================"
echo "  Re-running Test Suite with Latest Fixes"
echo "================================================"
echo ""

# Make executable
chmod +x run_all_phases.sh

# Run with output
./run_all_phases.sh

echo ""
echo "================================================"
echo "  Test Summary"
echo "================================================"

# Show key results
echo ""
echo "To view detailed error logs:"
echo "  cat /tmp/crisis_test_output_*.log | tail -50"
echo ""
echo "To view init errors specifically:"
echo "  cat /tmp/init_output_*.log"

