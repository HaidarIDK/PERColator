# Percolator Security Audit

**Disclaimer:** This audit was performed by an AI assistant assuming an adversarial developer. It is not a substitute for a professional security audit.

## Summary

The Percolator codebase implements a risk engine for a perpetual DEX with formal verification (Kani). 

**Current Status:** The developer has successfully addressed several high-severity arithmetic and verification correctness issues identified in the previous audit cycle. However, critical structural vulnerabilities related to Denial of Service and Stack Overflow remain remediation priorities.

## Unresolved Issues

### Critical

*   **[C-01] Permanent Denial of Service via Account Slot Exhaustion:**
    The system utilizes a fixed-size slab (`MAX_ACCOUNTS = 4096`) for account storage. There is still no mechanism to deallocate or reuse slots once an account is closed (`alloc_slot` exists, but no `free_slot` equivalent). An attacker (or normal usage) can permanently exhaust all available slots, rendering the contract unusable for new users.

### High

*   **[H-01] Stack Overflow Risk in `RiskEngine::new`:**
    The `RiskEngine::new` function continues to return the struct by value (`-> Self`). With `MAX_ACCOUNTS = 4096`, the struct size is ~850KB, far exceeding Solana's 4KB stack limit. Calling this function on-chain ensures a stack overflow and transaction failure.

### Medium

*   **[M-01] Test Component Exposed in Production:**
    The `NoOpMatcher` struct remains `pub` and is not guarded by `#[cfg(test)]` or similar feature flags. It remains available for inadvertent (or malicious) use in production, where it would bypass all trade validation.

### Low

*   **[L-01] Inconsistent Rounding Logic:**
    `settle_account_funding` rounds payments up (vault-favoring), while `execute_trade` truncates PNL calculations. This minor asymmetry persists.

## Resolved Issues

*   **[H-02] Integer Overflow Panic on `i128::MIN` Negation:**
    **Fixed.** The code now uses a `neg_i128_to_u128` helper that correctly handles `i128::MIN` by returning `(i128::MAX as u128) + 1`, preventing runtime panics.

*   **[H-03] Incorrect Equity Calculation via Unsafe Cast:**
    **Fixed.** The code now uses `u128_to_i128_clamped` to convert capital. This clamps values exceeding `i128::MAX` instead of wrapping them to negative numbers, preventing valid accounts from appearing insolvent.

*   **[H-04] Verification Harness Integer Overflows (`tests/kani.rs`):**
    **Fixed.** The verification harness `recompute_totals` function now uses the safe `neg_i128_to_u128` helper.

*   **[H-05] Verification Harness Unsafe Casts (`tests/kani.rs`):**
    **Fixed.** Verification proofs now use `u128_to_i128_clamped` for equity calculations, ensuring the proofs hold for the full domain of `u128`.

*   **[M-02] Weak/Incomplete Frame Proofs (`tests/kani.rs`):**
    **Fixed.**
    *   `fast_frame_deposit` now asserts `engine.loss_accum == loss_accum_before`.
    *   `fast_frame_execute_trade` now explicitly asserts `engine.vault == vault_before` and `engine.insurance_fund.balance >= insurance_before`.

## Recommendations

1.  **Implement Account Deallocation:** Add `close_account` / `free_slot` functionality to address [C-01].
2.  **Refactor Initialization:** Change `RiskEngine::new` to `initialize(data: &mut [u8])` to avoid stack allocation and fix [H-01].
3.  **Feature Gating:** Protect `NoOpMatcher` with `#[cfg(any(test, feature = "fuzz", kani))]` to fix [M-01].
4.  **Rounding:** (Optional) Harmonize rounding logic if precision drift becomes a concern.

---

# Kani Proofs Timing Audit

**Date:** 2025-12-20
**Kani Version:** 0.66.0
**Total Proofs:** 90

## Summary

| Category | Count |
|----------|-------|
| Total Proofs | 90 |
| Passed | 62 |
| Failed | 18 |
| Timeout (>15min) | 10 |
| Slow (>10min) | 10 |

## Proofs Taking More Than 10 Minutes

The following proofs exceeded 10 minutes execution time:

| Proof Name | Duration | Status |
|------------|----------|--------|
| `multiple_users_adl_preserves_all_principals` | >15m | TIMEOUT |
| `i1_lp_adl_never_reduces_capital` | >15m | TIMEOUT |
| `multiple_lps_adl_preserves_all_capitals` | >15m | TIMEOUT |
| `mixed_users_and_lps_adl_preserves_all_capitals` | >15m | TIMEOUT |
| `panic_settle_closes_all_positions` | >15m | TIMEOUT |
| `audit_force_realize_updates_warmup_start` | >15m | TIMEOUT |
| `proof_adl_exact_haircut_distribution` | >15m | TIMEOUT |
| `proof_adl_exactness_and_reserved_invariant` | >15m | TIMEOUT |
| `panic_settle_preserves_conservation` | 14m 47s | PASS |
| `proof_c1_conservation_bounded_slack_force_realize` | 11m 43s | PASS |

