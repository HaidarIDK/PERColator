//! Unit tests for negative PnL immediate settlement and equity-based margin checks
//!
//! These tests verify:
//! 1. Fix A: Negative PnL settles immediately (not time-gated by warmup slope)
//! 2. Fix B: Margin checks use equity (capital + pnl) not just collateral (capital + positive pnl)

use percolator::*;

// Helper to create test params
fn test_params() -> RiskParams {
    RiskParams {
        warmup_period_slots: 100,
        maintenance_margin_bps: 500,  // 5%
        initial_margin_bps: 1000,     // 10%
        trading_fee_bps: 10,
        max_accounts: 64,
        account_fee_bps: 0,
        risk_reduction_threshold: 0,
    }
}

// ============================================================================
// Fix A: Negative PnL Immediate Settlement Tests
// ============================================================================

/// Test: Withdrawal rejected when losses exist, even if position is closed
/// This is the repro of the bug: position_size = 0, capital = 10_000, pnl = -9_000
/// withdraw(10_000) must fail because after loss settlement, capital is only 1_000
#[test]
fn withdraw_rejected_when_losses_exist_even_if_position_closed() {
    let mut engine = RiskEngine::new(test_params());
    let user_idx = engine.add_user(1).unwrap();

    // Setup: position closed but with unrealized losses
    engine.accounts[user_idx as usize].capital = 10_000;
    engine.accounts[user_idx as usize].pnl = -9_000;
    engine.accounts[user_idx as usize].position_size = 0;  // No position
    engine.accounts[user_idx as usize].warmup_slope_per_step = 0;
    engine.vault = 10_000;

    // Attempt to withdraw full capital - should fail because losses must be realized first
    let result = engine.withdraw(user_idx, 10_000);

    // The withdraw should fail with InsufficientBalance
    assert!(
        result == Err(RiskError::InsufficientBalance),
        "Expected InsufficientBalance after loss realization reduces capital"
    );

    // After the failed withdraw call (which internally called settle_warmup_to_capital):
    // capital should be 1_000 (10_000 - 9_000 loss)
    // pnl should be 0 (loss fully realized)
    // warmed_neg_total should include 9_000
    assert_eq!(
        engine.accounts[user_idx as usize].capital, 1_000,
        "Capital should be reduced by loss amount"
    );
    assert_eq!(
        engine.accounts[user_idx as usize].pnl, 0,
        "PnL should be 0 after loss realization"
    );
    assert_eq!(
        engine.warmed_neg_total, 9_000,
        "warmed_neg_total should increase by realized loss"
    );
}

/// Test: After loss realization, remaining principal can be withdrawn
#[test]
fn withdraw_allows_only_remaining_principal_after_loss_realization() {
    let mut engine = RiskEngine::new(test_params());
    let user_idx = engine.add_user(1).unwrap();

    // Setup: position closed but with unrealized losses
    engine.accounts[user_idx as usize].capital = 10_000;
    engine.accounts[user_idx as usize].pnl = -9_000;
    engine.accounts[user_idx as usize].position_size = 0;
    engine.accounts[user_idx as usize].warmup_slope_per_step = 0;
    engine.vault = 10_000;

    // First, trigger loss settlement
    engine.settle_warmup_to_capital(user_idx).unwrap();

    // Now capital should be 1_000
    assert_eq!(engine.accounts[user_idx as usize].capital, 1_000);
    assert_eq!(engine.accounts[user_idx as usize].pnl, 0);

    // Withdraw remaining capital - should succeed
    let result = engine.withdraw(user_idx, 1_000);
    assert!(result.is_ok(), "Withdraw of remaining capital should succeed");
    assert_eq!(engine.accounts[user_idx as usize].capital, 0);
}

