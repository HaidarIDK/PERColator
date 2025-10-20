# Compute Unit Optimization Guide

This document contains strategies for optimizing Solana compute unit (CU) usage in Percolator programs.

## Solana Compute Limits

- **Per transaction**: 1.4 million CU max
- **Per account write lock**: 200k CU typical
- **Per CPI call**: ~10k CU overhead

## Current Optimization Strategies

### 1. Zero-Copy Deserialization

All account data is accessed via raw pointers (`borrow_account_data_mut`) instead of deserializing to intermediate structs.

**Savings**: ~50k CU per large account

### 2. Fixed-Point Math (No Floating Point)

All calculations use integer math with basis points for precision.

**Savings**: ~5k CU per calculation vs f64

### 3. O(1) Memory Allocation

Freelists for pools avoid dynamic allocation overhead.

**Savings**: ~2k CU per allocation

### 4. Minimal Logging

`msg!()` calls are expensive. Use sparingly in hot paths.

**Cost**: ~100 CU per msg! call

### 5. Inline Hot Functions

Critical path functions marked with `#[inline]` for compiler optimization.

**Savings**: ~500 CU per inlined call

### 6. Stack Allocation Where Possible

Small structs allocated on stack instead of heap.

**Savings**: ~1k CU per heap allocation avoided

## Program-Specific Optimizations

### Slab Program

**Reserve Instruction** (~50k CU est.):
- Walk orderbook (O(N) where N = price levels)
- Allocate reservation + slices
- Update reserved_qty on orders
- **Optimization**: Early exit if qty filled

**Commit Instruction** (~80k CU est.):
- Iterate slices (O(M) where M = fills)
- Execute trades, update positions
- Calculate fees, apply funding
- Anti-toxicity checks
- **Optimization**: Batch position updates

**Liquidation** (~120k CU est.):
- Walk positions (O(P) where P = positions)
- Close via market orders
- Multiple book walks possible
- **Optimization**: Limit max positions per call

### Router Program

**MultiReserve** (~150k CU est.):
- CPI to N slabs (N * 50k)
- Sort results (O(N log N))
- Mint N caps
- **Optimization**: Limit max slabs to 8

**MultiCommit** (~200k CU est.):
- CPI to N slabs (N * 80k)
- Update portfolio
- Burn caps
- **Optimization**: Sequential execution (abort early on failure)

## Measurement Tools

### Local Profiling

```bash
# Run with compute meter
cargo test-sbf --features test-sbf -- --nocapture

# Check program size
ls -lh target/deploy/*.so
```

### Devnet Testing

```typescript
// Add compute budget instruction
import { ComputeBudgetProgram } from '@solana/web3.js';

transaction.add(
  ComputeBudgetProgram.setComputeUnitLimit({
    units: 400_000,  // Set based on profiling
  })
);
```

## CU Budget Recommendations

### Per Instruction

| Instruction | Estimated CU | Budget | Notes |
|------------|--------------|---------|-------|
| Slab.Reserve | 50k | 100k | Depends on book depth |
| Slab.Commit | 80k | 150k | Depends on fill count |
| Slab.BatchOpen | 30k | 50k | Depends on pending count |
| Slab.Liquidate | 120k | 200k | Depends on position count |
| Router.Deposit | 20k | 50k | Simple transfer |
| Router.Withdraw | 30k | 50k | Margin check |
| Router.MultiReserve | 150k | 300k | N=3 slabs |
| Router.MultiCommit | 200k | 400k | N=3 slabs |

### Multi-Transaction Strategies

For operations exceeding 1.4M CU:

1. **Split across transactions**:
   ```
   Tx1: MultiReserve (3 slabs) - 150k CU
   Tx2: MultiCommit (3 slabs) - 200k CU
   ```

2. **Batch smaller operations**:
   ```
   Tx1: Reserve on Slabs A+B - 100k CU
   Tx2: Reserve on Slabs C+D - 100k CU
   Tx3: Commit all - 320k CU
   ```

## Optimization Checklist

Before deployment:

- [ ] Profile all instructions with `solana-test-validator`
- [ ] Add compute budget instructions to transactions
- [ ] Measure actual CU usage on devnet
- [ ] Optimize hot paths (commit, reserve, liquidate)
- [ ] Remove unnecessary `msg!()` calls in production
- [ ] Consider instruction batching for high-frequency ops
- [ ] Test under maximum load (full pools, deep books)

## Future Optimizations

### Phase 2
- Parallel CPI execution (when supported)
- Account compression for large states
- Incremental state updates (delta encoding)

### Phase 3
- Program-specific compute budget requests
- Custom syscalls for hot paths
- State sharding within slab

## References

- [Solana Compute Budget](https://docs.solana.com/developing/programming-model/runtime#compute-budget)
- [Pinocchio Performance](https://github.com/anza-xyz/pinocchio#performance)
- [BPF Loader](https://docs.solana.com/developing/on-chain-programs/deploying)

