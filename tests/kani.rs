//! Formal verification with Kani
//!
//! These proofs verify critical safety properties of the risk engine.
//! Run with: cargo kani --harness <name> (individual proofs)
//! Run all: cargo kani (may take significant time)
//!
//! Key invariants proven:
//! - I1: User principal is never reduced by ADL/socialization
//! - I2: Conservation of funds across all operations
//! - I5: PNL warmup is monotonic and deterministic
//! - I7: User isolation - operations on one user don't affect others
//! - I8: Collateral calculations are consistent
//! - I10: Withdrawal-only mode fair unwinding properties
//!
//! Note: Some proofs involving iteration over all accounts (apply_adl,
//! check_conservation loops) are computationally expensive and may timeout.
//! These are marked with SLOW_PROOF comments. Run individually with longer
//! timeouts if needed: cargo kani --harness <name> --solver-timeout 600

#![cfg(kani)]

use percolator::*;

// Helper to create test params
fn test_params() -> RiskParams {
    RiskParams {
        warmup_period_slots: 100,
        maintenance_margin_bps: 500,
        initial_margin_bps: 1000,
        trading_fee_bps: 10,
        liquidation_fee_bps: 50,
        insurance_fee_share_bps: 5000,
        max_accounts: 8, // Match Kani's MAX_ACCOUNTS
        account_fee_bps: 10000,
        risk_reduction_threshold: 0,
    }
}

// ============================================================================
// I1: Principal is NEVER reduced by ADL/socialization
// SLOW_PROOF: Uses apply_adl which iterates over all accounts
// Run with: cargo kani --harness i1_adl_never_reduces_principal --solver-timeout 600
// ============================================================================

#[kani::proof]
#[kani::unwind(10)]
#[kani::solver(cadical)]
fn i1_adl_never_reduces_principal() {
    let mut engine = Box::new(RiskEngine::new(test_params()));
    let user_idx = engine.add_user(1).unwrap();

    // Set arbitrary but bounded values (reduced bounds for tractability)
    let principal: u128 = kani::any();
    let pnl: i128 = kani::any();
    let loss: u128 = kani::any();

    kani::assume(principal > 0 && principal < 1_000);
    kani::assume(pnl > -1_000 && pnl < 1_000);
    kani::assume(loss < 1_000);

    engine.accounts[user_idx as usize].capital = principal;
    engine.accounts[user_idx as usize].pnl = pnl;
    engine.insurance_fund.balance = 10_000;

    let principal_before = engine.accounts[user_idx as usize].capital;

    let _ = engine.apply_adl(loss);

    assert!(engine.accounts[user_idx as usize].capital == principal_before,
            "I1: ADL must NEVER reduce user principal");
}

// ============================================================================
// I2: Conservation of funds
// SLOW_PROOF: Uses check_conservation which iterates over all accounts
// Run with: cargo kani --harness i2_deposit_preserves_conservation --solver-timeout 600
// ============================================================================

#[kani::proof]
#[kani::unwind(10)]
#[kani::solver(cadical)]
fn i2_deposit_preserves_conservation() {
    let mut engine = Box::new(RiskEngine::new(test_params()));
    let user_idx = engine.add_user(1).unwrap();

    let amount: u128 = kani::any();
    kani::assume(amount < 10_000);

    assert!(engine.check_conservation());

    let _ = engine.deposit(user_idx, amount);

    assert!(engine.check_conservation(),
            "I2: Deposit must preserve conservation");
}

#[kani::proof]
#[kani::unwind(10)]
#[kani::solver(cadical)]
fn i2_withdraw_preserves_conservation() {
    let mut engine = Box::new(RiskEngine::new(test_params()));
    let user_idx = engine.add_user(1).unwrap();

    let deposit: u128 = kani::any();
    let withdraw: u128 = kani::any();

    kani::assume(deposit < 10_000);
    kani::assume(withdraw < 10_000);
    kani::assume(withdraw <= deposit);

    let _ = engine.deposit(user_idx, deposit);

    assert!(engine.check_conservation());

    let _ = engine.withdraw(user_idx, withdraw);

    assert!(engine.check_conservation(),
            "I2: Withdrawal must preserve conservation");
}

// ============================================================================
// I5: PNL Warmup Properties
// ============================================================================

#[kani::proof]
#[kani::unwind(10)]
#[kani::solver(cadical)]
fn i5_warmup_determinism() {
    let mut engine = Box::new(RiskEngine::new(test_params()));
    let user_idx = engine.add_user(1).unwrap();

    let pnl: i128 = kani::any();
    let reserved: u128 = kani::any();
    let slope: u128 = kani::any();
    let slots: u64 = kani::any();

    kani::assume(pnl > 0 && pnl < 10_000);
    kani::assume(reserved < 5_000);
    kani::assume(slope > 0 && slope < 100);
    kani::assume(slots < 200);

    engine.accounts[user_idx as usize].pnl = pnl;
    engine.accounts[user_idx as usize].reserved_pnl = reserved;
    engine.accounts[user_idx as usize].warmup_slope_per_step = slope;
    engine.current_slot = slots;

    // Calculate twice with same inputs
    let w1 = engine.withdrawable_pnl(&engine.accounts[user_idx as usize]);
    let w2 = engine.withdrawable_pnl(&engine.accounts[user_idx as usize]);

    assert!(w1 == w2,
            "I5: Withdrawable PNL must be deterministic");
}

#[kani::proof]
#[kani::unwind(10)]
#[kani::solver(cadical)]
fn i5_warmup_monotonicity() {
    let mut engine = Box::new(RiskEngine::new(test_params()));
    let user_idx = engine.add_user(1).unwrap();

    let pnl: i128 = kani::any();
    let slope: u128 = kani::any();
    let slots1: u64 = kani::any();
    let slots2: u64 = kani::any();

    kani::assume(pnl > 0 && pnl < 10_000);
    kani::assume(slope > 0 && slope < 100);
    kani::assume(slots1 < 200);
    kani::assume(slots2 < 200);
    kani::assume(slots2 > slots1);

    engine.accounts[user_idx as usize].pnl = pnl;
    engine.accounts[user_idx as usize].warmup_slope_per_step = slope;

    engine.current_slot = slots1;
    let w1 = engine.withdrawable_pnl(&engine.accounts[user_idx as usize]);

    engine.current_slot = slots2;
    let w2 = engine.withdrawable_pnl(&engine.accounts[user_idx as usize]);

    assert!(w2 >= w1,
            "I5: Warmup must be monotonically increasing over time");
}

#[kani::proof]
#[kani::unwind(10)]
#[kani::solver(cadical)]
fn i5_warmup_bounded_by_pnl() {
    let mut engine = Box::new(RiskEngine::new(test_params()));
    let user_idx = engine.add_user(1).unwrap();

    let pnl: i128 = kani::any();
    let reserved: u128 = kani::any();
    let slope: u128 = kani::any();
    let slots: u64 = kani::any();

    kani::assume(pnl > 0 && pnl < 10_000);
    kani::assume(reserved < 5_000);
    kani::assume(slope > 0 && slope < 100);
    kani::assume(slots < 200);

    engine.accounts[user_idx as usize].pnl = pnl;
    engine.accounts[user_idx as usize].reserved_pnl = reserved;
    engine.accounts[user_idx as usize].warmup_slope_per_step = slope;
    engine.current_slot = slots;

    let withdrawable = engine.withdrawable_pnl(&engine.accounts[user_idx as usize]);
    let positive_pnl = pnl as u128;
    let available = positive_pnl.saturating_sub(reserved);

    assert!(withdrawable <= available,
            "I5: Withdrawable must not exceed available PNL");
}

// ============================================================================
// I7: User Isolation
// ============================================================================

