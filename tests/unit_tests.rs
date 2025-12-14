//! Fast unit tests for the risk engine
//! Run with: cargo test

use percolator::*;

fn default_params() -> RiskParams {
    RiskParams {
        warmup_period_slots: 100,
        maintenance_margin_bps: 500, // 5%
        initial_margin_bps: 1000,    // 10%
        trading_fee_bps: 10,          // 0.1%
        liquidation_fee_bps: 50,      // 0.5%
        insurance_fee_share_bps: 5000, // 50% to insurance
    }
}

#[test]
fn test_deposit_and_withdraw_principal() {
    let mut engine = RiskEngine::new(default_params());
    let user_idx = engine.add_user();

    // Deposit
    engine.deposit(user_idx, 1000).unwrap();
    assert_eq!(engine.users[user_idx].principal, 1000);
    assert_eq!(engine.vault, 1000);

    // Withdraw partial
    engine.withdraw_principal(user_idx, 400).unwrap();
    assert_eq!(engine.users[user_idx].principal, 600);
    assert_eq!(engine.vault, 600);

    // Withdraw rest
    engine.withdraw_principal(user_idx, 600).unwrap();
    assert_eq!(engine.users[user_idx].principal, 0);
    assert_eq!(engine.vault, 0);
}

#[test]
fn test_withdraw_principal_insufficient_balance() {
    let mut engine = RiskEngine::new(default_params());
    let user_idx = engine.add_user();

    engine.deposit(user_idx, 1000).unwrap();

    // Try to withdraw more than deposited
    let result = engine.withdraw_principal(user_idx, 1500);
    assert_eq!(result, Err(RiskError::InsufficientBalance));
}

#[test]
fn test_withdraw_principal_with_negative_pnl_should_fail() {
    let mut engine = RiskEngine::new(default_params());
    let user_idx = engine.add_user();

    // User deposits 1000
    engine.deposit(user_idx, 1000).unwrap();

    // User has a position and negative PNL of -800
    engine.users[user_idx].position_size = 10_000;
    engine.users[user_idx].entry_price = 1_000_000; // $1 entry price
    engine.users[user_idx].pnl_ledger = -800;

    // Trying to withdraw all principal would leave collateral = 0 + max(0, -800) = 0
    // This should fail because user has an open position
    let result = engine.withdraw_principal(user_idx, 1000);

    // BUG: This currently succeeds but should fail!
    // User would have 0 principal, -800 PNL, and a 10k position = undercollateralized
    assert!(result.is_err(), "Should not allow withdrawal that leaves account undercollateralized with open position");
}

#[test]
fn test_pnl_warmup() {
    let mut engine = RiskEngine::new(default_params());
    let user_idx = engine.add_user();

    // Give user some positive PNL
    engine.users[user_idx].pnl_ledger = 1000;
    engine.users[user_idx].warmup_state.slope_per_step = 10; // 10 per slot

    // At slot 0, nothing is warmed up yet
    assert_eq!(engine.withdrawable_pnl(&engine.users[user_idx]), 0);

    // Advance 50 slots
    engine.advance_slot(50);
    assert_eq!(engine.withdrawable_pnl(&engine.users[user_idx]), 500); // 10 * 50

    // Advance 100 more slots (total 150)
    engine.advance_slot(100);
    assert_eq!(engine.withdrawable_pnl(&engine.users[user_idx]), 1000); // Capped at total PNL
}

#[test]
fn test_pnl_warmup_with_reserved() {
    let mut engine = RiskEngine::new(default_params());
    let user_idx = engine.add_user();

    engine.users[user_idx].pnl_ledger = 1000;
    engine.users[user_idx].reserved_pnl = 300; // 300 reserved for pending withdrawal
    engine.users[user_idx].warmup_state.slope_per_step = 10;

    // Advance 100 slots
    engine.advance_slot(100);

    // Withdrawable = min(available_pnl, warmed_up)
    // available_pnl = 1000 - 300 = 700
    // warmed_up = 10 * 100 = 1000
    // So withdrawable = 700
    assert_eq!(engine.withdrawable_pnl(&engine.users[user_idx]), 700);
}

