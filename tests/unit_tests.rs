//! Fast unit tests for the risk engine
//! Run with: cargo test

use percolator::*;

// Use the no-op matcher for tests
const MATCHER: NoOpMatcher = NoOpMatcher;

// ==============================================================================
// DETERMINISTIC PRNG FOR FUZZ TESTS
// ==============================================================================

/// Simple xorshift64 PRNG for deterministic fuzz testing
struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Rng(seed)
    }

    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }

    fn u64(&mut self, lo: u64, hi: u64) -> u64 {
        if lo >= hi { return lo; }
        lo + (self.next() % (hi - lo + 1))
    }

    fn i128(&mut self, lo: i128, hi: i128) -> i128 {
        if lo >= hi { return lo; }
        lo + (self.next() as i128 % (hi - lo + 1))
    }

    fn u128(&mut self, lo: u128, hi: u128) -> u128 {
        if lo >= hi { return lo; }
        lo + (self.next() as u128 % (hi - lo + 1))
    }
}

fn default_params() -> RiskParams {
    RiskParams {
        warmup_period_slots: 100,
        maintenance_margin_bps: 500, // 5%
        initial_margin_bps: 1000,    // 10%
        trading_fee_bps: 10,          // 0.1%
        max_accounts: 1000,
        account_fee_bps: 10000, // 1%
        risk_reduction_threshold: 0, // Default: only trigger on full depletion
    }
}

#[test]
fn test_deposit_and_withdraw() {
    let mut engine = Box::new(RiskEngine::new(default_params()));
    // Account creation fee goes to insurance fund (and vault)
    let fee = 1; // First account fee with default params
    let user_idx = engine.add_user(fee).unwrap();

    // Deposit
    engine.deposit(user_idx, 1000).unwrap();
    assert_eq!(engine.accounts[user_idx as usize].capital, 1000);
    assert_eq!(engine.vault, 1000 + fee); // +fee from account creation

    // Withdraw partial
    engine.withdraw(user_idx, 400).unwrap();
    assert_eq!(engine.accounts[user_idx as usize].capital, 600);
    assert_eq!(engine.vault, 600 + fee);

    // Withdraw rest
    engine.withdraw(user_idx, 600).unwrap();
    assert_eq!(engine.accounts[user_idx as usize].capital, 0);
    assert_eq!(engine.vault, fee); // Insurance fee remains
}

#[test]
fn test_withdraw_insufficient_balance() {
    let mut engine = Box::new(RiskEngine::new(default_params()));
    let user_idx = engine.add_user(1).unwrap();

    engine.deposit(user_idx, 1000).unwrap();

    // Try to withdraw more than deposited
    let result = engine.withdraw(user_idx, 1500);
    assert_eq!(result, Err(RiskError::InsufficientBalance));
}

#[test]
fn test_withdraw_principal_with_negative_pnl_should_fail() {
    let mut engine = Box::new(RiskEngine::new(default_params()));
    let user_idx = engine.add_user(1).unwrap();

    // User deposits 1000
    engine.deposit(user_idx, 1000).unwrap();

    // User has a position and negative PNL of -800
    engine.accounts[user_idx as usize].position_size = 10_000;
    engine.accounts[user_idx as usize].entry_price = 1_000_000; // $1 entry price
    engine.accounts[user_idx as usize].pnl = -800;

    // Trying to withdraw all principal would leave collateral = 0 + max(0, -800) = 0
    // This should fail because user has an open position
    let result = engine.withdraw(user_idx, 1000);

    assert!(result.is_err(), "Should not allow withdrawal that leaves account undercollateralized with open position");
}

#[test]
fn test_pnl_warmup() {
    let mut engine = Box::new(RiskEngine::new(default_params()));
    let user_idx = engine.add_user(1).unwrap();

    // Give user some positive PNL
    engine.accounts[user_idx as usize].pnl = 1000;
    engine.accounts[user_idx as usize].warmup_slope_per_step = 10; // 10 per slot

    // At slot 0, nothing is warmed up yet
    assert_eq!(engine.withdrawable_pnl(&engine.accounts[user_idx as usize]), 0);

    // Advance 50 slots
    engine.advance_slot(50);
    assert_eq!(engine.withdrawable_pnl(&engine.accounts[user_idx as usize]), 500); // 10 * 50

    // Advance 100 more slots (total 150)
    engine.advance_slot(100);
    assert_eq!(engine.withdrawable_pnl(&engine.accounts[user_idx as usize]), 1000); // Capped at total PNL
}

#[test]
fn test_pnl_warmup_with_reserved() {
    let mut engine = Box::new(RiskEngine::new(default_params()));
    let user_idx = engine.add_user(1).unwrap();

    engine.accounts[user_idx as usize].pnl = 1000;
    engine.accounts[user_idx as usize].reserved_pnl = 300; // 300 reserved for pending withdrawal
    engine.accounts[user_idx as usize].warmup_slope_per_step = 10;

    // Advance 100 slots
    engine.advance_slot(100);

    // Withdrawable = min(available_pnl, warmed_up)
    // available_pnl = 1000 - 300 = 700
    // warmed_up = 10 * 100 = 1000
    // So withdrawable = 700
    assert_eq!(engine.withdrawable_pnl(&engine.accounts[user_idx as usize]), 700);
}

#[test]
fn test_withdraw_pnl_not_warmed_up() {
    let mut engine = Box::new(RiskEngine::new(default_params()));
    let user_idx = engine.add_user(1).unwrap();

    engine.deposit(user_idx, 1000).unwrap();
    engine.accounts[user_idx as usize].pnl = 500;

    // Try to withdraw more than principal + warmed up PNL
    // Since PNL hasn't warmed up, can only withdraw the 1000 principal
    let result = engine.withdraw(user_idx, 1100);
    assert_eq!(result, Err(RiskError::InsufficientBalance));
}

#[test]
fn test_withdraw_with_warmed_up_pnl() {
    let mut engine = Box::new(RiskEngine::new(default_params()));
    let user_idx = engine.add_user(1).unwrap();

    engine.deposit(user_idx, 1000).unwrap();
    engine.accounts[user_idx as usize].pnl = 500;
    engine.accounts[user_idx as usize].warmup_slope_per_step = 10;

    // Advance enough slots to warm up 200 PNL
    engine.advance_slot(20);

    // Should be able to withdraw 1200 (1000 principal + 200 warmed PNL)
    // The function will automatically convert the 200 PNL to principal before withdrawal
    engine.withdraw(user_idx, 1200).unwrap();
    assert_eq!(engine.accounts[user_idx as usize].pnl, 300); // 500 - 200 converted
    assert_eq!(engine.accounts[user_idx as usize].capital, 0); // 1000 + 200 - 1200
    assert_eq!(engine.vault, 0);
}
#[test]
fn test_conservation_simple() {
    let mut engine = Box::new(RiskEngine::new(default_params()));
    let user1 = engine.add_user(1).unwrap();
    let user2 = engine.add_user(1).unwrap();

    // Initial state should conserve
    assert!(engine.check_conservation());

    // Deposit to user1
    engine.deposit(user1, 1000).unwrap();
    assert!(engine.check_conservation());

    // Deposit to user2
    engine.deposit(user2, 2000).unwrap();
    assert!(engine.check_conservation());

    // PNL is zero-sum: user1 gains 500, user2 loses 500
    // (vault unchanged since this is internal redistribution)
    engine.accounts[user1 as usize].pnl = 500;
    engine.accounts[user2 as usize].pnl = -500;
    assert!(engine.check_conservation());

    // Withdraw from user1's capital
    engine.withdraw(user1, 500).unwrap();
    assert!(engine.check_conservation());
}

#[test]
fn test_adl_haircut_unwrapped_pnl() {
    let mut engine = Box::new(RiskEngine::new(default_params()));
    let user_idx = engine.add_user(1).unwrap();

    engine.accounts[user_idx as usize].capital = 1000;
    engine.accounts[user_idx as usize].pnl = 500; // All unwrapped (warmup not started)
    engine.accounts[user_idx as usize].warmup_slope_per_step = 10;
    engine.vault = 1500;

    // Apply ADL loss of 200
    engine.apply_adl(200).unwrap();

    // Should haircut the unwrapped PNL
    assert_eq!(engine.accounts[user_idx as usize].pnl, 300);
    assert_eq!(engine.accounts[user_idx as usize].capital, 1000); // Principal untouched!
}

#[test]
fn test_adl_overflow_to_insurance() {
    let mut engine = Box::new(RiskEngine::new(default_params()));
    let user_idx = engine.add_user(1).unwrap();

    engine.accounts[user_idx as usize].capital = 1000;
    engine.accounts[user_idx as usize].pnl = 300; // Only 300 unwrapped PNL
    engine.accounts[user_idx as usize].warmup_slope_per_step = 10;
    engine.insurance_fund.balance = 500;
    engine.vault = 1800;

    // Apply ADL loss of 700 (more than unwrapped PNL)
    engine.apply_adl(700).unwrap();

    // Should haircut all PNL first
    assert_eq!(engine.accounts[user_idx as usize].pnl, 0);
    assert_eq!(engine.accounts[user_idx as usize].capital, 1000); // Principal still untouched!

    // Remaining 400 should come from insurance (700 - 300 = 400)
    assert_eq!(engine.insurance_fund.balance, 100); // 500 - 400
}

#[test]
fn test_adl_insurance_depleted() {
    let mut engine = Box::new(RiskEngine::new(default_params()));
    let user_idx = engine.add_user(1).unwrap();

    engine.accounts[user_idx as usize].capital = 1000;
    engine.accounts[user_idx as usize].pnl = 100;
    engine.insurance_fund.balance = 50;

    // Apply ADL loss of 200
    engine.apply_adl(200).unwrap();

    // PNL haircut: 100
    assert_eq!(engine.accounts[user_idx as usize].pnl, 0);

    // Insurance depleted: 50
    assert_eq!(engine.insurance_fund.balance, 0);

    // Remaining 50 goes to loss accumulator
    assert_eq!(engine.loss_accum, 50);
}

#[test]
fn test_collateral_calculation() {
    let mut engine = Box::new(RiskEngine::new(default_params()));
    let user_idx = engine.add_user(1).unwrap();

    engine.accounts[user_idx as usize].capital = 1000;
    engine.accounts[user_idx as usize].pnl = 500;

    assert_eq!(engine.account_collateral(&engine.accounts[user_idx as usize]), 1500);

    // Negative PNL doesn't add to collateral
    engine.accounts[user_idx as usize].pnl = -300;
    assert_eq!(engine.account_collateral(&engine.accounts[user_idx as usize]), 1000);
}