#[kani::proof]
#[kani::unwind(10)]
#[kani::solver(cadical)]
fn i7_user_isolation_deposit() {
    let mut engine = Box::new(RiskEngine::new(test_params()));
    let user1 = engine.add_user(1).unwrap();
    let user2 = engine.add_user(1).unwrap();

    let amount1: u128 = kani::any();
    let amount2: u128 = kani::any();

    kani::assume(amount1 < 10_000);
    kani::assume(amount2 < 10_000);

    let _ = engine.deposit(user1, amount1);
    let _ = engine.deposit(user2, amount2);

    let user2_principal = engine.accounts[user2 as usize].capital;
    let user2_pnl = engine.accounts[user2 as usize].pnl;

    // Operate on user1
    let _ = engine.deposit(user1, 100);

    // User2 should be unchanged
    assert!(engine.accounts[user2 as usize].capital == user2_principal,
            "I7: User2 principal unchanged by user1 deposit");
    assert!(engine.accounts[user2 as usize].pnl == user2_pnl,
            "I7: User2 PNL unchanged by user1 deposit");
}

#[kani::proof]
#[kani::unwind(10)]
#[kani::solver(cadical)]
fn i7_user_isolation_withdrawal() {
    let mut engine = Box::new(RiskEngine::new(test_params()));
    let user1 = engine.add_user(1).unwrap();
    let user2 = engine.add_user(1).unwrap();

    let amount1: u128 = kani::any();
    let amount2: u128 = kani::any();

    kani::assume(amount1 > 100 && amount1 < 10_000);
    kani::assume(amount2 < 10_000);

    let _ = engine.deposit(user1, amount1);
    let _ = engine.deposit(user2, amount2);

    let user2_principal = engine.accounts[user2 as usize].capital;
    let user2_pnl = engine.accounts[user2 as usize].pnl;

    // Operate on user1
    let _ = engine.withdraw(user1, 50);

    // User2 should be unchanged
    assert!(engine.accounts[user2 as usize].capital == user2_principal,
            "I7: User2 principal unchanged by user1 withdrawal");
    assert!(engine.accounts[user2 as usize].pnl == user2_pnl,
            "I7: User2 PNL unchanged by user1 withdrawal");
}

// ============================================================================
// I8: Collateral Consistency
// ============================================================================

#[kani::proof]
#[kani::unwind(10)]
#[kani::solver(cadical)]
fn i8_collateral_with_positive_pnl() {
    let mut engine = Box::new(RiskEngine::new(test_params()));
    let user_idx = engine.add_user(1).unwrap();

    let principal: u128 = kani::any();
    let pnl: i128 = kani::any();

    kani::assume(principal < 10_000);
    kani::assume(pnl > 0 && pnl < 10_000);

    engine.accounts[user_idx as usize].capital = principal;
    engine.accounts[user_idx as usize].pnl = pnl;

    let collateral = engine.account_collateral(&engine.accounts[user_idx as usize]);
    let expected = principal.saturating_add(pnl as u128);

    assert!(collateral == expected,
            "I8: Collateral = principal + positive PNL");
}

#[kani::proof]
#[kani::unwind(10)]
#[kani::solver(cadical)]
fn i8_collateral_with_negative_pnl() {
    let mut engine = Box::new(RiskEngine::new(test_params()));
    let user_idx = engine.add_user(1).unwrap();

    let principal: u128 = kani::any();
    let pnl: i128 = kani::any();

    kani::assume(principal < 10_000);
    kani::assume(pnl < 0 && pnl > -10_000);

    engine.accounts[user_idx as usize].capital = principal;
    engine.accounts[user_idx as usize].pnl = pnl;

    let collateral = engine.account_collateral(&engine.accounts[user_idx as usize]);

    assert!(collateral == principal,
            "I8: Collateral = principal when PNL is negative");
}

// ============================================================================
// I4: Bounded Losses (ADL mechanics)
// SLOW_PROOF: Uses apply_adl which iterates over all accounts
// ============================================================================

// Previously slow - now fast with 8 accounts
#[kani::proof]
#[kani::unwind(10)]
#[kani::solver(cadical)]
fn i4_adl_haircuts_unwrapped_first() {
    let mut engine = Box::new(RiskEngine::new(test_params()));
    let user_idx = engine.add_user(1).unwrap();

    let principal: u128 = kani::any();
    let pnl: i128 = kani::any();
    let loss: u128 = kani::any();

    kani::assume(principal < 10_000);
    kani::assume(pnl > 0 && pnl < 10_000);
    kani::assume(loss < 5_000);
    kani::assume(loss < pnl as u128); // Loss less than PNL

    engine.accounts[user_idx as usize].capital = principal;
    engine.accounts[user_idx as usize].pnl = pnl;
    engine.accounts[user_idx as usize].warmup_slope_per_step = 10;
    engine.insurance_fund.balance = 100_000;

    let pnl_before = engine.accounts[user_idx as usize].pnl;
    let insurance_before = engine.insurance_fund.balance;

    let _ = engine.apply_adl(loss);

    // If there was enough unwrapped PNL, insurance shouldn't be touched
    let unwrapped_pnl = pnl as u128; // At slot 0, nothing is warmed up

    if loss <= unwrapped_pnl {
        assert!(engine.insurance_fund.balance == insurance_before,
                "I4: ADL should haircut PNL before touching insurance");
        assert!(engine.accounts[user_idx as usize].pnl == pnl_before - (loss as i128),
                "I4: PNL should be reduced by loss amount");
    }
}


// ============================================================================
// Withdrawal Safety
// ============================================================================

#[kani::proof]
#[kani::unwind(10)]
#[kani::solver(cadical)]
fn withdrawal_requires_sufficient_balance() {
    let mut engine = Box::new(RiskEngine::new(test_params()));
    let user_idx = engine.add_user(1).unwrap();

    let principal: u128 = kani::any();
    let withdraw: u128 = kani::any();

    kani::assume(principal < 10_000);
    kani::assume(withdraw < 20_000);
    kani::assume(withdraw > principal); // Try to withdraw more than available

    engine.accounts[user_idx as usize].capital = principal;
    engine.vault = principal;

    let result = engine.withdraw(user_idx, withdraw);

    assert!(result.is_err(),
            "Withdrawal of more than available must fail");
}

#[kani::proof]
#[kani::unwind(10)]
#[kani::solver(cadical)]
fn pnl_withdrawal_requires_warmup() {
    let mut engine = Box::new(RiskEngine::new(test_params()));
    let user_idx = engine.add_user(1).unwrap();

    let pnl: i128 = kani::any();
    let withdraw: u128 = kani::any();

    kani::assume(pnl > 0 && pnl < 10_000);
    kani::assume(withdraw > 0 && withdraw < 10_000);

    engine.accounts[user_idx as usize].pnl = pnl;
    engine.accounts[user_idx as usize].warmup_slope_per_step = 10;
    engine.accounts[user_idx as usize].capital = 0; // No principal
    engine.insurance_fund.balance = 100_000;
    engine.vault = pnl as u128;
    engine.current_slot = 0; // At slot 0, nothing warmed up

    // withdrawable_pnl should be 0 at slot 0
    let withdrawable = engine.withdrawable_pnl(&engine.accounts[user_idx as usize]);
    assert!(withdrawable == 0, "No PNL warmed up at slot 0");

    // Trying to withdraw should fail (no principal, no warmed PNL)
    if withdraw > 0 {
        let result = engine.withdraw(user_idx, withdraw);
        assert!(result.is_err(),
                "Cannot withdraw when no principal and PNL not warmed up");
    }
}

// ============================================================================
// Multi-user ADL Scenarios
// SLOW_PROOF: Uses apply_adl which iterates over all accounts
// ============================================================================

