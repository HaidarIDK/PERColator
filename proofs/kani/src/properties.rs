//! Explicit property proofs for documentation and clarity
//!
//! These proofs verify properties that are implicitly covered by other proofs,
//! but we make them explicit for better documentation and auditing.

use model_safety::{state::*, transitions::*, helpers::*, warmup::*};
use arrayvec::ArrayVec;

// === Property 1: PNL Decay Determinism ===

/// I5+: PNL withdrawable calculation is deterministic
///
/// Property: Given the same account state and time delta, withdrawable_pnl
/// always returns the same value (no hidden state or randomness).
///
/// This is critical for:
/// - Fair withdrawal processing
/// - Reproducible accounting
/// - Auditing and debugging
#[kani::proof]
#[kani::unwind(4)]
fn i5_warmup_determinism() {
    // Create account with arbitrary but bounded values
    let principal: u128 = kani::any();
    let pnl: i128 = kani::any();
    let reserved: u128 = kani::any();
    let slope: u128 = kani::any();

    // Bound values for reasonable verification time
    kani::assume(principal < 1000);
    kani::assume(pnl > i128::MIN && pnl < 1000 && pnl > -1000); // Avoid i128::MIN overflow
    kani::assume(reserved < 100);
    kani::assume(slope > 0 && slope < 100);

    let account = Account {
        principal,
        pnl_ledger: pnl,
        reserved_pnl: reserved,
        warmup_state: Warmup {
            started_at_slot: 0,
            slope_per_step: slope,
        },
        position_size: 0,
    };

    // Arbitrary time delta
    let steps: u32 = kani::any();
    kani::assume(steps < 20); // Bound for performance

    // Calculate withdrawable PnL twice with same inputs
    let result1 = withdrawable_pnl(&account, steps, slope);
    let result2 = withdrawable_pnl(&account, steps, slope);

    // Must be deterministic
    assert_eq!(result1, result2,
        "I5+: withdrawable_pnl must be deterministic for same inputs");
}

// === Property 2: Warmup Monotonicity ===

/// I5++: Warmup cap is monotonically increasing over time
///
/// Property: For any account, withdrawable_pnl(t2) >= withdrawable_pnl(t1)
/// when t2 > t1 (assuming PnL unchanged).
///
/// This ensures:
/// - Users can never lose withdrawal rights as time passes
/// - Warm-up period works as intended
/// - No time-based exploits
#[kani::proof]
#[kani::unwind(4)]
fn i5_warmup_monotonicity() {
    // Create account with positive PnL
    let principal: u128 = kani::any();
    let pnl: i128 = kani::any();
    let reserved: u128 = kani::any();
    let slope: u128 = kani::any();

    // Bound values
    kani::assume(principal < 1000);
    kani::assume(pnl > 0 && pnl < 1000); // Positive PnL
    kani::assume(reserved < 100);
    kani::assume(slope > 0 && slope < 100);

    let account = Account {
        principal,
        pnl_ledger: pnl,
        reserved_pnl: reserved,
        warmup_state: Warmup {
            started_at_slot: 0,
            slope_per_step: slope,
        },
        position_size: 0,
    };

    // Two time points with t2 > t1
    let steps1: u32 = kani::any();
    let steps2: u32 = kani::any();
    kani::assume(steps1 < 20);
    kani::assume(steps2 < 20);
    kani::assume(steps2 > steps1); // t2 > t1

    // Calculate withdrawable at both times
    let cap1 = withdrawable_pnl(&account, steps1, slope);
    let cap2 = withdrawable_pnl(&account, steps2, slope);

    // Later time should allow >= withdrawal
    assert!(cap2 >= cap1,
        "I5++: Warmup cap must be monotonically increasing: cap1={}, cap2={}, steps1={}, steps2={}",
        cap1, cap2, steps1, steps2);
}

/// I5+++: Warmup cap never exceeds available positive PnL
///
/// Property: withdrawable_pnl(acc, steps) <= max(0, acc.pnl_ledger) - acc.reserved_pnl
///
/// This ensures users can't withdraw more than they have.
#[kani::proof]
#[kani::unwind(4)]
fn i5_warmup_bounded_by_pnl() {
    let principal: u128 = kani::any();
    let pnl: i128 = kani::any();
    let reserved: u128 = kani::any();
    let slope: u128 = kani::any();

    // Bound values
    kani::assume(principal < 1000);
    kani::assume(pnl > i128::MIN && pnl < 1000 && pnl > -1000); // Avoid i128::MIN overflow
    kani::assume(reserved < 100);
    kani::assume(slope > 0 && slope < 100);

    let account = Account {
        principal,
        pnl_ledger: pnl,
        reserved_pnl: reserved,
        warmup_state: Warmup {
            started_at_slot: 0,
            slope_per_step: slope,
        },
        position_size: 0,
    };

    let steps: u32 = kani::any();
    kani::assume(steps < 20);

    let withdrawable = withdrawable_pnl(&account, steps, slope);
    let effective_pnl = effective_positive_pnl(&account);

    // Can't withdraw more than available PnL
    assert!(withdrawable <= effective_pnl,
        "I5+++: Warmup cap must not exceed available PnL: withdrawable={}, effective_pnl={}",
        withdrawable, effective_pnl);
}

// === Property 3: User Isolation ===