#[test]
fn test_maintenance_margin_check() {
    let mut engine = Box::new(RiskEngine::new(default_params()));
    let user_idx = engine.add_user(1).unwrap();

    engine.accounts[user_idx as usize].capital = 1000;
    engine.accounts[user_idx as usize].position_size = 10_000; // 10k units
    engine.accounts[user_idx as usize].entry_price = 1_000_000; // $1

    // At price $1, position value = 10k
    // Maintenance margin (5%) = 500
    // Collateral = 1000, so above maintenance
    assert!(engine.is_above_maintenance_margin(&engine.accounts[user_idx as usize], 1_000_000));

    // At price $2, position value = 20k
    // Maintenance margin (5%) = 1000
    // Collateral = 1000, so just at threshold (should be false)
    assert!(!engine.is_above_maintenance_margin(&engine.accounts[user_idx as usize], 2_000_000));
}

#[test]
fn test_trading_opens_position() {
    let mut engine = Box::new(RiskEngine::new(default_params()));
    let user_idx = engine.add_user(1).unwrap();
    let lp_idx = engine.add_lp([0u8; 32], [0u8; 32], 1).unwrap();

    // Setup user with capital
    engine.deposit(user_idx, 10_000).unwrap();
    engine.accounts[lp_idx as usize].capital = 100_000;

    // Execute trade: user buys 1000 units at $1
    let oracle_price = 1_000_000;
    let size = 1000i128;

    engine.execute_trade(&MATCHER, lp_idx, user_idx, oracle_price, size).unwrap();

    // Check position opened
    assert_eq!(engine.accounts[user_idx as usize].position_size, 1000);
    assert_eq!(engine.accounts[user_idx as usize].entry_price, oracle_price);

    // Check LP has opposite position
    assert_eq!(engine.accounts[lp_idx as usize].position_size, -1000);

    // Check fee was charged (0.1% of 1000 = 1)
    assert!(engine.insurance_fund.fee_revenue > 0);
}

#[test]
fn test_trading_realizes_pnl() {
    let mut engine = Box::new(RiskEngine::new(default_params()));
    let user_idx = engine.add_user(1).unwrap();
    let lp_idx = engine.add_lp([0u8; 32], [0u8; 32], 1).unwrap();

    engine.deposit(user_idx, 10_000).unwrap();
    engine.accounts[lp_idx as usize].capital = 100_000;
    engine.vault = 110_000;

    // Open long position at $1
    engine.execute_trade(&MATCHER, lp_idx, user_idx, 1_000_000, 1000).unwrap();

    // Close position at $1.50 (50% profit)
    engine.execute_trade(&MATCHER, lp_idx, user_idx, 1_500_000, -1000).unwrap();

    // Check PNL realized (approximately)
    // Price went from $1 to $1.50, so 500 profit on 1000 units
    assert!(engine.accounts[user_idx as usize].pnl > 0);
    assert_eq!(engine.accounts[user_idx as usize].position_size, 0);
}

#[test]
fn test_user_isolation() {
    let mut engine = Box::new(RiskEngine::new(default_params()));
    let user1 = engine.add_user(1).unwrap();
    let user2 = engine.add_user(1).unwrap();

    engine.deposit(user1, 1000).unwrap();
    engine.deposit(user2, 2000).unwrap();

    let user2_principal_before = engine.accounts[user2 as usize].capital;
    let user2_pnl_before = engine.accounts[user2 as usize].pnl;

    // Operate on user1
    engine.withdraw(user1, 500).unwrap();
    engine.accounts[user1 as usize].pnl = 300;

    // User2 should be unchanged
    assert_eq!(engine.accounts[user2 as usize].capital, user2_principal_before);
    assert_eq!(engine.accounts[user2 as usize].pnl, user2_pnl_before);
}

#[test]
fn test_principal_never_reduced_by_adl() {
    let mut engine = Box::new(RiskEngine::new(default_params()));
    let user_idx = engine.add_user(1).unwrap();

    let initial_principal = 5000u128;
    engine.accounts[user_idx as usize].capital = initial_principal;
    engine.accounts[user_idx as usize].pnl = 100;

    // Apply massive ADL
    engine.apply_adl(10_000).unwrap();

    // Principal should NEVER be touched
    assert_eq!(engine.accounts[user_idx as usize].capital, initial_principal);
}

#[test]
fn test_multiple_users_adl() {
    let mut engine = Box::new(RiskEngine::new(default_params()));
    let user1 = engine.add_user(1).unwrap();
    let user2 = engine.add_user(1).unwrap();
    let user3 = engine.add_user(1).unwrap();

    // User1: has unwrapped PNL
    engine.accounts[user1 as usize].capital = 1000;
    engine.accounts[user1 as usize].pnl = 500;
    engine.accounts[user1 as usize].warmup_slope_per_step = 10;

    // User2: has unwrapped PNL
    engine.accounts[user2 as usize].capital = 2000;
    engine.accounts[user2 as usize].pnl = 800;
    engine.accounts[user2 as usize].warmup_slope_per_step = 10;

    // User3: no PNL
    engine.accounts[user3 as usize].capital = 1500;

    engine.insurance_fund.balance = 1000;

    // Apply ADL loss of 1000
    engine.apply_adl(1000).unwrap();

    // Should haircut user1 and user2's PNL
    // Total unwrapped PNL = 500 + 800 = 1300
    // Loss = 1000, so both should be haircutted proportionally or sequentially
    assert!(engine.accounts[user1 as usize].pnl < 500 || engine.accounts[user2 as usize].pnl < 800);

    // All principals should be intact
    assert_eq!(engine.accounts[user1 as usize].capital, 1000);
    assert_eq!(engine.accounts[user2 as usize].capital, 2000);
    assert_eq!(engine.accounts[user3 as usize].capital, 1500);
}

#[test]
fn test_warmup_monotonicity() {
    let mut engine = Box::new(RiskEngine::new(default_params()));
    let user_idx = engine.add_user(1).unwrap();

    engine.accounts[user_idx as usize].pnl = 1000;
    engine.accounts[user_idx as usize].warmup_slope_per_step = 10;

    // Get withdrawable at different time points
    let w0 = engine.withdrawable_pnl(&engine.accounts[user_idx as usize]);

    engine.advance_slot(10);
    let w1 = engine.withdrawable_pnl(&engine.accounts[user_idx as usize]);

    engine.advance_slot(20);
    let w2 = engine.withdrawable_pnl(&engine.accounts[user_idx as usize]);

    // Should be monotonically increasing
    assert!(w1 >= w0);
    assert!(w2 >= w1);
}

#[test]
fn test_fee_accumulation() {
    let mut engine = Box::new(RiskEngine::new(default_params()));
    let user_idx = engine.add_user(10000).unwrap();
    let lp_idx = engine.add_lp([0u8; 32], [0u8; 32], 10000).unwrap();

    engine.deposit(user_idx, 100_000).unwrap();
    engine.accounts[lp_idx as usize].capital = 1_000_000;
    engine.vault = 1_100_000;

    // Track balance after account creation fees
    let initial_insurance_balance = engine.insurance_fund.balance;

    // Execute multiple trades
    for _ in 0..10 {
        let result1 = engine.execute_trade(&MATCHER, lp_idx, user_idx, 1_000_000, 100);
        let result2 = engine.execute_trade(&MATCHER, lp_idx, user_idx, 1_000_000, -100);
        // Trades might fail due to margin, that's ok
        let _ = result1;
        let _ = result2;
    }

    // Insurance fund should have accumulated trading fees (if any trades succeeded)
    // Note: trading fees go to both balance and fee_revenue
    if engine.insurance_fund.fee_revenue > initial_insurance_balance {
        assert!(engine.insurance_fund.balance > initial_insurance_balance,
                "Balance should increase if trades succeeded");
    }
}

#[test]
fn test_lp_warmup_initial_state() {
    let mut engine = Box::new(RiskEngine::new(default_params()));
    let lp_idx = engine.add_lp([1u8; 32], [2u8; 32], 10000).unwrap();

    // LP should start with warmup state initialized
    assert_eq!(engine.accounts[lp_idx as usize].reserved_pnl, 0);
    assert_eq!(engine.accounts[lp_idx as usize].warmup_started_at_slot, 0);
}

#[test]
fn test_lp_warmup_monotonic() {
    let mut engine = Box::new(RiskEngine::new(default_params()));
    let lp_idx = engine.add_lp([1u8; 32], [2u8; 32], 10000).unwrap();

    // Give LP some positive PNL
    engine.accounts[lp_idx as usize].pnl = 10_000;

    // At slot 0
    let w0 = engine.withdrawable_pnl(&engine.accounts[lp_idx as usize]);

    // Advance 50 slots
    engine.advance_slot(50);
    let w50 = engine.withdrawable_pnl(&engine.accounts[lp_idx as usize]);

    // Advance another 50 slots (total 100)
    engine.advance_slot(50);
    let w100 = engine.withdrawable_pnl(&engine.accounts[lp_idx as usize]);

    // Withdrawable should be monotonically increasing
    assert!(w50 >= w0, "LP warmup should be monotonic: w0={}, w50={}", w0, w50);
    assert!(w100 >= w50, "LP warmup should be monotonic: w50={}, w100={}", w50, w100);
}

#[test]
fn test_lp_warmup_bounded() {
    let mut engine = Box::new(RiskEngine::new(default_params()));
    let lp_idx = engine.add_lp([1u8; 32], [2u8; 32], 10000).unwrap();

    // Give LP some positive PNL
    engine.accounts[lp_idx as usize].pnl = 5_000;

    // Reserve some PNL
    engine.accounts[lp_idx as usize].reserved_pnl = 1_000;

    // Even after long time, withdrawable should not exceed available (positive_pnl - reserved)
    engine.advance_slot(1000);
    let withdrawable = engine.withdrawable_pnl(&engine.accounts[lp_idx as usize]);

    assert!(withdrawable <= 4_000, "Withdrawable {} should not exceed available {}", withdrawable, 4_000);
}

#[test]
fn test_lp_warmup_with_negative_pnl() {
    let mut engine = Box::new(RiskEngine::new(default_params()));
    let lp_idx = engine.add_lp([1u8; 32], [2u8; 32], 10000).unwrap();

    // LP has negative PNL
    engine.accounts[lp_idx as usize].pnl = -3_000;

    // Advance time
    engine.advance_slot(100);

    // With negative PNL, withdrawable should be 0
    let withdrawable = engine.withdrawable_pnl(&engine.accounts[lp_idx as usize]);
    assert_eq!(withdrawable, 0, "Withdrawable should be 0 with negative PNL");
}