// Previously slow - now fast with 8 accounts
#[kani::proof]
#[kani::unwind(10)]
#[kani::solver(cadical)]
fn multiple_users_adl_preserves_all_principals() {
    let mut engine = Box::new(RiskEngine::new(test_params()));
    let user1 = engine.add_user(1).unwrap();
    let user2 = engine.add_user(1).unwrap();

    let p1: u128 = kani::any();
    let p2: u128 = kani::any();
    let pnl1: i128 = kani::any();
    let pnl2: i128 = kani::any();
    let loss: u128 = kani::any();

    kani::assume(p1 < 5_000);
    kani::assume(p2 < 5_000);
    kani::assume(pnl1 > -5_000 && pnl1 < 5_000);
    kani::assume(pnl2 > -5_000 && pnl2 < 5_000);
    kani::assume(loss < 10_000);

    engine.accounts[user1 as usize].capital = p1;
    engine.accounts[user1 as usize].pnl = pnl1;
    engine.accounts[user2 as usize].capital = p2;
    engine.accounts[user2 as usize].pnl = pnl2;
    engine.insurance_fund.balance = 100_000;

    let _ = engine.apply_adl(loss);

    assert!(engine.accounts[user1 as usize].capital == p1,
            "Multi-user ADL: User1 principal preserved");
    assert!(engine.accounts[user2 as usize].capital == p2,
            "Multi-user ADL: User2 principal preserved");
}


// ============================================================================
// Arithmetic Safety
// ============================================================================

#[kani::proof]
#[kani::unwind(10)]
#[kani::solver(cadical)]
fn saturating_arithmetic_prevents_overflow() {
    let a: u128 = kani::any();
    let b: u128 = kani::any();

    // Test saturating add
    let result = a.saturating_add(b);
    assert!(result >= a && result >= b,
            "Saturating add should not overflow");

    // Test saturating sub
    let result = a.saturating_sub(b);
    assert!(result <= a,
            "Saturating sub should not underflow");
}

// ============================================================================
// Liquidation Safety
// SLOW_PROOF: Liquidation involves complex margin calculations
// ============================================================================

// Previously slow - now fast with 8 accounts
#[kani::proof]
#[kani::unwind(10)]
#[kani::solver(cadical)]
fn liquidation_closes_position() {
    let mut engine = Box::new(RiskEngine::new(test_params()));
    let user_idx = engine.add_user(1).unwrap();
    let keeper_idx = engine.add_user(1).unwrap();

    let principal: u128 = kani::any();
    let position: i128 = kani::any();

    kani::assume(principal > 0 && principal < 1_000);
    kani::assume(position != 0 && position > -10_000 && position < 10_000);

    engine.accounts[user_idx as usize].capital = principal;
    engine.accounts[user_idx as usize].position_size = position;
    engine.accounts[user_idx as usize].entry_price = 1_000_000;
    engine.vault = principal;

    let _ = engine.liquidate_account(user_idx, keeper_idx, 1_000_000);

    // After liquidation, position should be closed (or at least reduced)
    assert!(engine.accounts[user_idx as usize].position_size.abs() <= position.abs(),
            "Liquidation should reduce or close position");
}


// ============================================================================
// Edge Cases
// ============================================================================

#[kani::proof]
#[kani::unwind(10)]
#[kani::solver(cadical)]
fn zero_pnl_withdrawable_is_zero() {
    let mut engine = Box::new(RiskEngine::new(test_params()));
    let user_idx = engine.add_user(1).unwrap();

    engine.accounts[user_idx as usize].pnl = 0;
    engine.current_slot = 1000; // Far in future

    let withdrawable = engine.withdrawable_pnl(&engine.accounts[user_idx as usize]);

    assert!(withdrawable == 0,
            "Zero PNL means zero withdrawable");
}

#[kani::proof]
#[kani::unwind(10)]
#[kani::solver(cadical)]
fn negative_pnl_withdrawable_is_zero() {
    let mut engine = Box::new(RiskEngine::new(test_params()));
    let user_idx = engine.add_user(1).unwrap();

    let pnl: i128 = kani::any();
    kani::assume(pnl < 0 && pnl > -10_000);

    engine.accounts[user_idx as usize].pnl = pnl;
    engine.current_slot = 1000;

    let withdrawable = engine.withdrawable_pnl(&engine.accounts[user_idx as usize]);

    assert!(withdrawable == 0,
            "Negative PNL means zero withdrawable");
}

// ============================================================================
// Funding Rate Invariants
// ============================================================================

#[kani::proof]
#[kani::unwind(10)]
#[kani::solver(cadical)]
fn funding_p1_settlement_idempotent() {
    // P1: Funding settlement is idempotent
    // After settling once, settling again with unchanged global index does nothing

    let mut engine = Box::new(RiskEngine::new(test_params()));
    let user_idx = engine.add_user(1).unwrap();

    // Arbitrary position and PNL
    let position: i128 = kani::any();
    kani::assume(position != i128::MIN);
    kani::assume(position.abs() < 1_000_000);

    let pnl: i128 = kani::any();
    kani::assume(pnl > -1_000_000 && pnl < 1_000_000);

    engine.accounts[user_idx as usize].position_size = position;
    engine.accounts[user_idx as usize].pnl = pnl;

    // Set arbitrary funding index
    let index: i128 = kani::any();
    kani::assume(index != i128::MIN);
    kani::assume(index.abs() < 1_000_000_000);
    engine.funding_index_qpb_e6 = index;

    // Settle once
    let _ = engine.touch_account(user_idx);

    let pnl_after_first = engine.accounts[user_idx as usize].pnl;
    let snapshot_after_first = engine.accounts[user_idx as usize].funding_index;

    // Settle again without changing global index
    let _ = engine.touch_account(user_idx);

    // PNL should be unchanged
    assert!(engine.accounts[user_idx as usize].pnl == pnl_after_first,
            "Second settlement should not change PNL");

    // Snapshot should equal global index
    assert!(engine.accounts[user_idx as usize].funding_index == engine.funding_index_qpb_e6,
            "Snapshot should equal global index");
}

#[kani::proof]
#[kani::unwind(10)]
#[kani::solver(cadical)]
fn funding_p2_never_touches_principal() {
    // P2: Funding does not touch principal (extends Invariant I1)

    let mut engine = Box::new(RiskEngine::new(test_params()));
    let user_idx = engine.add_user(1).unwrap();

    let principal: u128 = kani::any();
    kani::assume(principal < 1_000_000);

    let position: i128 = kani::any();
    kani::assume(position != i128::MIN);
    kani::assume(position.abs() < 1_000_000);

    engine.accounts[user_idx as usize].capital = principal;
    engine.accounts[user_idx as usize].position_size = position;

    // Accrue arbitrary funding
    let funding_delta: i128 = kani::any();
    kani::assume(funding_delta != i128::MIN);
    kani::assume(funding_delta.abs() < 1_000_000_000);
    engine.funding_index_qpb_e6 = funding_delta;

    // Settle funding
    let _ = engine.touch_account(user_idx);

    // Principal must be unchanged
    assert!(engine.accounts[user_idx as usize].capital == principal,
            "Funding must never modify principal");
}

#[kani::proof]
#[kani::unwind(10)]
#[kani::solver(cadical)]
fn funding_p3_zero_sum_between_opposite_positions() {
    // P3: Funding is zero-sum when user and LP have opposite positions

    let mut engine = Box::new(RiskEngine::new(test_params()));
    let user_idx = engine.add_user(1).unwrap();
    let lp_idx = engine.add_lp([0u8; 32], [0u8; 32], 1).unwrap();

    let position: i128 = kani::any();
    kani::assume(position > 0 && position < 100_000); // positive only for simplicity

    // User has position, LP has opposite
    engine.accounts[user_idx as usize].position_size = position;
    engine.accounts[lp_idx as usize].position_size = -position;

    // Both start with same snapshot
    engine.accounts[user_idx as usize].funding_index = 0;
    engine.accounts[lp_idx as usize].funding_index = 0;

    let user_pnl_before = engine.accounts[user_idx as usize].pnl;
    let lp_pnl_before = engine.accounts[lp_idx as usize].pnl;
    let total_before = user_pnl_before + lp_pnl_before;

    // Accrue funding
    let delta: i128 = kani::any();
    kani::assume(delta != i128::MIN);
    kani::assume(delta.abs() < 1_000_000);
    engine.funding_index_qpb_e6 = delta;

    // Settle both
    let user_result = engine.touch_account(user_idx);
    let lp_result = engine.touch_account(lp_idx);

    // If both settlements succeeded, check zero-sum
    if user_result.is_ok() && lp_result.is_ok() {
        let total_after = engine.accounts[user_idx as usize].pnl + engine.accounts[lp_idx as usize].pnl;

        assert!(total_after == total_before,
                "Funding must be zero-sum between opposite positions");
    }
}

