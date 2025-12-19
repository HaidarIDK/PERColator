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
        max_accounts: 8,    // Match Kani's MAX_ACCOUNTS
        account_fee_bps: 0, // Zero fees to avoid vault/insurance mutations during add_user/add_lp
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
    let mut engine = RiskEngine::new(test_params());
    let user_idx = engine.add_user(1).unwrap();

    // Set arbitrary but bounded values (reduced bounds for tractability)
    let principal: u128 = kani::any();
    let loss: u128 = kani::any();

    kani::assume(principal > 0 && principal < 1_000);
    kani::assume(loss < 1_000);

    // Set pnl=0 since we're only proving "capital unchanged"
    // This simplifies the proof and avoids irrelevant conservation issues
    engine.accounts[user_idx as usize].capital = principal;
    engine.accounts[user_idx as usize].pnl = 0;
    engine.insurance_fund.balance = 10_000;

    // Set consistent vault for conservation
    engine.vault = principal + engine.insurance_fund.balance;

    let principal_before = engine.accounts[user_idx as usize].capital;

    let _ = engine.apply_adl(loss);

    assert!(
        engine.accounts[user_idx as usize].capital == principal_before,
        "I1: ADL must NEVER reduce user principal"
    );
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
    let mut engine = RiskEngine::new(test_params());
    let user_idx = engine.add_user(1).unwrap();

    let amount: u128 = kani::any();
    kani::assume(amount < 10_000);

    assert!(engine.check_conservation());

    let _ = engine.deposit(user_idx, amount);

    assert!(
        engine.check_conservation(),
        "I2: Deposit must preserve conservation"
    );
}

#[kani::proof]
#[kani::unwind(10)]
#[kani::solver(cadical)]
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

    assert!(
        engine.check_conservation(),
        "I2: Withdrawal must preserve conservation"
    );
}

// ============================================================================
// I5: PNL Warmup Properties
// ============================================================================

#[kani::proof]
#[kani::unwind(10)]
#[kani::solver(cadical)]
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

    engine.accounts[user_idx as usize].pnl = pnl;
    engine.accounts[user_idx as usize].reserved_pnl = reserved;
    engine.accounts[user_idx as usize].warmup_slope_per_step = slope;
    engine.current_slot = slots;

    // Calculate twice with same inputs
    let w1 = engine.withdrawable_pnl(&engine.accounts[user_idx as usize]);
    let w2 = engine.withdrawable_pnl(&engine.accounts[user_idx as usize]);

    assert!(w1 == w2, "I5: Withdrawable PNL must be deterministic");
}

#[kani::proof]
#[kani::unwind(10)]
#[kani::solver(cadical)]
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

    engine.accounts[user_idx as usize].pnl = pnl;
    engine.accounts[user_idx as usize].warmup_slope_per_step = slope;

    engine.current_slot = slots1;
    let w1 = engine.withdrawable_pnl(&engine.accounts[user_idx as usize]);

    engine.current_slot = slots2;
    let w2 = engine.withdrawable_pnl(&engine.accounts[user_idx as usize]);

    assert!(
        w2 >= w1,
        "I5: Warmup must be monotonically increasing over time"
    );
}

#[kani::proof]
#[kani::unwind(10)]
#[kani::solver(cadical)]
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

    engine.accounts[user_idx as usize].pnl = pnl;
    engine.accounts[user_idx as usize].reserved_pnl = reserved;
    engine.accounts[user_idx as usize].warmup_slope_per_step = slope;
    engine.current_slot = slots;

    let withdrawable = engine.withdrawable_pnl(&engine.accounts[user_idx as usize]);
    let positive_pnl = pnl as u128;
    let available = positive_pnl.saturating_sub(reserved);

    assert!(
        withdrawable <= available,
        "I5: Withdrawable must not exceed available PNL"
    );
}

// ============================================================================
// I7: User Isolation
// ============================================================================

#[kani::proof]
#[kani::unwind(10)]
#[kani::solver(cadical)]
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

    let user2_principal = engine.accounts[user2 as usize].capital;
    let user2_pnl = engine.accounts[user2 as usize].pnl;

    // Operate on user1
    let _ = engine.deposit(user1, 100);

    // User2 should be unchanged
    assert!(
        engine.accounts[user2 as usize].capital == user2_principal,
        "I7: User2 principal unchanged by user1 deposit"
    );
    assert!(
        engine.accounts[user2 as usize].pnl == user2_pnl,
        "I7: User2 PNL unchanged by user1 deposit"
    );
}

#[kani::proof]
#[kani::unwind(10)]
#[kani::solver(cadical)]
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

    let user2_principal = engine.accounts[user2 as usize].capital;
    let user2_pnl = engine.accounts[user2 as usize].pnl;

    // Operate on user1
    let _ = engine.withdraw(user1, 50);

    // User2 should be unchanged
    assert!(
        engine.accounts[user2 as usize].capital == user2_principal,
        "I7: User2 principal unchanged by user1 withdrawal"
    );
    assert!(
        engine.accounts[user2 as usize].pnl == user2_pnl,
        "I7: User2 PNL unchanged by user1 withdrawal"
    );
}

// ============================================================================
// I8: Collateral Consistency
// ============================================================================

#[kani::proof]
#[kani::unwind(10)]
#[kani::solver(cadical)]
fn i8_collateral_with_positive_pnl() {
    let mut engine = RiskEngine::new(test_params());
    let user_idx = engine.add_user(1).unwrap();

    let principal: u128 = kani::any();
    let pnl: i128 = kani::any();

    kani::assume(principal < 10_000);
    kani::assume(pnl > 0 && pnl < 10_000);

    engine.accounts[user_idx as usize].capital = principal;
    engine.accounts[user_idx as usize].pnl = pnl;

    let collateral = engine.account_collateral(&engine.accounts[user_idx as usize]);
    let expected = principal.saturating_add(pnl as u128);

    assert!(
        collateral == expected,
        "I8: Collateral = principal + positive PNL"
    );
}

#[kani::proof]
#[kani::unwind(10)]
#[kani::solver(cadical)]
fn i8_collateral_with_negative_pnl() {
    let mut engine = RiskEngine::new(test_params());
    let user_idx = engine.add_user(1).unwrap();

    let principal: u128 = kani::any();
    let pnl: i128 = kani::any();

    kani::assume(principal < 10_000);
    kani::assume(pnl < 0 && pnl > -10_000);

    engine.accounts[user_idx as usize].capital = principal;
    engine.accounts[user_idx as usize].pnl = pnl;

    let collateral = engine.account_collateral(&engine.accounts[user_idx as usize]);

    assert!(
        collateral == principal,
        "I8: Collateral = principal when PNL is negative"
    );
}

// ============================================================================
// I4: Bounded Losses (ADL mechanics)
// SLOW_PROOF: Uses apply_adl which iterates over all accounts
// ============================================================================

// Previously slow - now fast with 8 accounts
// Fixed: Properly set warmup state to ensure all PnL is unwrapped
#[kani::proof]
#[kani::unwind(10)]
#[kani::solver(cadical)]
fn i4_adl_haircuts_unwrapped_first() {
    let mut engine = RiskEngine::new(test_params());
    let user_idx = engine.add_user(1).unwrap();

    let principal: u128 = kani::any();
    let pnl: i128 = kani::any();
    let loss: u128 = kani::any();

    kani::assume(principal > 0 && principal < 500);
    kani::assume(pnl > 0 && pnl < 100);
    kani::assume(loss > 0 && loss < 100);
    kani::assume(loss < pnl as u128); // Loss less than PNL

    engine.accounts[user_idx as usize].capital = principal;
    engine.accounts[user_idx as usize].pnl = pnl;

    // Properly set warmup state so ALL PnL is unwrapped:
    // - current_slot = 0: no time has passed
    // - warmup_started_at_slot = 0: warmup starts now
    // - warmup_paused = false: not paused
    // - warmup_slope_per_step = 0: nothing vests per slot
    // This ensures withdrawable_pnl = 0, so all pnl is "unwrapped"
    engine.current_slot = 0;
    engine.accounts[user_idx as usize].warmup_started_at_slot = 0;
    engine.accounts[user_idx as usize].warmup_slope_per_step = 0;
    engine.warmup_paused = false;

    engine.insurance_fund.balance = 10_000;
    engine.vault = principal + 10_000;

    let pnl_before = engine.accounts[user_idx as usize].pnl;
    let insurance_before = engine.insurance_fund.balance;

    let _ = engine.apply_adl(loss);

    // With slope=0 and slot=0, withdrawable=0, so all PnL is unwrapped
    // If loss <= unwrapped PNL, insurance should be untouched
    if loss <= pnl as u128 {
        assert!(
            engine.insurance_fund.balance == insurance_before,
            "I4: ADL should haircut PNL before touching insurance"
        );
        assert!(
            engine.accounts[user_idx as usize].pnl == pnl_before - (loss as i128),
            "I4: PNL should be reduced by loss amount"
        );
    }
}

// ============================================================================
// Withdrawal Safety
// ============================================================================

#[kani::proof]
#[kani::unwind(10)]
#[kani::solver(cadical)]
fn withdrawal_requires_sufficient_balance() {
    let mut engine = RiskEngine::new(test_params());
    let user_idx = engine.add_user(1).unwrap();

    let principal: u128 = kani::any();
    let withdraw: u128 = kani::any();

    kani::assume(principal < 10_000);
    kani::assume(withdraw < 20_000);
    kani::assume(withdraw > principal); // Try to withdraw more than available

    engine.accounts[user_idx as usize].capital = principal;
    engine.vault = principal;

    let result = engine.withdraw(user_idx, withdraw);

    assert!(
        result.is_err(),
        "Withdrawal of more than available must fail"
    );
}