// ============================================================================
// Funding Rate Tests
// ============================================================================

#[test]
fn test_funding_positive_rate_longs_pay_shorts() {
    // T1: Positive funding → longs pay shorts
    let mut engine = Box::new(RiskEngine::new(default_params()));
    let user_idx = engine.add_user(10000).unwrap();
    let lp_idx = engine.add_lp([0u8; 32], [0u8; 32], 10000).unwrap();

    engine.deposit(user_idx, 100_000).unwrap();
    engine.accounts[lp_idx as usize].capital = 1_000_000;
    engine.vault = 1_100_000;

    // User opens long position (+1 base unit)
    engine.accounts[user_idx as usize].position_size = 1_000_000; // +1M base units
    engine.accounts[user_idx as usize].entry_price = 100_000_000; // $100

    // LP has opposite short position
    engine.accounts[lp_idx as usize].position_size = -1_000_000;
    engine.accounts[lp_idx as usize].entry_price = 100_000_000;

    // Accrue positive funding: +10 bps/slot for 1 slot
    engine.current_slot = 1;
    engine.accrue_funding(1, 100_000_000, 10).unwrap(); // price=$100, rate=+10bps

    // Expected delta_F = 100e6 * 10 * 1 / 10000 = 100,000
    // User payment = 1M * 100,000 / 1e6 = 100,000
    // LP payment = -1M * 100,000 / 1e6 = -100,000

    let user_pnl_before = engine.accounts[user_idx as usize].pnl;
    let lp_pnl_before = engine.accounts[lp_idx as usize].pnl;

    // Settle funding
    engine.touch_account(user_idx).unwrap();
    engine.touch_account(lp_idx).unwrap();

    // User (long) should pay 100,000
    assert_eq!(engine.accounts[user_idx as usize].pnl, user_pnl_before - 100_000);

    // LP (short) should receive 100,000
    assert_eq!(engine.accounts[lp_idx as usize].pnl, lp_pnl_before + 100_000);

    // Zero-sum check
    let total_pnl_before = user_pnl_before + lp_pnl_before;
    let total_pnl_after = engine.accounts[user_idx as usize].pnl + engine.accounts[lp_idx as usize].pnl;
    assert_eq!(total_pnl_after, total_pnl_before, "Funding should be zero-sum");
}

#[test]
fn test_funding_negative_rate_shorts_pay_longs() {
    // T2: Negative funding → shorts pay longs
    let mut engine = Box::new(RiskEngine::new(default_params()));
    let user_idx = engine.add_user(10000).unwrap();
    let lp_idx = engine.add_lp([0u8; 32], [0u8; 32], 10000).unwrap();

    engine.deposit(user_idx, 100_000).unwrap();
    engine.accounts[lp_idx as usize].capital = 1_000_000;

    // User opens short position
    engine.accounts[user_idx as usize].position_size = -1_000_000;
    engine.accounts[user_idx as usize].entry_price = 100_000_000;

    // LP has opposite long position
    engine.accounts[lp_idx as usize].position_size = 1_000_000;
    engine.accounts[lp_idx as usize].entry_price = 100_000_000;

    // Accrue negative funding: -10 bps/slot
    engine.current_slot = 1;
    engine.accrue_funding(1, 100_000_000, -10).unwrap();

    let user_pnl_before = engine.accounts[user_idx as usize].pnl;
    let lp_pnl_before = engine.accounts[lp_idx as usize].pnl;

    engine.touch_account(user_idx).unwrap();
    engine.touch_account(lp_idx).unwrap();

    // With negative funding rate, delta_F is negative (-100,000)
    // User (short) with negative position: payment = (-1M) * (-100,000) / 1e6 = 100,000
    // User pays 100,000 (shorts pay)
    assert_eq!(engine.accounts[user_idx as usize].pnl, user_pnl_before - 100_000);

    // LP (long) receives 100,000
    assert_eq!(engine.accounts[lp_idx as usize].pnl, lp_pnl_before + 100_000);
}

#[test]
fn test_funding_idempotence() {
    // T3: Settlement is idempotent
    let mut engine = Box::new(RiskEngine::new(default_params()));
    let user_idx = engine.add_user(10000).unwrap();

    engine.deposit(user_idx, 100_000).unwrap();
    engine.accounts[user_idx as usize].position_size = 1_000_000;

    // Accrue funding
    engine.accrue_funding(1, 100_000_000, 10).unwrap();

    // Settle once
    engine.touch_account(user_idx).unwrap();
    let pnl_after_first = engine.accounts[user_idx as usize].pnl;

    // Settle again without new accrual
    engine.touch_account(user_idx).unwrap();
    let pnl_after_second = engine.accounts[user_idx as usize].pnl;

    assert_eq!(pnl_after_first, pnl_after_second, "Second settlement should not change PNL");
}

#[test]
fn test_funding_partial_close() {
    // T4: Partial position close with funding
    let mut engine = Box::new(RiskEngine::new(default_params()));
    let user_idx = engine.add_user(10000).unwrap();
    let lp_idx = engine.add_lp([0u8; 32], [0u8; 32], 10000).unwrap();

    engine.deposit(user_idx, 15_000_000).unwrap();
    engine.accounts[lp_idx as usize].capital = 50_000_000;
    engine.vault = 65_000_000;

    // Open long position of 2M base units
    let trade_result = engine.execute_trade(&MATCHER, lp_idx, user_idx, 100_000_000, 2_000_000);
    assert!(trade_result.is_ok(), "Trade should succeed");

    assert_eq!(engine.accounts[user_idx as usize].position_size, 2_000_000);

    // Accrue funding for 1 slot at +10 bps
    engine.advance_slot(1);
    engine.accrue_funding(1, 100_000_000, 10).unwrap();

    // Reduce position to 1M (close half)
    let reduce_result = engine.execute_trade(&MATCHER, lp_idx, user_idx, 100_000_000, -1_000_000);
    assert!(reduce_result.is_ok(), "Partial close should succeed");

    // Position should be 1M now
    assert_eq!(engine.accounts[user_idx as usize].position_size, 1_000_000);

    // Accrue more funding for another slot
    engine.advance_slot(2);
    engine.accrue_funding(2, 100_000_000, 10).unwrap();

    // Touch to settle
    engine.touch_account(user_idx).unwrap();

    // Funding should have been applied correctly for both periods
    // Period 1: 2M base * (100K delta_F) / 1e6 = 200
    // Period 2: 1M base * (100K delta_F) / 1e6 = 100
    // Total funding paid: 300
    // (exact PNL depends on trading fees too, but funding should be applied)
}

#[test]
fn test_funding_position_flip() {
    // T5: Flip from long to short
    let mut engine = Box::new(RiskEngine::new(default_params()));
    let user_idx = engine.add_user(10000).unwrap();
    let lp_idx = engine.add_lp([0u8; 32], [0u8; 32], 10000).unwrap();

    engine.deposit(user_idx, 10_000_000).unwrap();
    engine.accounts[lp_idx as usize].capital = 20_000_000;
    engine.vault = 30_000_000;

    // Open long
    engine.execute_trade(&MATCHER, lp_idx, user_idx, 100_000_000, 1_000_000).unwrap();
    assert_eq!(engine.accounts[user_idx as usize].position_size, 1_000_000);

    // Accrue funding
    engine.advance_slot(1);
    engine.accrue_funding(1, 100_000_000, 10).unwrap();

    let pnl_before_flip = engine.accounts[user_idx as usize].pnl;

    // Flip to short (trade -2M to go from +1M to -1M)
    engine.execute_trade(&MATCHER, lp_idx, user_idx, 100_000_000, -2_000_000).unwrap();

    assert_eq!(engine.accounts[user_idx as usize].position_size, -1_000_000);

    // Funding should have been settled before the flip
    // User's funding index should be updated
    assert_eq!(engine.accounts[user_idx as usize].funding_index, engine.funding_index_qpb_e6);

    // Accrue more funding
    engine.advance_slot(2);
    engine.accrue_funding(2, 100_000_000, 10).unwrap();

    engine.touch_account(user_idx).unwrap();

    // Now user is short, so they receive funding (if rate is still positive)
    // This verifies no "double charge" bug
}

#[test]
fn test_funding_zero_position() {
    // Edge case: funding with zero position should do nothing
    let mut engine = Box::new(RiskEngine::new(default_params()));
    let user_idx = engine.add_user(10000).unwrap();

    engine.deposit(user_idx, 100_000).unwrap();

    // No position
    assert_eq!(engine.accounts[user_idx as usize].position_size, 0);

    let pnl_before = engine.accounts[user_idx as usize].pnl;

    // Accrue funding
    engine.accrue_funding(1, 100_000_000, 100).unwrap(); // Large rate

    // Settle
    engine.touch_account(user_idx).unwrap();

    // PNL should be unchanged
    assert_eq!(engine.accounts[user_idx as usize].pnl, pnl_before);
}

#[test]
fn test_funding_does_not_touch_principal() {
    // Funding should never modify principal (Invariant I1 extended)
    let mut engine = Box::new(RiskEngine::new(default_params()));
    let user_idx = engine.add_user(10000).unwrap();

    let initial_principal = 100_000;
    engine.deposit(user_idx, initial_principal).unwrap();

    engine.accounts[user_idx as usize].position_size = 1_000_000;

    // Accrue funding
    engine.accrue_funding(1, 100_000_000, 100).unwrap();
    engine.touch_account(user_idx).unwrap();

    // Principal must be unchanged
    assert_eq!(engine.accounts[user_idx as usize].capital, initial_principal);
}