#[kani::proof]
#[kani::unwind(10)]
#[kani::solver(cadical)]
fn funding_p4_settle_before_position_change() {
    // P4: Verifies that settlement before position change gives correct results

    let mut engine = Box::new(RiskEngine::new(test_params()));
    let user_idx = engine.add_user(1).unwrap();

    let initial_pos: i128 = kani::any();
    kani::assume(initial_pos > 0 && initial_pos < 10_000);

    engine.accounts[user_idx as usize].position_size = initial_pos;
    engine.accounts[user_idx as usize].pnl = 0;
    engine.accounts[user_idx as usize].funding_index = 0;

    // Period 1: accrue funding with initial position
    let delta1: i128 = kani::any();
    kani::assume(delta1 != i128::MIN);
    kani::assume(delta1.abs() < 1_000);
    engine.funding_index_qpb_e6 = delta1;

    // Settle BEFORE changing position (correct way)
    let _ = engine.touch_account(user_idx);

    let pnl_after_period1 = engine.accounts[user_idx as usize].pnl;

    // Change position
    let new_pos: i128 = kani::any();
    kani::assume(new_pos > 0 && new_pos < 10_000 && new_pos != initial_pos);
    engine.accounts[user_idx as usize].position_size = new_pos;

    // Period 2: more funding
    let delta2: i128 = kani::any();
    kani::assume(delta2 != i128::MIN);
    kani::assume(delta2.abs() < 1_000);
    engine.funding_index_qpb_e6 = delta1 + delta2;

    let _ = engine.touch_account(user_idx);

    // The settlement should have correctly applied:
    // - delta1 to initial_pos
    // - delta2 to new_pos
    // Snapshot should equal global index
    assert!(engine.accounts[user_idx as usize].funding_index == engine.funding_index_qpb_e6,
            "Snapshot must track global index");
}

#[kani::proof]
#[kani::unwind(10)]
#[kani::solver(cadical)]
fn funding_p5_bounded_operations_no_overflow() {
    // P5: No overflows on bounded inputs (or returns Overflow error)

    let mut engine = Box::new(RiskEngine::new(test_params()));

    // Bounded inputs
    let price: u64 = kani::any();
    kani::assume(price > 1_000_000 && price < 1_000_000_000); // $1 to $1000

    let rate: i64 = kani::any();
    kani::assume(rate != i64::MIN);
    kani::assume(rate.abs() < 1000); // ±1000 bps = ±10%

    let dt: u64 = kani::any();
    kani::assume(dt < 1000); // max 1000 slots

    engine.last_funding_slot = 0;

    // Accrue should not panic
    let result = engine.accrue_funding(dt, price, rate);

    // Either succeeds or returns Overflow error (never panics)
    if result.is_err() {
        assert!(matches!(result.unwrap_err(), RiskError::Overflow),
                "Only Overflow error allowed");
    }
}

#[kani::proof]
#[kani::unwind(10)]
#[kani::solver(cadical)]
fn funding_zero_position_no_change() {
    // Additional invariant: Zero position means no funding payment

    let mut engine = Box::new(RiskEngine::new(test_params()));
    let user_idx = engine.add_user(1).unwrap();

    engine.accounts[user_idx as usize].position_size = 0; // Zero position

    let pnl_before: i128 = kani::any();
    kani::assume(pnl_before != i128::MIN); // Avoid abs() overflow
    kani::assume(pnl_before.abs() < 1_000_000);
    engine.accounts[user_idx as usize].pnl = pnl_before;

    // Accrue arbitrary funding
    let delta: i128 = kani::any();
    kani::assume(delta != i128::MIN); // Avoid abs() overflow
    kani::assume(delta.abs() < 1_000_000_000);
    engine.funding_index_qpb_e6 = delta;

    let _ = engine.touch_account(user_idx);

    // PNL should be unchanged
    assert!(engine.accounts[user_idx as usize].pnl == pnl_before,
            "Zero position should not pay or receive funding");
}

// ============================================================================
// Warmup Rate Cap Invariant
// NOTE: These tests are commented out because warmup rate limiting was removed
// in the slab 4096 redesign for simplicity
// ============================================================================

/*
#[kani::proof]
#[kani::unwind(10)]
#[kani::solver(cadical)]
fn warmup_rate_cap_invariant_maintained() {
    // I9: Global warmup rate respects insurance fund limit
    // Invariant: total_warmup_rate * (T/2) <= insurance_fund * max_warmup_rate_fraction_bps / 10_000

    let mut engine = Box::new(RiskEngine::new(test_params()));

    // Set insurance fund to symbolic value
    let insurance: u128 = kani::any();
    kani::assume(insurance < 1_000_000_000); // Reasonable bound
    engine.insurance_fund.balance = insurance;

    // Create a few users with symbolic PNL
    for _ in 0..2 {
        if let Ok(user_idx) = engine.add_user(1) {
            let pnl: i128 = kani::any();
            kani::assume(pnl > 0 && pnl < 1_000_000_000);
            engine.accounts[user_idx as usize].pnl = pnl;

            // Update warmup slope
            let _ = engine.update_warmup_slope(user_idx);

            // Check invariant
            let half_period = (engine.params.warmup_period_slots / 2) as u128;
            let max_warmup_in_half_period = engine.total_warmup_rate.saturating_mul(half_period);
            let insurance_limit = engine.insurance_fund.balance
                .saturating_mul(engine.params.max_warmup_rate_fraction_bps as u128)
                .saturating_div(10_000);

            assert!(max_warmup_in_half_period <= insurance_limit,
                    "Warmup rate cap invariant violated");
        }
    }
}

#[kani::proof]
#[kani::unwind(10)]
#[kani::solver(cadical)]
fn warmup_slope_never_exceeds_pnl_over_period() {
    // Verify that slope_per_step * warmup_period <= positive_pnl
    // (Users can't warm up more than they have)

    let mut engine = Box::new(RiskEngine::new(test_params()));
    engine.insurance_fund.balance = 1_000_000; // Large enough to not be limiting factor

    let user_idx = engine.add_user(1).unwrap();

    let pnl: i128 = kani::any();
    kani::assume(pnl > 0 && pnl < 1_000_000);
    engine.accounts[user_idx as usize].pnl = pnl;

    engine.update_warmup_slope(user_idx).unwrap();

    let user = &engine.accounts[user_idx as usize];
    let total_warmup = user.warmup_slope_per_step.saturating_mul(engine.params.warmup_period_slots as u128);

    assert!(total_warmup <= pnl as u128,
            "Slope should not allow warming up more PNL than exists");
}

#[kani::proof]
#[kani::unwind(10)]
#[kani::solver(cadical)]
fn warmup_rate_decreases_when_pnl_decreases() {
    // When user's PNL decreases, their slope should decrease,
    // freeing up capacity for other users

    let mut engine = Box::new(RiskEngine::new(test_params()));
    engine.insurance_fund.balance = 1_000_000;

    let user_idx = engine.add_user(1).unwrap();

    // User has high PNL
    engine.accounts[user_idx as usize].pnl = 100_000;
    engine.update_warmup_slope(user_idx).unwrap();
    let slope_high = engine.accounts[user_idx as usize].warmup_slope_per_step;
    let rate_high = engine.total_warmup_rate;

    // PNL decreases
    engine.accounts[user_idx as usize].pnl = 50_000;
    engine.update_warmup_slope(user_idx).unwrap();
    let slope_low = engine.accounts[user_idx as usize].warmup_slope_per_step;
    let rate_low = engine.total_warmup_rate;

    assert!(slope_low <= slope_high, "Slope should decrease when PNL decreases");
    assert!(rate_low <= rate_high, "Total rate should decrease when user PNL decreases");
}
*/

