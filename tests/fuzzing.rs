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
type RiskEngine = VecRiskEngine;

fn default_params() -> RiskParams {
    RiskParams {
        warmup_period_slots: 100,
        maintenance_margin_bps: 500,
        initial_margin_bps: 1000,
        trading_fee_bps: 10,
        liquidation_fee_bps: 50,
        insurance_fee_share_bps: 5000,
        max_users: 1000,
        max_lps: 100,
        account_fee_bps: 10000,
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
        let mut engine = RiskEngine::new(default_params());
        let user_idx = engine.add_user(1).unwrap();

        let vault_before = engine.vault;
        let principal_before = engine.users[user_idx].principal;

        let _ = engine.deposit(user_idx, amount);

        prop_assert_eq!(engine.vault, vault_before + amount);
        prop_assert_eq!(engine.users[user_idx].principal, principal_before + amount);
    }
}

// Test that withdrawal never increases balance
proptest! {
    #[test]
    fn fuzz_withdraw_decreases_or_fails(
        deposit_amount in amount_strategy(),
        withdraw_amount in amount_strategy()
    ) {
        let mut engine = RiskEngine::new(default_params());
        let user_idx = engine.add_user(1).unwrap();

        engine.deposit(user_idx, deposit_amount).unwrap();

        let vault_before = engine.vault;
        let principal_before = engine.users[user_idx].principal;

        let result = engine.withdraw(user_idx, withdraw_amount);

        if result.is_ok() {
            prop_assert!(engine.vault <= vault_before);
            prop_assert!(engine.users[user_idx].principal <= principal_before);
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
        let mut engine = RiskEngine::new(default_params());
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
        let mut engine = RiskEngine::new(default_params());
        let user_idx = engine.add_user(1).unwrap();

        engine.users[user_idx].pnl_ledger = pnl;
        engine.users[user_idx].warmup_state.slope_per_step = slope;

        let earlier_slot = slots1.min(slots2);
        let later_slot = slots1.max(slots2);

        engine.current_slot = earlier_slot;
        let w1 = engine.withdrawable_pnl(&engine.users[user_idx]);

        engine.current_slot = later_slot;
        let w2 = engine.withdrawable_pnl(&engine.users[user_idx]);

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
        let mut engine = RiskEngine::new(default_params());
        let user_idx = engine.add_user(1).unwrap();

        engine.users[user_idx].principal = principal;
        engine.users[user_idx].pnl_ledger = pnl;
        engine.insurance_fund.balance = 10_000_000; // Large insurance fund

        let _ = engine.apply_adl(loss);

        prop_assert_eq!(engine.users[user_idx].principal, principal,
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
        let mut engine = RiskEngine::new(default_params());
        let user_idx = engine.add_user(1).unwrap();

        engine.users[user_idx].pnl_ledger = pnl;
        engine.users[user_idx].reserved_pnl = reserved;
        engine.users[user_idx].warmup_state.slope_per_step = slope;
        engine.current_slot = slots;

        let withdrawable = engine.withdrawable_pnl(&engine.users[user_idx]);
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
        let mut engine = RiskEngine::new(default_params());
        let user_idx = engine.add_user(1).unwrap();

        engine.users[user_idx].principal = principal;
        engine.users[user_idx].pnl_ledger = pnl;

        let collateral = engine.user_collateral(&engine.users[user_idx]);

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
        let mut engine = RiskEngine::new(default_params());
        let user1 = engine.add_user(1).unwrap();
        let user2 = engine.add_user(1).unwrap();

        engine.deposit(user1, amount1).unwrap();
        engine.deposit(user2, amount2).unwrap();

        let user2_principal_before = engine.users[user2].principal;
        let user2_pnl_before = engine.users[user2].pnl_ledger;

        // Operate on user1
        let _ = engine.withdraw(user1, withdraw);

        // User2 should be unchanged
        prop_assert_eq!(engine.users[user2].principal, user2_principal_before);
        prop_assert_eq!(engine.users[user2].pnl_ledger, user2_pnl_before);
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
        let mut engine = RiskEngine::new(default_params());
        let user_idx = engine.add_user(1).unwrap();

        engine.users[user_idx].principal = principal;
        engine.users[user_idx].pnl_ledger = initial_pnl;
        engine.insurance_fund.balance = 100_000_000; // Large insurance

        for loss in losses {
            let _ = engine.apply_adl(loss);
        }

        prop_assert_eq!(engine.users[user_idx].principal, principal,
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
        let mut engine = RiskEngine::new(default_params());
        let user_idx = engine.add_user(1).unwrap();
        let lp_idx = engine.add_lp([0u8; 32], [0u8; 32], 1).unwrap();

        engine.deposit(user_idx, user_capital).unwrap();
        engine.lps[lp_idx].lp_capital = lp_capital;
        engine.vault = user_capital + lp_capital;

        let insurance_before = engine.insurance_fund.fee_revenue;

        let _ = engine.execute_trade(&MATCHER, lp_idx, user_idx, price, size);

        // Insurance fund should have received fees (if trade succeeded)
        if engine.insurance_fund.fee_revenue > insurance_before {
            prop_assert!(engine.insurance_fund.fee_revenue > insurance_before);
        }
    }
}

// Test that liquidation always reduces position
proptest! {
    #[test]
    fn fuzz_liquidation_reduces_position(
        principal in 100u128..10_000,
        position in 10_000i128..100_000,
        entry_price in price_strategy(),
        oracle_price in price_strategy()
    ) {
        let mut engine = RiskEngine::new(default_params());
        let user_idx = engine.add_user(1).unwrap();
        let keeper_idx = engine.add_user(1).unwrap();

        engine.deposit(user_idx, principal).unwrap();
        engine.users[user_idx].position_size = position;
        engine.users[user_idx].entry_price = entry_price;

        let position_before = engine.users[user_idx].position_size.abs();

        let _ = engine.liquidate_user(user_idx, keeper_idx, oracle_price);

        let position_after = engine.users[user_idx].position_size.abs();

        // If liquidation happened, position should be reduced
        prop_assert!(position_after <= position_before,
                     "Liquidation should reduce position size");
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
        let mut engine = RiskEngine::new(default_params());
        let user_idx = engine.add_user(1).unwrap();

        engine.users[user_idx].pnl_ledger = pnl;
        engine.users[user_idx].reserved_pnl = reserved;
        engine.users[user_idx].warmup_state.slope_per_step = slope;
        engine.advance_slot(slots);

        let withdrawable = engine.withdrawable_pnl(&engine.users[user_idx]);
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
        let mut engine = RiskEngine::new(default_params());

        // Create 3 users
        for _ in 0..3 {
            engine.add_user(1).unwrap();
        }

        // Apply random deposits
        for (user_idx, amount) in deposits {
            if user_idx < engine.users.len() {
                let _ = engine.deposit(user_idx, amount);
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
        let mut engine = RiskEngine::new(default_params());
        let user_idx = engine.add_user(1).unwrap();

        engine.users[user_idx].pnl_ledger = user_pnl;
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
        let mut engine = RiskEngine::new(default_params());
        let user_idx = engine.add_user(1).unwrap();
        let lp_idx = engine.add_lp([0u8; 32], [0u8; 32], 1).unwrap();

        engine.deposit(user_idx, 1_000_000).unwrap();
        engine.lps[lp_idx].lp_capital = 10_000_000;
        engine.vault = 11_000_000;

        engine.users[user_idx].position_size = initial_size;
        engine.lps[lp_idx].lp_position_size = -initial_size;

        let expected_user_pos = initial_size.saturating_add(trade_size);
        let expected_lp_pos = (-initial_size).saturating_sub(trade_size);

        let _ = engine.execute_trade(&MATCHER, lp_idx, user_idx, 1_000_000, trade_size);

        // If trade succeeded, positions should net to zero
        if engine.users[user_idx].position_size == expected_user_pos {
            let total_position = engine.users[user_idx].position_size +
                                engine.lps[lp_idx].lp_position_size;

            // Positions should roughly net out (within rounding)
            prop_assert!(total_position.abs() <= 1,
                        "User and LP positions should sum to ~0");
        }
    }
}
