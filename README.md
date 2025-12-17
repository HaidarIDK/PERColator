# Percolator: Risk Engine for Perpetual DEXs

**Educational research project. NOT production ready. Do NOT use with real funds.**

A formally verified risk engine for perpetual futures DEXs on Solana. Provides mathematical guarantees about fund safety under oracle manipulation.

## Design

### Core Insight

Oracle manipulation allows attackers to create artificial profits. PNL warmup ensures profits cannot be withdrawn instantly - they "warm up" over time T. During ADL events, unwrapped PNL is haircutted first, protecting deposited capital.

**Guarantee:** If oracle manipulation lasts at most time T, and PNL requires time T to vest, then deposited capital is always withdrawable (subject to margin requirements).

### Invariants

| ID | Property |
|----|----------|
| **I1** | Account capital is NEVER reduced by ADL or socialization |
| **I2** | Conservation: `vault + loss_accum == sum(capital) + sum(pnl) + insurance_fund.balance` |
| **I4** | ADL haircuts unwrapped PNL before insurance fund |
| **I5** | PNL warmup is deterministic and monotonically increasing |
| **I7** | Account isolation - operations on one account don't affect others |

### Warmup Budget Invariant

Warmup converts PnL into principal with sign:
- **Positive PnL** can increase capital (profits become withdrawable principal)
- **Negative PnL** can decrease capital (losses paid from principal, up to available capital)

A global budget prevents warmed profits from exceeding paid losses plus spendable insurance:

```
W⁺ ≤ W⁻ + max(0, I - I_min)
```

**Definitions:**
- `W⁺` = `warmed_pos_total` - cumulative positive PnL converted to capital
- `W⁻` = `warmed_neg_total` - cumulative negative PnL paid from capital
- `I` = `insurance_fund.balance` - current insurance fund balance
- `I_min` = `risk_reduction_threshold` - minimum insurance floor

**Rationale:** This invariant prevents "profit maturation / withdrawal" from outrunning realized loss payments. Without this constraint, an attacker could create artificial profits via oracle manipulation, wait for warmup, and withdraw before corresponding losses are paid - effectively extracting value that doesn't exist in the vault.

### Key Operations

**Trading:** Zero-sum PNL between user and LP. Fees go to insurance fund.

**ADL (Auto-Deleveraging):** When losses exceed insurance capacity:
1. Haircut unwrapped (young) PNL proportionally across all accounts
2. Use insurance fund for remaining losses
3. If insurance depleted, enter risk-reduction-only mode

**Risk-Reduction-Only Mode:** Triggered when insurance fund depleted or below threshold:
- Warmup frozen (no more PNL vests)
- Risk-increasing trades blocked
- Capital withdrawals and position closing allowed
- Exit via insurance fund top-up

**Funding:** O(1) cumulative index pattern. Settled lazily before any account operation.

### Conservation

The conservation formula is exact (no tolerance):

```
vault + loss_accum = sum(capital) + sum(pnl) + insurance_fund.balance
```

Where `loss_accum` tracks unrecoverable losses when insurance is depleted.

This holds because:
- Deposits/withdrawals adjust both vault and capital
- Trading PNL is zero-sum between counterparties
- Fees transfer from user PNL to insurance (net zero)
- ADL redistributes PNL (net zero)

## Running Tests

All tests require increased stack size due to fixed 4096-account array:

```bash
# Unit tests (52 tests)
RUST_MIN_STACK=16777216 cargo test

# Fuzzing (property-based tests)
RUST_MIN_STACK=16777216 cargo test --features fuzz

# Formal verification (Kani)
cargo install --locked kani-verifier
cargo kani setup
cargo kani
```

## Formal Verification

Kani proofs verify all critical invariants via bounded model checking. See `tests/kani.rs` for proof harnesses.

```bash
# Run specific proof
cargo kani --harness i1_adl_never_reduces_principal
cargo kani --harness i2_deposit_preserves_conservation
```

Note: Kani proofs use 64-account arrays for tractability. Production uses 4096.

## Architecture

- `#![no_std]` - no heap allocation
- `#![forbid(unsafe_code)]` - safe Rust only
- Fixed 4096-account slab (~664 KB)
- Bitmap for O(1) slot allocation
- O(N) ADL via bitmap scan

## Limitations

- No signature verification (external concern)
- No oracle implementation (external concern)
- No account deallocation
- Maximum 4096 accounts
- Not audited for production

## License

Apache-2.0