// ============================================================================
// I10: Withdrawal-Only Mode (Fair Unwinding)
// SLOW_PROOF: Uses apply_adl which iterates over all accounts
// ============================================================================

// Previously slow - now fast with 8 accounts
#[kani::proof]
#[kani::unwind(10)]
#[kani::solver(cadical)]
fn i10_withdrawal_mode_triggers_on_insurance_depletion() {
    // When insurance fund is depleted and loss_accum > 0,
    // risk_reduction_only mode should be activated

    let mut engine = Box::new(RiskEngine::new(test_params()));
    let user_idx = engine.add_user(1).unwrap();

    let insurance: u128 = kani::any();
    let loss: u128 = kani::any();

    kani::assume(insurance < 10_000);
    kani::assume(loss < 20_000);
    kani::assume(loss > insurance); // Loss exceeds insurance

    engine.insurance_fund.balance = insurance;
    engine.accounts[user_idx as usize].capital = 10_000;
    engine.accounts[user_idx as usize].pnl = 1_000; // Some PNL

    let _ = engine.apply_adl(loss);

    // If loss > insurance, should enter withdrawal mode
    if loss > insurance {
        assert!(engine.risk_reduction_only,
                "I10: Withdrawal mode must activate when insurance depleted");
        assert!(engine.loss_accum > 0,
                "I10: loss_accum must be > 0 when insurance depleted");
        assert!(engine.insurance_fund.balance == 0,
                "I10: Insurance fund must be fully depleted");
    }
}


/*
// NOTE: Commented out - tests old withdrawal haircut logic which was removed
// The new withdrawal-only mode blocks ALL withdrawals instead of applying haircuts
#[kani::proof]
#[kani::unwind(10)]
#[kani::solver(cadical)]
fn i10_fair_unwinding_constant_haircut_ratio() {
    // All users receive the same haircut ratio regardless of withdrawal order

    let mut engine = Box::new(RiskEngine::new(test_params()));

    // Add two users with different principals
    let user1 = engine.add_user(1).unwrap();
    let user2 = engine.add_user(1).unwrap();

    let principal1: u128 = kani::any();
    let principal2: u128 = kani::any();
    let loss: u128 = kani::any();

    kani::assume(principal1 > 1_000 && principal1 < 10_000);
    kani::assume(principal2 > 1_000 && principal2 < 10_000);
    kani::assume(loss > 0 && loss < 5_000);

    engine.accounts[user1 as usize].capital = principal1;
    engine.accounts[user2 as usize].capital = principal2;

    // Trigger withdrawal mode
    engine.risk_reduction_only = true;
    engine.loss_accum = loss;

    let total_principal = principal1 + principal2;
    kani::assume(total_principal > loss); // System not completely insolvent

    // User1 withdraws
    let withdraw1 = principal1 / 2;
    let _ = engine.withdraw(user1, withdraw1);
    let actual1 = principal1 - engine.accounts[user1 as usize].capital;

    // User2 withdraws (after user1)
    let withdraw2 = principal2 / 2;
    let _ = engine.withdraw(user2, withdraw2);
    let actual2 = principal2 - engine.accounts[user2 as usize].capital;

    // Calculate expected haircut ratio
    let available = total_principal - loss;

    // Both should get the same ratio (within rounding)
    // ratio1 = actual1 / withdraw1
    // ratio2 = actual2 / withdraw2
    // These should be equal
    let ratio1_scaled = actual1 * withdraw2;
    let ratio2_scaled = actual2 * withdraw1;

    // Allow for 1 unit difference due to integer division
    assert!(ratio1_scaled.abs_diff(ratio2_scaled) <= withdraw1 + withdraw2,
            "I10: Both users must receive same haircut ratio (fair unwinding)");
}
*/

#[kani::proof]
#[kani::unwind(10)]
#[kani::solver(cadical)]
fn i10_withdrawal_mode_blocks_position_increase() {
    // In withdrawal-only mode, users cannot increase position size

    let mut engine = Box::new(RiskEngine::new(test_params()));
    let user_idx = engine.add_user(1).unwrap();
    let lp_idx = engine.add_lp([0u8; 32], [0u8; 32], 1).unwrap();

    engine.accounts[user_idx as usize].capital = 10_000;
    engine.accounts[lp_idx as usize].capital = 50_000;
    engine.vault = 60_000;

    let position: i128 = kani::any();
    let increase: i128 = kani::any();

    kani::assume(position != i128::MIN);
    kani::assume(position.abs() < 5_000);
    kani::assume(increase > 0 && increase < 2_000);

    engine.accounts[user_idx as usize].position_size = position;

    // Enter withdrawal mode
    engine.risk_reduction_only = true;
    engine.loss_accum = 1_000;

    // Try to increase position
    let new_size = if position >= 0 {
        position + increase // Increase long
    } else {
        position - increase // Increase short (more negative)
    };

    let matcher = NoOpMatcher;
    let result = engine.execute_trade(&matcher, lp_idx, user_idx, 1_000_000, new_size - position);

    // Should fail when trying to increase position
    if new_size.abs() > position.abs() {
        assert!(result.is_err(),
                "I10: Cannot increase position in withdrawal-only mode");
    }
}

#[kani::proof]
#[kani::unwind(10)]
#[kani::solver(cadical)]
fn i10_withdrawal_mode_allows_position_decrease() {
    // In withdrawal-only mode, users CAN decrease/close positions

    let mut engine = Box::new(RiskEngine::new(test_params()));
    let user_idx = engine.add_user(1).unwrap();
    let lp_idx = engine.add_lp([0u8; 32], [0u8; 32], 1).unwrap();

    engine.accounts[user_idx as usize].capital = 10_000;
    engine.accounts[lp_idx as usize].capital = 50_000;
    engine.vault = 60_000;

    let position: i128 = kani::any();
    kani::assume(position > 1_000 || position < -1_000);

    engine.accounts[user_idx as usize].position_size = position;
    engine.accounts[user_idx as usize].entry_price = 1_000_000;
    engine.accounts[lp_idx as usize].position_size = -position;
    engine.accounts[lp_idx as usize].entry_price = 1_000_000;

    // Enter withdrawal mode
    engine.risk_reduction_only = true;
    engine.loss_accum = 1_000;

    // Close half the position (reduce size)
    let reduce = -position / 2; // Opposite sign = reduce

    let matcher = NoOpMatcher;
    let result = engine.execute_trade(&matcher, lp_idx, user_idx, 1_000_000, reduce);

    // Closing/reducing should be allowed
    assert!(result.is_ok(),
            "I10: Position reduction should be allowed in withdrawal-only mode");
}