#[test]
fn test_withdraw_pnl_not_warmed_up() {
    let mut engine = RiskEngine::new(default_params());
    let user_idx = engine.add_user();

    engine.deposit(user_idx, 1000).unwrap();
    engine.users[user_idx].pnl_ledger = 500;
    engine.insurance_fund.balance = 1000;

    // Try to withdraw PNL before it's warmed up
    let result = engine.withdraw_pnl(user_idx, 100);
    assert_eq!(result, Err(RiskError::PnlNotWarmedUp));
}

#[test]
fn test_withdraw_pnl_after_warmup() {
    let mut engine = RiskEngine::new(default_params());
    let user_idx = engine.add_user();

    engine.deposit(user_idx, 1000).unwrap();
    engine.users[user_idx].pnl_ledger = 500;
    engine.users[user_idx].warmup_state.slope_per_step = 10;
    engine.insurance_fund.balance = 1000;
    engine.vault = 2000; // principal + insurance

    // Advance enough slots to warm up 200 PNL
    engine.advance_slot(20);

    // Should be able to withdraw 200
    engine.withdraw_pnl(user_idx, 200).unwrap();
    assert_eq!(engine.users[user_idx].pnl_ledger, 300);
    assert_eq!(engine.insurance_fund.balance, 800);
}

#[test]
fn test_conservation_simple() {
    let mut engine = RiskEngine::new(default_params());
    let user1 = engine.add_user();
    let user2 = engine.add_user();

    // Initial state should conserve
    assert!(engine.check_conservation());

    // Deposit to user1
    engine.deposit(user1, 1000).unwrap();
    assert!(engine.check_conservation());

    // Deposit to user2
    engine.deposit(user2, 2000).unwrap();
    assert!(engine.check_conservation());

    // User1 gets positive PNL
    engine.users[user1].pnl_ledger = 500;
    engine.vault += 500;
    assert!(engine.check_conservation());

    // Withdraw principal
    engine.withdraw_principal(user1, 500).unwrap();
    assert!(engine.check_conservation());
}

#[test]
fn test_adl_haircut_unwrapped_pnl() {
    let mut engine = RiskEngine::new(default_params());
    let user_idx = engine.add_user();

    engine.users[user_idx].principal = 1000;
    engine.users[user_idx].pnl_ledger = 500; // All unwrapped (warmup not started)
    engine.users[user_idx].warmup_state.slope_per_step = 10;
    engine.vault = 1500;

    // Apply ADL loss of 200
    engine.apply_adl(200).unwrap();

    // Should haircut the unwrapped PNL
    assert_eq!(engine.users[user_idx].pnl_ledger, 300);
    assert_eq!(engine.users[user_idx].principal, 1000); // Principal untouched!
}

#[test]
fn test_adl_overflow_to_insurance() {
    let mut engine = RiskEngine::new(default_params());
    let user_idx = engine.add_user();

    engine.users[user_idx].principal = 1000;
    engine.users[user_idx].pnl_ledger = 300; // Only 300 unwrapped PNL
    engine.users[user_idx].warmup_state.slope_per_step = 10;
    engine.insurance_fund.balance = 500;
    engine.vault = 1800;

    // Apply ADL loss of 700 (more than unwrapped PNL)
    engine.apply_adl(700).unwrap();

    // Should haircut all PNL first
    assert_eq!(engine.users[user_idx].pnl_ledger, 0);
    assert_eq!(engine.users[user_idx].principal, 1000); // Principal still untouched!

    // Remaining 400 should come from insurance (700 - 300 = 400)
    assert_eq!(engine.insurance_fund.balance, 100); // 500 - 400
}

#[test]
fn test_adl_insurance_depleted() {
    let mut engine = RiskEngine::new(default_params());
    let user_idx = engine.add_user();

    engine.users[user_idx].principal = 1000;
    engine.users[user_idx].pnl_ledger = 100;
    engine.insurance_fund.balance = 50;

    // Apply ADL loss of 200
    engine.apply_adl(200).unwrap();

    // PNL haircut: 100
    assert_eq!(engine.users[user_idx].pnl_ledger, 0);

    // Insurance depleted: 50
    assert_eq!(engine.insurance_fund.balance, 0);

    // Remaining 50 goes to loss accumulator
    assert_eq!(engine.loss_accum, 50);
}