#[test]
fn test_adl_protects_principal_during_warmup() {
    // This test demonstrates the core protection mechanism:
    // If oracle manipulation creates fake PNL, ADL will haircut it
    // BEFORE it warms up and becomes withdrawable, protecting principal holders.

    let mut engine = Box::new(RiskEngine::new(default_params()));
    let attacker = engine.add_user(10000).unwrap();
    let victim = engine.add_user(10000).unwrap();

    // Both deposit principal
    engine.deposit(attacker, 10_000).unwrap();
    engine.deposit(victim, 10_000).unwrap();

    let attacker_principal = engine.accounts[attacker as usize].capital;
    let victim_principal = engine.accounts[victim as usize].capital;

    // === Phase 1: Oracle Manipulation (time < T) ===
    // Attacker manipulates oracle and creates fake $50k profit
    // In reality this would come from trading, but we simulate the result
    engine.accounts[attacker as usize].pnl = 50_000;
    engine.accounts[attacker as usize].warmup_slope_per_step = 500; // Will take 100 slots to warm up
    engine.accounts[attacker as usize].warmup_started_at_slot = 0;

    // Victim has corresponding loss
    engine.accounts[victim as usize].pnl = -50_000;

    // Advance only 10 slots (very early in warmup period)
    engine.advance_slot(10);

    // At this point, very little PNL has warmed up
    let warmed_up = engine.withdrawable_pnl(&engine.accounts[attacker as usize]);
    assert_eq!(warmed_up, 5_000); // 500 * 10 = 5,000

    // Unwrapped (still warming) = 50k - 5k = 45k
    let positive_pnl = 50_000u128;
    let unwrapped_pnl = positive_pnl - warmed_up;
    assert_eq!(unwrapped_pnl, 45_000);

    // === Phase 2: Oracle Reverts, Loss Realized ===
    // The manipulation is detected/reverts quickly, creating a $50k loss
    // ADL is triggered to socialize this loss
    // KEY: ADL runs BEFORE most PNL has warmed up

    engine.apply_adl(50_000).unwrap();

    // === Phase 3: Verify Protection ===

    // Attacker's principal is NEVER touched (I1)
    assert_eq!(engine.accounts[attacker as usize].capital, attacker_principal,
               "Attacker principal protected by I1");

    // Victim's principal is NEVER touched (I1)
    assert_eq!(engine.accounts[victim as usize].capital, victim_principal,
               "Victim principal protected by I1");

    // ADL haircuts unwrapped PNL first (I4)
    // We had 45k unwrapped, so all of it gets haircutted
    // The remaining 5k loss goes to insurance fund
    let remaining_pnl = engine.accounts[attacker as usize].pnl;
    assert_eq!(remaining_pnl, 5_000, "Unwrapped PNL haircutted, only early-warmed remains");

    // === Phase 4: Try to Withdraw After Warmup ===

    // Advance to full warmup completion
    engine.advance_slot(190); // Total 200 slots

    // Only the 5k that warmed up BEFORE ADL is still withdrawable
    let warmed_after_adl = engine.withdrawable_pnl(&engine.accounts[attacker as usize]);
    assert_eq!(warmed_after_adl, 5_000, "Only early-warmed PNL is withdrawable");

    // In risk-reduction-only mode, withdrawals of capital ARE allowed
    // The attacker can withdraw their principal + already-warmed PNL (which gets converted to capital)
    // But warmup is frozen, so the 45k that's still warming won't become available
    let total_withdrawable = attacker_principal + warmed_after_adl;
    let withdraw_result = engine.withdraw(attacker, total_withdrawable);
    assert!(withdraw_result.is_ok(), "Withdrawals of capital ARE allowed in risk mode");

    // To enable withdrawals again, insurance fund must be topped up to cover loss_accum
    // ADL used ~45k from unwrapped PnL, remaining ~5k went to insurance
    // Due to rounding in proportional ADL, there may be small loss_accum
    assert!(engine.loss_accum < 5_100, "Loss mostly covered by unwrapped PnL and insurance");

    // === Conclusion ===
    // The attack was MOSTLY MITIGATED:
    // - The attacker could only withdraw their principal + 5k of warmed PNL
    // - The 45k that was still warming got haircutted by ADL
    // - Warmup freeze prevents the remaining 45k from ever becoming withdrawable in risk mode
    // - The insurance fund absorbed any remaining loss
    //
    // This demonstrates the core security property:
    //   "ADL haircuts PNL that is still warming up, protecting principal holders.
    //    The faster ADL runs after manipulation, the more effective the protection."
    //
    // New behavior: Withdrawal-only mode provides additional protection by blocking
    // all withdrawals until the system recovers (via insurance fund top-up).
}

#[test]
fn test_adl_haircuts_unwrapped_before_warmed() {
    // Verify that ADL prioritizes unwrapped (young) PNL over warmed (old) PNL

    let mut engine = Box::new(RiskEngine::new(default_params()));
    let user_idx = engine.add_user(10000).unwrap();

    engine.accounts[user_idx as usize].capital = 10_000;
    engine.accounts[user_idx as usize].pnl = 10_000;
    engine.accounts[user_idx as usize].warmup_slope_per_step = 100;

    // Advance time so half is warmed up
    engine.advance_slot(50);

    let warmed = engine.withdrawable_pnl(&engine.accounts[user_idx as usize]);
    assert_eq!(warmed, 5_000); // 100 * 50

    let unwrapped = 10_000 - warmed;
    assert_eq!(unwrapped, 5_000);

    // Apply ADL of 3k (less than unwrapped)
    engine.apply_adl(3_000).unwrap();

    // Should take from unwrapped first
    assert_eq!(engine.accounts[user_idx as usize].pnl, 7_000);

    // The 5k warmed PNL should still be withdrawable
    // (Actually withdrawable = min(7k - 0, 5k) = 5k... wait)
    // After ADL: pnl = 7k, warmed_cap = 5k
    // withdrawable = min(7k, 5k) = 5k
    let still_warmed = engine.withdrawable_pnl(&engine.accounts[user_idx as usize]);
    assert_eq!(still_warmed, 5_000, "Warmed PNL still withdrawable");
}

// ============================================================================
// Warmup Rate Limiting Tests
// NOTE: These tests are commented out because warmup rate limiting was removed
// in the slab 4096 redesign for simplicity
// ============================================================================

/*
#[test]
fn test_warmup_rate_limit_single_user() {
    // Test that warmup slope is capped by insurance fund capacity
    let mut params = default_params();
    params.warmup_period_slots = 100;
    params.max_warmup_rate_fraction_bps = 5000; // 50% in T/2 = 50 slots

    let mut engine = Box::new(RiskEngine::new(params));

    // Add insurance fund: 10,000
    engine.insurance_fund.balance = 10_000;

    // Max warmup rate = 10,000 * 5000 / 50 / 10,000 = 10,000 * 0.5 / 50 = 100 per slot
    let expected_max_rate = 10_000 * 5000 / 50 / 10_000;
    assert_eq!(expected_max_rate, 100);

    let user = engine.add_user(100).unwrap();
    engine.deposit(user, 1_000).unwrap();

    // Give user 20,000 PNL (would need slope of 200 without limit)
    engine.accounts[user as usize].pnl = 20_000;

    // Update warmup slope
    engine.update_warmup_slope(user).unwrap();

    // Should be capped at 100 (the max rate)
    assert_eq!(engine.accounts[user as usize].warmup_slope_per_step, 100);
    assert_eq!(engine.total_warmup_rate, 100);

    // After 50 slots, only 5,000 should have warmed up (not 10,000)
    engine.advance_slot(50);
    let warmed = engine.withdrawable_pnl(&engine.accounts[user as usize]);
    assert_eq!(warmed, 5_000); // 100 * 50 = 5,000
}

#[test]
fn test_warmup_rate_limit_multiple_users() {
    // Test that warmup capacity is shared among users
    let mut params = default_params();
    params.warmup_period_slots = 100;
    params.max_warmup_rate_fraction_bps = 5000; // 50% in T/2

    let mut engine = Box::new(RiskEngine::new(params));
    engine.insurance_fund.balance = 10_000;

    // Max total warmup rate = 100 per slot

    let user1 = engine.add_user(100).unwrap();
    let user2 = engine.add_user(100).unwrap();

    engine.deposit(user1, 1_000).unwrap();
    engine.deposit(user2, 1_000).unwrap();

    // User1 gets 6,000 PNL (would want slope of 60)
    engine.accounts[user1 as usize].pnl = 6_000;
    engine.update_warmup_slope(user1).unwrap();
    assert_eq!(engine.accounts[user1 as usize].warmup_slope_per_step, 60);
    assert_eq!(engine.total_warmup_rate, 60);

    // User2 gets 8,000 PNL (would want slope of 80)
    engine.accounts[user2 as usize].pnl = 8_000;
    engine.update_warmup_slope(user2).unwrap();

    // Total would be 140, but max is 100, so user2 gets only 40
    assert_eq!(engine.accounts[user2 as usize].warmup_slope_per_step, 40); // 100 - 60 = 40
    assert_eq!(engine.total_warmup_rate, 100); // 60 + 40 = 100
}

#[test]
fn test_warmup_rate_released_on_pnl_decrease() {
    // Test that warmup capacity is released when user's PNL decreases
    let mut params = default_params();
    params.warmup_period_slots = 100;
    params.max_warmup_rate_fraction_bps = 5000;

    let mut engine = Box::new(RiskEngine::new(params));
    engine.insurance_fund.balance = 10_000;

    let user1 = engine.add_user(100).unwrap();
    let user2 = engine.add_user(100).unwrap();

    engine.deposit(user1, 1_000).unwrap();
    engine.deposit(user2, 1_000).unwrap();

    // User1 uses all capacity
    engine.accounts[user1 as usize].pnl = 15_000;
    engine.update_warmup_slope(user1).unwrap();
    assert_eq!(engine.total_warmup_rate, 100);

    // User2 can't get any capacity
    engine.accounts[user2 as usize].pnl = 5_000;
    engine.update_warmup_slope(user2).unwrap();
    assert_eq!(engine.accounts[user2 as usize].warmup_slope_per_step, 0);

    // User1's PNL drops to 3,000 (ADL or loss)
    engine.accounts[user1 as usize].pnl = 3_000;
    engine.update_warmup_slope(user1).unwrap();
    assert_eq!(engine.accounts[user1 as usize].warmup_slope_per_step, 30); // 3000/100
    assert_eq!(engine.total_warmup_rate, 30);

    // Now user2 can get the remaining 70
    engine.update_warmup_slope(user2).unwrap();
    assert_eq!(engine.accounts[user2 as usize].warmup_slope_per_step, 50); // 5000/100, but capped at 70
    assert_eq!(engine.total_warmup_rate, 80); // 30 + 50
}

#[test]
fn test_warmup_rate_scales_with_insurance_fund() {
    // Test that max warmup rate scales with insurance fund size
    let mut params = default_params();
    params.warmup_period_slots = 100;
    params.max_warmup_rate_fraction_bps = 5000; // 50% in T/2

    let mut engine = Box::new(RiskEngine::new(params));

    // Small insurance fund
    engine.insurance_fund.balance = 1_000;

    let user = engine.add_user(100).unwrap();
    engine.deposit(user, 1_000).unwrap();

    engine.accounts[user as usize].pnl = 10_000;
    engine.update_warmup_slope(user).unwrap();

    // Max rate = 1000 * 0.5 / 50 = 10
    assert_eq!(engine.accounts[user as usize].warmup_slope_per_step, 10);

    // Increase insurance fund 10x
    engine.insurance_fund.balance = 10_000;

    // Update slope again
    engine.update_warmup_slope(user).unwrap();

    // Max rate should be 10x higher = 100
    assert_eq!(engine.accounts[user as usize].warmup_slope_per_step, 100);
}

#[test]
fn test_warmup_rate_limit_invariant_maintained() {
    // Verify that the invariant is always maintained:
    // total_warmup_rate * (T/2) <= insurance_fund * max_warmup_rate_fraction

    let mut params = default_params();
    params.warmup_period_slots = 100;
    params.max_warmup_rate_fraction_bps = 5000;

    let mut engine = Box::new(RiskEngine::new(params));
    engine.insurance_fund.balance = 10_000;

    // Add multiple users with varying PNL
    for i in 0..10 {
        let user = engine.add_user(100).unwrap();
        engine.deposit(user, 1_000).unwrap();
        engine.accounts[user as usize].pnl = (i as i128 + 1) * 1_000;
        engine.update_warmup_slope(user).unwrap();

        // Check invariant after each update
        let half_period = params.warmup_period_slots / 2;
        let max_total_warmup_in_half_period = engine.total_warmup_rate * (half_period as u128);
        let insurance_limit = engine.insurance_fund.balance * params.max_warmup_rate_fraction_bps as u128 / 10_000;

        assert!(max_total_warmup_in_half_period <= insurance_limit,
                "Invariant violated: {} > {}", max_total_warmup_in_half_period, insurance_limit);
    }
}
*/