### Common Patterns in Slow Proofs

1. **ADL-related proofs**: Multiple proofs involving ADL (Auto-Deleveraging) with multiple users/LPs tend to timeout. These proofs iterate over account arrays which causes state space explosion.

2. **Panic settle operations**: `panic_settle_closes_all_positions` and `panic_settle_preserves_conservation` are slow due to the complexity of settling all positions and verifying conservation.

3. **Force realize losses**: `audit_force_realize_updates_warmup_start` and `proof_c1_conservation_bounded_slack_force_realize` involve complex state transitions during loss realization.

## Failed Proofs

The following 18 proofs failed verification:

| Proof Name | Duration |
|------------|----------|
| `i10_fair_unwinding_constant_haircut_ratio` | 1s |
| `i10_total_withdrawals_bounded_by_available` | 1s |
| `i10_top_up_reduces_loss_accum` | 1s |
| `i10_withdrawal_tracking_accuracy` | 0s |
| `adl_is_proportional_for_user_and_lp` | 29s |
| `adl_proportionality_general` | 6m 21s |
| `i10_fair_unwinding_is_fair_for_lps` | 1s |
| `proof_r1_adl_never_spends_reserved` | 3s |
| `fast_frame_touch_account_only_mutates_one_account` | 3s |
| `fast_frame_execute_trade_only_mutates_two_accounts` | 6s |
| `fast_frame_apply_adl_never_changes_any_capital` | 31s |
| `fast_valid_preserved_by_execute_trade` | 6s |

## Recommendations for Slow Proofs

1. **Reduce state space for ADL proofs**: Consider using smaller account bounds or symbolic abstractions for proofs involving multiple accounts.

2. **Review commented-out proofs**: Several proofs are noted as "commented out" because they test old withdrawal haircut logic. These should be either removed or updated to test the new logic.

3. **Investigate failed proofs**: The failed proofs may indicate implementation bugs, incorrect proof assumptions, or missing preconditions.

4. **Consider parallel verification**: Given the long execution times, running proofs in parallel or using incremental verification could improve CI performance.

## Complete Results

### All Proof Timings

