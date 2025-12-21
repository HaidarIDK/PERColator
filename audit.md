# Kani Proof Timing Report
Generated: 2025-12-21

## Summary

- **Total Proofs**: 90
- **Passed**: 83
- **Failed**: 0
- **Timeout**: 7 (commented out / deprecated)
- **Slow (>60s)**: 7

### Proofs Needing Attention

**Timeout** (commented out or deprecated):
- `i10_fair_unwinding_constant_haircut_ratio` - commented out
- `i10_total_withdrawals_bounded_by_available` - commented out
- `i10_top_up_reduces_loss_accum` - commented out
- `i10_withdrawal_tracking_accuracy` - commented out
- `i10_fair_unwinding_is_fair_for_lps` - commented out
- `adl_proportionality_general` - commented out (complex multi-account remainder issue)
- `panic_settle_preserves_conservation` - 660s, TIMEOUT

**Slow (>60s)**:
- `proof_ps5_panic_settle_no_insurance_minting` - 77s
- `fast_valid_preserved_by_panic_settle_all` - 183s
- `fast_valid_preserved_by_force_realize_losses` - 208s
- `panic_settle_clamps_negative_pnl` - 245s
- `proof_c1_conservation_bounded_slack_panic_settle` - 323s
- `fast_valid_preserved_by_apply_adl` - 518s
- `proof_c1_conservation_bounded_slack_force_realize` - 540s

## Full Timing Results

