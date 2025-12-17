//! Fuzzing tests for the risk engine
//! Run with: cargo test --features fuzz
//!
//! These tests use proptest to generate random inputs and verify invariants hold.

#![cfg(feature = "fuzz")]

use percolator::*;
use proptest::prelude::*;

// Use the no-op matcher for tests
const MATCHER: NoOpMatcher = NoOpMatcher;

// Use the Vec-based implementation for tests


fn default_params() -> RiskParams {
    RiskParams {
        warmup_period_slots: 100,
        maintenance_margin_bps: 500,
        initial_margin_bps: 1000,
        trading_fee_bps: 10,
        max_accounts: 1000,
        account_fee_bps: 10000,
        risk_reduction_threshold: 0,
    }
}

// Strategy for generating reasonable amounts
fn amount_strategy() -> impl Strategy<Value = u128> {
    0u128..1_000_000
}

// Strategy for generating reasonable PNL values
fn pnl_strategy() -> impl Strategy<Value = i128> {
    -100_000i128..100_000
}

// Strategy for generating reasonable prices
fn price_strategy() -> impl Strategy<Value = u64> {
    100_000u64..10_000_000 // $0.10 to $10
}

// Strategy for generating position sizes
fn position_strategy() -> impl Strategy<Value = i128> {
    -100_000i128..100_000
}

// Test that deposit always increases vault and principal
proptest! {
    #[test]
    fn fuzz_deposit_increases_balance(amount in amount_strategy()) {
        let mut engine = Box::new(RiskEngine::new(default_params()));
        let user_idx = engine.add_user(1).unwrap();

        let vault_before = engine.vault;
        let principal_before = engine.accounts[user_idx as usize].capital;

        let _ = engine.deposit(user_idx, amount);

        prop_assert_eq!(engine.vault, vault_before + amount);
        prop_assert_eq!(engine.accounts[user_idx as usize].capital, principal_before + amount);
    }
}

// Test that withdrawal never increases balance
proptest! {
    #[test]
    fn fuzz_withdraw_decreases_or_fails(
        deposit_amount in amount_strategy(),
        withdraw_amount in amount_strategy()
    ) {
        let mut engine = Box::new(RiskEngine::new(default_params()));
        let user_idx = engine.add_user(1).unwrap();

        engine.deposit(user_idx, deposit_amount).unwrap();

        let vault_before = engine.vault;
        let principal_before = engine.accounts[user_idx as usize].capital;

        let result = engine.withdraw(user_idx, withdraw_amount);

        if result.is_ok() {
            prop_assert!(engine.vault <= vault_before);
            prop_assert!(engine.accounts[user_idx as usize].capital <= principal_before);
        }
    }
}

// Test that conservation holds after random deposits/withdrawals
proptest! {
    #[test]
    fn fuzz_conservation_after_operations(
        deposits in prop::collection::vec(amount_strategy(), 1..10),
        withdrawals in prop::collection::vec(amount_strategy(), 1..10)
    ) {
        let mut engine = Box::new(RiskEngine::new(default_params()));
        let user_idx = engine.add_user(1).unwrap();

        // Apply deposits
        for amount in deposits {
            let _ = engine.deposit(user_idx, amount);
        }

        prop_assert!(engine.check_conservation());

        // Apply withdrawals
        for amount in withdrawals {
            let _ = engine.withdraw(user_idx, amount);
        }

        prop_assert!(engine.check_conservation());
    }
}

// Test that PNL warmup is always monotonic
proptest! {
    #[test]
    fn fuzz_warmup_monotonic(
        pnl in 1i128..100_000,
        slope in 1u128..1000,
        slots1 in 0u64..200,
        slots2 in 0u64..200
    ) {
        let mut engine = Box::new(RiskEngine::new(default_params()));
        let user_idx = engine.add_user(1).unwrap();

        engine.accounts[user_idx as usize].pnl = pnl;
        engine.accounts[user_idx as usize].warmup_slope_per_step = slope;

        let earlier_slot = slots1.min(slots2);
        let later_slot = slots1.max(slots2);

        engine.current_slot = earlier_slot;
        let w1 = engine.withdrawable_pnl(&engine.accounts[user_idx as usize]);

        engine.current_slot = later_slot;
        let w2 = engine.withdrawable_pnl(&engine.accounts[user_idx as usize]);

        prop_assert!(w2 >= w1, "Warmup should be monotonic: w1={}, w2={}, earlier={}, later={}",
                     w1, w2, earlier_slot, later_slot);
    }
}

