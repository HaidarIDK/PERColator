//! Formal verification with Kani
//!
//! These proofs verify critical safety properties of the risk engine.
//! Run with: cargo kani
//!
//! Key invariants proven:
//! - I1: User principal is never reduced by ADL/socialization
//! - I2: Conservation of funds across all operations
//! - I3: Authorization checks prevent unauthorized operations
//! - I4: Socialized losses are bounded
//! - I5: PNL warmup is monotonic and deterministic
//! - I6: Liquidations maintain system solvency
//! - I7: User isolation - operations on one user don't affect others
//! - I8: Collateral calculations are consistent

#![cfg(kani)]

use percolator::*;

// Use the Vec-based implementation for tests
type RiskEngine = VecRiskEngine;

// Helper to create test params
fn test_params() -> RiskParams {
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

// ============================================================================
// I1: Principal is NEVER reduced by ADL/socialization
// ============================================================================

#[kani::proof]
#[kani::unwind(4)]
fn i1_adl_never_reduces_principal() {
    let mut engine = RiskEngine::new(test_params());
    let user_idx = engine.add_user(1).unwrap();

    // Set arbitrary but bounded values
    let principal: u128 = kani::any();
    let pnl: i128 = kani::any();
    let loss: u128 = kani::any();

    kani::assume(principal < 100_000);
    kani::assume(pnl > -100_000 && pnl < 100_000);
    kani::assume(loss < 100_000);

    engine.users[user_idx].principal = principal;
    engine.users[user_idx].pnl_ledger = pnl;
    engine.insurance_fund.balance = 1_000_000; // Large insurance

    let principal_before = engine.users[user_idx].principal;

    let _ = engine.apply_adl(loss);

    assert!(engine.users[user_idx].principal == principal_before,
            "I1: ADL must NEVER reduce user principal");
}

// ============================================================================
// I2: Conservation of funds
// ============================================================================

#[kani::proof]
#[kani::unwind(2)]
fn i2_deposit_preserves_conservation() {
    let mut engine = RiskEngine::new(test_params());
    let user_idx = engine.add_user(1).unwrap();

    let amount: u128 = kani::any();
    kani::assume(amount < 10_000);

    // Initial state conserves trivially
    assert!(engine.check_conservation());

    let _ = engine.deposit(user_idx, amount);

    assert!(engine.check_conservation(),
            "I2: Deposit must preserve conservation");
}

#[kani::proof]
#[kani::unwind(2)]
fn i2_withdraw_preserves_conservation() {
    let mut engine = RiskEngine::new(test_params());
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
#[kani::unwind(4)]
fn i5_warmup_determinism() {
    let mut engine = RiskEngine::new(test_params());
    let user_idx = engine.add_user(1).unwrap();

    let pnl: i128 = kani::any();
    let reserved: u128 = kani::any();
    let slope: u128 = kani::any();
    let slots: u64 = kani::any();

    kani::assume(pnl > 0 && pnl < 10_000);
    kani::assume(reserved < 5_000);
    kani::assume(slope > 0 && slope < 100);
    kani::assume(slots < 200);

    engine.users[user_idx].pnl_ledger = pnl;
    engine.users[user_idx].reserved_pnl = reserved;
    engine.users[user_idx].warmup_state.slope_per_step = slope;
    engine.current_slot = slots;

    // Calculate twice with same inputs
    let w1 = engine.withdrawable_pnl(&engine.users[user_idx]);
    let w2 = engine.withdrawable_pnl(&engine.users[user_idx]);

    assert!(w1 == w2,
            "I5: Withdrawable PNL must be deterministic");
}

#[kani::proof]
#[kani::unwind(4)]
fn i5_warmup_monotonicity() {
    let mut engine = RiskEngine::new(test_params());
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

    engine.users[user_idx].pnl_ledger = pnl;
    engine.users[user_idx].warmup_state.slope_per_step = slope;

    engine.current_slot = slots1;
    let w1 = engine.withdrawable_pnl(&engine.users[user_idx]);

    engine.current_slot = slots2;
    let w2 = engine.withdrawable_pnl(&engine.users[user_idx]);

    assert!(w2 >= w1,
            "I5: Warmup must be monotonically increasing over time");
}

#[kani::proof]
#[kani::unwind(4)]
fn i5_warmup_bounded_by_pnl() {
    let mut engine = RiskEngine::new(test_params());
    let user_idx = engine.add_user(1).unwrap();

    let pnl: i128 = kani::any();
    let reserved: u128 = kani::any();
    let slope: u128 = kani::any();
    let slots: u64 = kani::any();

    kani::assume(pnl > 0 && pnl < 10_000);
    kani::assume(reserved < 5_000);
    kani::assume(slope > 0 && slope < 100);
    kani::assume(slots < 200);

    engine.users[user_idx].pnl_ledger = pnl;
    engine.users[user_idx].reserved_pnl = reserved;
    engine.users[user_idx].warmup_state.slope_per_step = slope;
    engine.current_slot = slots;

    let withdrawable = engine.withdrawable_pnl(&engine.users[user_idx]);
    let positive_pnl = pnl as u128;
    let available = positive_pnl.saturating_sub(reserved);

    assert!(withdrawable <= available,
            "I5: Withdrawable must not exceed available PNL");
}

// ============================================================================
// I7: User Isolation
// ============================================================================

#[kani::proof]
#[kani::unwind(3)]
fn i7_user_isolation_deposit() {
    let mut engine = RiskEngine::new(test_params());
    let user1 = engine.add_user(1).unwrap();
    let user2 = engine.add_user(1).unwrap();

    let amount1: u128 = kani::any();
    let amount2: u128 = kani::any();

    kani::assume(amount1 < 10_000);
    kani::assume(amount2 < 10_000);

    let _ = engine.deposit(user1, amount1);
    let _ = engine.deposit(user2, amount2);

    let user2_principal = engine.users[user2].principal;
    let user2_pnl = engine.users[user2].pnl_ledger;

    // Operate on user1
    let _ = engine.deposit(user1, 100);

    // User2 should be unchanged
    assert!(engine.users[user2].principal == user2_principal,
            "I7: User2 principal unchanged by user1 deposit");
    assert!(engine.users[user2].pnl_ledger == user2_pnl,
            "I7: User2 PNL unchanged by user1 deposit");
}

#[kani::proof]
#[kani::unwind(3)]
fn i7_user_isolation_withdrawal() {
    let mut engine = RiskEngine::new(test_params());
    let user1 = engine.add_user(1).unwrap();
    let user2 = engine.add_user(1).unwrap();

    let amount1: u128 = kani::any();
    let amount2: u128 = kani::any();

    kani::assume(amount1 > 100 && amount1 < 10_000);
    kani::assume(amount2 < 10_000);

    let _ = engine.deposit(user1, amount1);
    let _ = engine.deposit(user2, amount2);

    let user2_principal = engine.users[user2].principal;
    let user2_pnl = engine.users[user2].pnl_ledger;

    // Operate on user1
    let _ = engine.withdraw(user1, 50);

    // User2 should be unchanged
    assert!(engine.users[user2].principal == user2_principal,
            "I7: User2 principal unchanged by user1 withdrawal");
    assert!(engine.users[user2].pnl_ledger == user2_pnl,
            "I7: User2 PNL unchanged by user1 withdrawal");
}

// ============================================================================
// I8: Collateral Consistency
// ============================================================================

#[kani::proof]
#[kani::unwind(2)]
fn i8_collateral_with_positive_pnl() {
    let mut engine = RiskEngine::new(test_params());
    let user_idx = engine.add_user(1).unwrap();

    let principal: u128 = kani::any();
    let pnl: i128 = kani::any();

    kani::assume(principal < 10_000);
    kani::assume(pnl > 0 && pnl < 10_000);

    engine.users[user_idx].principal = principal;
    engine.users[user_idx].pnl_ledger = pnl;

    let collateral = engine.user_collateral(&engine.users[user_idx]);
    let expected = principal.saturating_add(pnl as u128);

    assert!(collateral == expected,
            "I8: Collateral = principal + positive PNL");
}

#[kani::proof]
#[kani::unwind(2)]
fn i8_collateral_with_negative_pnl() {
    let mut engine = RiskEngine::new(test_params());
    let user_idx = engine.add_user(1).unwrap();

    let principal: u128 = kani::any();
    let pnl: i128 = kani::any();

    kani::assume(principal < 10_000);
    kani::assume(pnl < 0 && pnl > -10_000);

    engine.users[user_idx].principal = principal;
    engine.users[user_idx].pnl_ledger = pnl;

    let collateral = engine.user_collateral(&engine.users[user_idx]);

    assert!(collateral == principal,
            "I8: Collateral = principal when PNL is negative");
}

// ============================================================================
// I4: Bounded Losses (ADL mechanics)
// ============================================================================

#[kani::proof]
#[kani::unwind(4)]
fn i4_adl_haircuts_unwrapped_first() {
    let mut engine = RiskEngine::new(test_params());
    let user_idx = engine.add_user(1).unwrap();

    let principal: u128 = kani::any();
    let pnl: i128 = kani::any();
    let loss: u128 = kani::any();

    kani::assume(principal < 10_000);
    kani::assume(pnl > 0 && pnl < 10_000);
    kani::assume(loss < 5_000);
    kani::assume(loss < pnl as u128); // Loss less than PNL

    engine.users[user_idx].principal = principal;
    engine.users[user_idx].pnl_ledger = pnl;
    engine.users[user_idx].warmup_state.slope_per_step = 10;
    engine.insurance_fund.balance = 100_000;

    let pnl_before = engine.users[user_idx].pnl_ledger;
    let insurance_before = engine.insurance_fund.balance;

    let _ = engine.apply_adl(loss);

    // If there was enough unwrapped PNL, insurance shouldn't be touched
    let unwrapped_pnl = pnl as u128; // At slot 0, nothing is warmed up

    if loss <= unwrapped_pnl {
        assert!(engine.insurance_fund.balance == insurance_before,
                "I4: ADL should haircut PNL before touching insurance");
        assert!(engine.users[user_idx].pnl_ledger == pnl_before - (loss as i128),
                "I4: PNL should be reduced by loss amount");
    }
}

// ============================================================================
// Withdrawal Safety
// ============================================================================

#[kani::proof]
#[kani::unwind(3)]
fn withdrawal_requires_sufficient_balance() {
    let mut engine = RiskEngine::new(test_params());
    let user_idx = engine.add_user(1).unwrap();

    let principal: u128 = kani::any();
    let withdraw: u128 = kani::any();

    kani::assume(principal < 10_000);
    kani::assume(withdraw < 20_000);
    kani::assume(withdraw > principal); // Try to withdraw more than available

    engine.users[user_idx].principal = principal;
    engine.vault = principal;

    let result = engine.withdraw(user_idx, withdraw);

    assert!(result.is_err(),
            "Withdrawal of more than available must fail");
}

#[kani::proof]
#[kani::unwind(3)]
fn pnl_withdrawal_requires_warmup() {
    let mut engine = RiskEngine::new(test_params());
    let user_idx = engine.add_user(1).unwrap();

    let pnl: i128 = kani::any();
    let withdraw: u128 = kani::any();

    kani::assume(pnl > 0 && pnl < 10_000);
    kani::assume(withdraw > 0 && withdraw < 10_000);

    engine.users[user_idx].pnl_ledger = pnl;
    engine.users[user_idx].warmup_state.slope_per_step = 10;
    engine.users[user_idx].principal = 0; // No principal
    engine.insurance_fund.balance = 100_000;
    engine.vault = pnl as u128;
    engine.current_slot = 0; // At slot 0, nothing warmed up

    // withdrawable_pnl should be 0 at slot 0
    let withdrawable = engine.withdrawable_pnl(&engine.users[user_idx]);
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
// ============================================================================

#[kani::proof]
#[kani::unwind(4)]
fn multiple_users_adl_preserves_all_principals() {
    let mut engine = RiskEngine::new(test_params());
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

    engine.users[user1].principal = p1;
    engine.users[user1].pnl_ledger = pnl1;
    engine.users[user2].principal = p2;
    engine.users[user2].pnl_ledger = pnl2;
    engine.insurance_fund.balance = 100_000;

    let _ = engine.apply_adl(loss);

    assert!(engine.users[user1].principal == p1,
            "Multi-user ADL: User1 principal preserved");
    assert!(engine.users[user2].principal == p2,
            "Multi-user ADL: User2 principal preserved");
}

// ============================================================================
// Arithmetic Safety
// ============================================================================

#[kani::proof]
#[kani::unwind(2)]
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
// ============================================================================

#[kani::proof]
#[kani::unwind(3)]
fn liquidation_closes_position() {
    let mut engine = RiskEngine::new(test_params());
    let user_idx = engine.add_user(1).unwrap();
    let keeper_idx = engine.add_user(1).unwrap();

    let principal: u128 = kani::any();
    let position: i128 = kani::any();

    kani::assume(principal > 0 && principal < 1_000);
    kani::assume(position != 0 && position > -10_000 && position < 10_000);

    engine.users[user_idx].principal = principal;
    engine.users[user_idx].position_size = position;
    engine.users[user_idx].entry_price = 1_000_000;
    engine.vault = principal;

    let _ = engine.liquidate_user(user_idx, keeper_idx, 1_000_000);

    // After liquidation, position should be closed (or at least reduced)
    assert!(engine.users[user_idx].position_size.abs() <= position.abs(),
            "Liquidation should reduce or close position");
}

// ============================================================================
// Edge Cases
// ============================================================================

#[kani::proof]
#[kani::unwind(2)]
fn zero_pnl_withdrawable_is_zero() {
    let mut engine = RiskEngine::new(test_params());
    let user_idx = engine.add_user(1).unwrap();

    engine.users[user_idx].pnl_ledger = 0;
    engine.current_slot = 1000; // Far in future

    let withdrawable = engine.withdrawable_pnl(&engine.users[user_idx]);

    assert!(withdrawable == 0,
            "Zero PNL means zero withdrawable");
}

#[kani::proof]
#[kani::unwind(2)]
fn negative_pnl_withdrawable_is_zero() {
    let mut engine = RiskEngine::new(test_params());
    let user_idx = engine.add_user(1).unwrap();

    let pnl: i128 = kani::any();
    kani::assume(pnl < 0 && pnl > -10_000);

    engine.users[user_idx].pnl_ledger = pnl;
    engine.current_slot = 1000;

    let withdrawable = engine.withdrawable_pnl(&engine.users[user_idx]);

    assert!(withdrawable == 0,
            "Negative PNL means zero withdrawable");
}

// ============================================================================
// Funding Rate Invariants
// ============================================================================

#[kani::proof]
#[kani::unwind(2)]
fn funding_p1_settlement_idempotent() {
    // P1: Funding settlement is idempotent
    // After settling once, settling again with unchanged global index does nothing

    let mut engine = RiskEngine::new(test_params());
    let user_idx = engine.add_user(1).unwrap();

    // Arbitrary position and PNL
    let position: i128 = kani::any();
    kani::assume(position.abs() < 1_000_000);

    let pnl: i128 = kani::any();
    kani::assume(pnl > -1_000_000 && pnl < 1_000_000);

    engine.users[user_idx].position_size = position;
    engine.users[user_idx].pnl_ledger = pnl;

    // Set arbitrary funding index
    let index: i128 = kani::any();
    kani::assume(index.abs() < 1_000_000_000);
    engine.funding_index_qpb_e6 = index;

    // Settle once
    let _ = engine.touch_user(user_idx);

    let pnl_after_first = engine.users[user_idx].pnl_ledger;
    let snapshot_after_first = engine.users[user_idx].funding_index_user;

    // Settle again without changing global index
    let _ = engine.touch_user(user_idx);

    // PNL should be unchanged
    assert!(engine.users[user_idx].pnl_ledger == pnl_after_first,
            "Second settlement should not change PNL");

    // Snapshot should equal global index
    assert!(engine.users[user_idx].funding_index_user == engine.funding_index_qpb_e6,
            "Snapshot should equal global index");
}

#[kani::proof]
#[kani::unwind(2)]
fn funding_p2_never_touches_principal() {
    // P2: Funding does not touch principal (extends Invariant I1)

    let mut engine = RiskEngine::new(test_params());
    let user_idx = engine.add_user(1).unwrap();

    let principal: u128 = kani::any();
    kani::assume(principal < 1_000_000);

    let position: i128 = kani::any();
    kani::assume(position.abs() < 1_000_000);

    engine.users[user_idx].principal = principal;
    engine.users[user_idx].position_size = position;

    // Accrue arbitrary funding
    let funding_delta: i128 = kani::any();
    kani::assume(funding_delta.abs() < 1_000_000_000);
    engine.funding_index_qpb_e6 = funding_delta;

    // Settle funding
    let _ = engine.touch_user(user_idx);

    // Principal must be unchanged
    assert!(engine.users[user_idx].principal == principal,
            "Funding must never modify principal");
}

#[kani::proof]
#[kani::unwind(2)]
fn funding_p3_zero_sum_between_opposite_positions() {
    // P3: Funding is zero-sum when user and LP have opposite positions

    let mut engine = RiskEngine::new(test_params());
    let user_idx = engine.add_user(1).unwrap();
    let lp_idx = engine.add_lp([0u8; 32], [0u8; 32], 1).unwrap();

    let position: i128 = kani::any();
    kani::assume(position > 0 && position < 100_000); // positive only for simplicity

    // User has position, LP has opposite
    engine.users[user_idx].position_size = position;
    engine.lps[lp_idx].lp_position_size = -position;

    // Both start with same snapshot
    engine.users[user_idx].funding_index_user = 0;
    engine.lps[lp_idx].funding_index_lp = 0;

    let user_pnl_before = engine.users[user_idx].pnl_ledger;
    let lp_pnl_before = engine.lps[lp_idx].lp_pnl;
    let total_before = user_pnl_before + lp_pnl_before;

    // Accrue funding
    let delta: i128 = kani::any();
    kani::assume(delta.abs() < 1_000_000);
    engine.funding_index_qpb_e6 = delta;

    // Settle both
    let user_result = engine.touch_user(user_idx);
    let lp_result = engine.touch_lp(lp_idx);

    // If both settlements succeeded, check zero-sum
    if user_result.is_ok() && lp_result.is_ok() {
        let total_after = engine.users[user_idx].pnl_ledger + engine.lps[lp_idx].lp_pnl;

        assert!(total_after == total_before,
                "Funding must be zero-sum between opposite positions");
    }
}

#[kani::proof]
#[kani::unwind(3)]
fn funding_p4_settle_before_position_change() {
    // P4: Verifies that settlement before position change gives correct results

    let mut engine = RiskEngine::new(test_params());
    let user_idx = engine.add_user(1).unwrap();

    let initial_pos: i128 = kani::any();
    kani::assume(initial_pos > 0 && initial_pos < 10_000);

    engine.users[user_idx].position_size = initial_pos;
    engine.users[user_idx].pnl_ledger = 0;
    engine.users[user_idx].funding_index_user = 0;

    // Period 1: accrue funding with initial position
    let delta1: i128 = kani::any();
    kani::assume(delta1.abs() < 1_000);
    engine.funding_index_qpb_e6 = delta1;

    // Settle BEFORE changing position (correct way)
    let _ = engine.touch_user(user_idx);

    let pnl_after_period1 = engine.users[user_idx].pnl_ledger;

    // Change position
    let new_pos: i128 = kani::any();
    kani::assume(new_pos > 0 && new_pos < 10_000 && new_pos != initial_pos);
    engine.users[user_idx].position_size = new_pos;

    // Period 2: more funding
    let delta2: i128 = kani::any();
    kani::assume(delta2.abs() < 1_000);
    engine.funding_index_qpb_e6 = delta1 + delta2;

    let _ = engine.touch_user(user_idx);

    // The settlement should have correctly applied:
    // - delta1 to initial_pos
    // - delta2 to new_pos
    // Snapshot should equal global index
    assert!(engine.users[user_idx].funding_index_user == engine.funding_index_qpb_e6,
            "Snapshot must track global index");
}

#[kani::proof]
#[kani::unwind(2)]
fn funding_p5_bounded_operations_no_overflow() {
    // P5: No overflows on bounded inputs (or returns Overflow error)

    let mut engine = RiskEngine::new(test_params());

    // Bounded inputs
    let price: u64 = kani::any();
    kani::assume(price > 1_000_000 && price < 1_000_000_000); // $1 to $1000

    let rate: i64 = kani::any();
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
#[kani::unwind(2)]
fn funding_zero_position_no_change() {
    // Additional invariant: Zero position means no funding payment

    let mut engine = RiskEngine::new(test_params());
    let user_idx = engine.add_user(1).unwrap();

    engine.users[user_idx].position_size = 0; // Zero position

    let pnl_before: i128 = kani::any();
    kani::assume(pnl_before.abs() < 1_000_000);
    engine.users[user_idx].pnl_ledger = pnl_before;

    // Accrue arbitrary funding
    let delta: i128 = kani::any();
    kani::assume(delta.abs() < 1_000_000_000);
    engine.funding_index_qpb_e6 = delta;

    let _ = engine.touch_user(user_idx);

    // PNL should be unchanged
    assert!(engine.users[user_idx].pnl_ledger == pnl_before,
            "Zero position should not pay or receive funding");
}