/// Test: Negative PnL settles immediately, independent of warmup slope
#[test]
fn negative_pnl_settles_immediately_independent_of_slope() {
    let mut engine = RiskEngine::new(test_params());
    let user_idx = engine.add_user(1).unwrap();

    // Setup: loss with zero slope - under old code this would NOT settle
    let capital = 10_000u128;
    let loss = 3_000i128;
    engine.accounts[user_idx as usize].capital = capital;
    engine.accounts[user_idx as usize].pnl = -loss;
    engine.accounts[user_idx as usize].warmup_slope_per_step = 0; // Zero slope
    engine.accounts[user_idx as usize].warmup_started_at_slot = 0;
    engine.vault = capital;
    engine.current_slot = 100; // Time has passed

    let warmed_neg_before = engine.warmed_neg_total;

    // Call settle
    engine.settle_warmup_to_capital(user_idx).unwrap();

    // Assertions: loss should settle immediately despite zero slope
    assert_eq!(
        engine.accounts[user_idx as usize].capital,
        capital - (loss as u128),
        "Capital should be reduced by full loss amount"
    );
    assert_eq!(
        engine.accounts[user_idx as usize].pnl, 0,
        "PnL should be 0 after immediate settlement"
    );
    assert_eq!(
        engine.warmed_neg_total,
        warmed_neg_before + (loss as u128),
        "warmed_neg_total should increase by loss amount"
    );
}

/// Test: When loss exceeds capital, capital goes to zero and pnl becomes remaining negative
#[test]
fn loss_exceeding_capital_leaves_negative_pnl() {
    let mut engine = RiskEngine::new(test_params());
    let user_idx = engine.add_user(1).unwrap();

    // Setup: loss greater than capital
    let capital = 5_000u128;
    let loss = 8_000i128;
    engine.accounts[user_idx as usize].capital = capital;
    engine.accounts[user_idx as usize].pnl = -loss;
    engine.accounts[user_idx as usize].warmup_slope_per_step = 0;
    engine.vault = capital;

    // Call settle
    engine.settle_warmup_to_capital(user_idx).unwrap();

    // Capital should be fully consumed
    assert_eq!(
        engine.accounts[user_idx as usize].capital, 0,
        "Capital should be reduced to zero"
    );
    // Remaining loss stays as negative pnl
    assert_eq!(
        engine.accounts[user_idx as usize].pnl,
        -(loss - (capital as i128)),
        "Remaining loss should stay as negative pnl"
    );
    assert_eq!(
        engine.warmed_neg_total, capital,
        "warmed_neg_total should increase by capital (the amount actually paid)"
    );
}

// ============================================================================
// Fix B: Equity-Based Margin Tests
// ============================================================================

/// Test: Withdraw with open position respects negative PnL in equity calculation
#[test]
fn withdraw_with_open_position_respects_negative_pnl_in_equity() {
    let mut engine = RiskEngine::new(test_params());
    let user_idx = engine.add_user(1).unwrap();

    // Setup:
    // capital = 10_000
    // pnl = -9_000 (will be settled immediately to capital = 1_000)
    // position_size = 1_000
    // entry_price = 1_000_000 (1.0 in 1e6 scale)
    // position_notional = 1_000 * 1_000_000 / 1_000_000 = 1_000
    // initial_margin_required = 1_000 * 1000 / 10_000 = 100 (10%)
    //
    // After settle: capital = 1_000, pnl = 0, equity = 1_000
    // Try to withdraw 950: new_capital = 50, new_equity = 50
    // 50 < 100 (IM) => should fail

    engine.accounts[user_idx as usize].capital = 10_000;
    engine.accounts[user_idx as usize].pnl = -9_000;
    engine.accounts[user_idx as usize].position_size = 1_000;
    engine.accounts[user_idx as usize].entry_price = 1_000_000;
    engine.accounts[user_idx as usize].warmup_slope_per_step = 0;
    engine.vault = 10_000;

    // First settle to realize loss
    engine.settle_warmup_to_capital(user_idx).unwrap();

    // Now capital = 1_000, pnl = 0
    assert_eq!(engine.accounts[user_idx as usize].capital, 1_000);

    // Try to withdraw 950 - would leave 50 equity < 100 IM required
    let result = engine.withdraw(user_idx, 950);
    assert!(
        result == Err(RiskError::Undercollateralized),
        "Withdraw should fail due to insufficient equity for initial margin"
    );

    // Withdraw 900 should succeed (leaves 100 equity = 100 IM required)
    let result = engine.withdraw(user_idx, 900);
    assert!(
        result.is_ok(),
        "Withdraw should succeed when equity >= IM required"
    );
}