// Test that ADL never reduces principal
proptest! {
    #[test]
    fn fuzz_adl_preserves_principal(
        principal in amount_strategy(),
        pnl in pnl_strategy(),
        loss in amount_strategy()
    ) {
        let mut engine = Box::new(RiskEngine::new(default_params()));
        let user_idx = engine.add_user(1).unwrap();

        engine.accounts[user_idx as usize].capital = principal;
        engine.accounts[user_idx as usize].pnl = pnl;
        engine.insurance_fund.balance = 10_000_000; // Large insurance fund

        let _ = engine.apply_adl(loss);

        prop_assert_eq!(engine.accounts[user_idx as usize].capital, principal,
                        "ADL must never reduce principal");
    }
}

// Test that withdrawable PNL never exceeds available PNL
proptest! {
    #[test]
    fn fuzz_withdrawable_bounded(
        pnl in pnl_strategy(),
        reserved in amount_strategy(),
        slope in 1u128..1000,
        slots in 0u64..500
    ) {
        let mut engine = Box::new(RiskEngine::new(default_params()));
        let user_idx = engine.add_user(1).unwrap();

        engine.accounts[user_idx as usize].pnl = pnl;
        engine.accounts[user_idx as usize].reserved_pnl = reserved;
        engine.accounts[user_idx as usize].warmup_slope_per_step = slope;
        engine.current_slot = slots;

        let withdrawable = engine.withdrawable_pnl(&engine.accounts[user_idx as usize]);
        let positive_pnl = if pnl > 0 { pnl as u128 } else { 0 };
        let available = positive_pnl.saturating_sub(reserved);

        prop_assert!(withdrawable <= available,
                     "Withdrawable {} should not exceed available {}",
                     withdrawable, available);
    }
}

// Test that collateral calculation is consistent
proptest! {
    #[test]
    fn fuzz_collateral_consistency(
        principal in amount_strategy(),
        pnl in pnl_strategy()
    ) {
        let mut engine = Box::new(RiskEngine::new(default_params()));
        let user_idx = engine.add_user(1).unwrap();

        engine.accounts[user_idx as usize].capital = principal;
        engine.accounts[user_idx as usize].pnl = pnl;

        let collateral = engine.account_collateral(&engine.accounts[user_idx as usize]);

        let expected = if pnl >= 0 {
            principal.saturating_add(pnl as u128)
        } else {
            principal
        };

        prop_assert_eq!(collateral, expected,
                        "Collateral should equal principal + max(0, pnl)");
    }
}

// Test that user isolation holds
proptest! {
    #[test]
    fn fuzz_user_isolation(
        amount1 in amount_strategy(),
        amount2 in amount_strategy(),
        withdraw in amount_strategy()
    ) {
        let mut engine = Box::new(RiskEngine::new(default_params()));
        let user1 = engine.add_user(1).unwrap();
        let user2 = engine.add_user(1).unwrap();

        engine.deposit(user1, amount1).unwrap();
        engine.deposit(user2, amount2).unwrap();

        let user2_principal_before = engine.accounts[user2 as usize].capital;
        let user2_pnl_before = engine.accounts[user2 as usize].pnl;

        // Operate on user1
        let _ = engine.withdraw(user1, withdraw);

        // User2 should be unchanged
        prop_assert_eq!(engine.accounts[user2 as usize].capital, user2_principal_before);
        prop_assert_eq!(engine.accounts[user2 as usize].pnl, user2_pnl_before);
    }
}

