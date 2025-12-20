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

/// Test 1 (Plan 4.1): Withdrawal rejected when position closed and negative PnL exists
/// Setup: capital=10_000, pnl=-9_000, pos=0, slope=0, current_slot large, vault=10_000
/// withdraw(10_000) must be Err(InsufficientBalance)
/// State after: capital=1_000, pnl=0
#[test]
fn withdraw_rejected_when_closed_and_negative_pnl() {
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

/// Test 2 (Plan 4.2): After loss realization, remaining principal can be withdrawn
/// Same setup then withdraw(1_000) is ok
#[test]
fn withdraw_allows_remaining_principal_after_loss_realization() {
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

/// Test 3 (Plan 4.3): Withdraw with open position blocked due to equity
/// Deterministic IM requirement scenario per plan 2.3A:
/// - position_size = 1000, entry_price = 1_000_000
/// - notional = 1000, IM = 1000 * 1000 / 10000 = 100
/// - capital = 150, pnl = -100, vault = capital
/// - After settle: capital = 50, pnl = 0
/// - withdraw(60) > capital=50 => Err(InsufficientBalance)
///
/// Alternate test that checks margin: after settle, try withdraw(40)
/// - new_capital = 10, new_equity = 10 < IM=100 => Err(Undercollateralized)
#[test]
fn withdraw_open_position_blocks_due_to_equity() {
    let mut engine = RiskEngine::new(test_params());
    let user_idx = engine.add_user(1).unwrap();

    // Setup per plan:
    // position_size = 1000, entry_price = 1_000_000
    // notional = 1000, IM = 100
    // capital = 150, pnl = -100
    // After settle: capital = 50, pnl = 0, equity = 50

    engine.accounts[user_idx as usize].capital = 150;
    engine.accounts[user_idx as usize].pnl = -100;
    engine.accounts[user_idx as usize].position_size = 1_000;
    engine.accounts[user_idx as usize].entry_price = 1_000_000;
    engine.accounts[user_idx as usize].warmup_slope_per_step = 0;
    engine.vault = 150;

    // withdraw(60) should fail - loss settles first, then balance check fails
    let result = engine.withdraw(user_idx, 60);
    assert!(
        result == Err(RiskError::InsufficientBalance),
        "withdraw(60) must fail: after settling 100 loss, capital=50 < 60"
    );

    // Now capital = 50, pnl = 0
    assert_eq!(engine.accounts[user_idx as usize].capital, 50);
    assert_eq!(engine.accounts[user_idx as usize].pnl, 0);

    // Try withdraw(40) - would leave 10 equity < 100 IM required
    let result = engine.withdraw(user_idx, 40);
    assert!(
        result == Err(RiskError::Undercollateralized),
        "withdraw(40) must fail: new_equity=10 < IM=100"
    );
}

/// Test 4 (Plan 4.4): Maintenance margin uses equity
/// Per plan 2.3B:
/// - position_size = 1000, oracle_price = 1_000_000
/// - position_value = 1000, MM = 1000 * 500 / 10000 = 50
/// Case 1: capital = 40, pnl = 0 => equity = 40 < 50 => false
/// Case 2: capital = 100, pnl = -60 => equity = 40 < 50 => false
#[test]
fn maintenance_margin_uses_equity() {
    let engine = RiskEngine::new(test_params());

    let oracle_price = 1_000_000u64;

    // Case 1: capital = 40, pnl = 0
    let account1 = Account {
        kind: AccountKind::User,
        account_id: 1,
        capital: 40,
        pnl: 0,
        reserved_pnl: 0,
        warmup_started_at_slot: 0,
        warmup_slope_per_step: 0,
        position_size: 1_000,
        entry_price: 1_000_000,
        funding_index: 0,
        matcher_program: [0; 32],
        matcher_context: [0; 32],
    };

    // equity = 40, MM = 50, 40 < 50 => not above MM
    assert!(
        !engine.is_above_maintenance_margin(&account1, oracle_price),
        "Case 1: equity 40 < MM 50, should be below MM"
    );

    // Case 2: capital = 100, pnl = -60
    let account2 = Account {
        kind: AccountKind::User,
        account_id: 2,
        capital: 100,
        pnl: -60,
        reserved_pnl: 0,
        warmup_started_at_slot: 0,
        warmup_slope_per_step: 0,
        position_size: 1_000,
        entry_price: 1_000_000,
        funding_index: 0,
        matcher_program: [0; 32],
        matcher_context: [0; 32],
    };

    // equity = max(0, 100 - 60) = 40, MM = 50, 40 < 50 => not above MM
    assert!(
        !engine.is_above_maintenance_margin(&account2, oracle_price),
        "Case 2: equity 40 (100-60) < MM 50, should be below MM"
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