| Proof Name | Time (s) | Status |
|------------|----------|--------|
| i1_adl_never_reduces_principal | 2s | PASS |
| fast_i2_deposit_preserves_conservation | 2s | PASS |
| fast_i2_withdraw_preserves_conservation | 2s | PASS |
| i5_warmup_determinism | 3s | PASS |
| i5_warmup_monotonicity | 3s | PASS |
| i5_warmup_bounded_by_pnl | 1s | PASS |
| i7_user_isolation_deposit | 2s | PASS |
| i7_user_isolation_withdrawal | 2s | PASS |
| i8_equity_with_positive_pnl | 1s | PASS |
| i8_equity_with_negative_pnl | 1s | PASS |
| i4_adl_haircuts_unwrapped_first | 22s | PASS |
| withdrawal_requires_sufficient_balance | 2s | PASS |
| pnl_withdrawal_requires_warmup | 2s | PASS |
| multiple_users_adl_preserves_all_principals | 25s | PASS |
| saturating_arithmetic_prevents_overflow | 1s | PASS |
| zero_pnl_withdrawable_is_zero | 1s | PASS |
| negative_pnl_withdrawable_is_zero | 1s | PASS |
| funding_p1_settlement_idempotent | 11s | PASS |
| funding_p2_never_touches_principal | 2s | PASS |
| funding_p3_bounded_drift_between_opposite_positions | 4s | PASS |
| funding_p4_settle_before_position_change | 9s | PASS |
| funding_p5_bounded_operations_no_overflow | 1s | PASS |
| funding_zero_position_no_change | 1s | PASS |
| i10_risk_mode_triggers_at_floor | 8s | PASS |
| i10_fair_unwinding_constant_haircut_ratio | 0s | TIMEOUT |
| i10_withdrawal_mode_blocks_position_increase | 10s | PASS |
| i10_withdrawal_mode_allows_position_decrease | 4s | PASS |
| i10_total_withdrawals_bounded_by_available | 1s | TIMEOUT |
| i10_top_up_reduces_loss_accum | 0s | TIMEOUT |
| i10_top_up_exits_withdrawal_mode_when_loss_zero | 1s | PASS |
| fast_i10_withdrawal_mode_preserves_conservation | 3s | PASS |
| i10_withdrawal_tracking_accuracy | 0s | TIMEOUT |
| i1_lp_adl_never_reduces_capital | 16s | PASS |
| adl_is_proportional_for_user_and_lp | 22s | PASS |
| adl_proportionality_general | 1s | TIMEOUT |
| i10_fair_unwinding_is_fair_for_lps | 0s | TIMEOUT |
| multiple_lps_adl_preserves_all_capitals | 27s | PASS |
| mixed_users_and_lps_adl_preserves_all_capitals | 30s | PASS |
| proof_warmup_frozen_when_paused | 7s | PASS |
| proof_withdraw_only_decreases_via_conversion | 3s | PASS |
| proof_risk_increasing_trades_rejected | 9s | PASS |
| panic_settle_closes_all_positions | 3s | PASS |
| panic_settle_clamps_negative_pnl | 245s | PASS |
| panic_settle_enters_risk_mode | 2s | PASS |
| panic_settle_preserves_conservation | 660s | TIMEOUT |
| warmup_budget_a_invariant_holds_after_settlement | 2s | PASS |
| warmup_budget_b_negative_settlement_no_increase_pos | 2s | PASS |
| warmup_budget_c_positive_settlement_bounded_by_budget | 2s | PASS |
| warmup_budget_d_paused_settlement_time_invariant | 3s | PASS |
| audit_settle_idempotent_when_paused | 3s | PASS |
| audit_warmup_started_at_updated_to_effective_slot | 2s | PASS |
| audit_multiple_settlements_when_paused_idempotent | 4s | PASS |
| proof_r1_adl_never_spends_reserved | 3s | PASS |
| proof_r2_reserved_bounded_and_monotone | 3s | PASS |
| proof_r3_warmup_reservation_safety | 3s | PASS |
| proof_ps5_panic_settle_no_insurance_minting | 77s | PASS |
| proof_c1_conservation_bounded_slack_panic_settle | 323s | PASS |
| proof_c1_conservation_bounded_slack_force_realize | 540s | PASS |
| audit_force_realize_updates_warmup_start | 3s | PASS |
| proof_warmup_slope_nonzero_when_positive_pnl | 2s | PASS |
| proof_reserved_equals_derived_formula | 2s | PASS |
| proof_adl_exact_haircut_distribution | 15s | PASS |
| fast_proof_adl_reserved_invariant | 12s | PASS |
| fast_proof_adl_conservation | 18s | PASS |
| fast_frame_touch_account_only_mutates_one_account | 2s | PASS |
| fast_frame_deposit_only_mutates_one_account_vault_and_warmup | 2s | PASS |
| fast_frame_withdraw_only_mutates_one_account_vault_and_warmup | 2s | PASS |
| fast_frame_execute_trade_only_mutates_two_accounts | 4s | PASS |
| fast_frame_top_up_only_mutates_vault_insurance_loss_mode | 1s | PASS |
| fast_frame_enter_risk_mode_only_mutates_flags | 1s | PASS |
| fast_frame_apply_adl_never_changes_any_capital | 24s | PASS |
| fast_frame_settle_warmup_only_mutates_one_account_and_warmup_globals | 2s | PASS |
| fast_frame_update_warmup_slope_only_mutates_one_account | 1s | PASS |
| fast_valid_preserved_by_deposit | 2s | PASS |
| fast_valid_preserved_by_withdraw | 2s | PASS |
| fast_valid_preserved_by_execute_trade | 7s | PASS |
| fast_valid_preserved_by_apply_adl | 518s | PASS |
| fast_valid_preserved_by_settle_warmup_to_capital | 3s | PASS |
| fast_valid_preserved_by_panic_settle_all | 183s | PASS |
| fast_valid_preserved_by_force_realize_losses | 208s | PASS |
| fast_valid_preserved_by_top_up_insurance_fund | 1s | PASS |
| fast_neg_pnl_settles_into_capital_independent_of_warm_cap | 2s | PASS |
| fast_withdraw_cannot_bypass_losses_when_position_zero | 2s | PASS |
| fast_neg_pnl_after_settle_implies_zero_capital | 1s | PASS |
| neg_pnl_settlement_does_not_depend_on_elapsed_or_slope | 3s | PASS |
| withdraw_calls_settle_enforces_pnl_or_zero_capital_post | 2s | PASS |
| fast_maintenance_margin_uses_equity_including_negative_pnl | 3s | PASS |
| fast_account_equity_computes_correctly | 1s | PASS |
| withdraw_im_check_blocks_when_equity_after_withdraw_below_im | 1s | PASS |
| maintenance_margin_uses_equity_negative_pnl | 1s | PASS |
| neg_pnl_is_realized_immediately_by_settle | 2s | PASS |