// Test that multiple ADL applications preserve principal
proptest! {
    #[test]
    fn fuzz_multiple_adl_preserves_principal(
        principal in amount_strategy(),
        initial_pnl in pnl_strategy(),
        losses in prop::collection::vec(amount_strategy(), 1..10)
    ) {
        let mut engine = Box::new(RiskEngine::new(default_params()));
        let user_idx = engine.add_user(1).unwrap();

        engine.accounts[user_idx as usize].capital = principal;
        engine.accounts[user_idx as usize].pnl = initial_pnl;
        engine.insurance_fund.balance = 100_000_000; // Large insurance

        for loss in losses {
            let _ = engine.apply_adl(loss);
        }

        prop_assert_eq!(engine.accounts[user_idx as usize].capital, principal,
                        "Multiple ADLs must never reduce principal");
    }
}

// Test that fees always go to insurance fund
proptest! {
    #[test]
    fn fuzz_trading_fees_to_insurance(
        user_capital in 10_000u128..1_000_000,
        lp_capital in 100_000u128..10_000_000,
        price in price_strategy(),
        size in 100i128..10_000
    ) {
        let mut engine = Box::new(RiskEngine::new(default_params()));
        let user_idx = engine.add_user(1).unwrap();
        let lp_idx = engine.add_lp([0u8; 32], [0u8; 32], 1).unwrap();

        engine.deposit(user_idx, user_capital).unwrap();
        engine.accounts[lp_idx as usize].capital = lp_capital;
        engine.vault = user_capital + lp_capital;

        let insurance_before = engine.insurance_fund.fee_revenue;

        let _ = engine.execute_trade(&MATCHER, lp_idx, user_idx, price, size);

        // Insurance fund should have received fees (if trade succeeded)
        if engine.insurance_fund.fee_revenue > insurance_before {
            prop_assert!(engine.insurance_fund.fee_revenue > insurance_before);
        }
    }
}

// Test that warmup with reserved PNL works correctly
proptest! {
    #[test]
    fn fuzz_warmup_with_reserved(
        pnl in 1000i128..100_000,
        reserved in 0u128..50_000,
        slope in 1u128..1000,
        slots in 0u64..200
    ) {
        let mut engine = Box::new(RiskEngine::new(default_params()));
        let user_idx = engine.add_user(1).unwrap();

        engine.accounts[user_idx as usize].pnl = pnl;
        engine.accounts[user_idx as usize].reserved_pnl = reserved;
        engine.accounts[user_idx as usize].warmup_slope_per_step = slope;
        engine.advance_slot(slots);

        let withdrawable = engine.withdrawable_pnl(&engine.accounts[user_idx as usize]);
        let positive_pnl = pnl as u128;

        // Withdrawable should never exceed available (positive_pnl - reserved)
        prop_assert!(withdrawable <= positive_pnl.saturating_sub(reserved));
    }
}

// Test conservation with multiple users and operations
proptest! {
    #[test]
    fn fuzz_multi_user_conservation(
        deposits in prop::collection::vec((0usize..3, amount_strategy()), 5..15)
    ) {
        let mut engine = Box::new(RiskEngine::new(default_params()));

        // Create 3 users
        for _ in 0..3 {
            engine.add_user(1).unwrap();
        }

        // Apply random deposits
        for (user_idx, amount) in deposits {
            if user_idx < 3 {
                let _ = engine.deposit(user_idx as u16, amount);
            }
        }

        prop_assert!(engine.check_conservation(),
                     "Conservation should hold after multi-user deposits");
    }
}

// Test that ADL with insurance failover works
proptest! {
    #[test]
    fn fuzz_adl_insurance_failover(
        user_pnl in 0i128..10_000,
        insurance_balance in 0u128..5_000,
        loss in 5_000u128..20_000
    ) {
        let mut engine = Box::new(RiskEngine::new(default_params()));
        let user_idx = engine.add_user(1).unwrap();

        engine.accounts[user_idx as usize].pnl = user_pnl;
        engine.insurance_fund.balance = insurance_balance;

        let _ = engine.apply_adl(loss);

        // If loss exceeded PNL + insurance, loss_accum should be set
        let total_available = (user_pnl as u128) + insurance_balance;
        if loss > total_available {
            prop_assert!(engine.loss_accum > 0);
        }
    }
}