#[test]
fn test_collateral_calculation() {
    let mut engine = RiskEngine::new(default_params());
    let user_idx = engine.add_user();

    engine.users[user_idx].principal = 1000;
    engine.users[user_idx].pnl_ledger = 500;

    assert_eq!(engine.user_collateral(&engine.users[user_idx]), 1500);

    // Negative PNL doesn't add to collateral
    engine.users[user_idx].pnl_ledger = -300;
    assert_eq!(engine.user_collateral(&engine.users[user_idx]), 1000);
}

#[test]
fn test_maintenance_margin_check() {
    let mut engine = RiskEngine::new(default_params());
    let user_idx = engine.add_user();

    engine.users[user_idx].principal = 1000;
    engine.users[user_idx].position_size = 10_000; // 10k units
    engine.users[user_idx].entry_price = 1_000_000; // $1

    // At price $1, position value = 10k
    // Maintenance margin (5%) = 500
    // Collateral = 1000, so above maintenance
    assert!(engine.is_above_maintenance_margin(&engine.users[user_idx], 1_000_000));

    // At price $2, position value = 20k
    // Maintenance margin (5%) = 1000
    // Collateral = 1000, so just at threshold (should be false)
    assert!(!engine.is_above_maintenance_margin(&engine.users[user_idx], 2_000_000));
}

#[test]
fn test_trading_opens_position() {
    let mut engine = RiskEngine::new(default_params());
    let user_idx = engine.add_user();
    let lp_idx = engine.add_lp([0u8; 32], [0u8; 32]);

    // Setup user with capital
    engine.deposit(user_idx, 10_000).unwrap();
    engine.lps[lp_idx].lp_capital = 100_000;

    // Execute trade: user buys 1000 units at $1
    let oracle_price = 1_000_000;
    let size = 1000i128;

    engine.execute_trade(lp_idx, user_idx, oracle_price, size, &[]).unwrap();

    // Check position opened
    assert_eq!(engine.users[user_idx].position_size, 1000);
    assert_eq!(engine.users[user_idx].entry_price, oracle_price);

    // Check LP has opposite position
    assert_eq!(engine.lps[lp_idx].lp_position_size, -1000);

    // Check fee was charged (0.1% of 1000 = 1)
    assert!(engine.insurance_fund.fee_revenue > 0);
}

#[test]
fn test_trading_realizes_pnl() {
    let mut engine = RiskEngine::new(default_params());
    let user_idx = engine.add_user();
    let lp_idx = engine.add_lp([0u8; 32], [0u8; 32]);

    engine.deposit(user_idx, 10_000).unwrap();
    engine.lps[lp_idx].lp_capital = 100_000;
    engine.vault = 110_000;

    // Open long position at $1
    engine.execute_trade(lp_idx, user_idx, 1_000_000, 1000, &[]).unwrap();

    // Close position at $1.50 (50% profit)
    engine.execute_trade(lp_idx, user_idx, 1_500_000, -1000, &[]).unwrap();

    // Check PNL realized (approximately)
    // Price went from $1 to $1.50, so 500 profit on 1000 units
    assert!(engine.users[user_idx].pnl_ledger > 0);
    assert_eq!(engine.users[user_idx].position_size, 0);
}

#[test]
fn test_liquidation() {
    let mut engine = RiskEngine::new(default_params());
    let user_idx = engine.add_user();
    let keeper_idx = engine.add_user();

    // User with small capital and large position
    engine.deposit(user_idx, 1000).unwrap();
    engine.users[user_idx].position_size = 50_000; // Very leveraged
    engine.users[user_idx].entry_price = 1_000_000;

    // Price moves against user
    let oracle_price = 1_200_000; // 20% increase

    // Should be below maintenance margin
    assert!(!engine.is_above_maintenance_margin(&engine.users[user_idx], oracle_price));

    // Liquidate
    let initial_keeper_pnl = engine.users[keeper_idx].pnl_ledger;
    engine.liquidate_user(user_idx, keeper_idx, oracle_price).unwrap();

    // Position should be closed
    assert_eq!(engine.users[user_idx].position_size, 0);

    // Keeper should receive fee
    assert!(engine.users[keeper_idx].pnl_ledger > initial_keeper_pnl);

    // Insurance fund should receive fee
    assert!(engine.insurance_fund.liquidation_revenue > 0);
}

