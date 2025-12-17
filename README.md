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
| **I2** | Conservation: `vault + loss_accum == sum(capital) + sum(pnl) + insurance + rounding_surplus` |
| **I4** | ADL haircuts unwrapped PNL before insurance fund |
| **I5** | PNL warmup is deterministic and monotonically increasing |
| **I7** | Account isolation - operations on one account don't affect others |
| **I8** | Insurance floor: insurance spending never reduces `insurance_fund.balance` below `I_min` |
| **I9** | Threshold unstick: if `I <= I_min`, running the scan reduces total open interest to zero and forces loss payment from capital before ADL |

### Warmup Budget Invariant

Warmup converts PnL into principal with sign:
- **Positive PnL** can increase capital (profits become withdrawable principal)
- **Negative PnL** can decrease capital (losses paid from principal, up to available capital)

A global budget prevents warmed profits from exceeding paid losses plus unreserved spendable insurance:

```
W⁺ ≤ W⁻ + max(0, I - I_min) - R
```

**Definitions:**
- `W⁺` = `warmed_pos_total` - cumulative positive PnL converted to capital
- `W⁻` = `warmed_neg_total` - cumulative negative PnL paid from capital
- `I` = `insurance_fund.balance` - current insurance fund balance
- `I_min` = `risk_reduction_threshold` - minimum insurance floor (protected)
- `R` = `warmup_insurance_reserved` - insurance above the floor committed to backing warmed profits (monotone counter)
- `S` = `max(0, I - I_min) - R` (saturating) = unreserved spendable insurance

`R` reserves part of the spendable insurance above the floor to back already-warmed profits, so the invariant remains true even if insurance is later spent on losses.

**Enforcement:** The invariant is enforced at the moment PnL would be converted into capital (warmup settlement), and losses are settled before gains.

**Rationale:** This invariant prevents "profit maturation / withdrawal" from outrunning realized loss payments. Without this constraint, an attacker could create artificial profits via oracle manipulation, wait for warmup, and withdraw before corresponding losses are paid - effectively extracting value that doesn't exist in the vault.

### Key Operations

**Trading:** Zero-sum PNL between user and LP. Fees go to insurance fund.

**ADL (Auto-Deleveraging):** When losses must be covered:
1. Haircut unwrapped (young) PNL proportionally across all accounts
2. Spend only unreserved insurance above the protected floor: `max(0, I - I_min) - R`
3. Any remaining loss is added to `loss_accum`

Insurance spending is capped: the engine will only spend unreserved insurance above `I_min` to cover losses. Reserved insurance (`R`) backs already-warmed profits and cannot be spent by ADL.

**Risk-Reduction-Only Mode:** Triggered when insurance fund at or below threshold:
- Warmup frozen (no more PNL vests)
- Risk-increasing trades blocked
- Capital withdrawals and position closing allowed
- Exit via insurance fund top-up

**Forced Loss Realization Scan (Threshold Unstick):** When `insurance_fund.balance <= I_min`, the engine can perform an atomic scan that settles all open positions at the oracle price and forces negative PnL to be paid from capital (up to available capital). Any unpaid remainder is socialized via ADL (unwrapped first, then spendable insurance above floor, then `loss_accum`). This prevents profit maturation/withdrawal from outrunning realized loss payments.

**Funding:** O(1) cumulative index pattern. Settled lazily before any account operation.

### Conservation

The conservation formula is exact (no tolerance):

```
vault + loss_accum = sum(capital) + sum(pnl) + insurance_fund.balance + rounding_surplus
```

Where:
- `loss_accum` tracks uncovered losses after consuming unwrapped PnL and all spendable insurance above the floor
- `rounding_surplus` tracks value in vault unclaimed due to integer division rounding during position settlement

This holds because:
- Deposits/withdrawals adjust both vault and capital
- Trading PNL is zero-sum between counterparties
- Fees transfer from user PNL to insurance (net zero)
- ADL redistributes PNL (net zero)
- Rounding slippage from position settlement is tracked explicitly

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