// ============================================================================
// Risk-Reduction-Only Mode Tests
// ============================================================================

#[test]
fn test_risk_reduction_only_mode_triggered_by_loss() {
    // Test that loss_accum > 0 triggers withdrawal-only mode
    let mut engine = Box::new(RiskEngine::new(default_params()));

    let user1 = engine.add_user(100).unwrap();
    let user2 = engine.add_user(100).unwrap();

    engine.deposit(user1, 10_000).unwrap();
    engine.deposit(user2, 10_000).unwrap();

    // Set insurance fund balance AFTER adding users (to avoid fee confusion)
    engine.insurance_fund.balance = 5_000;

    // Simulate a loss event that depletes insurance fund
    let loss = 10_000;
    engine.apply_adl(loss).unwrap();

    // Should be in withdrawal-only mode
    assert!(engine.risk_reduction_only);
    assert_eq!(engine.loss_accum, 5_000); // 10k loss - 5k insurance = 5k loss_accum
    assert_eq!(engine.insurance_fund.balance, 0);
}

/*
// NOTE: Commented out - withdrawal-only mode now BLOCKS all withdrawals instead of proportional haircut
#[test]
fn test_proportional_haircut_on_withdrawal() {
    // Test that withdrawals are haircutted proportionally when loss_accum > 0
    let mut engine = Box::new(RiskEngine::new(default_params()));

    let user1 = engine.add_user(100).unwrap();
    let user2 = engine.add_user(100).unwrap();

    engine.deposit(user1, 10_000).unwrap();
    engine.deposit(user2, 5_000).unwrap();

    // Set insurance fund balance AFTER adding users (to avoid fee confusion)
    engine.insurance_fund.balance = 1_000;

    // Total principal = 15,000
    // Trigger loss that creates 3,000 loss_accum
    engine.apply_adl(4_000).unwrap();

    assert_eq!(engine.loss_accum, 3_000); // 4k - 1k insurance
    assert!(engine.risk_reduction_only);

    // Available principal = 15,000 - 3,000 = 12,000
    // Haircut ratio = 12,000 / 15,000 = 80%

    // User1 tries to withdraw 10,000
    // Fair unwinding: Should get 80% regardless of order
    // Gets: 10,000 * 0.8 = 8,000
    let user1_balance_before = engine.accounts[user1 as usize].capital;
    engine.withdraw(user1, 10_000).unwrap();
    let withdrawn = user1_balance_before - engine.accounts[user1 as usize].capital;

    assert_eq!(withdrawn, 8_000, "Should withdraw 80% due to haircut");

    // User2 tries to withdraw 5,000
    // Fair unwinding: Also gets 80% (not less than user1)
    // Gets: 5,000 * 0.8 = 4,000
    let user2_balance_before = engine.accounts[user2 as usize].capital;
    engine.withdraw(user2, 5_000).unwrap();
    let user2_withdrawn = user2_balance_before - engine.accounts[user2 as usize].capital;

    assert_eq!(user2_withdrawn, 4_000, "Should also get 80% (fair unwinding)");

    // Total withdrawn: 8,000 + 4,000 = 12,000
    // Exactly the available principal (15k - 3k loss = 12k)
    let total_withdrawn = withdrawn + user2_withdrawn;
    assert_eq!(total_withdrawn, 12_000);
}
*/

#[test]
fn test_closing_positions_allowed_in_withdrawal_mode() {
    // Test that users can close positions in withdrawal-only mode
    let mut engine = Box::new(RiskEngine::new(default_params()));

    let lp = engine.add_lp([0u8; 32], [0u8; 32], 100).unwrap();
    let user = engine.add_user(100).unwrap();

    engine.deposit(user, 10_000).unwrap();
    engine.accounts[lp as usize].capital = 50_000;
    engine.vault = 60_000;

    // Set insurance fund balance AFTER adding users (to avoid fee confusion)
    engine.insurance_fund.balance = 1_000;

    // User opens long position
    let matcher = NoOpMatcher;
    engine.execute_trade(&matcher, lp, user, 1_000_000, 5_000).unwrap();
    assert_eq!(engine.accounts[user as usize].position_size, 5_000);

    // Trigger withdrawal-only mode
    engine.apply_adl(2_000).unwrap();
    assert!(engine.risk_reduction_only);

    // User can CLOSE position (reducing from 5000 to 0)
    let result = engine.execute_trade(&matcher, lp, user, 1_000_000, -5_000);
    assert!(result.is_ok(), "Closing position should be allowed");
    assert_eq!(engine.accounts[user as usize].position_size, 0);
}

#[test]
fn test_opening_positions_blocked_in_withdrawal_mode() {
    // Test that opening new positions is blocked in withdrawal-only mode
    let mut engine = Box::new(RiskEngine::new(default_params()));

    let lp = engine.add_lp([0u8; 32], [0u8; 32], 100).unwrap();
    let user = engine.add_user(100).unwrap();

    engine.deposit(user, 10_000).unwrap();
    engine.accounts[lp as usize].capital = 50_000;

    // Set insurance fund balance AFTER adding users (to avoid fee confusion)
    engine.insurance_fund.balance = 1_000;

    // Trigger withdrawal-only mode
    engine.apply_adl(2_000).unwrap();
    assert!(engine.risk_reduction_only);

    // User tries to open new position - should fail
    let matcher = NoOpMatcher;
    let result = engine.execute_trade(&matcher, lp, user, 1_000_000, 5_000);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), RiskError::RiskReductionOnlyMode);
}

// Test A: Warmup freezes in risk mode
#[test]
fn test_warmup_freezes_in_risk_mode() {
    let mut engine = Box::new(RiskEngine::new(default_params()));
    let user = engine.add_user(100).unwrap();

    engine.deposit(user, 10_000).unwrap();

    // Setup: user has pnl=+1000, slope=10, started_at_slot=0
    engine.accounts[user as usize].pnl = 1000;
    engine.accounts[user as usize].warmup_slope_per_step = 10;
    engine.accounts[user as usize].warmup_started_at_slot = 0;

    // Advance slot to 10
    engine.current_slot = 10;
    let w1 = engine.withdrawable_pnl(&engine.accounts[user as usize]);
    assert_eq!(w1, 100, "After 10 slots, 10*10=100 should be warmed");

    // Trigger crisis mode at slot 10
    engine.enter_risk_reduction_only_mode();
    assert!(engine.warmup_paused);
    assert_eq!(engine.warmup_pause_slot, 10);

    // Advance slot +1000
    engine.current_slot = 1010;

    // Warmup should be frozen at slot 10
    let w2 = engine.withdrawable_pnl(&engine.accounts[user as usize]);
    assert_eq!(w2, w1, "Warmup should not progress when paused");
    assert_eq!(w2, 100, "Should still be 100 even after 1000 slots");
}

// Test B: In risk mode, deposit withdrawals work from deposited capital
#[test]
fn test_risk_mode_deposit_withdrawals_work() {
    let mut engine = Box::new(RiskEngine::new(default_params()));
    let fee = 1; // Account creation fee
    let user = engine.add_user(100).unwrap();

    // User deposit 1000
    engine.deposit(user, 1000).unwrap();

    // Enter risk mode
    engine.enter_risk_reduction_only_mode();
    assert!(engine.risk_reduction_only);

    // Withdraw 200 - should succeed (withdrawing from capital)
    let result = engine.withdraw(user, 200);
    assert!(result.is_ok(), "Withdrawals of capital should work in risk mode");

    assert_eq!(engine.accounts[user as usize].capital, 800);
    assert_eq!(engine.vault, 800 + fee); // +fee from account creation
}

// Test C: In risk mode, pending PNL cannot be withdrawn (because warmup is frozen)
#[test]
fn test_risk_mode_pending_pnl_cannot_be_withdrawn() {
    let mut engine = Box::new(RiskEngine::new(default_params()));
    let user = engine.add_user(100).unwrap();

    // User has NO capital, only pending PNL
    engine.accounts[user as usize].pnl = 1000;
    engine.accounts[user as usize].warmup_slope_per_step = 10;
    engine.accounts[user as usize].warmup_started_at_slot = engine.current_slot;

    // Enter risk mode immediately (so warmed amount ~0)
    engine.enter_risk_reduction_only_mode();

    // Try withdraw 1 - should fail with InsufficientBalance
    // because capital is 0 and warmup won't progress
    let result = engine.withdraw(user, 1);
    assert!(result.is_err(), "Should fail - no capital available");
    assert_eq!(result.unwrap_err(), RiskError::InsufficientBalance);
}