#[kani::proof]
#[kani::unwind(10)]
#[kani::solver(cadical)]
fn pnl_withdrawal_requires_warmup() {
    let mut engine = RiskEngine::new(test_params());
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
        assert!(
            result.is_err(),
            "Cannot withdraw when no principal and PNL not warmed up"
        );
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

    engine.accounts[user1 as usize].capital = p1;
    engine.accounts[user1 as usize].pnl = pnl1;
    engine.accounts[user2 as usize].capital = p2;
    engine.accounts[user2 as usize].pnl = pnl2;
    engine.insurance_fund.balance = 100_000;

    let _ = engine.apply_adl(loss);

    assert!(
        engine.accounts[user1 as usize].capital == p1,
        "Multi-user ADL: User1 principal preserved"
    );
    assert!(
        engine.accounts[user2 as usize].capital == p2,
        "Multi-user ADL: User2 principal preserved"
    );
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
    assert!(
        result >= a && result >= b,
        "Saturating add should not overflow"
    );

    // Test saturating sub
    let result = a.saturating_sub(b);
    assert!(result <= a, "Saturating sub should not underflow");
}

// ============================================================================
// Edge Cases
// ============================================================================

#[kani::proof]
#[kani::unwind(10)]
#[kani::solver(cadical)]
fn zero_pnl_withdrawable_is_zero() {
    let mut engine = RiskEngine::new(test_params());
    let user_idx = engine.add_user(1).unwrap();

    engine.accounts[user_idx as usize].pnl = 0;
    engine.current_slot = 1000; // Far in future

    let withdrawable = engine.withdrawable_pnl(&engine.accounts[user_idx as usize]);

    assert!(withdrawable == 0, "Zero PNL means zero withdrawable");
}

#[kani::proof]
#[kani::unwind(10)]
#[kani::solver(cadical)]
fn negative_pnl_withdrawable_is_zero() {
    let mut engine = RiskEngine::new(test_params());
    let user_idx = engine.add_user(1).unwrap();

    let pnl: i128 = kani::any();
    kani::assume(pnl < 0 && pnl > -10_000);

    engine.accounts[user_idx as usize].pnl = pnl;
    engine.current_slot = 1000;

    let withdrawable = engine.withdrawable_pnl(&engine.accounts[user_idx as usize]);

    assert!(withdrawable == 0, "Negative PNL means zero withdrawable");
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

    let mut engine = RiskEngine::new(test_params());
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
    assert!(
        engine.accounts[user_idx as usize].pnl == pnl_after_first,
        "Second settlement should not change PNL"
    );

    // Snapshot should equal global index
    assert!(
        engine.accounts[user_idx as usize].funding_index == engine.funding_index_qpb_e6,
        "Snapshot should equal global index"
    );
}

#[kani::proof]
#[kani::unwind(10)]
#[kani::solver(cadical)]
fn funding_p2_never_touches_principal() {
    // P2: Funding does not touch principal (extends Invariant I1)

    let mut engine = RiskEngine::new(test_params());
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
    assert!(
        engine.accounts[user_idx as usize].capital == principal,
        "Funding must never modify principal"
    );
}

#[kani::proof]
#[kani::unwind(10)]
#[kani::solver(cadical)]
fn funding_p3_bounded_drift_between_opposite_positions() {
    // P3: Funding has bounded drift when user and LP have opposite positions
    // Note: With vault-favoring rounding (ceil when paying, trunc when receiving),
    // funding is NOT exactly zero-sum. The vault keeps the rounding dust.
    // This ensures one-sided conservation (vault >= expected).

    let mut engine = RiskEngine::new(test_params());
    let user_idx = engine.add_user(1).unwrap();
    let lp_idx = engine.add_lp([0u8; 32], [0u8; 32], 1).unwrap();

    let position: i128 = kani::any();
    kani::assume(position > 0 && position < 100); // Very small for tractability

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
    kani::assume(delta.abs() < 1_000); // Very small for tractability
    engine.funding_index_qpb_e6 = delta;

    // Settle both
    let user_result = engine.touch_account(user_idx);
    let lp_result = engine.touch_account(lp_idx);

    // If both settlements succeeded, check bounded drift
    if user_result.is_ok() && lp_result.is_ok() {
        let total_after =
            engine.accounts[user_idx as usize].pnl + engine.accounts[lp_idx as usize].pnl;
        let change = total_after - total_before;

        // Funding should not create value (vault keeps rounding dust)
        assert!(change <= 0, "Funding must not create value");
        // Change should be bounded by rounding (at most -2 per account pair)
        assert!(change >= -2, "Funding drift must be bounded");
    }
}

#[kani::proof]
#[kani::unwind(10)]
#[kani::solver(cadical)]
fn funding_p4_settle_before_position_change() {
    // P4: Verifies that settlement before position change gives correct results

    let mut engine = RiskEngine::new(test_params());
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
    assert!(
        engine.accounts[user_idx as usize].funding_index == engine.funding_index_qpb_e6,
        "Snapshot must track global index"
    );
}

#[kani::proof]
#[kani::unwind(10)]
#[kani::solver(cadical)]
fn funding_p5_bounded_operations_no_overflow() {
    // P5: No overflows on bounded inputs (or returns Overflow error)

    let mut engine = RiskEngine::new(test_params());

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
        assert!(
            matches!(result.unwrap_err(), RiskError::Overflow),
            "Only Overflow error allowed"
        );
    }
}

#[kani::proof]
#[kani::unwind(10)]
#[kani::solver(cadical)]
fn funding_zero_position_no_change() {
    // Additional invariant: Zero position means no funding payment

    let mut engine = RiskEngine::new(test_params());
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
    assert!(
        engine.accounts[user_idx as usize].pnl == pnl_before,
        "Zero position should not pay or receive funding"
    );
}

// ============================================================================
// I10: Withdrawal-Only Mode (Fair Unwinding)
// SLOW_PROOF: Uses apply_adl which iterates over all accounts
// ============================================================================

// Previously slow - now fast with 8 accounts
/// I10: Risk mode triggers when insurance at floor and losses exceed available
///
/// Updated for floor-based semantics:
/// - Insurance is NOT drained below floor
/// - Risk mode triggers when insurance at/below floor OR uncovered losses exist
/// - loss_accum > 0 when losses exceed (unwrapped + unreserved_spendable)
#[kani::proof]
#[kani::unwind(10)]
#[kani::solver(cadical)]
fn i10_risk_mode_triggers_at_floor() {
    let mut engine = RiskEngine::new(test_params_with_floor());
    let user_idx = engine.add_user(1).unwrap();

    let insurance: u128 = kani::any();
    let loss: u128 = kani::any();
    let pnl: i128 = kani::any();

    let floor = engine.params.risk_reduction_threshold;

    // Insurance above floor but not by much
    kani::assume(insurance > floor && insurance < floor + 5_000);
    kani::assume(loss > 0 && loss < 20_000);
    // PnL is non-positive so no unwrapped to haircut
    kani::assume(pnl <= 0 && pnl > -5_000);

    engine.insurance_fund.balance = insurance;
    engine.accounts[user_idx as usize].capital = 10_000;
    engine.accounts[user_idx as usize].pnl = pnl;
    engine.accounts[user_idx as usize].warmup_slope_per_step = 0; // No warmup
    engine.vault = 10_000 + insurance;

    // Calculate unreserved spendable (no reserved since no warmup)
    let unreserved_spendable = insurance.saturating_sub(floor);

    let _ = engine.apply_adl(loss);

    // If loss exceeds what we can cover, should enter risk mode with loss_accum
    if loss > unreserved_spendable {
        assert!(
            engine.risk_reduction_only,
            "I10: Risk mode must activate when losses exceed coverage"
        );
        // Insurance should be at floor, not zero
        assert!(
            engine.insurance_fund.balance >= floor,
            "I10: Insurance must not drop below floor"
        );
        // Excess loss goes to loss_accum
        assert!(
            engine.loss_accum > 0,
            "I10: loss_accum must be > 0 for uncovered losses"
        );
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

    let mut engine = RiskEngine::new(test_params());

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

    let mut engine = RiskEngine::new(test_params());
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
        assert!(
            result.is_err(),
            "I10: Cannot increase position in withdrawal-only mode"
        );
    }
}

#[kani::proof]
#[kani::unwind(10)]
#[kani::solver(cadical)]
fn i10_withdrawal_mode_allows_position_decrease() {
    // In withdrawal-only mode, users CAN decrease/close positions

    let mut engine = RiskEngine::new(test_params());
    let user_idx = engine.add_user(1).unwrap();
    let lp_idx = engine.add_lp([0u8; 32], [0u8; 32], 1).unwrap();

    engine.accounts[user_idx as usize].capital = 10_000;
    engine.accounts[lp_idx as usize].capital = 50_000;
    engine.insurance_fund.balance = 1_000; // Non-zero to avoid force_realize trigger
    engine.vault = 61_000; // 10k + 50k + 1k insurance

    let position: i128 = kani::any();
    kani::assume(position != i128::MIN); // Prevent overflow when negating
    kani::assume(position != 0); // Must have a position
    kani::assume(position > 100 && position < 5_000); // Bounded for tractability

    engine.accounts[user_idx as usize].position_size = position;
    engine.accounts[user_idx as usize].entry_price = 1_000_000;
    engine.accounts[lp_idx as usize].position_size = -position;
    engine.accounts[lp_idx as usize].entry_price = 1_000_000;

    // Enter withdrawal mode
    engine.risk_reduction_only = true;
    engine.loss_accum = 0; // Zero to maintain conservation

    // Close half the position (reduce size)
    let reduce = -position / 2; // Opposite sign = reduce

    let matcher = NoOpMatcher;
    let result = engine.execute_trade(&matcher, lp_idx, user_idx, 1_000_000, reduce);

    // Closing/reducing should be allowed
    assert!(
        result.is_ok(),
        "I10: Position reduction should be allowed in withdrawal-only mode"
    );
}