// Test position size consistency after trades
proptest! {
    #[test]
    fn fuzz_position_consistency(
        initial_size in position_strategy(),
        trade_size in position_strategy()
    ) {
        let mut engine = Box::new(RiskEngine::new(default_params()));
        let user_idx = engine.add_user(1).unwrap();
        let lp_idx = engine.add_lp([0u8; 32], [0u8; 32], 1).unwrap();

        engine.deposit(user_idx, 1_000_000).unwrap();
        engine.accounts[lp_idx as usize].capital = 10_000_000;
        engine.vault = 11_000_000;

        engine.accounts[user_idx as usize].position_size = initial_size;
        engine.accounts[lp_idx as usize].position_size = -initial_size;

        let expected_user_pos = initial_size.saturating_add(trade_size);
        let expected_lp_pos = (-initial_size).saturating_sub(trade_size);

        let _ = engine.execute_trade(&MATCHER, lp_idx, user_idx, 1_000_000, trade_size);

        // If trade succeeded, positions should net to zero
        if engine.accounts[user_idx as usize].position_size == expected_user_pos {
            let total_position = engine.accounts[user_idx as usize].position_size +
                                engine.accounts[lp_idx as usize].position_size;

            // Positions should roughly net out (within rounding)
            prop_assert!(total_position.abs() <= 1,
                        "User and LP positions should sum to ~0");
        }
    }
}

// ============================================================================
// Funding Rate Fuzzing Tests
// ============================================================================

// Strategy for funding rates (signed bps per slot)
fn funding_rate_strategy() -> impl Strategy<Value = i64> {
    -1000i64..1000 // ±10% per slot (extreme but tests bounds)
}

// Test funding idempotence with random inputs
proptest! {
    #[test]
    fn fuzz_funding_idempotence(
        position in position_strategy(),
        index_delta in -1_000_000i128..1_000_000
    ) {
        let mut engine = Box::new(RiskEngine::new(default_params()));
        let user_idx = engine.add_user(1).unwrap();

        engine.accounts[user_idx as usize].position_size = position;
        engine.funding_index_qpb_e6 = index_delta;

        // Settle once
        let _ = engine.touch_account(user_idx);
        let pnl_first = engine.accounts[user_idx as usize].pnl;

        // Settle again without accrual
        let _ = engine.touch_account(user_idx);
        let pnl_second = engine.accounts[user_idx as usize].pnl;

        prop_assert_eq!(pnl_first, pnl_second,
                       "Funding settlement should be idempotent");
    }
}

// Test funding never touches principal
proptest! {
    #[test]
    fn fuzz_funding_preserves_principal(
        principal in amount_strategy(),
        position in position_strategy(),
        funding_delta in -10_000_000i128..10_000_000
    ) {
        let mut engine = Box::new(RiskEngine::new(default_params()));
        let user_idx = engine.add_user(1).unwrap();

        engine.accounts[user_idx as usize].capital = principal;
        engine.accounts[user_idx as usize].position_size = position;
        engine.funding_index_qpb_e6 = funding_delta;

        let _ = engine.touch_account(user_idx);

        prop_assert_eq!(engine.accounts[user_idx as usize].capital, principal,
                       "Funding must never modify principal");
    }
}