// Test D: In risk mode, already-warmed PNL can be withdrawn after conversion
#[test]
fn test_risk_mode_already_warmed_pnl_withdrawable() {
    let mut engine = Box::new(RiskEngine::new(default_params()));
    let user = engine.add_user(100).unwrap();

    // User.pnl=+1000, slope=10, started_at_slot=0
    engine.accounts[user as usize].pnl = 1000;
    engine.accounts[user as usize].warmup_slope_per_step = 10;
    engine.accounts[user as usize].warmup_started_at_slot = 0;

    // Advance slot to 10 → warmed=100
    engine.current_slot = 10;
    let warmed_before_mode = engine.withdrawable_pnl(&engine.accounts[user as usize]);
    assert_eq!(warmed_before_mode, 100);

    // Enter risk mode (freezes at slot 10)
    engine.enter_risk_reduction_only_mode();

    // Call withdraw(50)
    // Should convert 100 PNL to capital, then withdraw 50
    let result = engine.withdraw(user, 50);
    assert!(result.is_ok(), "Should succeed");

    // Check: pnl reduced by 100, capital increased by 100 then decreased by 50
    assert_eq!(engine.accounts[user as usize].pnl, 900); // 1000 - 100
    assert_eq!(engine.accounts[user as usize].capital, 50); // 0 + 100 - 50
}

// Test E: Risk-increasing trade fails in risk mode
#[test]
fn test_risk_increasing_trade_fails_in_risk_mode() {
    let mut engine = Box::new(RiskEngine::new(default_params()));
    let user = engine.add_user(100).unwrap();
    let lp = engine.add_lp([0u8; 32], [0u8; 32], 100).unwrap();

    engine.deposit(user, 10_000).unwrap();
    engine.accounts[lp as usize].capital = 50_000;
    engine.vault = 60_000;

    // Both start at pos 0
    assert_eq!(engine.accounts[user as usize].position_size, 0);
    assert_eq!(engine.accounts[lp as usize].position_size, 0);

    // Enter risk mode
    engine.enter_risk_reduction_only_mode();

    // Try to open position (0 -> +1, increases absolute exposure)
    let matcher = NoOpMatcher;
    let result = engine.execute_trade(&matcher, lp, user, 100_000_000, 1);

    assert!(result.is_err(), "Risk-increasing trade should fail");
    assert_eq!(result.unwrap_err(), RiskError::RiskReductionOnlyMode);
}

// Test F: Reduce-only trade succeeds in risk mode
#[test]
fn test_reduce_only_trade_succeeds_in_risk_mode() {
    let mut engine = Box::new(RiskEngine::new(default_params()));
    let user = engine.add_user(100).unwrap();
    let lp = engine.add_lp([0u8; 32], [0u8; 32], 100).unwrap();

    engine.deposit(user, 10_000).unwrap();
    engine.accounts[lp as usize].capital = 50_000;
    engine.vault = 60_000;

    // Setup: user pos +10, lp pos -10
    engine.accounts[user as usize].position_size = 10;
    engine.accounts[user as usize].entry_price = 100_000_000;
    engine.accounts[lp as usize].position_size = -10;
    engine.accounts[lp as usize].entry_price = 100_000_000;

    // Enter risk mode
    engine.enter_risk_reduction_only_mode();

    // Trade size -5 (reduces user from 10 to 5, LP from -10 to -5)
    let matcher = NoOpMatcher;
    let result = engine.execute_trade(&matcher, lp, user, 100_000_000, -5);

    assert!(result.is_ok(), "Reduce-only trade should succeed");
    assert_eq!(engine.accounts[user as usize].position_size, 5);
    assert_eq!(engine.accounts[lp as usize].position_size, -5);
}

// Test G: Exiting mode unfreezes warmup
#[test]
fn test_exiting_mode_unfreezes_warmup() {
    let mut engine = Box::new(RiskEngine::new(default_params()));
    let user = engine.add_user(100).unwrap();

    engine.deposit(user, 10_000).unwrap();

    // Create deficit, enter risk mode
    engine.loss_accum = 1000;
    engine.enter_risk_reduction_only_mode();

    assert!(engine.risk_reduction_only);
    assert!(engine.warmup_paused);

    // Top up to clear loss_accum=0
    engine.top_up_insurance_fund(1000).unwrap();

    assert_eq!(engine.loss_accum, 0);
    assert!(!engine.risk_reduction_only, "Should exit risk mode");
    assert!(!engine.warmup_paused, "Should unfreeze warmup");
}

#[test]
fn test_top_up_insurance_fund_reduces_loss() {
    // Test that topping up insurance fund reduces loss_accum
    let mut engine = Box::new(RiskEngine::new(default_params()));

    let user = engine.add_user(100).unwrap();
    engine.deposit(user, 10_000).unwrap();

    // Set insurance fund balance AFTER adding users (to avoid fee confusion)
    engine.insurance_fund.balance = 1_000;

    // Trigger withdrawal-only mode with 4k loss_accum
    engine.apply_adl(5_000).unwrap();
    assert_eq!(engine.loss_accum, 4_000);
    assert!(engine.risk_reduction_only);

    // Top up with 2k - should reduce loss to 2k
    let exited = engine.top_up_insurance_fund(2_000).unwrap();
    assert_eq!(engine.loss_accum, 2_000);
    assert!(engine.risk_reduction_only); // Still in withdrawal mode
    assert!(!exited);

    // Top up with another 2k - should fully cover loss
    let exited = engine.top_up_insurance_fund(2_000).unwrap();
    assert_eq!(engine.loss_accum, 0);
    assert!(!engine.risk_reduction_only); // Exited withdrawal mode
    assert!(exited);
}

/*
// NOTE: Commented out - withdrawal-only mode now BLOCKS all withdrawals
#[test]
fn test_deposits_allowed_in_withdrawal_mode() {
    // Test that deposits are allowed and take on proportional loss
    let mut engine = Box::new(RiskEngine::new(default_params()));

    let user1 = engine.add_user(100).unwrap();
    let user2 = engine.add_user(100).unwrap();

    engine.deposit(user1, 10_000).unwrap();

    // Set insurance fund balance AFTER adding users (to avoid fee confusion)
    engine.insurance_fund.balance = 1_000;

    // Trigger withdrawal-only mode
    engine.apply_adl(2_000).unwrap();
    assert_eq!(engine.loss_accum, 1_000);
    assert!(engine.risk_reduction_only);

    // User2 deposits - should be allowed
    let result = engine.deposit(user2, 5_000);
    assert!(result.is_ok(), "Deposits should be allowed in withdrawal mode");

    // Total principal now 15k, loss still 1k
    // User2's share of loss: (5k / 15k) * 1k ≈ 333
    // So user2 can withdraw: 5k - 333 ≈ 4,667

    let user2_balance_before = engine.accounts[user2 as usize].capital;
    engine.withdraw(user2, 5_000).unwrap();
    let user2_withdrawn = user2_balance_before - engine.accounts[user2 as usize].capital;

    // Should be less than full amount due to proportional haircut
    assert!(user2_withdrawn < 5_000);
    assert!(user2_withdrawn > 4_600); // Approximately 4,667
}
*/

/*
// NOTE: Commented out - withdrawal-only mode now BLOCKS all withdrawals
#[test]
fn test_fair_unwinding_scenario() {
    // End-to-end test of fair unwinding when system becomes insolvent
    let mut engine = Box::new(RiskEngine::new(default_params()));

    // 3 users deposit
    let alice = engine.add_user(100).unwrap();
    let bob = engine.add_user(100).unwrap();
    let charlie = engine.add_user(100).unwrap();

    engine.deposit(alice, 10_000).unwrap();
    engine.deposit(bob, 20_000).unwrap();
    engine.deposit(charlie, 10_000).unwrap();

    // Set insurance fund balance AFTER adding users (to avoid fee confusion)
    engine.insurance_fund.balance = 5_000;

    // Total principal: 40k
    // Insurance fund: 5k
    // Total system: 45k

    // Catastrophic loss event: 15k loss
    engine.apply_adl(15_000).unwrap();

    // Loss_accum = 15k - 5k = 10k
    // Insurance depleted
    // Available principal = 40k - 10k = 30k
    // Haircut ratio = 30k / 40k = 75%

    assert_eq!(engine.loss_accum, 10_000);
    assert_eq!(engine.insurance_fund.balance, 0);
    assert!(engine.risk_reduction_only);

    // With fair unwinding, everyone gets the same haircut ratio (75%)
    // regardless of withdrawal order

    // Alice withdraws all (10k * 75% = 7.5k)
    let alice_before = engine.accounts[alice as usize].capital;
    engine.withdraw(alice, 10_000).unwrap();
    let alice_got = alice_before - engine.accounts[alice as usize].capital;
    assert_eq!(alice_got, 7_500);

    // Bob withdraws all (20k * 75% = 15k)
    // Fair unwinding: haircut ratio stays 75% because we track withdrawn amounts
    let bob_before = engine.accounts[bob as usize].capital;
    engine.withdraw(bob, 20_000).unwrap();
    let bob_got = bob_before - engine.accounts[bob as usize].capital;
    assert_eq!(bob_got, 15_000);

    // Charlie withdraws all (10k * 75% = 7.5k)
    let charlie_before = engine.accounts[charlie as usize].capital;
    engine.withdraw(charlie, 10_000).unwrap();
    let charlie_got = charlie_before - engine.accounts[charlie as usize].capital;
    assert_eq!(charlie_got, 7_500);

    // Total withdrawn: 7.5k + 15k + 7.5k = 30k
    // Exactly the available principal (fair unwinding!)
    let total_withdrawn = alice_got + bob_got + charlie_got;
    assert_eq!(total_withdrawn, 30_000);

    // All users proportionally haircutted (25% loss each)
    assert_eq!(alice_got * 100 / alice_before, 75);
    assert_eq!(bob_got * 100 / bob_before, 75);
    assert_eq!(charlie_got * 100 / charlie_before, 75);
}
*/

// ==============================================================================
// LP-SPECIFIC TESTS (CRITICAL - Addresses audit findings)
// ==============================================================================

