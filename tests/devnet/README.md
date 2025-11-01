# Devnet Testing Suite

Tests for deployed Percolator programs on Solana devnet.

## Test Files

### Core Tests

#### `portfolio_amm.js`
Tests portfolio creation, AMM pool initialization, and collateral deposits.

**Usage:**
```bash
npm run test:portfolio
# or
node tests/devnet/portfolio_amm.js
```

**Tests:**
- Create user portfolio (500KB, cross-margin enabled)
- Create AMM liquidity pool (constant product, x*y=k)
- Deposit collateral to portfolio

#### `program_deployment.js`
Verifies program deployment and basic functionality.

**Usage:**
```bash
npm run test:deployment
# or
node tests/devnet/program_deployment.js
```

**Tests:**
- Verify all programs are deployed
- Calculate and verify PDAs
- Check registry status
- Create test slab (order book)

### Additional Tests

#### `basic_programs.js`
Basic program verification and slab creation (original test_programs.js).

**Usage:**
```bash
node tests/devnet/basic_programs.js
```

#### `advanced_testing.js`
Create large 1MB slabs for high-capacity trading (original test_advanced.js).

**Usage:**
```bash
node tests/devnet/advanced_testing.js
```

**Note:** Requires ~7.3 SOL for rent

#### `amm_size_finder.js`
Utility to find correct AMM pool size (original test_amm_sizes.js).

**Usage:**
```bash
node tests/devnet/amm_size_finder.js
```

**Result:** Correct size is 448 bytes

## Configuration

Tests automatically load configuration from:
- `keypairs/config.json` - Program IDs and deployment info
- `keypairs/devwallet.json` - Dev wallet private key

## Running Tests

```bash
# Run organized tests
npm run test:all           # All tests
npm run test:deployment    # Deployment verification
npm run test:portfolio     # Portfolio and AMM

# Run individual tests
node tests/devnet/basic_programs.js
node tests/devnet/advanced_testing.js
node tests/devnet/amm_size_finder.js
```

## Account Sizes

- **Portfolio**: 500,000 bytes (500 KB)
- **AMM Pool**: 448 bytes
- **Slab (small)**: 4,096 bytes (4 KB)
- **Slab (large)**: 1,048,576 bytes (1 MB)

## Rent Requirements

- **Portfolio**: 3.48 SOL
- **AMM Pool**: 0.004 SOL
- **Slab (4KB)**: 0.03 SOL
- **Slab (1MB)**: 7.30 SOL

## Deposit Limits

Maximum deposit per transaction: 0.1 SOL (100M lamports)

## Explorer Links

View transactions on Solana Explorer:
```
https://explorer.solana.com/tx/{SIGNATURE}?cluster=devnet
https://explorer.solana.com/address/{ADDRESS}?cluster=devnet
```

## Troubleshooting

### Insufficient Balance
If you see "insufficient balance" errors:
```bash
solana airdrop 5 YOUR_PUBKEY --url devnet
```

### Account Already Exists
If portfolio or AMM already exists, tests will skip creation and use existing accounts.

### Incorrect Size Errors
AMM pools must be exactly 448 bytes. Use `amm_size_finder.js` to verify.
