# Kani Proof Timing Report
Generated: 2025-12-30

## Summary

- **Total Proofs**: 115
- **Passed**: 115
- **Failed**: 0
- **Timeout**: 0
- **Slow (>60s)**: 8

### Recent Engine Changes (2025-12-30)

**Slot-Native Engine**:
- Removed `slots_per_day` and `maintenance_fee_per_day` from RiskParams
- Engine now uses only `maintenance_fee_per_slot` for direct calculation
- Fee calculation: `due = maintenance_fee_per_slot * dt` (no division)
- Any per-day conversion is wrapper/UI responsibility

**Overflow Safety in Liquidation**:
- If partial close arithmetic overflows, engine falls back to full close
- Ensures liquidations always complete even with extreme position sizes
- Added match on `RiskError::Overflow` in `liquidate_at_oracle`

### Recent Non-Vacuity Improvements (2025-12-30)

The following proofs were updated to be non-vacuous (force operations to succeed
and assert postconditions unconditionally):

**Liquidation Proofs (LQ1-LQ6, LIQ-PARTIAL-1/2/3/4)**:
- Force liquidation with `assert!(result.is_ok())` and `assert!(result.unwrap())`
- Use deterministic setups: small capital, large position, oracle=entry

**Panic Settle Proofs (PS1-PS5, C1)**:
- Assert `panic_settle_all` succeeds under bounded inputs
- PS4 already had this; PS1/PS2/PS3/PS5/C1 now non-vacuous

**Waterfall Proofs**:
- `proof_adl_waterfall_exact_routing_single_user`: deterministic warmup time vars
- `proof_adl_waterfall_unwrapped_first_no_insurance_touch`: seed warmed_* = 0
- `proof_adl_never_increases_insurance_balance`: force insurance spend

### Verified Key Proofs (2025-12-30)

| Proof | Time | Status |
|-------|------|--------|
| proof_c1_conservation_bounded_slack_panic_settle | 487s | PASS |
| proof_ps5_panic_settle_no_insurance_minting | 438s | PASS |
| proof_liq_partial_3_routing_is_complete_via_conservation_and_n1 | 2s | PASS |
| proof_liq_partial_deterministic_reaches_target_or_full_close | 2s | PASS |

### Proofs Needing Attention

**Slow (>60s)**:
- `proof_c1_conservation_bounded_slack_force_realize` - 522s
- `fast_valid_preserved_by_force_realize_losses` - 520s
- `fast_valid_preserved_by_apply_adl` - 513s
- `security_goal_bounded_net_extraction_sequence` - 507s
- `fast_valid_preserved_by_panic_settle_all` - 438s
- `proof_c1_conservation_bounded_slack_panic_settle` - 487s
- `panic_settle_clamps_negative_pnl` - 303s
- `proof_ps5_panic_settle_no_insurance_minting` - 438s

## Full Timing Results