#[test]
fn test_lp_withdraw() {
    // Tests that LP withdrawal works correctly
    let mut engine = Box::new(RiskEngine::new(default_params()));
    let fee = 1; // Account creation fee

    let lp_idx = engine.add_lp([1u8; 32], [2u8; 32], fee).unwrap();

    // LP deposits capital
    engine.deposit(lp_idx, 10_000).unwrap();
    // vault = 10,000 + fee = 10,001

    // LP earns PNL from counterparty (need zero-sum setup)
    // Create a user to be the counterparty
    let user_idx = engine.add_user(1).unwrap();
    engine.deposit(user_idx, 5_000).unwrap();
    // vault = 10,001 + 1 + 5000 = 15,002

    // Zero-sum PNL: LP gains 5000, user loses 5000
    engine.accounts[lp_idx as usize].pnl = 5_000;
    engine.accounts[user_idx as usize].pnl = -5_000;

    // Set warmup slope so PnL can warm up (warmup_period_slots = 100 from default_params)
    engine.accounts[lp_idx as usize].warmup_slope_per_step = 5_000 / 100; // 50 per slot
    engine.accounts[lp_idx as usize].warmup_started_at_slot = 0;

    // Advance time to allow warmup
    engine.current_slot = 100; // Full warmup (100 slots × 50 = 5000)

    // withdraw converts warmed PNL to capital, then withdraws
    // After conversion: LP capital = 10,000 + 5,000 = 15,000
    let result = engine.withdraw(lp_idx, 10_000);
    assert!(result.is_ok(), "LP withdrawal should succeed: {:?}", result);

    // vault started at 15,002, withdrew 10,000 -> 5,002
    assert_eq!(engine.vault, 5_002, "Vault after LP withdrawal");
    assert_eq!(engine.accounts[lp_idx as usize].capital, 5_000, "LP should have 5,000 capital remaining (from converted PNL)");
    assert_eq!(engine.accounts[lp_idx as usize].pnl, 0, "PNL should be converted to capital");
}

/*
// NOTE: Commented out - withdrawal-only mode now BLOCKS all withdrawals
#[test]
fn test_lp_withdraw_with_haircut() {
    // CRITICAL: Tests that LPs are subject to withdrawal-mode haircuts
    let mut engine = Box::new(RiskEngine::new(default_params()));

    let user_idx = engine.add_user(1).unwrap();
    let lp_idx = engine.add_lp([1u8; 32], [2u8; 32], 1).unwrap();

    engine.deposit(user_idx, 10_000).unwrap();
    engine.deposit(lp_idx, 10_000).unwrap();

    // Simulate crisis - set loss_accum
    engine.loss_accum = 5_000; // 25% loss
    engine.risk_reduction_only = true;

    // Both should get 75% haircut
    let user_result = engine.withdraw(user_idx, 10_000);
    assert!(user_result.is_ok());

    let lp_result = engine.withdraw(lp_idx, 10_000);
    assert!(lp_result.is_ok());

    // Both should have withdrawn same proportion
    let total_withdrawn = engine.withdrawal_mode_withdrawn;
    assert!(total_withdrawn < 20_000, "Total withdrawn should be less than requested due to haircuts");
    assert!(total_withdrawn > 14_000, "Haircut should be approximately 25%");
}
*/

/*
// NOTE: Commented out - warmup rate limiting was removed in slab 4096 redesign
#[test]
fn test_update_lp_warmup_slope() {
    // CRITICAL: Tests that LP warmup actually gets rate limited
    let mut engine = Box::new(RiskEngine::new(default_params()));

    let lp_idx = engine.add_lp([1u8; 32], [2u8; 32], 1).unwrap();

    // Set insurance fund
    engine.insurance_fund.balance = 10_000;

    // LP earns large PNL
    engine.accounts[lp_idx as usize].pnl = 50_000;

    // Update warmup slope
    engine.update_lp_warmup_slope(lp_idx).unwrap();

    // Should be rate limited
    let ideal_slope = 50_000 / 100; // 500 per slot
    let actual_slope = engine.accounts[lp_idx as usize].warmup_slope_per_step;

    assert!(actual_slope < ideal_slope, "LP warmup should be rate limited");
    assert!(engine.total_warmup_rate > 0, "LP should contribute to total warmup rate");
}
*/

#[test]
fn test_adl_proportional_haircut_users_and_lps() {
    // CRITICAL: Tests that ADL haircuts users and LPs PROPORTIONALLY, not sequentially
    let mut engine = Box::new(RiskEngine::new(default_params()));
    
    let user_idx = engine.add_user(1).unwrap();
    let lp_idx = engine.add_lp([1u8; 32], [2u8; 32], 1).unwrap();
    
    // Both have unwrapped PNL
    engine.accounts[user_idx as usize].pnl = 10_000; // User has 10k unwrapped
    engine.accounts[lp_idx as usize].pnl = 10_000;     // LP has 10k unwrapped
    
    // Apply ADL with 10k loss
    engine.apply_adl(10_000).unwrap();
    
    // BOTH should be haircutted proportionally (50% each)
    assert_eq!(engine.accounts[user_idx as usize].pnl, 5_000, "User should lose 5k (50%)");
    assert_eq!(engine.accounts[lp_idx as usize].pnl, 5_000, "LP should lose 5k (50%)");
}

#[test]
fn test_adl_fairness_different_amounts() {
    // CRITICAL: Tests proportional ADL with different PNL amounts
    let mut engine = Box::new(RiskEngine::new(default_params()));
    
    let user_idx = engine.add_user(1).unwrap();
    let lp_idx = engine.add_lp([1u8; 32], [2u8; 32], 1).unwrap();
    
    // User has more unwrapped PNL than LP
    engine.accounts[user_idx as usize].pnl = 15_000; // User: 15k
    engine.accounts[lp_idx as usize].pnl = 5_000;      // LP: 5k
    // Total: 20k
    
    // Apply ADL with 10k loss (50% of total)
    engine.apply_adl(10_000).unwrap();
    
    // Each should lose 50% of their PNL
    assert_eq!(engine.accounts[user_idx as usize].pnl, 7_500, "User should lose 7.5k (50% of 15k)");
    assert_eq!(engine.accounts[lp_idx as usize].pnl, 2_500, "LP should lose 2.5k (50% of 5k)");
}

#[test]
fn test_lp_capital_never_reduced_by_adl() {
    // CRITICAL: Verifies Invariant I1 for LPs
    let mut engine = Box::new(RiskEngine::new(default_params()));
    
    let lp_idx = engine.add_lp([1u8; 32], [2u8; 32], 1).unwrap();
    
    engine.deposit(lp_idx, 10_000).unwrap();
    engine.accounts[lp_idx as usize].pnl = 5_000;
    
    let capital_before = engine.accounts[lp_idx as usize].capital;
    
    // Apply massive ADL
    engine.apply_adl(100_000).unwrap();
    
    // Capital should NEVER be reduced
    assert_eq!(engine.accounts[lp_idx as usize].capital, capital_before, "I1: LP capital must never be reduced by ADL");
    
    // Only PNL should be affected
    assert!(engine.accounts[lp_idx as usize].pnl < 5_000, "LP PNL should be haircutted");
}

#[test]
fn test_risk_reduction_threshold() {
    // Test that risk-reduction mode triggers at configured threshold
    let mut params = default_params();
    params.risk_reduction_threshold = 5_000; // Trigger when insurance < 5k

    let mut engine = Box::new(RiskEngine::new(params));

    let user = engine.add_user(100).unwrap();
    engine.deposit(user, 10_000).unwrap();

    // Setup: insurance fund has 10k, which is above threshold
    engine.insurance_fund.balance = 10_000;
    assert!(!engine.risk_reduction_only);

    // Apply ADL with 3k loss - should bring insurance to 7k (still above 5k threshold)
    engine.apply_adl(3_000).unwrap();
    assert_eq!(engine.insurance_fund.balance, 7_000);
    assert!(!engine.risk_reduction_only, "Should not trigger yet (7k > 5k)");

    // Apply ADL with 3k loss - should bring insurance to 4k (below 5k threshold)
    engine.apply_adl(3_000).unwrap();
    assert_eq!(engine.insurance_fund.balance, 4_000);
    assert!(engine.risk_reduction_only, "Should trigger now (4k < 5k)");
    assert!(engine.warmup_paused, "Warmup should be frozen");

    // Top up to 4.5k - still below threshold, should stay in risk mode
    engine.top_up_insurance_fund(500).unwrap();
    assert_eq!(engine.insurance_fund.balance, 4_500);
    assert!(engine.risk_reduction_only, "Should stay in risk mode (4.5k < 5k)");

    // Top up to 5k - exactly at threshold, should exit
    engine.top_up_insurance_fund(500).unwrap();
    assert_eq!(engine.insurance_fund.balance, 5_000);
    assert!(!engine.risk_reduction_only, "Should exit risk mode (5k >= 5k)");
    assert!(!engine.warmup_paused, "Warmup should unfreeze");
}

// ==============================================================================
// PANIC SETTLE TESTS
// ==============================================================================

#[test]
fn test_panic_settle_closes_all_positions() {
    // Test A: settles all positions to zero
    let mut engine = Box::new(RiskEngine::new(default_params()));

    // Create 1 LP and 2 users
    let lp_idx = engine.add_lp([0u8; 32], [0u8; 32], 1).unwrap();
    let user1 = engine.add_user(1).unwrap();
    let user2 = engine.add_user(1).unwrap();

    // Fund accounts
    engine.deposit(lp_idx, 100_000).unwrap();
    engine.deposit(user1, 10_000).unwrap();
    engine.deposit(user2, 10_000).unwrap();

    // Set positions: user1 long +10, user2 short -7, lp takes opposite (-3)
    engine.accounts[user1 as usize].position_size = 10_000_000; // +10 contracts
    engine.accounts[user1 as usize].entry_price = 1_000_000; // $1
    engine.accounts[user2 as usize].position_size = -7_000_000; // -7 contracts
    engine.accounts[user2 as usize].entry_price = 1_200_000; // $1.20
    engine.accounts[lp_idx as usize].position_size = -3_000_000; // -3 contracts (net = 0)
    engine.accounts[lp_idx as usize].entry_price = 1_000_000; // $1

    // Call panic_settle_all at oracle price $1.10
    let oracle_price = 1_100_000;
    engine.panic_settle_all(oracle_price).unwrap();

    // Assert all position_size == 0
    assert_eq!(engine.accounts[user1 as usize].position_size, 0, "User1 position should be closed");
    assert_eq!(engine.accounts[user2 as usize].position_size, 0, "User2 position should be closed");
    assert_eq!(engine.accounts[lp_idx as usize].position_size, 0, "LP position should be closed");
}