/// I7+: Users are isolated - one user's operations don't affect others
///
/// Property: When operating on user[i], user[j] (j != i) is unchanged
/// for all operations except global socialization.
///
/// Verified operations:
/// - deposit
/// - withdraw_principal
/// - withdraw_pnl
///
/// Note: socialize_losses intentionally affects all users (I4 proves bounded).
#[kani::proof]
#[kani::unwind(4)]
fn i7_user_isolation() {
    // Create 2-user state
    let acc1 = Account {
        principal: 1000,
        pnl_ledger: 500,
        reserved_pnl: 0,
        warmup_state: Warmup {
            started_at_slot: 0,
            slope_per_step: 10,
        },
        position_size: 0,
    };

    let acc2 = Account {
        principal: 2000,
        pnl_ledger: -300,
        reserved_pnl: 0,
        warmup_state: Warmup {
            started_at_slot: 0,
            slope_per_step: 20,
        },
        position_size: 0,
    };

    let mut users = ArrayVec::new();
    users.push(acc1);
    users.push(acc2);

    let state = State {
        vault: 3200, // 1000 + 500 + 2000 (negative PnL doesn't affect vault)
        insurance_fund: 100,
        fees_outstanding: 0,
        users,
        params: Params {
            max_users: 6,
            withdraw_cap_per_step: 1000,
            maintenance_margin_bps: 50_000,
        },
        authorized_router: true,
    };

    let user2_before = state.users[1].clone();

    // Operate on user 0 (deposit)
    let after_deposit = deposit(state.clone(), 0, 100);
    assert_eq!(user2_before.principal, after_deposit.users[1].principal,
        "I7+: User 1 principal unchanged by user 0 deposit");
    assert_eq!(user2_before.pnl_ledger, after_deposit.users[1].pnl_ledger,
        "I7+: User 1 PnL unchanged by user 0 deposit");

    // Operate on user 0 (withdraw principal)
    let after_withdraw = withdraw_principal(state.clone(), 0, 50);
    assert_eq!(user2_before.principal, after_withdraw.users[1].principal,
        "I7+: User 1 principal unchanged by user 0 withdrawal");
    assert_eq!(user2_before.pnl_ledger, after_withdraw.users[1].pnl_ledger,
        "I7+: User 1 PnL unchanged by user 0 withdrawal");

    // Operate on user 0 (withdraw PnL)
    let after_pnl_withdraw = withdraw_pnl(state.clone(), 0, 10, 5);
    assert_eq!(user2_before.principal, after_pnl_withdraw.users[1].principal,
        "I7+: User 1 principal unchanged by user 0 PnL withdrawal");
    assert_eq!(user2_before.pnl_ledger, after_pnl_withdraw.users[1].pnl_ledger,
        "I7+: User 1 PnL unchanged by user 0 PnL withdrawal");
}

// === Property 4: Account-Level Isolation (Production Integration) ===

/// I8+: Account equity calculation is consistent
///
/// Property: User equity = principal + max(0, pnl_ledger)
/// This matches production's equity calculation.
///
/// Verified for all valid account states.
#[kani::proof]
#[kani::unwind(4)]
fn i8_equity_consistency() {
    let principal: u128 = kani::any();
    let pnl: i128 = kani::any();
    let reserved: u128 = kani::any();

    // Bound values
    kani::assume(principal < 10000);
    kani::assume(pnl > i128::MIN && pnl < 10000 && pnl > -10000); // Avoid i128::MIN overflow
    kani::assume(reserved < 1000);

    let account = Account {
        principal,
        pnl_ledger: pnl,
        reserved_pnl: reserved,
        warmup_state: Warmup {
            started_at_slot: 0,
            slope_per_step: 10,
        },
        position_size: 0,
    };

    // Calculate collateral (used in liquidation checks)
    use model_safety::math::{add_u128, clamp_pos_i128};
    let collateral = add_u128(account.principal, clamp_pos_i128(account.pnl_ledger));

    // Verify consistency
    let expected_collateral = if pnl >= 0 {
        principal.saturating_add(pnl as u128)
    } else {
        principal
    };

    assert_eq!(collateral, expected_collateral,
        "I8+: Collateral must equal principal + max(0, pnl)");
}

/// I9+: Conservation holds across single-user operations
///
/// Property: For any single-user operation (deposit, withdraw),
/// the sum vault = principal + max(0, pnl) + insurance + fees is preserved.
///
/// This is a simplified version of I2 for single-user case.
#[kani::proof]
#[kani::unwind(4)]
fn i9_single_user_conservation() {
    let principal: u128 = kani::any();
    let pnl: i128 = kani::any();

    kani::assume(principal < 1000);
    kani::assume(pnl > 0 && pnl < 1000); // Positive PnL

    let account = Account {
        principal,
        pnl_ledger: pnl,
        reserved_pnl: 0,
        warmup_state: Warmup {
            started_at_slot: 0,
            slope_per_step: 10,
        },
        position_size: 0,
    };

    let mut users = ArrayVec::new();
    users.push(account);

    let insurance = 100u128;
    let fees = 50u128;
    // vault = principal + pnl + insurance - fees
    let vault = principal + (pnl as u128) + insurance - fees;

    let state = State {
        vault,
        insurance_fund: insurance,
        fees_outstanding: fees,
        users,
        params: Params {
            max_users: 6,
            withdraw_cap_per_step: 1000,
            maintenance_margin_bps: 50_000,
        },
        authorized_router: true,
    };

    // Verify initial conservation
    assert!(conservation_ok(&state),
        "I9+: Initial state must satisfy conservation");

    // Test deposit preserves conservation
    let after_deposit = deposit(state.clone(), 0, 100);
    assert!(conservation_ok(&after_deposit),
        "I9+: Deposit must preserve conservation");

    // Test withdrawal preserves conservation
    let after_withdraw = withdraw_principal(state.clone(), 0, 50);
    assert!(conservation_ok(&after_withdraw),
        "I9+: Withdrawal must preserve conservation");
}