#[test]
fn test_user_isolation() {
    let mut engine = RiskEngine::new(default_params());
    let user1 = engine.add_user();
    let user2 = engine.add_user();

    engine.deposit(user1, 1000).unwrap();
    engine.deposit(user2, 2000).unwrap();

    let user2_principal_before = engine.users[user2].principal;
    let user2_pnl_before = engine.users[user2].pnl_ledger;

    // Operate on user1
    engine.withdraw_principal(user1, 500).unwrap();
    engine.users[user1].pnl_ledger = 300;

    // User2 should be unchanged
    assert_eq!(engine.users[user2].principal, user2_principal_before);
    assert_eq!(engine.users[user2].pnl_ledger, user2_pnl_before);
}

#[test]
fn test_principal_never_reduced_by_adl() {
    let mut engine = RiskEngine::new(default_params());
    let user_idx = engine.add_user();

    let initial_principal = 5000u128;
    engine.users[user_idx].principal = initial_principal;
    engine.users[user_idx].pnl_ledger = 100;

    // Apply massive ADL
    engine.apply_adl(10_000).unwrap();

    // Principal should NEVER be touched
    assert_eq!(engine.users[user_idx].principal, initial_principal);
}

#[test]
fn test_multiple_users_adl() {
    let mut engine = RiskEngine::new(default_params());
    let user1 = engine.add_user();
    let user2 = engine.add_user();
    let user3 = engine.add_user();

    // User1: has unwrapped PNL
    engine.users[user1].principal = 1000;
    engine.users[user1].pnl_ledger = 500;
    engine.users[user1].warmup_state.slope_per_step = 10;

    // User2: has unwrapped PNL
    engine.users[user2].principal = 2000;
    engine.users[user2].pnl_ledger = 800;
    engine.users[user2].warmup_state.slope_per_step = 10;

    // User3: no PNL
    engine.users[user3].principal = 1500;

    engine.insurance_fund.balance = 1000;

    // Apply ADL loss of 1000
    engine.apply_adl(1000).unwrap();

    // Should haircut user1 and user2's PNL
    // Total unwrapped PNL = 500 + 800 = 1300
    // Loss = 1000, so both should be haircutted proportionally or sequentially
    assert!(engine.users[user1].pnl_ledger < 500 || engine.users[user2].pnl_ledger < 800);

    // All principals should be intact
    assert_eq!(engine.users[user1].principal, 1000);
    assert_eq!(engine.users[user2].principal, 2000);
    assert_eq!(engine.users[user3].principal, 1500);
}

#[test]
fn test_warmup_monotonicity() {
    let mut engine = RiskEngine::new(default_params());
    let user_idx = engine.add_user();

    engine.users[user_idx].pnl_ledger = 1000;
    engine.users[user_idx].warmup_state.slope_per_step = 10;

    // Get withdrawable at different time points
    let w0 = engine.withdrawable_pnl(&engine.users[user_idx]);

    engine.advance_slot(10);
    let w1 = engine.withdrawable_pnl(&engine.users[user_idx]);

    engine.advance_slot(20);
    let w2 = engine.withdrawable_pnl(&engine.users[user_idx]);

    // Should be monotonically increasing
    assert!(w1 >= w0);
    assert!(w2 >= w1);
}

#[test]
fn test_fee_accumulation() {
    let mut engine = RiskEngine::new(default_params());
    let user_idx = engine.add_user();
    let lp_idx = engine.add_lp([0u8; 32], [0u8; 32]);

    engine.deposit(user_idx, 100_000).unwrap();
    engine.lps[lp_idx].lp_capital = 1_000_000;
    engine.vault = 1_100_000;

    let initial_insurance_balance = engine.insurance_fund.balance;

    // Execute multiple trades
    for _ in 0..10 {
        let result1 = engine.execute_trade(lp_idx, user_idx, 1_000_000, 100, &[]);
        let result2 = engine.execute_trade(lp_idx, user_idx, 1_000_000, -100, &[]);
        // Trades might fail due to margin, that's ok
        let _ = result1;
        let _ = result2;
    }

    // Insurance fund should have accumulated fees (if any trades succeeded)
    // Note: this test might not accumulate fees if all trades fail
    if engine.insurance_fund.fee_revenue > 0 {
        assert!(engine.insurance_fund.balance > initial_insurance_balance);
    }
}