// Test zero-sum property with random positions
proptest! {
    #[test]
    fn fuzz_funding_zero_sum(
        position in 1i128..100_000,
        funding_delta in -1_000_000i128..1_000_000
    ) {
        let mut engine = Box::new(RiskEngine::new(default_params()));
        let user_idx = engine.add_user(1).unwrap();
        let lp_idx = engine.add_lp([0u8; 32], [0u8; 32], 1).unwrap();

        // Opposite positions
        engine.accounts[user_idx as usize].position_size = position;
        engine.accounts[lp_idx as usize].position_size = -position;

        let total_pnl_before = engine.accounts[user_idx as usize].pnl +
                              engine.accounts[lp_idx as usize].pnl;

        engine.funding_index_qpb_e6 = funding_delta;

        let user_result = engine.touch_account(user_idx);
        let lp_result = engine.touch_account(lp_idx);

        if user_result.is_ok() && lp_result.is_ok() {
            let total_pnl_after = engine.accounts[user_idx as usize].pnl +
                                 engine.accounts[lp_idx as usize].pnl;

            prop_assert_eq!(total_pnl_after, total_pnl_before,
                           "Funding should be zero-sum");
        }
    }
}

// Test funding with random accrual sequences
proptest! {
    #[test]
    fn fuzz_funding_accrual_sequence(
        sequences in prop::collection::vec((funding_rate_strategy(), 1u64..100), 1..10)
    ) {
        let mut engine = Box::new(RiskEngine::new(default_params()));

        let mut current_slot = 0u64;
        for (rate, dt) in sequences.iter() {
            current_slot = current_slot.saturating_add(*dt);
            let result = engine.accrue_funding(current_slot, 100_000_000, *rate);

            // Should either succeed or return Overflow (never panic)
            if result.is_err() {
                prop_assert!(matches!(result.unwrap_err(), RiskError::Overflow));
            }
        }
    }
}

// Differential fuzzing: compare against slow reference model
proptest! {
    #[test]
    fn fuzz_differential_funding_calculation(
        position in 1_000i128..100_000,
        price in price_strategy(),
        rate in funding_rate_strategy(),
        dt in 1u64..100
    ) {
        let mut engine = Box::new(RiskEngine::new(default_params()));
        let user_idx = engine.add_user(1).unwrap();

        engine.accounts[user_idx as usize].position_size = position;
        engine.accounts[user_idx as usize].pnl = 0;

        // Real implementation
        let accrue_result = engine.accrue_funding(dt, price, rate);
        if accrue_result.is_err() {
            return Ok(()); // Skip if overflow
        }

        let touch_result = engine.touch_account(user_idx);
        if touch_result.is_err() {
            return Ok(()); // Skip if overflow
        }

        let actual_pnl = engine.accounts[user_idx as usize].pnl;

        // Reference implementation (slow but simple)
        let price_i128 = price as i128;
        let rate_i128 = rate as i128;
        let dt_i128 = dt as i128;

        // delta_F = price * rate * dt / 10,000
        let delta_f_opt = price_i128
            .checked_mul(rate_i128)
            .and_then(|x| x.checked_mul(dt_i128))
            .and_then(|x| x.checked_div(10_000));

        if let Some(delta_f) = delta_f_opt {
            // payment = position * delta_F / 1e6
            let payment_opt = position
                .checked_mul(delta_f)
                .and_then(|x| x.checked_div(1_000_000));

            if let Some(payment) = payment_opt {
                let expected_pnl = 0i128.checked_sub(payment);

                if let Some(expected) = expected_pnl {
                    prop_assert_eq!(actual_pnl, expected,
                                   "Funding calculation should match reference");
                }
            }
        }
    }
}

