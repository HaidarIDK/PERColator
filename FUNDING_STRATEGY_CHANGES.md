# Funding Strategy Changes

## Overview
Modified the test suite to use the funded wallet (with 44 SOL) to fund all accounts that need SOL, instead of relying on airdrops which can be rate-limited or unreliable.

## Changes Made

### 1. Script: `run_all_phases.sh`

#### Initial Payer Funding (Lines 263-326)
**Strategy**: Transfer from funded wallet → Airdrop fallback
- Checks if payer needs funding (< 10 SOL)
- If funded wallet (44 SOL) has enough, transfers SOL directly
- Falls back to airdrop only if transfer fails
- Logs clearly which method was used

**Before**:
```bash
# Always used airdrops
solana airdrop 5 "$PAYER_ADDRESS"
```

**After**:
```bash
# Try transfer first
if solana transfer --allow-unfunded-recipient "$PAYER_ADDRESS" "$NEEDED_SOL" --from "$WALLET_ADDRESS"; then
    # Success via transfer
else
    # Fall back to airdrop
    solana airdrop 5 "$PAYER_ADDRESS"
fi
```

#### CLI Payer Funding (Lines 328-363)
**Strategy**: Transfer from funded wallet → Airdrop fallback
- Before running init, ensures CLI payer has at least 1 SOL
- Transfers 5 SOL from funded wallet if needed
- Only uses airdrop as last resort

**Benefits**:
- Faster (no rate limiting)
- More reliable (funded wallet is guaranteed to exist on validator)
- Uses existing funds efficiently

### 2. Test Code: `cli/src/tests.rs` (Kitchen Sink E2E)

#### Actor Funding (Lines 2273-2325)
**Strategy**: Transfer first → Airdrop fallback
- Tries to transfer from payer (the funded wallet) to test actors
- Only falls back to airdrop if transfer fails
- Clear logging shows which method succeeded

**Before**:
```rust
// Always used airdrops on localnet
let airdrop_sig = rpc_client.request_airdrop(&keypair.pubkey(), amount)?;
```

**After**:
```rust
// Try transfer first
match rpc_client.send_and_confirm_transaction(&transfer_tx) {
    Ok(_) => println!("funded via transfer from funded wallet"),
    Err(_) => {
        // Fall back to airdrop only if transfer fails
        rpc_client.request_airdrop(&keypair.pubkey(), amount)?;
    }
}
```

## How It Works

### Funding Hierarchy

1. **Funded Wallet** (`A6PBghCz9VqgRecJtKcewGaEdSfhpNYDV9xxJWkHctbU`)
   - Has 44 SOL (or 22+ SOL)
   - Used as primary funding source
   - Default Solana CLI wallet

2. **CLI Payer** (`3eiGbENLNdE2rww7PL9CYVWCyLWkdv3x1wR9FeZfp7mM`)
   - Gets funded FROM the wallet via transfer
   - Used for CLI operations and registry initialization
   - Needs ~5-10 SOL

3. **Test Actors** (Alice, Bob, Dave, Erin)
   - Get funded FROM the CLI payer via transfer
   - Each needs 10 SOL for transactions
   - Total: 40 SOL for all actors

### Transfer Flow

```
Funded Wallet (44 SOL)
  └─> Transfer 5-10 SOL to CLI Payer
        └─> Transfer 10 SOL to Alice
        └─> Transfer 10 SOL to Bob
        └─> Transfer 10 SOL to Dave
        └─> Transfer 10 SOL to Erin
```

### Fallback Strategy

If any transfer fails (e.g., recipient doesn't exist yet):
1. Log the failure
2. Use airdrop as fallback
3. Continue execution

## Benefits

### 1. **Reliability**
- Funded wallet exists on validator (already has balance)
- Transfers are instant and don't hit rate limits
- No waiting for airdrop confirmations

### 2. **Speed**
- Transfers complete in 1-2 seconds
- Airdrops can take 5-10 seconds each
- Multiple airdrops can hit rate limits

### 3. **Efficiency**
- Uses existing funds rather than requesting new ones
- Cleaner transaction history
- More realistic (closer to production behavior)

### 4. **Robustness**
- Airdrop fallback ensures tests still work if transfers fail
- Clear logging shows which method was used
- Handles edge cases gracefully

## Expected Output

### Successful Transfer Path
```
  Wallet Address: A6PBghCz9VqgRecJtKcewGaEdSfhpNYDV9xxJWkHctbU
  Transferring 5 SOL from funded wallet to payer...
  ✓ Payer funded via transfer: 5 SOL

  Alice funded with 10 SOL (transfer from funded wallet)
  Bob funded with 10 SOL (transfer from funded wallet)
  Dave funded with 10 SOL (transfer from funded wallet)
  Erin funded with 10 SOL (transfer from funded wallet)
  ✓ All actors funded
```

### Fallback to Airdrop (if needed)
```
  Transfer failed, using airdrop...
  ✓ Payer funded via airdrop fallback: 5 SOL
  
  Alice transfer failed, using airdrop...
  Alice funded with 10 SOL (airdrop fallback)
```

## Testing

Run the updated test suite:

```bash
cd /mnt/c/Users/7haid/OneDrive/Desktop/percolator
./run_all_phases.sh
```

Expected improvements:
- ✅ Faster execution (no airdrop delays)
- ✅ Registry initialization succeeds (CLI payer is funded)
- ✅ All tests pass (actors funded from wallet)
- ✅ Clear logs showing transfer success

## Troubleshooting

### If transfers still fail:

1. **Check wallet balance**:
   ```bash
   solana balance A6PBghCz9VqgRecJtKcewGaEdSfhpNYDV9xxJWkHctbU
   ```

2. **Verify wallet exists on validator**:
   ```bash
   solana account A6PBghCz9VqgRecJtKcewGaEdSfhpNYDV9xxJWkHctbU
   ```

3. **Check logs**:
   ```bash
   cat /tmp/percolator_test_*.log | grep -i "transfer\|airdrop"
   ```

4. **Manual transfer test**:
   ```bash
   solana transfer --allow-unfunded-recipient <target> 1
   ```

The fallback to airdrops ensures the tests will still work even if transfers fail!