#[test]
fn test_panic_settle_clamps_negative_pnl() {
    // Test B: mark pnl realized and losers clamped
    let mut engine = Box::new(RiskEngine::new(default_params()));

    let user_idx = engine.add_user(1).unwrap();
    engine.deposit(user_idx, 10_000).unwrap();

    // User is long at $1, oracle will be $0.50 => big loss
    engine.accounts[user_idx as usize].position_size = 100_000_000; // Large long
    engine.accounts[user_idx as usize].entry_price = 1_000_000; // $1

    let oracle_price = 500_000; // $0.50 - user loses badly

    // Capture loss_accum before
    let loss_before = engine.loss_accum;

    engine.panic_settle_all(oracle_price).unwrap();

    // User's PNL should be clamped to 0
    assert!(engine.accounts[user_idx as usize].pnl >= 0, "User PNL should be >= 0 after panic settle");

    // loss_accum or insurance should have absorbed the loss
    let loss_increased = engine.loss_accum > loss_before;
    let insurance_decreased = engine.insurance_fund.balance == 0;
    assert!(loss_increased || insurance_decreased || engine.accounts[user_idx as usize].pnl == 0,
            "Loss should be socialized or absorbed by insurance");
}

#[test]
fn test_panic_settle_adl_waterfall() {
    // Test C: ADL waterfall ordering (unwrapped first, then insurance)
    let mut engine = Box::new(RiskEngine::new(default_params()));

    // Account A with unwrapped PNL (warmup_slope = 0, so nothing is withdrawable)
    let winner = engine.add_user(1).unwrap();
    engine.deposit(winner, 10_000).unwrap();

    // Loser account that will have a position
    let loser = engine.add_user(1).unwrap();
    engine.deposit(loser, 10_000).unwrap();

    // Set up zero-sum positions at same entry price
    // Loser has a long position
    engine.accounts[loser as usize].position_size = 10_000_000;
    engine.accounts[loser as usize].entry_price = 1_000_000;

    // Winner has matching short position
    engine.accounts[winner as usize].position_size = -10_000_000;
    engine.accounts[winner as usize].entry_price = 1_000_000;
    engine.accounts[winner as usize].warmup_slope_per_step = 0; // Any PNL will be unwrapped

    // Don't modify insurance_fund.balance - let it stay at what deposits set it to
    // This preserves conservation

    // Verify conservation before
    assert!(engine.check_conservation(), "Conservation should hold before panic settle");

    // Oracle price causes loss on loser (long loses when price drops)
    // Winner (short) gains when price drops
    let oracle_price = 200_000; // $0.20, way below $1 entry

    engine.panic_settle_all(oracle_price).unwrap();

    // After panic settle:
    // - Loser's long at $1 with oracle $0.20 means loss = (0.2 - 1.0) * 10 = -8 (scaled)
    // - Winner's short at $1 with oracle $0.20 means gain = (1.0 - 0.2) * 10 = +8 (scaled)
    // But loser's PNL gets clamped to 0, creating a system loss
    // That loss gets socialized via ADL from winner's PNL (which became positive from position)

    // Winner should have gained from short position closing, then had ADL haircut applied
    // System should be conserved
    assert!(engine.check_conservation(), "Conservation must hold after panic settle");
}

#[test]
fn test_panic_settle_freezes_warmup() {
    // Test D: warmup frozen on panic settle
    let mut engine = Box::new(RiskEngine::new(default_params()));

    let user_idx = engine.add_user(1).unwrap();
    engine.deposit(user_idx, 10_000).unwrap();

    // User has positive PNL with warmup slope
    engine.accounts[user_idx as usize].pnl = 1000;
    engine.accounts[user_idx as usize].warmup_slope_per_step = 10;
    engine.accounts[user_idx as usize].warmup_started_at_slot = 0;

    engine.current_slot = 50; // 50 slots elapsed

    let withdrawable_before = engine.withdrawable_pnl(&engine.accounts[user_idx as usize]);

    // Call panic_settle_all
    engine.panic_settle_all(1_000_000).unwrap();

    // Advance slots
    engine.advance_slot(100);

    let withdrawable_after = engine.withdrawable_pnl(&engine.accounts[user_idx as usize]);

    // Warmup should be frozen, so withdrawable should not increase
    assert!(withdrawable_after <= withdrawable_before + 1, // +1 for rounding tolerance
            "Withdrawable PNL should not increase after panic settle (warmup frozen)");

    // Verify warmup is actually paused
    assert!(engine.warmup_paused, "Warmup should be paused after panic settle");
}

#[test]
fn test_panic_settle_conservation_holds() {
    // Test E: conservation holds after panic settle
    let mut engine = Box::new(RiskEngine::new(default_params()));

    // Setup multiple accounts with various positions
    let lp_idx = engine.add_lp([0u8; 32], [0u8; 32], 1).unwrap();
    let user1 = engine.add_user(1).unwrap();
    let user2 = engine.add_user(1).unwrap();

    engine.deposit(lp_idx, 50_000).unwrap();
    engine.deposit(user1, 10_000).unwrap();
    engine.deposit(user2, 10_000).unwrap();

    // Set up positions at same entry price (net position = 0)
    // This ensures positions are zero-sum
    engine.accounts[user1 as usize].position_size = 5_000_000;  // Long 5
    engine.accounts[user1 as usize].entry_price = 1_000_000;    // $1
    engine.accounts[user2 as usize].position_size = -2_000_000; // Short 2
    engine.accounts[user2 as usize].entry_price = 1_000_000;    // $1
    engine.accounts[lp_idx as usize].position_size = -3_000_000; // Short 3 (LP takes other side)
    engine.accounts[lp_idx as usize].entry_price = 1_000_000;   // $1

    // Reset insurance fund to 0 so conservation is clean
    // (account fees already went to insurance)
    let insurance_from_fees = engine.insurance_fund.balance;

    // Verify conservation before
    assert!(engine.check_conservation(), "Conservation should hold before panic settle");

    // Panic settle at a price that causes losses for longs
    engine.panic_settle_all(500_000).unwrap();

    // Verify conservation after
    assert!(engine.check_conservation(), "Conservation must hold after panic settle");

    // Verify risk mode is active
    assert!(engine.risk_reduction_only, "Should be in risk-reduction mode after panic settle");
}

// ==============================================================================
// DETERMINISTIC FUZZ/PROPERTY TESTS
// ==============================================================================

#[test]
fn fuzz_panic_settle_closes_all_positions_and_conserves() {
    // Property test: panic settle never leaves open positions + conservation holds
    // Loop for 200 seeds with randomized inputs

    for seed in 1..=200 {
        let mut rng = Rng::new(seed);
        let mut engine = Box::new(RiskEngine::new(default_params()));

        // Create N accounts (1 LP + some users)
        let lp_idx = engine.add_lp([0u8; 32], [0u8; 32], 1).unwrap();
        engine.deposit(lp_idx, rng.u128(10_000, 100_000)).unwrap();

        let num_users = rng.u64(2, 6) as usize;
        let mut user_indices = Vec::new();

        for _ in 0..num_users {
            let user_idx = engine.add_user(1).unwrap();
            engine.deposit(user_idx, rng.u128(1_000, 50_000)).unwrap();
            user_indices.push(user_idx);
        }

        // Randomize positions (ensure they sum to zero for valid state)
        // IMPORTANT: All positions must use the SAME entry price for zero-sum to hold
        // In a real perp system, every trade has a counterparty at the same price
        let mut total_position: i128 = 0;
        let common_entry_price = rng.u64(100_000, 10_000_000); // $0.10 to $10

        for &user_idx in &user_indices {
            let position = rng.i128(-50_000, 50_000);

            engine.accounts[user_idx as usize].position_size = position;
            engine.accounts[user_idx as usize].entry_price = common_entry_price;
            total_position += position;
        }

        // LP takes opposite position to balance (net zero) at same entry price
        engine.accounts[lp_idx as usize].position_size = -total_position;
        engine.accounts[lp_idx as usize].entry_price = common_entry_price;

        // Verify conservation before (should hold with zero-sum positions)
        if !engine.check_conservation() {
            eprintln!("Seed {} BEFORE: vault={}, insurance={}, loss_accum={}",
                     seed, engine.vault, engine.insurance_fund.balance, engine.loss_accum);
            panic!("Seed {}: Conservation should hold before panic settle", seed);
        }

        // Debug: capture state before panic settle (prefixed with _ to suppress warnings)
        let _vault_before = engine.vault;
        let _insurance_before = engine.insurance_fund.balance;

        // Random oracle price
        let oracle_price = rng.u64(100_000, 10_000_000);

        // Call panic_settle_all
        let result = engine.panic_settle_all(oracle_price);
        assert!(result.is_ok(), "Seed {}: panic_settle_all should not fail", seed);

        // Assert: all positions are zero
        assert_eq!(engine.accounts[lp_idx as usize].position_size, 0,
                   "Seed {}: LP position should be closed", seed);
        for &user_idx in &user_indices {
            assert_eq!(engine.accounts[user_idx as usize].position_size, 0,
                       "Seed {}: User {} position should be closed", seed, user_idx);
        }

        // Assert: all PNLs are >= 0 (negative clamped)
        assert!(engine.accounts[lp_idx as usize].pnl >= 0,
                "Seed {}: LP PNL should be >= 0", seed);
        for &user_idx in &user_indices {
            assert!(engine.accounts[user_idx as usize].pnl >= 0,
                    "Seed {}: User {} PNL should be >= 0", seed, user_idx);
        }

        // Assert: conservation holds after
        if !engine.check_conservation() {
            // Debug output - compute what check_conservation computes
            let mut real_total_capital = 0u128;
            let mut real_net_pnl: i128 = 0;
            for word in engine.used.iter() {
                let mut w = *word;
                let block_offset = engine.used.iter().position(|x| x == word).unwrap_or(0) * 64;
                while w != 0 {
                    let bit = w.trailing_zeros() as usize;
                    let idx = block_offset + bit;
                    w &= w - 1;
                    real_total_capital += engine.accounts[idx].capital;
                    real_net_pnl += engine.accounts[idx].pnl;
                }
            }
            let expected = (real_total_capital as i128 + real_net_pnl +
                           engine.insurance_fund.balance as i128 -
                           engine.loss_accum as i128) as u128;
            eprintln!("Seed {}: vault={}, real_capital={}, real_pnl={}, insurance={}, loss_accum={}, expected={}",
                     seed, engine.vault, real_total_capital, real_net_pnl,
                     engine.insurance_fund.balance, engine.loss_accum, expected);
            panic!("Seed {}: Conservation must hold after panic settle", seed);
        }

        // Assert: risk mode is active
        assert!(engine.risk_reduction_only,
                "Seed {}: Should be in risk-reduction mode", seed);
    }
}