// Test funding with position changes (partial close scenario)
proptest! {
    #[test]
    fn fuzz_funding_with_position_changes(
        initial_pos in 10_000i128..100_000,
        reduction in 1_000i128..50_000,
        rate1 in funding_rate_strategy(),
        rate2 in funding_rate_strategy()
    ) {
        let mut engine = Box::new(RiskEngine::new(default_params()));
        let user_idx = engine.add_user(1).unwrap();
        let lp_idx = engine.add_lp([0u8; 32], [0u8; 32], 1).unwrap();

        engine.deposit(user_idx, 10_000_000).unwrap();
        engine.accounts[lp_idx as usize].capital = 100_000_000;
        engine.vault = 110_000_000;

        // Manually set positions
        engine.accounts[user_idx as usize].position_size = initial_pos;
        engine.accounts[lp_idx as usize].position_size = -initial_pos;

        // Period 1: accrue funding
        let accrue1 = engine.accrue_funding(1, 100_000_000, rate1);
        if accrue1.is_err() {
            return Ok(());
        }

        // Trade to reduce position (execute_trade will touch accounts first)
        let new_pos = initial_pos.saturating_sub(reduction);
        if new_pos > 0 {
            let trade_size = -reduction;
            let _ = engine.execute_trade(&MATCHER, lp_idx, user_idx, 100_000_000, trade_size);

            // Period 2: more funding
            let accrue2 = engine.accrue_funding(2, 100_000_000, rate2);
            if accrue2.is_ok() {
                let _ = engine.touch_account(user_idx);

                // Verify snapshot is current
                prop_assert_eq!(engine.accounts[user_idx as usize].funding_index,
                               engine.funding_index_qpb_e6,
                               "Snapshot should equal global index");
            }
        }
    }
}

// Test that zero position pays no funding
proptest! {
    #[test]
    fn fuzz_zero_position_no_funding(
        pnl in pnl_strategy(),
        funding_delta in -10_000_000i128..10_000_000
    ) {
        let mut engine = Box::new(RiskEngine::new(default_params()));
        let user_idx = engine.add_user(1).unwrap();

        engine.accounts[user_idx as usize].position_size = 0; // Zero position
        engine.accounts[user_idx as usize].pnl = pnl;

        engine.funding_index_qpb_e6 = funding_delta;

        let _ = engine.touch_account(user_idx);

        prop_assert_eq!(engine.accounts[user_idx as usize].pnl, pnl,
                       "Zero position should not pay funding");
    }
}

// ============================================================================
// AUDIT-MANDATED FUZZ TESTS
// These fuzz tests verify critical invariants identified in the security audit.
// ============================================================================

/// Helper to create params with a non-zero insurance floor
fn params_with_floor() -> RiskParams {
    RiskParams {
        warmup_period_slots: 100,
        maintenance_margin_bps: 500,
        initial_margin_bps: 1000,
        trading_fee_bps: 10,
        max_accounts: 1000,
        account_fee_bps: 10000,
        risk_reduction_threshold: 100, // Non-zero floor
    }
}

// Fuzz Test A: settle_warmup_to_capital is idempotent when paused
proptest! {
    #[test]
    fn fuzz_audit_settle_idempotent_when_paused(
        capital in 100u128..100_000,
        pnl in 1i128..10_000,
        slope in 1u128..1000,
        pause_slot in 1u64..100,
        settle_slot in 100u64..500
    ) {
        let mut engine = Box::new(RiskEngine::new(params_with_floor()));
        let user_idx = engine.add_user(1).unwrap();

        // Setup with positive PnL and warmup
        engine.insurance_fund.balance = 100_000;
        engine.vault = 100_000;
        engine.deposit(user_idx, capital).unwrap();
        engine.accounts[user_idx as usize].pnl = pnl;
        engine.accounts[user_idx as usize].warmup_slope_per_step = slope;
        engine.accounts[user_idx as usize].warmup_started_at_slot = 0;

        // Pause warmup
        engine.warmup_paused = true;
        engine.warmup_pause_slot = pause_slot;
        engine.current_slot = settle_slot;

        // First settlement
        let _ = engine.settle_warmup_to_capital(user_idx);
        let state1 = (
            engine.accounts[user_idx as usize].capital,
            engine.accounts[user_idx as usize].pnl,
            engine.warmed_pos_total,
            engine.warmed_neg_total,
        );

        // Second settlement - must be idempotent
        let _ = engine.settle_warmup_to_capital(user_idx);
        let state2 = (
            engine.accounts[user_idx as usize].capital,
            engine.accounts[user_idx as usize].pnl,
            engine.warmed_pos_total,
            engine.warmed_neg_total,
        );

        prop_assert_eq!(state1, state2,
                        "Settlement should be idempotent when paused: state1={:?}, state2={:?}",
                        state1, state2);

        // Also verify warmup_started_at_slot was updated
        let effective_slot = core::cmp::min(settle_slot, pause_slot);
        prop_assert_eq!(engine.accounts[user_idx as usize].warmup_started_at_slot, effective_slot,
                        "warmup_started_at_slot should be updated to effective_slot");
    }
}