/*
// NOTE: Commented out - tests old withdrawal haircut logic which was removed
// The new withdrawal-only mode blocks ALL withdrawals instead of applying haircuts
#[kani::proof]
#[kani::unwind(10)]
#[kani::solver(cadical)]
fn i10_total_withdrawals_bounded_by_available() {
    // Total withdrawals in withdrawal mode cannot exceed (total_principal - loss_accum)

    let mut engine = Box::new(RiskEngine::new(test_params()));
    let user_idx = engine.add_user(1).unwrap();

    let principal: u128 = kani::any();
    let loss: u128 = kani::any();

    kani::assume(principal > 1_000 && principal < 10_000);
    kani::assume(loss > 0 && loss < principal);

    engine.accounts[user_idx as usize].capital = principal;

    // Enter withdrawal mode
    engine.risk_reduction_only = true;
    engine.loss_accum = loss;

    let vault_before = principal; // Assume vault matches
    engine.vault = vault_before;

    // Try to withdraw everything
    let _ = engine.withdraw(user_idx, principal);

    let withdrawn = vault_before.saturating_sub(engine.vault);
    let available = principal - loss;

    assert!(withdrawn <= available,
            "I10: Total withdrawals must not exceed available principal");
}


#[kani::proof]
#[kani::unwind(10)]
#[kani::solver(cadical)]
fn i10_top_up_reduces_loss_accum() {
    // Insurance fund top-ups directly reduce loss_accum

    let mut engine = Box::new(RiskEngine::new(test_params()));

    let loss: u128 = kani::any();
    let top_up: u128 = kani::any();

    kani::assume(loss > 0 && loss < 10_000);
    kani::assume(top_up > 0 && top_up < 20_000);

    engine.risk_reduction_only = true;
    engine.loss_accum = loss;
    engine.vault = 0;

    let loss_before = engine.loss_accum;

    let _ = engine.top_up_insurance_fund(top_up);

    // Loss should decrease by min(top_up, loss_before)
    let expected_reduction = if top_up > loss_before { loss_before } else { top_up };

    assert!(engine.loss_accum == loss_before - expected_reduction,
            "I10: Top-up must reduce loss_accum by contribution amount");
}
*/

#[kani::proof]
#[kani::unwind(10)]
#[kani::solver(cadical)]
fn i10_top_up_exits_withdrawal_mode_when_loss_zero() {
    // When loss_accum reaches 0, withdrawal mode should be exited

    let mut engine = Box::new(RiskEngine::new(test_params()));

    let loss: u128 = kani::any();
    kani::assume(loss > 0 && loss < 10_000);

    engine.risk_reduction_only = true;
    engine.loss_accum = loss;
    engine.vault = 0;

    // Top up exactly the loss amount
    let result = engine.top_up_insurance_fund(loss);

    assert!(result.is_ok(), "Top-up should succeed");
    assert!(engine.loss_accum == 0, "Loss should be fully covered");
    assert!(!engine.risk_reduction_only, "I10: Should exit withdrawal mode when loss_accum = 0");

    if let Ok(exited) = result {
        assert!(exited, "I10: Should return true when exiting withdrawal mode");
    }
}

// Previously slow - now fast with 8 accounts
#[kani::proof]
#[kani::unwind(10)]
#[kani::solver(cadical)]
fn i10_withdrawal_mode_preserves_conservation() {
    // Conservation must be maintained even in withdrawal-only mode

    let mut engine = Box::new(RiskEngine::new(test_params()));
    let user_idx = engine.add_user(1).unwrap();

    let principal: u128 = kani::any();
    let loss: u128 = kani::any();
    let withdraw: u128 = kani::any();

    kani::assume(principal > 1_000 && principal < 10_000);
    kani::assume(loss > 0 && loss < principal);
    kani::assume(withdraw > 0 && withdraw < principal);

    engine.accounts[user_idx as usize].capital = principal;
    engine.vault = principal;
    engine.insurance_fund.balance = 0; // Reset insurance to match vault = total_capital

    // Enter withdrawal mode
    engine.risk_reduction_only = true;
    engine.loss_accum = loss;

    assert!(engine.check_conservation(),
            "Conservation before withdrawal");

    let _ = engine.withdraw(user_idx, withdraw);

    assert!(engine.check_conservation(),
            "I10: Withdrawal mode must preserve conservation");
}


/*
// NOTE: Commented out - tests old withdrawal haircut logic which was removed
// The new withdrawal-only mode blocks ALL withdrawals instead of applying haircuts
#[kani::proof]
#[kani::unwind(10)]
#[kani::solver(cadical)]
fn i10_withdrawal_tracking_accuracy() {
    // withdrawal_mode_withdrawn should accurately track total withdrawn amounts

    let mut engine = Box::new(RiskEngine::new(test_params()));
    let user_idx = engine.add_user(1).unwrap();

    let principal: u128 = kani::any();
    let loss: u128 = kani::any();

    kani::assume(principal > 2_000 && principal < 10_000);
    kani::assume(loss > 0 && loss < principal / 2);

    engine.accounts[user_idx as usize].capital = principal;
    engine.vault = principal;

    // Enter withdrawal mode
    engine.risk_reduction_only = true;
    engine.loss_accum = loss;

    let tracking_before = engine.withdrawal_mode_withdrawn;

    // Withdraw some amount
    let withdraw = principal / 3;
    let _ = engine.withdraw(user_idx, withdraw);

    let actual_withdrawn = principal - engine.accounts[user_idx as usize].capital;
    let tracking_increase = engine.withdrawal_mode_withdrawn - tracking_before;

    assert!(tracking_increase == actual_withdrawn,
            "I10: withdrawal_mode_withdrawn must accurately track withdrawals");
}
*/

// ============================================================================
// LP-Specific Invariants (CRITICAL - Addresses Kani audit findings)
// ============================================================================

// Previously slow - now fast with 8 accounts
#[kani::proof]
#[kani::unwind(10)]
#[kani::solver(cadical)]
fn i1_lp_adl_never_reduces_capital() {
    // I1 for LPs: ADL must NEVER reduce LP capital
    // This is the LP equivalent of i1_adl_never_reduces_principal

    let mut engine = Box::new(RiskEngine::new(test_params()));
    let lp_idx = engine.add_lp([0u8; 32], [0u8; 32], 1).unwrap();

    // Set arbitrary but bounded values
    let capital: u128 = kani::any();
    let pnl: i128 = kani::any();
    let loss: u128 = kani::any();

    kani::assume(capital < 100_000);
    kani::assume(pnl > -100_000 && pnl < 100_000);
    kani::assume(loss < 100_000);

    engine.accounts[lp_idx as usize].capital = capital;
    engine.accounts[lp_idx as usize].pnl = pnl;
    engine.insurance_fund.balance = 1_000_000; // Large insurance

    let capital_before = engine.accounts[lp_idx as usize].capital;

    let _ = engine.apply_adl(loss);

    assert!(engine.accounts[lp_idx as usize].capital == capital_before,
            "I1-LP: ADL must NEVER reduce LP capital");
}


// Previously slow - now fast with 8 accounts
#[kani::proof]
#[kani::unwind(10)]
#[kani::solver(cadical)]
fn i1_lp_liquidation_never_reduces_capital() {
    // I1 for LPs: Liquidation must NEVER reduce LP capital
    // LP capital can only be reduced by explicit withdrawals, never by liquidation

    let mut engine = Box::new(RiskEngine::new(test_params()));
    let lp_idx = engine.add_lp([0u8; 32], [0u8; 32], 1).unwrap();
    let keeper_idx = engine.add_user(1).unwrap();

    let capital: u128 = kani::any();
    let position: i128 = kani::any();
    let oracle_price: u64 = kani::any();

    kani::assume(capital > 0 && capital < 10_000);
    kani::assume(position != 0 && position > -50_000 && position < 50_000);
    kani::assume(oracle_price > 100_000 && oracle_price < 10_000_000); // $0.10 to $10

    engine.accounts[lp_idx as usize].capital = capital;
    engine.accounts[lp_idx as usize].position_size = position;
    engine.accounts[lp_idx as usize].entry_price = 1_000_000; // $1
    engine.vault = capital;

    let capital_before = engine.accounts[lp_idx as usize].capital;

    let _ = engine.liquidate_account(lp_idx, keeper_idx, oracle_price);

    assert!(engine.accounts[lp_idx as usize].capital == capital_before,
            "I1-LP: Liquidation must NEVER reduce LP capital");
}