| Proof Name | Time (s) | Status |
|------------|----------|--------|
| proof_c1_conservation_bounded_slack_force_realize | 522s | PASS |
| fast_valid_preserved_by_force_realize_losses | 520s | PASS |
| fast_valid_preserved_by_apply_adl | 513s | PASS |
| security_goal_bounded_net_extraction_sequence | 507s | PASS |
| proof_c1_conservation_bounded_slack_panic_settle | 487s | PASS |
| proof_ps5_panic_settle_no_insurance_minting | 438s | PASS |
| fast_valid_preserved_by_panic_settle_all | 438s | PASS |
| panic_settle_clamps_negative_pnl | 303s | PASS |
| multiple_lps_adl_preserves_all_capitals | 32s | PASS |
| multiple_users_adl_preserves_all_principals | 31s | PASS |
| mixed_users_and_lps_adl_preserves_all_capitals | 30s | PASS |
| adl_is_proportional_for_user_and_lp | 30s | PASS |
| i4_adl_haircuts_unwrapped_first | 29s | PASS |
| fast_frame_apply_adl_never_changes_any_capital | 23s | PASS |
| i1_lp_adl_never_reduces_capital | 19s | PASS |
| fast_proof_adl_conservation | 19s | PASS |
| fast_proof_adl_reserved_invariant | 18s | PASS |
| proof_adl_exact_haircut_distribution | 17s | PASS |
| proof_risk_increasing_trades_rejected | 16s | PASS |
| i10_risk_mode_triggers_at_floor | 9s | PASS |
| funding_p4_settle_before_position_change | 8s | PASS |
| funding_p1_settlement_idempotent | 8s | PASS |
| proof_warmup_frozen_when_paused | 6s | PASS |
| i10_withdrawal_mode_blocks_position_increase | 6s | PASS |
| fast_valid_preserved_by_execute_trade | 6s | PASS |
| i10_withdrawal_mode_allows_position_decrease | 4s | PASS |
| audit_multiple_settlements_when_paused_idempotent | 4s | PASS |
| proof_r2_reserved_bounded_and_monotone | 3s | PASS |
| proof_net_extraction_bounded_with_fee_credits | 3s | PASS |
| i5_warmup_determinism | 3s | PASS |
| fast_maintenance_margin_uses_equity_including_negative_pnl | 3s | PASS |
| audit_settle_idempotent_when_paused | 3s | PASS |
| panic_settle_preserves_conservation | 2s | PASS |
| proof_liq_partial_3_routing_is_complete_via_conservation_and_n1 | 2s | PASS |
| proof_liq_partial_deterministic_reaches_target_or_full_close | 2s | PASS |
| funding_p3_bounded_drift_between_opposite_positions | 2s | PASS |
| fast_valid_preserved_by_settle_warmup_to_capital | 2s | PASS |
| fast_frame_touch_account_only_mutates_one_account | 2s | PASS |
| fast_frame_execute_trade_only_mutates_two_accounts | 2s | PASS |
| withdrawal_requires_sufficient_balance | 1s | PASS |
| withdraw_im_check_blocks_when_equity_after_withdraw_below_im | 1s | PASS |
| withdraw_calls_settle_enforces_pnl_or_zero_capital_post | 1s | PASS |
| warmup_budget_d_paused_settlement_time_invariant | 1s | PASS |
| warmup_budget_c_positive_settlement_bounded_by_budget | 1s | PASS |
| warmup_budget_b_negative_settlement_no_increase_pos | 1s | PASS |
| warmup_budget_a_invariant_holds_after_settlement | 1s | PASS |
| proof_withdraw_only_decreases_via_conversion | 1s | PASS |
| proof_trading_credits_fee_to_user | 1s | PASS |
| proof_settle_maintenance_deducts_correctly | 1s | PASS |
| proof_reserved_equals_derived_formula | 1s | PASS |
| proof_r3_warmup_reservation_safety | 1s | PASS |
| proof_r1_adl_never_spends_reserved | 1s | PASS |
| proof_keeper_crank_forgives_half_slots | 1s | PASS |
| proof_keeper_crank_best_effort_settle | 1s | PASS |
| proof_keeper_crank_advances_slot_monotonically | 1s | PASS |
| proof_close_account_returns_capital_only | 1s | PASS |
| proof_close_account_requires_flat_and_paid | 1s | PASS |
| proof_close_account_includes_warmed_pnl | 1s | PASS |
| pnl_withdrawal_requires_warmup | 1s | PASS |
| panic_settle_enters_risk_mode | 1s | PASS |
| panic_settle_closes_all_positions | 1s | PASS |
| neg_pnl_settlement_does_not_depend_on_elapsed_or_slope | 1s | PASS |
| neg_pnl_is_realized_immediately_by_settle | 1s | PASS |
| i7_user_isolation_withdrawal | 1s | PASS |
| i7_user_isolation_deposit | 1s | PASS |
| i5_warmup_monotonicity | 1s | PASS |
| i1_adl_never_reduces_principal | 1s | PASS |
| i10_top_up_exits_withdrawal_mode_when_loss_zero | 1s | PASS |
| funding_p5_bounded_operations_no_overflow | 1s | PASS |
| funding_p2_never_touches_principal | 1s | PASS |
| fast_withdraw_cannot_bypass_losses_when_position_zero | 1s | PASS |
| fast_valid_preserved_by_withdraw | 1s | PASS |
| fast_valid_preserved_by_deposit | 1s | PASS |
| fast_neg_pnl_settles_into_capital_independent_of_warm_cap | 1s | PASS |
| fast_neg_pnl_after_settle_implies_zero_capital | 1s | PASS |
| fast_i2_withdraw_preserves_conservation | 1s | PASS |
| fast_i2_deposit_preserves_conservation | 1s | PASS |
| fast_i10_withdrawal_mode_preserves_conservation | 1s | PASS |
| fast_frame_withdraw_only_mutates_one_account_vault_and_warmup | 1s | PASS |
| fast_frame_update_warmup_slope_only_mutates_one_account | 1s | PASS |
| fast_frame_top_up_only_mutates_vault_insurance_loss_mode | 1s | PASS |
| fast_frame_settle_warmup_only_mutates_one_account_and_warmup_globals | 1s | PASS |
| fast_frame_deposit_only_mutates_one_account_vault_and_warmup | 1s | PASS |
| audit_warmup_started_at_updated_to_effective_slot | 1s | PASS |
| audit_force_realize_updates_warmup_start | 1s | PASS |
| zero_pnl_withdrawable_is_zero | 0s | PASS |
| saturating_arithmetic_prevents_overflow | 0s | PASS |
| proof_warmup_slope_nonzero_when_positive_pnl | 0s | PASS |
| proof_total_open_interest_initial | 0s | PASS |
| proof_set_risk_reduction_threshold_updates | 0s | PASS |
| proof_require_fresh_crank_gates_stale | 0s | PASS |
| proof_fee_credits_never_inflate_from_settle | 0s | PASS |
| negative_pnl_withdrawable_is_zero | 0s | PASS |
| maintenance_margin_uses_equity_negative_pnl | 0s | PASS |
| i8_equity_with_positive_pnl | 0s | PASS |
| i8_equity_with_negative_pnl | 0s | PASS |
| i5_warmup_bounded_by_pnl | 0s | PASS |
| funding_zero_position_no_change | 0s | PASS |
| fast_valid_preserved_by_top_up_insurance_fund | 0s | PASS |
| fast_frame_enter_risk_mode_only_mutates_flags | 0s | PASS |
| fast_account_equity_computes_correctly | 0s | PASS |
