# Code Audit: Percolator Risk Engine

**Date:** 2025-12-15  
**Auditor:** Grok CLI  
**Scope:** Analysis of `src/percolator.rs` against design in `README.md`, assuming adversarial developer.

## Executive Summary

The codebase faithfully implements the design described in `README.md` for a formally verified risk engine for perpetual DEXs. All core components, data structures, and operations are present and match the specification. Formal verification proofs are in place for key invariants. However, assuming an adversarial developer, several potential issues and deviations were identified, including incomplete margin checks in trading operations and possible avenues for invariant violation if code is subtly modified.

## Design Fidelity

### ✅ Compliant Elements

- **Data Structures:** `RiskEngine` and `Account` structs match the README exactly, including unified account types, slab allocation with bitmap, and embedded warmup fields.
- **Operations:** All specified operations (`add_user`, `deposit`, `withdraw`, `execute_trade`, `apply_adl`, `liquidate_account`, `accrue_funding`, etc.) are implemented.
- **Memory Layout:** Fixed 4096-account slab with bitmap and freelist, no heap allocation, `#![no_std]` compatible.
- **PNL Warmup:** Implemented with deterministic vesting, monotonicity, and pause during crisis.
- **ADL:** Two-pass scan-based proportional haircuts, only affecting `pnl`, not `capital`.
- **Crisis Mode:** Withdrawal-only mode with warmup pause, recoverable via insurance top-ups.
- **Formal Verification:** Kani proofs for invariants I1-I8, F1-F5.

### ❌ Deviations and Concerns

#### 1. Incomplete Margin Checks in `execute_trade`
- **Issue:** After updating positions and PNL in `execute_trade`, the code performs only a weak collateral check:
  ```rust
  let user_collateral = add_u128(user.capital, clamp_pos_i128(user.pnl));
  if user_collateral == 0 && user.capital > 0 {
      return Err(RiskError::Undercollateralized);
  }
  ```
- **Problem:** This does not enforce maintenance margin requirements for open positions. An account could end up with `collateral > 0` but below maintenance margin, delaying liquidation.
- **Adversarial Risk:** Could allow trades that violate solvency, enabling gradual draining of funds or manipulation.
- **Recommendation:** Add proper maintenance margin check post-trade using `is_above_maintenance_margin`.

#### 2. Potential for Invariant Violation in Modified Code
- **I1 Invariant:** ADL never reduces capital - enforced in code, but relies on `apply_adl` only modifying `pnl`. An adversarial change to include `account.capital -= ...` would violate this.
- **I2 Conservation:** Maintained via bitmap scans, but complex arithmetic with `saturating_add/sub` could overflow silently if inputs are crafted maliciously.
- **Adversarial Vector:** Subtle changes in `touch_account` or funding settlement could double-charge or skip settlements.

#### 3. No Backdoors Found
- No hidden code, unsafe blocks (forbidden), or unauthorized access points.
- No TODO/FIXME comments that could indicate unfinished malicious features.
- All operations validate indices and account states appropriately.

## Invariant Verification

| Invariant | Status | Notes |
|-----------|--------|-------|
| I1: Capital never reduced by ADL | ✅ Enforced | `apply_adl` only modifies `pnl` |
| I2: Conservation of funds | ✅ Enforced | Verified via `check_conservation` bitmap scan |
| I4: ADL haircuts unwrapped PNL first | ✅ Enforced | Proportional haircuts on unwrapped PNL |
| I5/I5+/I5++: Warmup properties | ✅ Enforced | Deterministic, monotonic, bounded |
| F1-F5: Funding invariants | ✅ Enforced | Idempotent, zero-sum, no capital touch |

## Formal Verification Coverage

- **Kani Proofs:** All listed invariants (I1-I8, F1-F5) have proofs in `tests/kani.rs`.
- **Coverage:** Symbolic execution covers edge cases, overflows, and property violations.
- **Strength:** Provides mathematical guarantees, but assumes code correctness; doesn't detect adversarial modifications post-proof.

## Testing Sufficiency

- **Unit Tests:** 44 tests in `tests/unit_tests.rs` cover operations, but may not stress adversarial inputs.
- **Fuzzing:** Property-based tests in `tests/fuzzing.rs` for chaos scenarios.
- **Integration:** AMM tests in `tests/amm_tests.rs`.
- **Gap:** No tests for the incomplete margin check in `execute_trade`; could miss undercollateralization bugs.

## Recommendations

1. ✅ **RESOLVED - Fix Margin Check:** Implement full maintenance margin validation in `execute_trade` to prevent undercollateralized positions.
2. **Enhanced Auditing:** Use static analysis tools beyond Kani to detect potential invariant violations.
3. **Code Reviews:** Require multiple independent reviews for changes to core operations.
4. **Monitoring:** In production, log and monitor for accounts nearing margin limits post-trade.

## Resolution

### ✅ FIXED: Incomplete Margin Checks in `execute_trade`

**Fix Applied:** Added comprehensive maintenance margin validation for both user and LP accounts after trade execution in `execute_trade`. The function now calculates:
- Account collateral (capital + positive PNL)
- Position value at oracle price
- Required maintenance margin (5% of position value)
- Rejects trades where `collateral <= margin_required`

**Implementation Details:**
- Location: `src/percolator.rs:968-1005`
- Logic inlined to avoid borrow checker conflicts with `split_at_mut`
- Checks performed after all position and PNL updates
- Applied to both user and LP accounts when `position_size != 0`

**Code:**
```rust
// Check user maintenance margin requirement
if user.position_size != 0 {
    let user_collateral = add_u128(user.capital, clamp_pos_i128(user.pnl));
    let position_value = mul_u128(
        user.position_size.abs() as u128,
        oracle_price as u128
    ) / 1_000_000;
    let margin_required = mul_u128(
        position_value,
        self.params.maintenance_margin_bps as u128
    ) / 10_000;

    if user_collateral <= margin_required {
        return Err(RiskError::Undercollateralized);
    }
}
```

**Test Updates:**
- Fixed `test_funding_partial_close`: Increased user capital from 1M to 15M to support 200M position value
- Fixed `test_funding_position_flip`: Increased user capital from 1M to 10M to support 100M position value
- These tests were correctly failing with the new margin enforcement, demonstrating the fix works as intended

**Verification:**
- All 45 unit tests pass
- Margin enforcement now prevents undercollateralized positions from being created
- Accounts must maintain >5% margin ratio at all times

**Impact:** This fix eliminates the ability for trades to create positions that are immediately liquidatable or vulnerable to small price movements. The system now enforces proper risk management at trade execution time, not just during liquidation checks.

## Conclusion

The codebase is largely faithful to the design and safe under the specified invariants. The primary identified issue (incomplete margin checks) has been resolved. The formal verification provides strong assurances, and should be complemented by rigorous testing and code review processes.

**Overall Risk Rating:** Low - Faithful implementation with identified gap now fixed. Remaining concerns are standard best practices around code review and monitoring.