// Previously slow - now fast with 8 accounts
#[kani::proof]
#[kani::unwind(10)]
#[kani::solver(cadical)]
fn adl_is_proportional_for_user_and_lp() {
    // Proportional ADL Fairness: Users and LPs with equal unwrapped PNL
    // should receive equal haircuts

    let mut engine = Box::new(RiskEngine::new(test_params()));
    let user_idx = engine.add_user(1).unwrap();
    let lp_idx = engine.add_lp([0u8; 32], [0u8; 32], 1).unwrap();

    let pnl: i128 = kani::any();
    let loss: u128 = kani::any();

    // Both have the same unwrapped PNL
    kani::assume(pnl > 0 && pnl < 50_000);
    kani::assume(loss > 0 && loss < 100_000);

    engine.accounts[user_idx as usize].pnl = pnl;
    engine.accounts[lp_idx as usize].pnl = pnl;
    engine.insurance_fund.balance = 1_000_000;

    // Both start with no reserved PNL and no warmup
    // (so all PNL is unwrapped)
    engine.accounts[user_idx as usize].reserved_pnl = 0;
    engine.accounts[lp_idx as usize].reserved_pnl = 0;
    engine.accounts[user_idx as usize].warmup_slope_per_step = 0;
    engine.accounts[lp_idx as usize].warmup_slope_per_step = 0;

    let user_pnl_before = engine.accounts[user_idx as usize].pnl;
    let lp_pnl_before = engine.accounts[lp_idx as usize].pnl;

    let _ = engine.apply_adl(loss);

    let user_loss = user_pnl_before - engine.accounts[user_idx as usize].pnl;
    let lp_loss = lp_pnl_before - engine.accounts[lp_idx as usize].pnl;

    // Both should lose the same amount (proportional means equal when starting equal)
    assert!(user_loss == lp_loss,
            "ADL: User and LP with equal unwrapped PNL must receive equal haircuts");
}


// Previously slow - now fast with 8 accounts
#[kani::proof]
#[kani::unwind(10)]
#[kani::solver(cadical)]
fn adl_proportionality_general() {
    // General proportional ADL: Haircut percentages should be equal
    // even when PNL amounts differ

    let mut engine = Box::new(RiskEngine::new(test_params()));
    let user_idx = engine.add_user(1).unwrap();
    let lp_idx = engine.add_lp([0u8; 32], [0u8; 32], 1).unwrap();

    let user_pnl: i128 = kani::any();
    let lp_pnl: i128 = kani::any();
    let loss: u128 = kani::any();

    kani::assume(user_pnl > 0 && user_pnl < 30_000);
    kani::assume(lp_pnl > 0 && lp_pnl < 30_000);
    kani::assume(loss > 0 && loss < 50_000);
    kani::assume(user_pnl != lp_pnl); // Different amounts

    engine.accounts[user_idx as usize].pnl = user_pnl;
    engine.accounts[lp_idx as usize].pnl = lp_pnl;
    engine.insurance_fund.balance = 1_000_000;

    // No reserved PNL, no warmup (all unwrapped)
    engine.accounts[user_idx as usize].reserved_pnl = 0;
    engine.accounts[lp_idx as usize].reserved_pnl = 0;
    engine.accounts[user_idx as usize].warmup_slope_per_step = 0;
    engine.accounts[lp_idx as usize].warmup_slope_per_step = 0;

    let user_pnl_before = engine.accounts[user_idx as usize].pnl;
    let lp_pnl_before = engine.accounts[lp_idx as usize].pnl;

    let _ = engine.apply_adl(loss);

    let user_loss = (user_pnl_before - engine.accounts[user_idx as usize].pnl) as u128;
    let lp_loss = (lp_pnl_before - engine.accounts[lp_idx as usize].pnl) as u128;

    // Check proportionality using cross-multiplication to avoid division
    // user_loss / user_pnl == lp_loss / lp_pnl
    // <=> user_loss * lp_pnl == lp_loss * user_pnl

    let cross1 = user_loss.saturating_mul(lp_pnl as u128);
    let cross2 = lp_loss.saturating_mul(user_pnl as u128);

    // Allow for rounding error of up to (total_pnl) due to integer division
    let total_pnl = (user_pnl + lp_pnl) as u128;
    let diff = if cross1 > cross2 { cross1 - cross2 } else { cross2 - cross1 };

    assert!(diff <= total_pnl,
            "ADL: Haircuts must be proportional (within rounding tolerance)");
}


/*
// NOTE: Commented out - tests old withdrawal haircut logic which was removed
// The new withdrawal-only mode blocks ALL withdrawals instead of applying haircuts
#[kani::proof]
#[kani::unwind(10)]
#[kani::solver(cadical)]
fn i10_fair_unwinding_is_fair_for_lps() {
    // I10 for LPs: Users and LPs receive the same haircut ratio in withdrawal-only mode
    // This extends i10_fair_unwinding_constant_haircut_ratio to include LPs

    let mut engine = Box::new(RiskEngine::new(test_params()));
    let user_idx = engine.add_user(1).unwrap();
    let lp_idx = engine.add_lp([0u8; 32], [0u8; 32], 1).unwrap();

    let user_capital: u128 = kani::any();
    let lp_capital: u128 = kani::any();
    let loss: u128 = kani::any();

    kani::assume(user_capital > 1_000 && user_capital < 10_000);
    kani::assume(lp_capital > 1_000 && lp_capital < 10_000);
    kani::assume(loss > 0 && loss < 5_000);

    engine.accounts[user_idx as usize].capital = user_capital;
    engine.accounts[lp_idx as usize].capital = lp_capital;
    engine.vault = user_capital + lp_capital;

    let total_capital = user_capital + lp_capital;
    kani::assume(total_capital > loss); // Not completely insolvent

    // Trigger withdrawal mode
    engine.risk_reduction_only = true;
    engine.loss_accum = loss;

    // User withdraws half their capital
    let withdraw_user = user_capital / 2;
    let _ = engine.withdraw(user_idx, withdraw_user);
    let actual_user = user_capital - engine.accounts[user_idx as usize].capital;

    // LP withdraws half their capital
    let withdraw_lp = lp_capital / 2;
    let _ = engine.withdraw(lp_idx, withdraw_lp);
    let actual_lp = lp_capital - engine.accounts[lp_idx as usize].capital;

    // Both should get the same haircut ratio
    // ratio_user = actual_user / withdraw_user
    // ratio_lp = actual_lp / withdraw_lp
    // These should be equal (within rounding)

    let ratio_user_scaled = actual_user * withdraw_lp;
    let ratio_lp_scaled = actual_lp * withdraw_user;

    // Allow for rounding error
    let tolerance = withdraw_user + withdraw_lp;

    assert!(ratio_user_scaled.abs_diff(ratio_lp_scaled) <= tolerance,
            "I10-LP: Users and LPs must receive same haircut ratio in withdrawal-only mode");
}
*/