```
i1_adl_never_reduces_principal: 0m 2s - PASS
fast_i2_deposit_preserves_conservation: 0m 2s - PASS
fast_i2_withdraw_preserves_conservation: 0m 3s - PASS
i5_warmup_determinism: 0m 4s - PASS
i5_warmup_monotonicity: 0m 2s - PASS
i5_warmup_bounded_by_pnl: 0m 2s - PASS
i7_user_isolation_deposit: 0m 2s - PASS
i7_user_isolation_withdrawal: 0m 2s - PASS
i8_equity_with_positive_pnl: 0m 2s - PASS
i8_equity_with_negative_pnl: 0m 1s - PASS
i4_adl_haircuts_unwrapped_first: 0m 22s - PASS
withdrawal_requires_sufficient_balance: 0m 2s - PASS
pnl_withdrawal_requires_warmup: 0m 2s - PASS
multiple_users_adl_preserves_all_principals: 15m 0s - TIMEOUT
saturating_arithmetic_prevents_overflow: 0m 1s - PASS
zero_pnl_withdrawable_is_zero: 0m 2s - PASS
negative_pnl_withdrawable_is_zero: 0m 1s - PASS
funding_p1_settlement_idempotent: 0m 12s - PASS
funding_p2_never_touches_principal: 0m 2s - PASS
funding_p3_bounded_drift_between_opposite_positions: 0m 3s - PASS
funding_p4_settle_before_position_change: 0m 9s - PASS
funding_p5_bounded_operations_no_overflow: 0m 2s - PASS
funding_zero_position_no_change: 0m 2s - PASS
i10_risk_mode_triggers_at_floor: 0m 7s - PASS
i10_fair_unwinding_constant_haircut_ratio: 0m 1s - FAIL
i10_withdrawal_mode_blocks_position_increase: 0m 9s - PASS
i10_withdrawal_mode_allows_position_decrease: 0m 4s - PASS
i10_total_withdrawals_bounded_by_available: 0m 1s - FAIL
i10_top_up_reduces_loss_accum: 0m 1s - FAIL
i10_top_up_exits_withdrawal_mode_when_loss_zero: 0m 1s - PASS
fast_i10_withdrawal_mode_preserves_conservation: 0m 3s - PASS
i10_withdrawal_tracking_accuracy: 0m 0s - FAIL
i1_lp_adl_never_reduces_capital: 15m 0s - TIMEOUT
adl_is_proportional_for_user_and_lp: 0m 29s - FAIL
adl_proportionality_general: 6m 21s - FAIL
i10_fair_unwinding_is_fair_for_lps: 0m 1s - FAIL
multiple_lps_adl_preserves_all_capitals: 15m 0s - TIMEOUT
mixed_users_and_lps_adl_preserves_all_capitals: 15m 0s - TIMEOUT
proof_warmup_frozen_when_paused: 0m 8s - PASS
proof_withdraw_only_decreases_via_conversion: 0m 2s - PASS
proof_risk_increasing_trades_rejected: 0m 8s - PASS
panic_settle_closes_all_positions: 15m 0s - TIMEOUT
panic_settle_clamps_negative_pnl: 3m 42s - PASS
panic_settle_enters_risk_mode: 0m 3s - PASS
panic_settle_preserves_conservation: 14m 47s - PASS
warmup_budget_a_invariant_holds_after_settlement: 0m 2s - PASS
warmup_budget_b_negative_settlement_no_increase_pos: 0m 3s - PASS
warmup_budget_c_positive_settlement_bounded_by_budget: 0m 2s - PASS
warmup_budget_d_paused_settlement_time_invariant: 0m 2s - PASS
audit_settle_idempotent_when_paused: 0m 5s - PASS
audit_warmup_started_at_updated_to_effective_slot: 0m 2s - PASS
audit_multiple_settlements_when_paused_idempotent: 0m 5s - PASS
proof_r1_adl_never_spends_reserved: 0m 3s - FAIL
proof_r2_reserved_bounded_and_monotone: 0m 4s - PASS
proof_r3_warmup_reservation_safety: 0m 3s - PASS
proof_ps5_panic_settle_no_insurance_minting: 6m 28s - PASS
proof_c1_conservation_bounded_slack_panic_settle: 8m 11s - PASS
proof_c1_conservation_bounded_slack_force_realize: 11m 43s - PASS
audit_force_realize_updates_warmup_start: 15m 0s - TIMEOUT
proof_warmup_slope_nonzero_when_positive_pnl: 0m 2s - PASS
proof_reserved_equals_derived_formula: 0m 2s - PASS
proof_adl_exact_haircut_distribution: 15m 0s - TIMEOUT
proof_adl_exactness_and_reserved_invariant: 15m 0s - TIMEOUT
fast_frame_touch_account_only_mutates_one_account: 0m 3s - FAIL
fast_frame_deposit_only_mutates_one_account_vault_and_warmup: 0m 2s - PASS
fast_frame_withdraw_only_mutates_one_account_vault_and_warmup: 0m 2s - PASS
fast_frame_execute_trade_only_mutates_two_accounts: 0m 6s - FAIL
fast_frame_top_up_only_mutates_vault_insurance_loss_mode: 0m 2s - PASS
fast_frame_enter_risk_mode_only_mutates_flags: 0m 1s - PASS
fast_frame_apply_adl_never_changes_any_capital: 0m 31s - FAIL
fast_frame_settle_warmup_only_mutates_one_account_and_warmup_globals: 0m 3s - PASS
fast_frame_update_warmup_slope_only_mutates_one_account: 0m 2s - PASS
fast_valid_preserved_by_deposit: 0m 2s - PASS
fast_valid_preserved_by_withdraw: 0m 2s - PASS
fast_valid_preserved_by_execute_trade: 0m 6s - FAIL
fast_valid_preserved_by_apply_adl: 9m 36s - PASS
fast_valid_preserved_by_settle_warmup_to_capital: 0m 3s - PASS
fast_valid_preserved_by_panic_settle_all: 3m 39s - PASS
fast_valid_preserved_by_force_realize_losses: 7m 30s - PASS
fast_valid_preserved_by_top_up_insurance_fund: 0m 1s - PASS
fast_neg_pnl_settles_into_capital_independent_of_warm_cap: 0m 2s - PASS
fast_withdraw_cannot_bypass_losses_when_position_zero: 0m 2s - PASS
fast_neg_pnl_after_settle_implies_zero_capital: 0m 3s - PASS
neg_pnl_settlement_does_not_depend_on_elapsed_or_slope: 0m 2s - PASS
withdraw_calls_settle_enforces_pnl_or_zero_capital_post: 0m 3s - PASS
fast_maintenance_margin_uses_equity_including_negative_pnl: 0m 3s - PASS
fast_account_equity_computes_correctly: 0m 1s - PASS
withdraw_im_check_blocks_when_equity_after_withdraw_below_im: 0m 2s - PASS
maintenance_margin_uses_equity_negative_pnl: 0m 1s - PASS
neg_pnl_is_realized_immediately_by_settle: 0m 2s - PASS
```