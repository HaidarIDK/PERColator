#!/bin/bash
# Debug script to check init failure

echo "=== Checking Init Error Details ==="
echo ""

# Find the latest init log
LATEST_INIT_LOG=$(ls -t /tmp/init_output_*.log 2>/dev/null | head -1)

if [ -n "$LATEST_INIT_LOG" ]; then
    echo "Latest init log: $LATEST_INIT_LOG"
    echo ""
    echo "=== Full Init Output ==="
    cat "$LATEST_INIT_LOG"
    echo ""
    echo "=== Error Lines ==="
    grep -i "error\|failed\|Error" "$LATEST_INIT_LOG" || echo "No explicit errors found"
else
    echo "No init log found"
fi

echo ""
echo "=== Checking CLI Payer Account ==="
CLI_PAYER="3eiGbENLNdE2rww7PL9CYVWCyLWkdv3x1wR9FeZfp7mM"
echo "CLI Payer: $CLI_PAYER"
solana balance "$CLI_PAYER" || echo "Failed to get balance"
echo ""
solana account "$CLI_PAYER" || echo "Account doesn't exist"

echo ""
echo "=== Checking Registry Account ==="
REGISTRY="F8VsKvNbHHQzX8wQTZ8amgEXJxJJFxW9pFzEbPvuYv3f"
echo "Registry: $REGISTRY"
solana account "$REGISTRY" || echo "Registry doesn't exist (expected before init)"

echo ""
echo "=== Validator Status ==="
solana ping -c 1 || echo "Validator not responding"