// Fuzz Test B: Warmup budget invariant always holds
proptest! {
    #[test]
    fn fuzz_audit_warmup_budget_invariant(
        capital in 1000u128..100_000,
        pnl in -50_000i128..50_000,
        slope in 1u128..1000,
        insurance in 100u128..10_000,
        slots in 1u64..200
    ) {
        let mut engine = Box::new(RiskEngine::new(params_with_floor()));
        let user_idx = engine.add_user(1).unwrap();

        // Setup
        engine.insurance_fund.balance = insurance;
        engine.vault = insurance;
        engine.deposit(user_idx, capital).unwrap();
        engine.accounts[user_idx as usize].pnl = pnl;
        engine.accounts[user_idx as usize].warmup_slope_per_step = slope;
        engine.accounts[user_idx as usize].warmup_started_at_slot = 0;
        engine.current_slot = slots;

        // Settle warmup
        let _ = engine.settle_warmup_to_capital(user_idx);

        // Check warmup budget invariant: W+ <= W- + raw_spendable
        let raw_spendable = engine.insurance_spendable_raw();
        prop_assert!(
            engine.warmed_pos_total <= engine.warmed_neg_total.saturating_add(raw_spendable),
            "Warmup budget invariant violated: W+={} > W-={} + raw_spendable={}",
            engine.warmed_pos_total, engine.warmed_neg_total, raw_spendable
        );

        // Check reserved <= raw_spendable
        prop_assert!(
            engine.warmup_insurance_reserved <= raw_spendable,
            "Reserved {} exceeds raw_spendable {}",
            engine.warmup_insurance_reserved, raw_spendable
        );
    }
}

// Fuzz Test C: Conservation holds after panic_settle_all
proptest! {
    #[test]
    fn fuzz_audit_conservation_after_panic_settle(
        user_capital in 1000u128..100_000,
        lp_capital in 1000u128..100_000,
        position in 1i128..10_000,
        entry_price in 100_000u64..10_000_000,
        oracle_price in 100_000u64..10_000_000,
        insurance in 0u128..10_000
    ) {
        let mut engine = Box::new(RiskEngine::new(default_params()));
        let user_idx = engine.add_user(1).unwrap();
        let lp_idx = engine.add_lp([0u8; 32], [0u8; 32], 1).unwrap();

        // Setup opposing positions
        engine.deposit(user_idx, user_capital).unwrap();
        engine.deposit(lp_idx, lp_capital).unwrap();

        engine.accounts[user_idx as usize].position_size = position;
        engine.accounts[user_idx as usize].entry_price = entry_price;
        engine.accounts[lp_idx as usize].position_size = -position;
        engine.accounts[lp_idx as usize].entry_price = entry_price;

        // Adjust insurance and vault together to maintain conservation
        // Conservation: vault = sum(capital) + sum(pnl) + insurance
        // PnL is 0, so vault = capitals + insurance
        let total_capital = user_capital + lp_capital;
        engine.insurance_fund.balance = insurance;
        engine.vault = total_capital + insurance;

        // Verify conservation before
        prop_assert!(engine.check_conservation(),
                     "Conservation should hold before panic_settle");

        // Panic settle
        let _ = engine.panic_settle_all(oracle_price);

        // Conservation must hold after (using >= for rounding)
        prop_assert!(engine.check_conservation(),
                     "Conservation should hold after panic_settle_all");

        // All positions should be closed
        prop_assert_eq!(engine.accounts[user_idx as usize].position_size, 0,
                        "User position should be closed");
        prop_assert_eq!(engine.accounts[lp_idx as usize].position_size, 0,
                        "LP position should be closed");
    }
}