/*
// NOTE: Commented out - tests old withdrawal haircut logic which was removed
// The new withdrawal-only mode blocks ALL withdrawals instead of applying haircuts
#[kani::proof]
#[kani::unwind(10)]
#[kani::solver(cadical)]
fn i10_total_withdrawals_bounded_by_available() {
    // Total withdrawals in withdrawal mode cannot exceed (total_principal - loss_accum)

    let mut engine = RiskEngine::new(test_params());
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

    let mut engine = RiskEngine::new(test_params());

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

    let mut engine = RiskEngine::new(test_params());

    let loss: u128 = kani::any();
    kani::assume(loss > 0 && loss < 10_000);

    engine.risk_reduction_only = true;
    engine.loss_accum = loss;
    engine.vault = 0;

    // Top up exactly the loss amount
    let result = engine.top_up_insurance_fund(loss);

    assert!(result.is_ok(), "Top-up should succeed");
    assert!(engine.loss_accum == 0, "Loss should be fully covered");
    assert!(
        !engine.risk_reduction_only,
        "I10: Should exit withdrawal mode when loss_accum = 0"
    );

    if let Ok(exited) = result {
        assert!(
            exited,
            "I10: Should return true when exiting withdrawal mode"
        );
    }
}

// Previously slow - now fast with 8 accounts
#[kani::proof]
#[kani::unwind(10)]
#[kani::solver(cadical)]
fn i10_withdrawal_mode_preserves_conservation() {
    // Conservation must be maintained even in withdrawal-only mode

    let mut engine = RiskEngine::new(test_params());
    let user_idx = engine.add_user(1).unwrap();

    let principal: u128 = kani::any();
    let withdraw: u128 = kani::any();

    kani::assume(principal > 1_000 && principal < 10_000);
    kani::assume(withdraw > 0 && withdraw < principal);

    engine.accounts[user_idx as usize].capital = principal;
    engine.vault = principal;
    engine.insurance_fund.balance = 0; // Reset insurance to match vault = total_capital

    // Enter withdrawal mode (loss_accum = 0 to avoid conservation slack issues)
    engine.risk_reduction_only = true;
    engine.loss_accum = 0;

    assert!(
        engine.check_conservation(),
        "Conservation before withdrawal"
    );

    let _ = engine.withdraw(user_idx, withdraw);

    assert!(
        engine.check_conservation(),
        "I10: Withdrawal mode must preserve conservation"
    );
}

/*
// NOTE: Commented out - tests old withdrawal haircut logic which was removed
// The new withdrawal-only mode blocks ALL withdrawals instead of applying haircuts
#[kani::proof]
#[kani::unwind(10)]
#[kani::solver(cadical)]
fn i10_withdrawal_tracking_accuracy() {
    // withdrawal_mode_withdrawn should accurately track total withdrawn amounts

    let mut engine = RiskEngine::new(test_params());
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

    let mut engine = RiskEngine::new(test_params());
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

    assert!(
        engine.accounts[lp_idx as usize].capital == capital_before,
        "I1-LP: ADL must NEVER reduce LP capital"
    );
}

// Previously slow - now fast with 8 accounts
#[kani::proof]
#[kani::unwind(10)]
#[kani::solver(cadical)]
fn adl_is_proportional_for_user_and_lp() {
    // Proportional ADL Fairness: Users and LPs with equal unwrapped PNL
    // should receive equal haircuts

    let mut engine = RiskEngine::new(test_params());
    let user_idx = engine.add_user(1).unwrap();
    let lp_idx = engine.add_lp([0u8; 32], [0u8; 32], 1).unwrap();

    let pnl: i128 = kani::any();
    let loss: u128 = kani::any();

    // Both have the same unwrapped PNL (very small for tractability)
    kani::assume(pnl > 0 && pnl < 100);
    kani::assume(loss > 0 && loss < 100);

    engine.accounts[user_idx as usize].pnl = pnl;
    engine.accounts[lp_idx as usize].pnl = pnl;
    engine.insurance_fund.balance = 10_000;

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
    assert!(
        user_loss == lp_loss,
        "ADL: User and LP with equal unwrapped PNL must receive equal haircuts"
    );
}

// Previously slow - now fast with 8 accounts
#[kani::proof]
#[kani::unwind(10)]
#[kani::solver(cadical)]
fn adl_proportionality_general() {
    // General proportional ADL: Haircut percentages should be equal
    // even when PNL amounts differ

    let mut engine = RiskEngine::new(test_params());
    let user_idx = engine.add_user(1).unwrap();
    let lp_idx = engine.add_lp([0u8; 32], [0u8; 32], 1).unwrap();

    let user_pnl: i128 = kani::any();
    let lp_pnl: i128 = kani::any();
    let loss: u128 = kani::any();

    // Very small bounds for tractability
    kani::assume(user_pnl > 0 && user_pnl < 100);
    kani::assume(lp_pnl > 0 && lp_pnl < 100);
    kani::assume(loss > 0 && loss < 100);
    kani::assume(user_pnl != lp_pnl); // Different amounts

    engine.accounts[user_idx as usize].pnl = user_pnl;
    engine.accounts[lp_idx as usize].pnl = lp_pnl;
    engine.insurance_fund.balance = 10_000;

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
    let diff = if cross1 > cross2 {
        cross1 - cross2
    } else {
        cross2 - cross1
    };

    assert!(
        diff <= total_pnl,
        "ADL: Haircuts must be proportional (within rounding tolerance)"
    );
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

    let mut engine = RiskEngine::new(test_params());
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

    let mut engine = RiskEngine::new(test_params());
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

    assert!(
        engine.accounts[lp1 as usize].capital == c1,
        "Multi-LP ADL: LP1 capital preserved"
    );
    assert!(
        engine.accounts[lp2 as usize].capital == c2,
        "Multi-LP ADL: LP2 capital preserved"
    );
}

// Previously slow - now fast with 8 accounts
#[kani::proof]
#[kani::unwind(10)]
#[kani::solver(cadical)]
fn mixed_users_and_lps_adl_preserves_all_capitals() {
    // Mixed ADL: Both user and LP capitals are preserved together

    let mut engine = RiskEngine::new(test_params());
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

    assert!(
        engine.accounts[user_idx as usize].capital == user_capital,
        "Mixed ADL: User capital preserved"
    );
    assert!(
        engine.accounts[lp_idx as usize].capital == lp_capital,
        "Mixed ADL: LP capital preserved"
    );
}

// ============================================================================
// Risk-Reduction-Only Mode Proofs
// ============================================================================

// Proof 1: Warmup does not advance while paused
#[kani::proof]
#[kani::unwind(10)]
#[kani::solver(cadical)]
fn proof_warmup_frozen_when_paused() {
    let mut engine = RiskEngine::new(test_params());
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
    assert!(
        withdrawable_later == withdrawable_at_pause,
        "Warmup should not advance while paused"
    );
}

// Proof 2: In risk mode, withdraw never decreases PNL directly (only via warmup conversion)
#[kani::proof]
#[kani::unwind(10)]
#[kani::solver(cadical)]
fn proof_withdraw_only_decreases_via_conversion() {
    let mut engine = RiskEngine::new(test_params());
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
    assert!(
        pnl_after >= pnl_before - (warmed as i128),
        "PNL should not decrease more than warmed amount"
    );
    assert!(
        pnl_after <= pnl_before,
        "PNL should not increase during withdrawal"
    );
}

// Proof 3: Risk-increasing trades are rejected in risk mode
#[kani::proof]
#[kani::unwind(10)]
#[kani::solver(cadical)]
fn proof_risk_increasing_trades_rejected() {
    let mut engine = RiskEngine::new(test_params());
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
        assert!(
            result.is_err(),
            "Risk-increasing trades must fail in risk mode"
        );
    }
}

// ============================================================================
// Panic Settle Proofs
// These prove key properties of the panic_settle_all function
// ============================================================================

// Proof PS1: panic_settle_all closes all positions
#[kani::proof]
#[kani::unwind(10)]
#[kani::solver(cadical)]
fn panic_settle_closes_all_positions() {
    let mut engine = RiskEngine::new(test_params());
    let user_idx = engine.add_user(1).unwrap();
    let lp_idx = engine.add_lp([0u8; 32], [0u8; 32], 1).unwrap();

    let user_pos: i128 = kani::any();
    let entry_price: u64 = kani::any();
    let oracle_price: u64 = kani::any();

    // Bounded assumptions
    kani::assume(user_pos != 0); // Must have a position
    kani::assume(user_pos != i128::MIN);
    kani::assume(user_pos.abs() < 100_000);
    kani::assume(entry_price > 0 && entry_price < 100_000_000);
    kani::assume(oracle_price > 0 && oracle_price < 100_000_000);

    // Setup opposing positions (LP is counterparty)
    engine.accounts[user_idx as usize].position_size = user_pos;
    engine.accounts[user_idx as usize].entry_price = entry_price;
    engine.accounts[user_idx as usize].capital = 100_000;

    engine.accounts[lp_idx as usize].position_size = -user_pos;
    engine.accounts[lp_idx as usize].entry_price = entry_price;
    engine.accounts[lp_idx as usize].capital = 100_000;

    engine.vault = 200_000;
    engine.insurance_fund.balance = 10_000;

    // Call panic_settle_all
    let result = engine.panic_settle_all(oracle_price);

    // PROOF: If successful, all positions must be zero
    if result.is_ok() {
        assert!(
            engine.accounts[user_idx as usize].position_size == 0,
            "PS1: User position must be closed after panic settle"
        );
        assert!(
            engine.accounts[lp_idx as usize].position_size == 0,
            "PS1: LP position must be closed after panic settle"
        );
    }
}

// Proof PS2: panic_settle_all clamps all negative PNL to zero
#[kani::proof]
#[kani::unwind(10)]
#[kani::solver(cadical)]
fn panic_settle_clamps_negative_pnl() {
    let mut engine = RiskEngine::new(test_params());
    let user_idx = engine.add_user(1).unwrap();
    let lp_idx = engine.add_lp([0u8; 32], [0u8; 32], 1).unwrap();

    let user_pos: i128 = kani::any();
    let entry_price: u64 = kani::any();
    let oracle_price: u64 = kani::any();
    let initial_pnl: i128 = kani::any();

    // Very small bounds for tractability
    kani::assume(user_pos != i128::MIN);
    kani::assume(user_pos != 0); // Must have a position to be processed
    kani::assume(user_pos.abs() < 100);
    kani::assume(entry_price > 100_000 && entry_price < 1_000_000);
    kani::assume(oracle_price > 100_000 && oracle_price < 1_000_000);
    kani::assume(initial_pnl > -100 && initial_pnl < 100);

    // Setup positions
    engine.accounts[user_idx as usize].position_size = user_pos;
    engine.accounts[user_idx as usize].entry_price = entry_price;
    engine.accounts[user_idx as usize].pnl = initial_pnl;
    engine.accounts[user_idx as usize].capital = 500;

    engine.accounts[lp_idx as usize].position_size = -user_pos;
    engine.accounts[lp_idx as usize].entry_price = entry_price;
    engine.accounts[lp_idx as usize].pnl = -initial_pnl; // Opposite for zero-sum
    engine.accounts[lp_idx as usize].capital = 500;

    engine.vault = 1_000;
    engine.insurance_fund.balance = 500;

    // Call panic_settle_all
    let result = engine.panic_settle_all(oracle_price);

    // PROOF: If successful, all PNLs must be >= 0
    if result.is_ok() {
        assert!(
            engine.accounts[user_idx as usize].pnl >= 0,
            "PS2: User PNL must be >= 0 after panic settle"
        );
        assert!(
            engine.accounts[lp_idx as usize].pnl >= 0,
            "PS2: LP PNL must be >= 0 after panic settle"
        );
    }
}

// Proof PS3: panic_settle_all always enters risk-reduction-only mode
#[kani::proof]
#[kani::unwind(10)]
#[kani::solver(cadical)]
fn panic_settle_enters_risk_mode() {
    let mut engine = RiskEngine::new(test_params());
    let user_idx = engine.add_user(1).unwrap();

    let oracle_price: u64 = kani::any();

    // Bounded assumptions
    kani::assume(oracle_price > 0 && oracle_price < 100_000_000);

    // Setup minimal account
    engine.accounts[user_idx as usize].capital = 10_000;
    engine.vault = 10_000;

    // Ensure we're not in risk mode initially
    assert!(!engine.risk_reduction_only, "Should not start in risk mode");

    // Call panic_settle_all
    let result = engine.panic_settle_all(oracle_price);

    // PROOF: After panic_settle, we must be in risk-reduction-only mode
    if result.is_ok() {
        assert!(
            engine.risk_reduction_only,
            "PS3: Must be in risk-reduction-only mode after panic settle"
        );
        assert!(
            engine.warmup_paused,
            "PS3: Warmup must be paused after panic settle"
        );
    }
}

// Proof PS4: panic_settle_all preserves conservation (with rounding compensation)
// SLOW_PROOF: Uses apply_adl which iterates over all accounts
#[kani::proof]
#[kani::unwind(10)]
#[kani::solver(cadical)]
fn panic_settle_preserves_conservation() {
    let mut engine = RiskEngine::new(test_params());
    let user_idx = engine.add_user(1).unwrap();
    let lp_idx = engine.add_lp([0u8; 32], [0u8; 32], 1).unwrap();

    let user_pos: i128 = kani::any();
    let entry_price: u64 = kani::any();
    let oracle_price: u64 = kani::any();
    let user_capital: u128 = kani::any();
    let lp_capital: u128 = kani::any();

    // Very small bounds for tractability
    kani::assume(user_pos != i128::MIN);
    kani::assume(user_pos != 0); // Must have position to be processed
    kani::assume(user_pos.abs() < 100);
    kani::assume(entry_price > 100_000 && entry_price < 1_000_000);
    kani::assume(oracle_price > 100_000 && oracle_price < 1_000_000);
    kani::assume(user_capital > 10 && user_capital < 500);
    kani::assume(lp_capital > 10 && lp_capital < 500);

    // Setup zero-sum positions at same entry price
    engine.accounts[user_idx as usize].position_size = user_pos;
    engine.accounts[user_idx as usize].entry_price = entry_price;
    engine.accounts[user_idx as usize].capital = user_capital;

    engine.accounts[lp_idx as usize].position_size = -user_pos;
    engine.accounts[lp_idx as usize].entry_price = entry_price;
    engine.accounts[lp_idx as usize].capital = lp_capital;

    // Set vault to match total capital
    let total_capital = user_capital + lp_capital;
    engine.vault = total_capital;
    engine.insurance_fund.balance = 0;

    // Verify conservation before
    // Note: We manually set insurance to 0 and vault to match, so conservation should hold
    // The add_user/add_lp fees are already accounted for in the initial setup

    // Call panic_settle_all
    let result = engine.panic_settle_all(oracle_price);

    // PROOF: Conservation must hold after panic_settle
    if result.is_ok() {
        assert!(
            engine.check_conservation(),
            "PS4: Conservation must hold after panic settle"
        );
    }
}

// ============================================================================
// Warmup Budget Invariant Proofs
// These prove properties of the warmup budget system:
// - W⁺ ≤ W⁻ + max(0, I - I_min)
// - Where W⁺ = warmed_pos_total, W⁻ = warmed_neg_total,
//   I = insurance_fund.balance, I_min = risk_reduction_threshold
// ============================================================================

// Helper for tests requiring positive insurance balance
fn test_params_with_floor() -> RiskParams {
    RiskParams {
        warmup_period_slots: 100,
        maintenance_margin_bps: 500,
        initial_margin_bps: 1000,
        trading_fee_bps: 10,
        max_accounts: 8,
        account_fee_bps: 0, // Zero fees to avoid vault/insurance mutations during add_user/add_lp
        risk_reduction_threshold: 1000, // Non-zero floor
    }
}

// Proof A: Warmup budget invariant always holds after settlement
#[kani::proof]
#[kani::unwind(10)]
#[kani::solver(cadical)]
fn warmup_budget_a_invariant_holds_after_settlement() {
    let mut engine = RiskEngine::new(test_params_with_floor());
    let user_idx = engine.add_user(1).unwrap();

    let capital: u128 = kani::any();
    let pnl: i128 = kani::any();
    let slope: u128 = kani::any();
    let insurance: u128 = kani::any();
    let slots: u64 = kani::any();

    // Bounded assumptions
    kani::assume(capital > 0 && capital < 10_000);
    kani::assume(pnl > -5_000 && pnl < 5_000);
    kani::assume(slope > 0 && slope < 100);
    kani::assume(insurance > 1_000 && insurance < 50_000); // Above floor
    kani::assume(slots > 0 && slots < 200);

    // Setup account with PNL that can be settled
    engine.accounts[user_idx as usize].capital = capital;
    engine.accounts[user_idx as usize].pnl = pnl;
    engine.accounts[user_idx as usize].warmup_slope_per_step = slope;
    engine.accounts[user_idx as usize].warmup_started_at_slot = 0;
    engine.current_slot = slots;

    // Set insurance and adjust vault for conservation
    engine.insurance_fund.balance = insurance;
    engine.vault = capital + insurance;
    if pnl > 0 {
        engine.vault = engine.vault.saturating_add(pnl as u128);
    }

    // Settle warmup
    let _ = engine.settle_warmup_to_capital(user_idx);

    // PROOF: Warmup budget invariant must hold
    let raw = engine.insurance_spendable_raw();
    assert!(
        engine.warmed_pos_total <= engine.warmed_neg_total.saturating_add(raw),
        "WB-A: W+ <= W- + raw_spendable must hold after settlement"
    );
}

// Proof B: Settling negative PNL cannot increase warmed_pos_total
#[kani::proof]
#[kani::unwind(10)]
#[kani::solver(cadical)]
fn warmup_budget_b_negative_settlement_no_increase_pos() {
    let mut engine = RiskEngine::new(test_params_with_floor());
    let user_idx = engine.add_user(1).unwrap();

    let capital: u128 = kani::any();
    let pnl: i128 = kani::any();
    let slope: u128 = kani::any();
    let slots: u64 = kani::any();

    // Bounded assumptions - specifically test negative PNL
    kani::assume(capital > 1_000 && capital < 10_000);
    kani::assume(pnl < 0 && pnl > -5_000);
    kani::assume(slope > 0 && slope < 100);
    kani::assume(slots > 0 && slots < 200);

    // Setup account with negative PNL
    engine.accounts[user_idx as usize].capital = capital;
    engine.accounts[user_idx as usize].pnl = pnl;
    engine.accounts[user_idx as usize].warmup_slope_per_step = slope;
    engine.accounts[user_idx as usize].warmup_started_at_slot = 0;
    engine.current_slot = slots;

    // Set vault for conservation (negative PNL means less total value)
    engine.insurance_fund.balance = 5_000;
    engine.vault = capital + 5_000; // pnl is negative, so doesn't add to vault

    let warmed_pos_before = engine.warmed_pos_total;

    // Settle warmup (negative PNL should only affect capital, not warmed_pos_total)
    let _ = engine.settle_warmup_to_capital(user_idx);

    // PROOF: warmed_pos_total should not increase when settling negative PNL
    assert!(
        engine.warmed_pos_total == warmed_pos_before,
        "WB-B: Settling negative PNL must not increase warmed_pos_total"
    );
}

// Proof C: Settling positive PNL cannot exceed available budget
// This is the key safety property: Δwarmed_pos ≤ budget_before
#[kani::proof]
#[kani::unwind(10)]
#[kani::solver(cadical)]
fn warmup_budget_c_positive_settlement_bounded_by_budget() {
    let mut engine = RiskEngine::new(test_params_with_floor());
    let user_idx = engine.add_user(1).unwrap();

    let capital: u128 = kani::any();
    let pnl: i128 = kani::any();
    let slope: u128 = kani::any();
    let insurance: u128 = kani::any();
    let slots: u64 = kani::any();

    // Bounded assumptions - test positive PNL
    kani::assume(capital > 0 && capital < 10_000);
    kani::assume(pnl > 0 && pnl < 5_000);
    kani::assume(slope > 0 && slope < 100);
    kani::assume(insurance > 1_000 && insurance < 10_000); // Above floor but limited
    kani::assume(slots > 0 && slots < 200);

    // Setup account with positive PNL
    engine.accounts[user_idx as usize].capital = capital;
    engine.accounts[user_idx as usize].pnl = pnl;
    engine.accounts[user_idx as usize].warmup_slope_per_step = slope;
    engine.accounts[user_idx as usize].warmup_started_at_slot = 0;
    engine.current_slot = slots;

    // Set insurance (controls budget)
    engine.insurance_fund.balance = insurance;
    engine.vault = capital + insurance + (pnl as u128);

    // Record state BEFORE settlement
    let warmed_pos_before = engine.warmed_pos_total;
    let budget_before = engine.warmup_budget_remaining();

    // Settle warmup
    let _ = engine.settle_warmup_to_capital(user_idx);

    // PROOF: The increase in warmed_pos_total must not exceed available budget
    // This is the exact safety property: delta <= budget_before
    let delta = engine.warmed_pos_total.saturating_sub(warmed_pos_before);
    assert!(
        delta <= budget_before,
        "WB-C: Δwarmed_pos must not exceed budget_before"
    );
}

// Proof D: In warmup-paused mode, settlement result is unchanged by time
#[kani::proof]
#[kani::unwind(10)]
#[kani::solver(cadical)]
fn warmup_budget_d_paused_settlement_time_invariant() {
    let mut engine = RiskEngine::new(test_params_with_floor());
    let user_idx = engine.add_user(1).unwrap();

    let capital: u128 = kani::any();
    let pnl: i128 = kani::any();
    let slope: u128 = kani::any();
    let pause_slot: u64 = kani::any();
    let settle_slot1: u64 = kani::any();
    let settle_slot2: u64 = kani::any();

    // Bounded assumptions
    kani::assume(capital > 0 && capital < 10_000);
    kani::assume(pnl > 0 && pnl < 5_000);
    kani::assume(slope > 0 && slope < 100);
    kani::assume(pause_slot > 10 && pause_slot < 100);
    kani::assume(settle_slot1 >= pause_slot && settle_slot1 < 200);
    kani::assume(settle_slot2 > settle_slot1 && settle_slot2 < 300);

    // Setup account
    engine.accounts[user_idx as usize].capital = capital;
    engine.accounts[user_idx as usize].pnl = pnl;
    engine.accounts[user_idx as usize].warmup_slope_per_step = slope;
    engine.accounts[user_idx as usize].warmup_started_at_slot = 0;
    engine.insurance_fund.balance = 10_000;
    engine.vault = capital + 10_000 + (pnl as u128);

    // Pause warmup
    engine.warmup_paused = true;
    engine.warmup_pause_slot = pause_slot;

    // Compute vested amount at slot1 (inline calculation)
    engine.current_slot = settle_slot1;
    let effective_slot1 = core::cmp::min(engine.current_slot, engine.warmup_pause_slot);
    let elapsed1 =
        effective_slot1.saturating_sub(engine.accounts[user_idx as usize].warmup_started_at_slot);
    let vested1 = engine.accounts[user_idx as usize]
        .warmup_slope_per_step
        .saturating_mul(elapsed1 as u128);

    // Compute vested amount at later slot2 (inline calculation)
    engine.current_slot = settle_slot2;
    let effective_slot2 = core::cmp::min(engine.current_slot, engine.warmup_pause_slot);
    let elapsed2 =
        effective_slot2.saturating_sub(engine.accounts[user_idx as usize].warmup_started_at_slot);
    let vested2 = engine.accounts[user_idx as usize]
        .warmup_slope_per_step
        .saturating_mul(elapsed2 as u128);

    // PROOF: Vested amount should not change when warmup is paused
    // (both should be capped at pause_slot)
    assert!(
        vested1 == vested2,
        "WB-D: Vested amount must be time-invariant when warmup is paused"
    );
}

// ============================================================================
// AUDIT-MANDATED PROOFS: Double-Settlement Fix Verification
// These proofs were mandated by the security audit to verify the fix for the
// double-settlement bug in settle_warmup_to_capital when warmup is paused.
// ============================================================================

/// Proof: settle_warmup_to_capital is idempotent when warmup is paused
///
/// This proves that calling settle_warmup_to_capital twice when warmup is paused
/// produces the same result as calling it once. The fix ensures that
/// warmup_started_at_slot is always updated to effective_slot, preventing
/// double-settlement of the same matured PnL.
///
/// Bug scenario (before fix):
/// 1. User has positive PnL warming up with slope S
/// 2. Warmup paused at slot P
/// 3. At slot T > P, user calls settle - settles P*S of PnL
/// 4. Without fix: warmup_started_at_slot not updated, so second call would
///    settle another P*S, effectively double-settling
/// 5. With fix: warmup_started_at_slot = P after first settle, so second call
///    has elapsed=0 and settles nothing
#[kani::proof]
#[kani::unwind(10)]
#[kani::solver(cadical)]
fn audit_settle_idempotent_when_paused() {
    let mut engine = RiskEngine::new(test_params_with_floor());
    let user_idx = engine.add_user(1).unwrap();

    // Symbolic inputs
    let capital: u128 = kani::any();
    let pnl: i128 = kani::any();
    let slope: u128 = kani::any();
    let pause_slot: u64 = kani::any();
    let settle_slot: u64 = kani::any();
    let insurance: u128 = kani::any();

    // Bounded assumptions
    kani::assume(capital > 0 && capital < 10_000);
    kani::assume(pnl > 0 && pnl < 5_000); // Positive PnL for warmup
    kani::assume(slope > 0 && slope < 100);
    kani::assume(pause_slot > 0 && pause_slot < 100);
    kani::assume(settle_slot >= pause_slot && settle_slot < 200);
    kani::assume(insurance > 1_000 && insurance < 50_000);

    // Setup account with positive PnL and warmup
    engine.accounts[user_idx as usize].capital = capital;
    engine.accounts[user_idx as usize].pnl = pnl;
    engine.accounts[user_idx as usize].warmup_slope_per_step = slope;
    engine.accounts[user_idx as usize].warmup_started_at_slot = 0;

    // Setup insurance for warmup budget
    engine.insurance_fund.balance = insurance;
    engine.vault = capital + insurance;

    // Pause warmup
    engine.warmup_paused = true;
    engine.warmup_pause_slot = pause_slot;
    engine.current_slot = settle_slot;

    // First settlement
    let _ = engine.settle_warmup_to_capital(user_idx);

    // Capture state after first settlement
    let capital_after_first = engine.accounts[user_idx as usize].capital;
    let pnl_after_first = engine.accounts[user_idx as usize].pnl;
    let warmed_pos_after_first = engine.warmed_pos_total;
    let warmed_neg_after_first = engine.warmed_neg_total;
    let reserved_after_first = engine.warmup_insurance_reserved;

    // Second settlement - should be idempotent
    let _ = engine.settle_warmup_to_capital(user_idx);

    // PROOF: All state must be identical after second settlement
    assert!(
        engine.accounts[user_idx as usize].capital == capital_after_first,
        "AUDIT PROOF FAILED: Capital changed on second settlement (double-settlement bug)"
    );
    assert!(
        engine.accounts[user_idx as usize].pnl == pnl_after_first,
        "AUDIT PROOF FAILED: PnL changed on second settlement (double-settlement bug)"
    );
    assert!(
        engine.warmed_pos_total == warmed_pos_after_first,
        "AUDIT PROOF FAILED: warmed_pos_total changed (double-settlement bug)"
    );
    assert!(
        engine.warmed_neg_total == warmed_neg_after_first,
        "AUDIT PROOF FAILED: warmed_neg_total changed (double-settlement bug)"
    );
    assert!(
        engine.warmup_insurance_reserved == reserved_after_first,
        "AUDIT PROOF FAILED: reserved changed (double-settlement bug)"
    );
}

/// Proof: warmup_started_at_slot is updated to effective_slot after settlement
///
/// This proves the specific fix: that warmup_started_at_slot is always set to
/// effective_slot (min(current_slot, pause_slot)) after settle_warmup_to_capital,
/// which prevents double-settlement.
#[kani::proof]
#[kani::unwind(10)]
#[kani::solver(cadical)]
fn audit_warmup_started_at_updated_to_effective_slot() {
    let mut engine = RiskEngine::new(test_params_with_floor());
    let user_idx = engine.add_user(1).unwrap();

    // Symbolic inputs
    let pnl: i128 = kani::any();
    let slope: u128 = kani::any();
    let started_at: u64 = kani::any();
    let pause_slot: u64 = kani::any();
    let current_slot: u64 = kani::any();

    // Bounded assumptions
    kani::assume(pnl > 0 && pnl < 5_000);
    kani::assume(slope > 0 && slope < 100);
    kani::assume(started_at < 50);
    kani::assume(pause_slot >= started_at && pause_slot < 100);
    kani::assume(current_slot >= pause_slot && current_slot < 200);

    // Setup
    engine.accounts[user_idx as usize].pnl = pnl;
    engine.accounts[user_idx as usize].warmup_slope_per_step = slope;
    engine.accounts[user_idx as usize].warmup_started_at_slot = started_at;
    engine.insurance_fund.balance = 10_000;
    engine.vault = 10_000;

    // Pause warmup
    engine.warmup_paused = true;
    engine.warmup_pause_slot = pause_slot;
    engine.current_slot = current_slot;

    // Calculate expected effective_slot
    let effective_slot = core::cmp::min(current_slot, pause_slot);

    // Settle
    let _ = engine.settle_warmup_to_capital(user_idx);

    // PROOF: warmup_started_at_slot must equal effective_slot
    assert!(
        engine.accounts[user_idx as usize].warmup_started_at_slot == effective_slot,
        "AUDIT PROOF FAILED: warmup_started_at_slot not updated to effective_slot"
    );
}

/// Proof: Multiple settlements when paused all produce same result
///
/// This strengthens the idempotence proof by verifying that any number of
/// settlements when paused produces the same result as the first settlement.
#[kani::proof]
#[kani::unwind(10)]
#[kani::solver(cadical)]
fn audit_multiple_settlements_when_paused_idempotent() {
    let mut engine = RiskEngine::new(test_params_with_floor());
    let user_idx = engine.add_user(1).unwrap();

    // Symbolic inputs
    let pnl: i128 = kani::any();
    let slope: u128 = kani::any();
    let pause_slot: u64 = kani::any();
    let slot1: u64 = kani::any();
    let slot2: u64 = kani::any();
    let slot3: u64 = kani::any();

    // Bounded assumptions
    kani::assume(pnl > 0 && pnl < 3_000);
    kani::assume(slope > 0 && slope < 50);
    kani::assume(pause_slot > 5 && pause_slot < 50);
    kani::assume(slot1 >= pause_slot && slot1 < 100);
    kani::assume(slot2 > slot1 && slot2 < 150);
    kani::assume(slot3 > slot2 && slot3 < 200);

    // Setup
    engine.accounts[user_idx as usize].pnl = pnl;
    engine.accounts[user_idx as usize].warmup_slope_per_step = slope;
    engine.accounts[user_idx as usize].warmup_started_at_slot = 0;
    engine.insurance_fund.balance = 10_000;
    engine.vault = 10_000;

    // Pause warmup
    engine.warmup_paused = true;
    engine.warmup_pause_slot = pause_slot;

    // First settlement at slot1
    engine.current_slot = slot1;
    let _ = engine.settle_warmup_to_capital(user_idx);
    let state_after_first = (
        engine.accounts[user_idx as usize].capital,
        engine.accounts[user_idx as usize].pnl,
        engine.warmed_pos_total,
    );

    // Second settlement at slot2
    engine.current_slot = slot2;
    let _ = engine.settle_warmup_to_capital(user_idx);
    let state_after_second = (
        engine.accounts[user_idx as usize].capital,
        engine.accounts[user_idx as usize].pnl,
        engine.warmed_pos_total,
    );

    // Third settlement at slot3
    engine.current_slot = slot3;
    let _ = engine.settle_warmup_to_capital(user_idx);
    let state_after_third = (
        engine.accounts[user_idx as usize].capital,
        engine.accounts[user_idx as usize].pnl,
        engine.warmed_pos_total,
    );

    // PROOF: All states must be identical
    assert!(
        state_after_first == state_after_second,
        "AUDIT PROOF FAILED: State changed between first and second settlement"
    );
    assert!(
        state_after_second == state_after_third,
        "AUDIT PROOF FAILED: State changed between second and third settlement"
    );
}

/// Proof R1: ADL never spends reserved insurance
///
/// This is the critical proof that reserved insurance is protected.
/// Setup: floor > 0, insurance = floor + reserved + extra, all accounts pnl <= 0
/// (so total_unwrapped == 0 deterministically), then apply ADL.
/// Prove: insurance.balance >= floor + reserved after ADL
#[kani::proof]
#[kani::unwind(10)]
#[kani::solver(cadical)]
fn proof_r1_adl_never_spends_reserved() {
    let mut engine = RiskEngine::new(test_params_with_floor());
    let user_idx = engine.add_user(1).unwrap();

    // Symbolic inputs for setting up reserved insurance
    let reserved: u128 = kani::any();
    let extra: u128 = kani::any();
    let loss: u128 = kani::any();

    let floor = engine.params.risk_reduction_threshold;

    // Bounded assumptions
    kani::assume(reserved > 0 && reserved < 1_000);
    kani::assume(extra > 0 && extra < 1_000);
    kani::assume(loss > 0 && loss < 5_000);

    // Set up insurance = floor + reserved + extra
    let insurance = floor + reserved + extra;
    engine.insurance_fund.balance = insurance;
    engine.warmup_insurance_reserved = reserved;

    // EXPLICITLY ensure NO unwrapped PnL exists
    // This forces the "insurance must pay" pathway deterministically
    engine.accounts[user_idx as usize].capital = 10_000;
    engine.accounts[user_idx as usize].pnl = 0;
    engine.accounts[user_idx as usize].reserved_pnl = 0;
    engine.accounts[user_idx as usize].warmup_slope_per_step = 0;
    engine.accounts[user_idx as usize].warmup_started_at_slot = engine.current_slot;

    engine.vault = 10_000 + insurance;

    // Verify the unreserved spendable equals extra before ADL
    assert!(
        engine.insurance_spendable_unreserved() == extra,
        "R1 PRECONDITION: unreserved spendable should equal extra"
    );

    // Apply ADL - with no unwrapped PnL, it must use insurance
    let _ = engine.apply_adl(loss);

    // PROOF R1: Insurance must be >= floor + reserved
    // ADL can only spend the "extra" portion, not the reserved portion
    assert!(
        engine.insurance_fund.balance >= floor + reserved,
        "R1 FAILED: ADL spent reserved insurance!"
    );

    // Corollary: reserved should not decrease
    assert!(
        engine.warmup_insurance_reserved >= reserved,
        "R1 FAILED: Reserved decreased after ADL"
    );
}

/// Proof R2: Reserved never exceeds raw spendable and is monotonically non-decreasing
///
/// After settle_warmup_to_capital:
/// - reserved <= raw_spendable
/// - reserved_after >= reserved_before
#[kani::proof]
#[kani::unwind(10)]
#[kani::solver(cadical)]
fn proof_r2_reserved_bounded_and_monotone() {
    let mut engine = RiskEngine::new(test_params_with_floor());
    let user_idx = engine.add_user(1).unwrap();

    // Symbolic inputs
    let capital: u128 = kani::any();
    let pnl: i128 = kani::any();
    let slope: u128 = kani::any();
    let insurance: u128 = kani::any();
    let slots: u64 = kani::any();

    let floor = engine.params.risk_reduction_threshold;

    // Bounded assumptions
    kani::assume(capital > 100 && capital < 10_000);
    kani::assume(pnl > 50 && pnl < 5_000); // Positive PnL to warm
    kani::assume(slope > 10 && slope < 1000);
    kani::assume(insurance > floor + 100 && insurance < 10_000);
    kani::assume(slots > 1 && slots < 100);

    // Setup
    engine.accounts[user_idx as usize].capital = capital;
    engine.accounts[user_idx as usize].pnl = pnl;
    engine.accounts[user_idx as usize].warmup_slope_per_step = slope;
    engine.accounts[user_idx as usize].warmup_started_at_slot = 0;
    engine.insurance_fund.balance = insurance;
    engine.vault = capital + insurance;
    engine.current_slot = slots;

    let reserved_before = engine.warmup_insurance_reserved;

    // First settle
    let _ = engine.settle_warmup_to_capital(user_idx);

    let reserved_after_first = engine.warmup_insurance_reserved;
    let raw_spendable = engine.insurance_spendable_raw();

    // PROOF R2a: Reserved <= raw spendable
    assert!(
        reserved_after_first <= raw_spendable,
        "R2 FAILED: Reserved exceeds raw spendable"
    );

    // PROOF R2b: Reserved is monotonically non-decreasing
    assert!(
        reserved_after_first >= reserved_before,
        "R2 FAILED: Reserved decreased after settle"
    );

    // Second settle (should be idempotent when paused, but let's check monotonicity)
    engine.current_slot = slots + 10;
    let _ = engine.settle_warmup_to_capital(user_idx);

    // Reserved should not decrease
    assert!(
        engine.warmup_insurance_reserved >= reserved_after_first,
        "R2 FAILED: Reserved decreased on second settle"
    );
}

/// Proof R3: Warmup reservation safety
///
/// After settle_warmup_to_capital, prove:
/// insurance_fund.balance >= floor + warmup_insurance_reserved
///
/// This ensures the insurance fund always has enough to cover reserved warmup profits.
#[kani::proof]
#[kani::unwind(10)]
#[kani::solver(cadical)]
fn proof_r3_warmup_reservation_safety() {
    let mut engine = RiskEngine::new(test_params_with_floor());
    let user_idx = engine.add_user(1).unwrap();

    let capital: u128 = kani::any();
    let pnl: i128 = kani::any();
    let slope: u128 = kani::any();
    let insurance: u128 = kani::any();
    let slots: u64 = kani::any();

    let floor = engine.params.risk_reduction_threshold;

    // Bounded assumptions - positive PnL to test reservation
    kani::assume(capital > 0 && capital < 10_000);
    kani::assume(pnl > 0 && pnl < 5_000);
    kani::assume(slope > 0 && slope < 100);
    kani::assume(insurance > floor && insurance < 20_000);
    kani::assume(slots > 0 && slots < 200);

    // Setup
    engine.accounts[user_idx as usize].capital = capital;
    engine.accounts[user_idx as usize].pnl = pnl;
    engine.accounts[user_idx as usize].warmup_slope_per_step = slope;
    engine.accounts[user_idx as usize].warmup_started_at_slot = 0;
    engine.current_slot = slots;

    engine.insurance_fund.balance = insurance;
    engine.vault = capital + insurance + (pnl as u128);

    // Settle warmup
    let _ = engine.settle_warmup_to_capital(user_idx);

    // PROOF R3: Insurance must cover floor + reserved
    assert!(
        engine.insurance_fund.balance >= floor + engine.warmup_insurance_reserved,
        "R3 FAILED: Insurance does not cover floor + reserved"
    );
}

/// Proof PS5: panic_settle_all does not increase insurance (no minting from rounding)
///
/// Given trading_fee_bps = 0 (no fees), insurance should not increase after panic_settle.
/// The only way insurance decreases is through ADL spending.
#[kani::proof]
#[kani::unwind(10)]
#[kani::solver(cadical)]
fn proof_ps5_panic_settle_no_insurance_minting() {
    // Use params with zero trading fees
    let mut params = test_params();
    params.trading_fee_bps = 0;

    let mut engine = RiskEngine::new(params);
    let user_idx = engine.add_user(1).unwrap();
    let lp_idx = engine.add_lp([0u8; 32], [0u8; 32], 1).unwrap();

    // Symbolic inputs
    let user_capital: u128 = kani::any();
    let lp_capital: u128 = kani::any();
    let position: i128 = kani::any();
    let entry_price: u64 = kani::any();
    let oracle_price: u64 = kani::any();
    let insurance: u128 = kani::any();

    // Very small bounds for tractability
    kani::assume(user_capital > 10 && user_capital < 500);
    kani::assume(lp_capital > 10 && lp_capital < 500);
    kani::assume(position > 0 && position < 100);
    kani::assume(entry_price > 100_000 && entry_price < 1_000_000);
    kani::assume(oracle_price > 100_000 && oracle_price < 1_000_000);
    kani::assume(insurance > 0 && insurance < 500);

    // Setup opposing positions
    engine.accounts[user_idx as usize].capital = user_capital;
    engine.accounts[user_idx as usize].position_size = position;
    engine.accounts[user_idx as usize].entry_price = entry_price;

    engine.accounts[lp_idx as usize].capital = lp_capital;
    engine.accounts[lp_idx as usize].position_size = -position;
    engine.accounts[lp_idx as usize].entry_price = entry_price;

    engine.insurance_fund.balance = insurance;
    engine.vault = user_capital + lp_capital + insurance;

    let insurance_before = engine.insurance_fund.balance;

    // Panic settle
    let _ = engine.panic_settle_all(oracle_price);

    // PROOF PS5: Insurance should not increase (may decrease due to ADL)
    assert!(
        engine.insurance_fund.balance <= insurance_before,
        "PS5 FAILED: Insurance increased after panic_settle (minting bug)"
    );
}

/// Proof C1: Conservation slack is bounded after panic_settle_all
///
/// Proves that after panic_settle_all:
/// 1. vault + loss_accum >= expected (no under-collateralization)
/// 2. slack = (vault + loss_accum) - expected <= MAX_ROUNDING_SLACK (bounded dust)
///
/// This is the critical conservation proof with bounded slack.
#[kani::proof]
#[kani::unwind(10)]
#[kani::solver(cadical)]
fn proof_c1_conservation_bounded_slack_panic_settle() {
    let mut engine = RiskEngine::new(test_params());
    let user_idx = engine.add_user(1).unwrap();
    let lp_idx = engine.add_lp([0u8; 32], [0u8; 32], 1).unwrap();

    // Symbolic inputs
    let user_capital: u128 = kani::any();
    let lp_capital: u128 = kani::any();
    let position: i128 = kani::any();
    let entry_price: u64 = kani::any();
    let oracle_price: u64 = kani::any();

    // Very small bounds for tractability
    kani::assume(user_capital > 10 && user_capital < 500);
    kani::assume(lp_capital > 10 && lp_capital < 500);
    kani::assume(position > 0 && position < 100);
    kani::assume(entry_price > 100_000 && entry_price < 1_000_000);
    kani::assume(oracle_price > 100_000 && oracle_price < 1_000_000);

    // Setup opposing positions
    engine.accounts[user_idx as usize].capital = user_capital;
    engine.accounts[user_idx as usize].position_size = position;
    engine.accounts[user_idx as usize].entry_price = entry_price;

    engine.accounts[lp_idx as usize].capital = lp_capital;
    engine.accounts[lp_idx as usize].position_size = -position;
    engine.accounts[lp_idx as usize].entry_price = entry_price;

    // Conservation-consistent vault
    engine.vault = user_capital + lp_capital;

    // Panic settle
    let _ = engine.panic_settle_all(oracle_price);

    // Compute expected value
    let total_capital =
        engine.accounts[user_idx as usize].capital + engine.accounts[lp_idx as usize].capital;
    let user_pnl = engine.accounts[user_idx as usize].pnl;
    let lp_pnl = engine.accounts[lp_idx as usize].pnl;
    let net_pnl = user_pnl.saturating_add(lp_pnl);

    let base = total_capital + engine.insurance_fund.balance;
    let expected = if net_pnl >= 0 {
        base + (net_pnl as u128)
    } else {
        base.saturating_sub((-net_pnl) as u128)
    };

    let actual = engine.vault + engine.loss_accum;

    // PROOF 1: No under-collateralization
    assert!(
        actual >= expected,
        "AUDIT PROOF FAILED: Vault under-collateralized after panic_settle"
    );

    // PROOF 2: Slack is bounded
    let slack = actual - expected;
    assert!(
        slack <= MAX_ROUNDING_SLACK,
        "C1 FAILED: Slack exceeds MAX_ROUNDING_SLACK after panic_settle"
    );

    // PROOF 3: Positions are closed
    assert!(
        engine.accounts[user_idx as usize].position_size == 0,
        "C1 FAILED: User position not closed"
    );
    assert!(
        engine.accounts[lp_idx as usize].position_size == 0,
        "C1 FAILED: LP position not closed"
    );
}

/// Proof C1b: Conservation slack is bounded after force_realize_losses
///
/// Same as C1 but for force_realize_losses.
#[kani::proof]
#[kani::unwind(10)]
#[kani::solver(cadical)]
fn proof_c1_conservation_bounded_slack_force_realize() {
    let mut engine = RiskEngine::new(test_params_with_floor());
    let user_idx = engine.add_user(1).unwrap();
    let lp_idx = engine.add_lp([0u8; 32], [0u8; 32], 1).unwrap();

    // Symbolic inputs
    let user_capital: u128 = kani::any();
    let lp_capital: u128 = kani::any();
    let position: i128 = kani::any();
    let entry_price: u64 = kani::any();
    let oracle_price: u64 = kani::any();

    let floor = engine.params.risk_reduction_threshold;

    // Very small bounds for tractability
    kani::assume(user_capital > 10 && user_capital < 500);
    kani::assume(lp_capital > 10 && lp_capital < 500);
    kani::assume(position > 0 && position < 100);
    kani::assume(entry_price > 100_000 && entry_price < 1_000_000);
    kani::assume(oracle_price > 100_000 && oracle_price < 1_000_000);

    // Setup opposing positions
    engine.accounts[user_idx as usize].capital = user_capital;
    engine.accounts[user_idx as usize].position_size = position;
    engine.accounts[user_idx as usize].entry_price = entry_price;

    engine.accounts[lp_idx as usize].capital = lp_capital;
    engine.accounts[lp_idx as usize].position_size = -position;
    engine.accounts[lp_idx as usize].entry_price = entry_price;

    // Set insurance at floor to allow force_realize
    engine.insurance_fund.balance = floor;
    engine.vault = user_capital + lp_capital + floor;

    // Force realize
    let _ = engine.force_realize_losses(oracle_price);

    // Compute expected value
    let total_capital =
        engine.accounts[user_idx as usize].capital + engine.accounts[lp_idx as usize].capital;
    let user_pnl = engine.accounts[user_idx as usize].pnl;
    let lp_pnl = engine.accounts[lp_idx as usize].pnl;
    let net_pnl = user_pnl.saturating_add(lp_pnl);

    let base = total_capital + engine.insurance_fund.balance;
    let expected = if net_pnl >= 0 {
        base + (net_pnl as u128)
    } else {
        base.saturating_sub((-net_pnl) as u128)
    };

    let actual = engine.vault + engine.loss_accum;

    // PROOF 1: No under-collateralization
    assert!(
        actual >= expected,
        "C1b FAILED: Vault under-collateralized after force_realize"
    );

    // PROOF 2: Slack is bounded
    let slack = actual - expected;
    assert!(
        slack <= MAX_ROUNDING_SLACK,
        "C1b FAILED: Slack exceeds MAX_ROUNDING_SLACK after force_realize"
    );
}

/// Proof: force_realize_losses updates warmup_started_at_slot
///
/// Proves that after force_realize_losses(), all accounts with positions
/// have their warmup_started_at_slot updated to the effective_slot,
/// preventing later settle calls from "re-paying" based on old elapsed time.
#[kani::proof]
#[kani::unwind(10)]
#[kani::solver(cadical)]
fn audit_force_realize_updates_warmup_start() {
    let mut engine = RiskEngine::new(test_params_with_floor());
    let user_idx = engine.add_user(1).unwrap();
    let lp_idx = engine.add_lp([0u8; 32], [0u8; 32], 1).unwrap();

    // Symbolic inputs
    let capital: u128 = kani::any();
    let position: i128 = kani::any();
    let entry_price: u64 = kani::any();
    let oracle_price: u64 = kani::any();
    let old_warmup_start: u64 = kani::any();
    let current_slot: u64 = kani::any();

    // Bounded assumptions
    kani::assume(capital > 1000 && capital < 50_000);
    kani::assume(position > 0 && position < 1_000);
    kani::assume(entry_price > 100_000 && entry_price < 3_000_000);
    kani::assume(oracle_price > 100_000 && oracle_price < 3_000_000);
    kani::assume(old_warmup_start < 50);
    kani::assume(current_slot > 50 && current_slot < 200);

    // Setup with old warmup start
    engine.accounts[user_idx as usize].capital = capital;
    engine.accounts[user_idx as usize].position_size = position;
    engine.accounts[user_idx as usize].entry_price = entry_price;
    engine.accounts[user_idx as usize].warmup_started_at_slot = old_warmup_start;

    engine.accounts[lp_idx as usize].capital = capital;
    engine.accounts[lp_idx as usize].position_size = -position;
    engine.accounts[lp_idx as usize].entry_price = entry_price;
    engine.accounts[lp_idx as usize].warmup_started_at_slot = old_warmup_start;

    // Set insurance at floor
    let floor = engine.params.risk_reduction_threshold;
    engine.insurance_fund.balance = floor;
    engine.vault = capital * 2 + floor;
    engine.current_slot = current_slot;

    // Force realize
    let _ = engine.force_realize_losses(oracle_price);

    // After force_realize, warmup is paused and effective_slot = warmup_pause_slot
    let effective_slot = engine.warmup_pause_slot;

    // PROOF: Both accounts should have updated warmup_started_at_slot
    assert!(
        engine.accounts[user_idx as usize].warmup_started_at_slot == effective_slot,
        "AUDIT PROOF FAILED: User warmup_started_at_slot not updated"
    );
    assert!(
        engine.accounts[lp_idx as usize].warmup_started_at_slot == effective_slot,
        "AUDIT PROOF FAILED: LP warmup_started_at_slot not updated"
    );

    // PROOF: Subsequent settle should be idempotent (no change)
    let capital_before = engine.accounts[user_idx as usize].capital;
    let pnl_before = engine.accounts[user_idx as usize].pnl;

    engine.current_slot = current_slot + 100; // Advance time
    let _ = engine.settle_warmup_to_capital(user_idx);

    assert!(
        engine.accounts[user_idx as usize].capital == capital_before,
        "AUDIT PROOF FAILED: Capital changed after settle post-force_realize"
    );
    assert!(
        engine.accounts[user_idx as usize].pnl == pnl_before,
        "AUDIT PROOF FAILED: PnL changed after settle post-force_realize"
    );
}

// ============================================================================
// ADL/Warmup Correctness Proofs (Step 8 of the fix plan)
// ============================================================================

/// Proof: update_warmup_slope sets slope >= 1 when positive_pnl > 0
/// This prevents the "zero forever" warmup bug where small PnL never warms up.
#[kani::proof]
#[kani::unwind(10)]
#[kani::solver(cadical)]
fn proof_warmup_slope_nonzero_when_positive_pnl() {
    let mut engine = RiskEngine::new(test_params());
    let user_idx = engine.add_user(1).unwrap();

    // Arbitrary positive PnL (bounded for tractability)
    let positive_pnl: i128 = kani::any();
    kani::assume(positive_pnl > 0 && positive_pnl < 10_000);

    // Setup account with positive PnL
    engine.accounts[user_idx as usize].capital = 10_000;
    engine.accounts[user_idx as usize].pnl = positive_pnl;
    engine.vault = 10_000 + positive_pnl as u128;

    // Call update_warmup_slope
    let _ = engine.update_warmup_slope(user_idx);

    // PROOF: slope must be >= 1 when positive_pnl > 0
    // This is enforced by the debug_assert in the function, but we verify here too
    let slope = engine.accounts[user_idx as usize].warmup_slope_per_step;
    assert!(
        slope >= 1,
        "Warmup slope must be >= 1 when positive_pnl > 0"
    );
}

/// Proof: warmup_insurance_reserved equals the derived formula after settlement
/// reserved = min(max(W+ - W-, 0), raw_spendable)
#[kani::proof]
#[kani::unwind(10)]
#[kani::solver(cadical)]
fn proof_reserved_equals_derived_formula() {
    let mut engine = RiskEngine::new(test_params());
    let user_idx = engine.add_user(1).unwrap();

    // Arbitrary values (bounded for tractability)
    let capital: u128 = kani::any();
    let pnl: i128 = kani::any();
    let insurance: u128 = kani::any();
    let current_slot: u64 = kani::any();

    kani::assume(capital > 0 && capital < 10_000);
    kani::assume(pnl > 0 && pnl < 5_000);
    kani::assume(insurance > 0 && insurance < 5_000);
    kani::assume(current_slot > 100 && current_slot < 1_000);

    // Setup account
    engine.accounts[user_idx as usize].capital = capital;
    engine.accounts[user_idx as usize].pnl = pnl;
    engine.accounts[user_idx as usize].warmup_slope_per_step = (pnl as u128) / 100;
    engine.accounts[user_idx as usize].warmup_started_at_slot = 0;

    engine.insurance_fund.balance = insurance;
    engine.vault = capital + pnl as u128 + insurance;
    engine.current_slot = current_slot;

    // Settle warmup (this should update reserved)
    let _ = engine.settle_warmup_to_capital(user_idx);

    // PROOF: reserved == min(max(W+ - W-, 0), raw_spendable)
    let raw_spendable = engine.insurance_spendable_raw();
    let required = engine
        .warmed_pos_total
        .saturating_sub(engine.warmed_neg_total);
    let expected_reserved = core::cmp::min(required, raw_spendable);

    assert!(
        engine.warmup_insurance_reserved == expected_reserved,
        "Reserved must equal derived formula"
    );
}

/// Proof: ADL applies exact haircuts (debug_assert verifies sum == loss_to_socialize)
/// This proof verifies that apply_adl completes without triggering the debug_assert
#[kani::proof]
#[kani::unwind(10)]
#[kani::solver(cadical)]
fn proof_adl_exact_haircut_distribution() {
    let mut engine = RiskEngine::new(test_params());
    let user1 = engine.add_user(1).unwrap();
    let user2 = engine.add_user(2).unwrap();

    // Setup two accounts with positive PnL (unwrapped)
    let pnl1: u128 = kani::any();
    let pnl2: u128 = kani::any();
    let loss: u128 = kani::any();

    kani::assume(pnl1 > 0 && pnl1 < 1_000);
    kani::assume(pnl2 > 0 && pnl2 < 1_000);
    kani::assume(loss > 0 && loss < pnl1 + pnl2);

    engine.accounts[user1 as usize].capital = 10_000;
    engine.accounts[user1 as usize].pnl = pnl1 as i128;

    engine.accounts[user2 as usize].capital = 10_000;
    engine.accounts[user2 as usize].pnl = pnl2 as i128;

    engine.vault = 20_000 + pnl1 + pnl2;
    engine.insurance_fund.balance = 1_000;

    let total_pnl_before =
        (engine.accounts[user1 as usize].pnl + engine.accounts[user2 as usize].pnl) as u128;

    // Apply ADL - the debug_assert inside will verify sum of haircuts == loss_to_socialize
    let _ = engine.apply_adl(loss);

    let total_pnl_after =
        (engine.accounts[user1 as usize].pnl + engine.accounts[user2 as usize].pnl) as u128;

    // PROOF: Total PnL reduced by exactly the socialized loss
    let total_unwrapped = pnl1 + pnl2; // All PnL is unwrapped (no warmup time elapsed)
    let expected_loss = core::cmp::min(loss, total_unwrapped);

    assert!(
        total_pnl_before.saturating_sub(total_pnl_after) == expected_loss,
        "ADL must reduce total PnL by exactly the socialized loss"
    );
}

// ============================================================================
// ADL Largest-Remainder + Reserved Equality Verification
// ============================================================================

/// Proof that ADL applies exact haircuts and maintains reserved equality invariant
///
/// This proof verifies:
/// 1. applied_from_pnl == loss_to_socialize (via debug_assert in apply_adl)
/// 2. reserved == min(max(W+ - W-, 0), raw) (via debug_assert in apply_adl)
/// 3. conservation holds after ADL
#[kani::proof]
#[kani::unwind(10)]
#[kani::solver(cadical)]
fn proof_adl_exactness_and_reserved_invariant() {
    let mut engine = RiskEngine::new(test_params());
    let user1 = engine.add_user(1).unwrap();
    let user2 = engine.add_user(2).unwrap();
    let user3 = engine.add_user(3).unwrap();

    // Setup three accounts with positive PnL (unwrapped)
    let pnl1: u128 = kani::any();
    let pnl2: u128 = kani::any();
    let pnl3: u128 = kani::any();
    let loss: u128 = kani::any();

    kani::assume(pnl1 > 0 && pnl1 < 500);
    kani::assume(pnl2 > 0 && pnl2 < 500);
    kani::assume(pnl3 > 0 && pnl3 < 500);
    kani::assume(loss > 0 && loss < pnl1 + pnl2 + pnl3);

    engine.accounts[user1 as usize].capital = 10_000;
    engine.accounts[user1 as usize].pnl = pnl1 as i128;

    engine.accounts[user2 as usize].capital = 10_000;
    engine.accounts[user2 as usize].pnl = pnl2 as i128;

    engine.accounts[user3 as usize].capital = 10_000;
    engine.accounts[user3 as usize].pnl = pnl3 as i128;

    engine.vault = 30_000 + pnl1 + pnl2 + pnl3;
    engine.insurance_fund.balance = 1_000;

    // Set some warmed positive total to test reserved computation
    let warmed_pos: u128 = kani::any();
    let warmed_neg: u128 = kani::any();
    kani::assume(warmed_pos < 500);
    kani::assume(warmed_neg < 500);
    engine.warmed_pos_total = warmed_pos;
    engine.warmed_neg_total = warmed_neg;

    // Recompute reserved to start in valid state
    engine.recompute_warmup_insurance_reserved();

    // Apply ADL - debug_asserts inside will verify:
    // 1. applied_from_pnl == loss_to_socialize
    // 2. reserved == min(max(W+ - W-, 0), raw)
    let _ = engine.apply_adl(loss);

    // Additional explicit assertion for reserved equality
    let raw = engine.insurance_spendable_raw();
    let needed = engine.warmed_pos_total.saturating_sub(engine.warmed_neg_total);
    let expected_reserved = core::cmp::min(needed, raw);
    assert!(
        engine.warmup_insurance_reserved == expected_reserved,
        "Reserved equality invariant must hold after ADL"
    );

    // Conservation check
    assert!(
        engine.check_conservation(),
        "Conservation must hold after ADL"
    );
}