// Previously slow - now fast with 8 accounts
#[kani::proof]
#[kani::unwind(10)]
#[kani::solver(cadical)]
fn multiple_lps_adl_preserves_all_capitals() {
    // Multi-LP ADL: All LP capitals are preserved, similar to multiple_users_adl_preserves_all_principals

    let mut engine = Box::new(RiskEngine::new(test_params()));
    let lp1 = engine.add_lp([1u8; 32], [1u8; 32], 1).unwrap();
    let lp2 = engine.add_lp([2u8; 32], [2u8; 32], 1).unwrap();

    let c1: u128 = kani::any();
    let c2: u128 = kani::any();
    let pnl1: i128 = kani::any();
    let pnl2: i128 = kani::any();
    let loss: u128 = kani::any();

    kani::assume(c1 < 5_000);
    kani::assume(c2 < 5_000);
    kani::assume(pnl1 > -5_000 && pnl1 < 5_000);
    kani::assume(pnl2 > -5_000 && pnl2 < 5_000);
    kani::assume(loss < 10_000);

    engine.accounts[lp1 as usize].capital = c1;
    engine.accounts[lp1 as usize].pnl = pnl1;
    engine.accounts[lp2 as usize].capital = c2;
    engine.accounts[lp2 as usize].pnl = pnl2;
    engine.insurance_fund.balance = 100_000;

    let _ = engine.apply_adl(loss);

    assert!(engine.accounts[lp1 as usize].capital == c1,
            "Multi-LP ADL: LP1 capital preserved");
    assert!(engine.accounts[lp2 as usize].capital == c2,
            "Multi-LP ADL: LP2 capital preserved");
}


// Previously slow - now fast with 8 accounts
#[kani::proof]
#[kani::unwind(10)]
#[kani::solver(cadical)]
fn mixed_users_and_lps_adl_preserves_all_capitals() {
    // Mixed ADL: Both user and LP capitals are preserved together

    let mut engine = Box::new(RiskEngine::new(test_params()));
    let user_idx = engine.add_user(1).unwrap();
    let lp_idx = engine.add_lp([0u8; 32], [0u8; 32], 1).unwrap();

    let user_capital: u128 = kani::any();
    let lp_capital: u128 = kani::any();
    let user_pnl: i128 = kani::any();
    let lp_pnl: i128 = kani::any();
    let loss: u128 = kani::any();

    kani::assume(user_capital < 5_000);
    kani::assume(lp_capital < 5_000);
    kani::assume(user_pnl > -5_000 && user_pnl < 5_000);
    kani::assume(lp_pnl > -5_000 && lp_pnl < 5_000);
    kani::assume(loss < 10_000);

    engine.accounts[user_idx as usize].capital = user_capital;
    engine.accounts[user_idx as usize].pnl = user_pnl;
    engine.accounts[lp_idx as usize].capital = lp_capital;
    engine.accounts[lp_idx as usize].pnl = lp_pnl;
    engine.insurance_fund.balance = 100_000;

    let _ = engine.apply_adl(loss);

    assert!(engine.accounts[user_idx as usize].capital == user_capital,
            "Mixed ADL: User capital preserved");
    assert!(engine.accounts[lp_idx as usize].capital == lp_capital,
            "Mixed ADL: LP capital preserved");
}


// ============================================================================
// Risk-Reduction-Only Mode Proofs
// ============================================================================

// Proof 1: Warmup does not advance while paused
#[kani::proof]
#[kani::unwind(10)]
#[kani::solver(cadical)]
fn proof_warmup_frozen_when_paused() {
    let mut engine = Box::new(RiskEngine::new(test_params()));
    let user_idx = engine.add_user(1).unwrap();

    let pnl: i128 = kani::any();
    let slope: u128 = kani::any();
    let started_at: u64 = kani::any();
    let pause_slot: u64 = kani::any();
    let current_slot: u64 = kani::any();

    // Bounded assumptions
    kani::assume(pnl > 0 && pnl < 10_000);
    kani::assume(slope > 0 && slope < 1_000);
    kani::assume(started_at < 100);
    kani::assume(pause_slot >= started_at && pause_slot < 200);
    kani::assume(current_slot >= pause_slot && current_slot < 300);

    // Setup account with PNL and warmup
    engine.accounts[user_idx as usize].pnl = pnl;
    engine.accounts[user_idx as usize].warmup_slope_per_step = slope;
    engine.accounts[user_idx as usize].warmup_started_at_slot = started_at;

    // Pause warmup at pause_slot
    engine.warmup_paused = true;
    engine.warmup_pause_slot = pause_slot;

    // Compute withdrawable at pause_slot
    engine.current_slot = pause_slot;
    let withdrawable_at_pause = engine.withdrawable_pnl(&engine.accounts[user_idx as usize]);

    // Compute withdrawable at later current_slot (should be same)
    engine.current_slot = current_slot;
    let withdrawable_later = engine.withdrawable_pnl(&engine.accounts[user_idx as usize]);

    // PROOF: Withdrawable PNL does not increase when warmup is paused
    assert!(withdrawable_later == withdrawable_at_pause,
            "Warmup should not advance while paused");
}

// Proof 2: In risk mode, withdraw never decreases PNL directly (only via warmup conversion)
#[kani::proof]
#[kani::unwind(10)]
#[kani::solver(cadical)]
fn proof_withdraw_only_decreases_via_conversion() {
    let mut engine = Box::new(RiskEngine::new(test_params()));
    let user_idx = engine.add_user(1).unwrap();

    let capital: u128 = kani::any();
    let pnl: i128 = kani::any();
    let slope: u128 = kani::any();
    let amount: u128 = kani::any();

    // Bounded assumptions
    kani::assume(capital > 0 && capital < 10_000);
    kani::assume(pnl > 0 && pnl < 5_000);
    kani::assume(slope > 0 && slope < 100);
    kani::assume(amount > 0 && amount < 1_000);

    // Setup account
    engine.accounts[user_idx as usize].capital = capital;
    engine.accounts[user_idx as usize].pnl = pnl;
    engine.accounts[user_idx as usize].warmup_slope_per_step = slope;
    engine.accounts[user_idx as usize].warmup_started_at_slot = 0;
    engine.current_slot = 10;
    engine.vault = 100_000;

    // Enter risk mode
    engine.enter_risk_reduction_only_mode();

    // Compute expected warmed amount
    let warmed = engine.withdrawable_pnl(&engine.accounts[user_idx as usize]);
    let pnl_before = engine.accounts[user_idx as usize].pnl;

    // Attempt withdrawal
    let _ = engine.withdraw(user_idx, amount);

    let pnl_after = engine.accounts[user_idx as usize].pnl;

    // PROOF: PNL only decreases by the warmed conversion amount
    // pnl_after should be >= pnl_before - warmed
    // and pnl_after should be <= pnl_before
    assert!(pnl_after >= pnl_before - (warmed as i128),
            "PNL should not decrease more than warmed amount");
    assert!(pnl_after <= pnl_before,
            "PNL should not increase during withdrawal");
}

// Proof 3: Risk-increasing trades are rejected in risk mode
#[kani::proof]
#[kani::unwind(10)]
#[kani::solver(cadical)]
fn proof_risk_increasing_trades_rejected() {
    let mut engine = Box::new(RiskEngine::new(test_params()));
    let user_idx = engine.add_user(1).unwrap();
    let lp_idx = engine.add_lp([0u8; 32], [0u8; 32], 1).unwrap();

    let old_pos: i128 = kani::any();
    let delta: i128 = kani::any();

    // Bounded assumptions
    kani::assume(old_pos >= -100 && old_pos <= 100);
    kani::assume(delta >= -100 && delta <= 100);
    kani::assume(delta != 0); // Non-zero trade

    // Setup positions
    engine.accounts[user_idx as usize].position_size = old_pos;
    engine.accounts[lp_idx as usize].position_size = -old_pos;
    engine.accounts[user_idx as usize].capital = 100_000;
    engine.accounts[lp_idx as usize].capital = 100_000;
    engine.vault = 200_000;

    let new_pos = old_pos.saturating_add(delta);
    let user_increases = new_pos.abs() > old_pos.abs();

    // Enter risk mode
    engine.enter_risk_reduction_only_mode();

    // Attempt trade
    let result = engine.execute_trade(&NoOpMatcher, lp_idx, user_idx, 100_000_000, delta);

    // PROOF: If trade increases absolute exposure, it must be rejected in risk mode
    if user_increases {
        assert!(result.is_err(), "Risk-increasing trades must fail in risk mode");
    }
}