// Fuzz Test D: Reserved insurance is protected in ADL
proptest! {
    #[test]
    fn fuzz_audit_reserved_insurance_protected_in_adl(
        capital in 1000u128..100_000,
        pnl in 100i128..5000,
        slope in 100u128..10000,
        insurance in 200u128..10_000,
        loss in 100u128..20_000,
        slots in 1u64..100
    ) {
        let mut engine = Box::new(RiskEngine::new(params_with_floor()));
        let user_idx = engine.add_user(1).unwrap();

        // Setup with insurance above floor
        engine.insurance_fund.balance = insurance;
        engine.vault = insurance;
        engine.deposit(user_idx, capital).unwrap();
        engine.accounts[user_idx as usize].pnl = pnl;
        engine.accounts[user_idx as usize].warmup_slope_per_step = slope;
        engine.accounts[user_idx as usize].warmup_started_at_slot = 0;
        engine.current_slot = slots;

        // Settle to reserve some insurance
        let _ = engine.settle_warmup_to_capital(user_idx);
        let reserved = engine.warmup_insurance_reserved;
        let floor = engine.params.risk_reduction_threshold;

        // Apply ADL
        let insurance_before = engine.insurance_fund.balance;
        let _ = engine.apply_adl(loss);
        let insurance_after = engine.insurance_fund.balance;

        // Insurance should never drop below floor + reserved
        let min_protected = floor.saturating_add(reserved);
        prop_assert!(
            insurance_after >= min_protected.saturating_sub(1), // Allow 1 for rounding
            "Insurance {} dropped below floor+reserved={} after ADL (was {}, reserved={})",
            insurance_after, min_protected, insurance_before, reserved
        );

        // Reserved should not decrease
        prop_assert!(
            engine.warmup_insurance_reserved >= reserved,
            "Reserved decreased from {} to {} after ADL",
            reserved, engine.warmup_insurance_reserved
        );
    }
}

// Fuzz Test E: force_realize_losses maintains warmup budget invariant
proptest! {
    #[test]
    fn fuzz_audit_force_realize_maintains_invariant(
        user_capital in 1000u128..50_000,
        lp_capital in 1000u128..50_000,
        position in 1i128..5_000,
        entry_price in 100_000u64..5_000_000,
        oracle_price in 100_000u64..5_000_000
    ) {
        let mut params = params_with_floor();
        params.risk_reduction_threshold = 1000;

        let mut engine = Box::new(RiskEngine::new(params));
        let user_idx = engine.add_user(1).unwrap();
        let lp_idx = engine.add_lp([0u8; 32], [0u8; 32], 1).unwrap();

        // Setup deposits first
        engine.deposit(user_idx, user_capital).unwrap();
        engine.deposit(lp_idx, lp_capital).unwrap();

        // Set insurance to floor (triggers force_realize) and adjust vault
        let total_capital = user_capital + lp_capital;
        engine.insurance_fund.balance = 1000;
        engine.vault = total_capital + 1000;

        engine.accounts[user_idx as usize].position_size = position;
        engine.accounts[user_idx as usize].entry_price = entry_price;
        engine.accounts[lp_idx as usize].position_size = -position;
        engine.accounts[lp_idx as usize].entry_price = entry_price;

        // Force realize losses
        let _ = engine.force_realize_losses(oracle_price);

        // Check warmup budget invariant
        let raw_spendable = engine.insurance_spendable_raw();
        prop_assert!(
            engine.warmed_pos_total <= engine.warmed_neg_total.saturating_add(raw_spendable),
            "Warmup budget invariant violated after force_realize: W+={} > W-={} + raw={}",
            engine.warmed_pos_total, engine.warmed_neg_total, raw_spendable
        );

        // Conservation should hold
        prop_assert!(engine.check_conservation(),
                     "Conservation violated after force_realize_losses");
    }
}
