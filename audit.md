# Kani Proof Timing Report
Generated: 2025-12-21

## Summary

- **Total Proofs**: 86
- **Passed**: 86
- **Failed**: 0
- **Timeout**: 0
- **Slow (>60s)**: 7

### Proofs Needing Attention

**Slow (>60s)**:
- `fast_valid_preserved_by_force_realize_losses` - 603s
- `proof_c1_conservation_bounded_slack_force_realize` - 539s
- `fast_valid_preserved_by_apply_adl` - 467s
- `fast_valid_preserved_by_panic_settle_all` - 372s
- `panic_settle_clamps_negative_pnl` - 287s
- `proof_c1_conservation_bounded_slack_panic_settle` - 268s
- `proof_ps5_panic_settle_no_insurance_minting` - 117s

## Full Timing Results

| Proof Name | Time (s) | Status |
|------------|----------|--------|
| security_goal_no_overwithdraw_beyond_other_losses_and_insurance | 2s | PASS |
| neg_pnl_is_realized_immediately_by_settle | 0s | PASS |
| maintenance_margin_uses_equity_negative_pnl | 0s | PASS |
| withdraw_im_check_blocks_when_equity_after_withdraw_below_im | 0s | PASS |
| fast_account_equity_computes_correctly | 0s | PASS |
| fast_maintenance_margin_uses_equity_including_negative_pnl | 2s | PASS |
| withdraw_calls_settle_enforces_pnl_or_zero_capital_post | 0s | PASS |
| neg_pnl_settlement_does_not_depend_on_elapsed_or_slope | 1s | PASS |
| fast_neg_pnl_after_settle_implies_zero_capital | 0s | PASS |
| fast_withdraw_cannot_bypass_losses_when_position_zero | 0s | PASS |
| fast_neg_pnl_settles_into_capital_independent_of_warm_cap | 0s | PASS |
| fast_valid_preserved_by_top_up_insurance_fund | 0s | PASS |
| fast_valid_preserved_by_force_realize_losses | 603s | PASS |
| fast_valid_preserved_by_panic_settle_all | 372s | PASS |
| fast_valid_preserved_by_settle_warmup_to_capital | 1s | PASS |
| fast_valid_preserved_by_apply_adl | 467s | PASS |
| fast_valid_preserved_by_execute_trade | 5s | PASS |
| fast_valid_preserved_by_withdraw | 0s | PASS |
| fast_valid_preserved_by_deposit | 0s | PASS |
| fast_frame_update_warmup_slope_only_mutates_one_account | 0s | PASS |
| fast_frame_settle_warmup_only_mutates_one_account_and_warmup_globals | 0s | PASS |
| fast_frame_apply_adl_never_changes_any_capital | 12s | PASS |
| fast_frame_enter_risk_mode_only_mutates_flags | 0s | PASS |
| fast_frame_top_up_only_mutates_vault_insurance_loss_mode | 0s | PASS |
| fast_frame_execute_trade_only_mutates_two_accounts | 2s | PASS |
| fast_frame_withdraw_only_mutates_one_account_vault_and_warmup | 0s | PASS |
| fast_frame_deposit_only_mutates_one_account_vault_and_warmup | 0s | PASS |
| fast_frame_touch_account_only_mutates_one_account | 1s | PASS |
| fast_proof_adl_conservation | 16s | PASS |
| fast_proof_adl_reserved_invariant | 10s | PASS |
| proof_adl_exact_haircut_distribution | 13s | PASS |
| proof_reserved_equals_derived_formula | 0s | PASS |
| proof_warmup_slope_nonzero_when_positive_pnl | 0s | PASS |
| audit_force_realize_updates_warmup_start | 1s | PASS |
| proof_c1_conservation_bounded_slack_force_realize | 539s | PASS |
| proof_c1_conservation_bounded_slack_panic_settle | 268s | PASS |
| proof_ps5_panic_settle_no_insurance_minting | 117s | PASS |
| proof_r3_warmup_reservation_safety | 1s | PASS |
| proof_r2_reserved_bounded_and_monotone | 2s | PASS |
| proof_r1_adl_never_spends_reserved | 0s | PASS |
| audit_multiple_settlements_when_paused_idempotent | 3s | PASS |
| audit_warmup_started_at_updated_to_effective_slot | 0s | PASS |
| audit_settle_idempotent_when_paused | 2s | PASS |
| warmup_budget_d_paused_settlement_time_invariant | 1s | PASS |
| warmup_budget_c_positive_settlement_bounded_by_budget | 0s | PASS |
| warmup_budget_b_negative_settlement_no_increase_pos | 0s | PASS |
| warmup_budget_a_invariant_holds_after_settlement | 0s | PASS |
| panic_settle_preserves_conservation | 1s | PASS |
| panic_settle_enters_risk_mode | 0s | PASS |
| panic_settle_clamps_negative_pnl | 287s | PASS |
| panic_settle_closes_all_positions | 1s | PASS |
| proof_risk_increasing_trades_rejected | 6s | PASS |
| proof_withdraw_only_decreases_via_conversion | 0s | PASS |
| proof_warmup_frozen_when_paused | 6s | PASS |
| mixed_users_and_lps_adl_preserves_all_capitals | 20s | PASS |
| multiple_lps_adl_preserves_all_capitals | 21s | PASS |
| adl_is_proportional_for_user_and_lp | 19s | PASS |
| i1_lp_adl_never_reduces_capital | 12s | PASS |
| fast_i10_withdrawal_mode_preserves_conservation | 0s | PASS |
| i10_top_up_exits_withdrawal_mode_when_loss_zero | 0s | PASS |
| i10_withdrawal_mode_allows_position_decrease | 2s | PASS |
| i10_withdrawal_mode_blocks_position_increase | 6s | PASS |
| i10_risk_mode_triggers_at_floor | 5s | PASS |
| funding_zero_position_no_change | 0s | PASS |
| funding_p5_bounded_operations_no_overflow | 0s | PASS |
| funding_p4_settle_before_position_change | 7s | PASS |
| funding_p3_bounded_drift_between_opposite_positions | 1s | PASS |
| funding_p2_never_touches_principal | 1s | PASS |
| funding_p1_settlement_idempotent | 8s | PASS |
| negative_pnl_withdrawable_is_zero | 0s | PASS |
| zero_pnl_withdrawable_is_zero | 0s | PASS |
| saturating_arithmetic_prevents_overflow | 0s | PASS |
| multiple_users_adl_preserves_all_principals | 26s | PASS |
| pnl_withdrawal_requires_warmup | 0s | PASS |
| withdrawal_requires_sufficient_balance | 0s | PASS |
| i4_adl_haircuts_unwrapped_first | 19s | PASS |
| i8_equity_with_negative_pnl | 0s | PASS |
| i8_equity_with_positive_pnl | 0s | PASS |
| i7_user_isolation_withdrawal | 0s | PASS |
| i7_user_isolation_deposit | 0s | PASS |
| i5_warmup_bounded_by_pnl | 0s | PASS |
| i5_warmup_monotonicity | 1s | PASS |
| i5_warmup_determinism | 2s | PASS |
| fast_i2_withdraw_preserves_conservation | 0s | PASS |
| fast_i2_deposit_preserves_conservation | 0s | PASS |
| i1_adl_never_reduces_principal | 0s | PASS |