/// Test: Maintenance margin check uses equity including negative PnL
#[test]
fn maintenance_margin_uses_equity_including_negative_pnl() {
    let mut engine = RiskEngine::new(test_params());
    let user_idx = engine.add_user(1).unwrap();

    // Setup with enough capital but negative pnl that drops equity below MM
    // capital = 1_000
    // pnl = -900 (after settle: capital = 100, pnl = 0)
    // position_size = 10_000
    // oracle_price = 1_000_000 (1.0)
    // position_value = 10_000 * 1_000_000 / 1_000_000 = 10_000
    // MM required = 10_000 * 500 / 10_000 = 500 (5%)
    // equity after settle = 100 < 500 => below MM

    engine.accounts[user_idx as usize].capital = 1_000;
    engine.accounts[user_idx as usize].pnl = -900;
    engine.accounts[user_idx as usize].position_size = 10_000;
    engine.accounts[user_idx as usize].entry_price = 1_000_000;
    engine.vault = 1_000;

    // First settle to realize loss
    engine.settle_warmup_to_capital(user_idx).unwrap();

    // Now check maintenance margin
    let account = &engine.accounts[user_idx as usize];
    let is_above_mm = engine.is_above_maintenance_margin(account, 1_000_000);

    assert!(
        !is_above_mm,
        "Should be below maintenance margin when equity (100) < MM required (500)"
    );
}

/// Test: When negative PnL is settled and equity is sufficient, MM check passes
#[test]
fn maintenance_margin_passes_with_sufficient_equity() {
    let mut engine = RiskEngine::new(test_params());
    let user_idx = engine.add_user(1).unwrap();

    // Setup:
    // capital = 10_000
    // pnl = -1_000 (after settle: capital = 9_000, pnl = 0)
    // position_size = 10_000
    // oracle_price = 1_000_000
    // position_value = 10_000
    // MM required = 500
    // equity = 9_000 > 500 => above MM

    engine.accounts[user_idx as usize].capital = 10_000;
    engine.accounts[user_idx as usize].pnl = -1_000;
    engine.accounts[user_idx as usize].position_size = 10_000;
    engine.accounts[user_idx as usize].entry_price = 1_000_000;
    engine.vault = 10_000;

    // Settle to realize loss
    engine.settle_warmup_to_capital(user_idx).unwrap();

    let account = &engine.accounts[user_idx as usize];
    let is_above_mm = engine.is_above_maintenance_margin(account, 1_000_000);

    assert!(
        is_above_mm,
        "Should be above maintenance margin when equity (9_000) > MM required (500)"
    );
}

/// Test: account_equity correctly computes max(0, capital + pnl)
#[test]
fn account_equity_computes_correctly() {
    let engine = RiskEngine::new(test_params());

    // Positive equity
    let account_pos = Account {
        kind: AccountKind::User,
        account_id: 1,
        capital: 10_000,
        pnl: -3_000,
        reserved_pnl: 0,
        warmup_started_at_slot: 0,
        warmup_slope_per_step: 0,
        position_size: 0,
        entry_price: 0,
        funding_index: 0,
        matcher_program: [0; 32],
        matcher_context: [0; 32],
    };
    assert_eq!(engine.account_equity(&account_pos), 7_000);

    // Negative sum clamped to zero
    let account_neg = Account {
        kind: AccountKind::User,
        account_id: 2,
        capital: 5_000,
        pnl: -8_000,
        reserved_pnl: 0,
        warmup_started_at_slot: 0,
        warmup_slope_per_step: 0,
        position_size: 0,
        entry_price: 0,
        funding_index: 0,
        matcher_program: [0; 32],
        matcher_context: [0; 32],
    };
    assert_eq!(engine.account_equity(&account_neg), 0);

    // Positive pnl adds to equity
    let account_profit = Account {
        kind: AccountKind::User,
        account_id: 3,
        capital: 10_000,
        pnl: 5_000,
        reserved_pnl: 0,
        warmup_started_at_slot: 0,
        warmup_slope_per_step: 0,
        position_size: 0,
        entry_price: 0,
        funding_index: 0,
        matcher_program: [0; 32],
        matcher_context: [0; 32],
    };
    assert_eq!(engine.account_equity(&account_profit), 15_000);
}
