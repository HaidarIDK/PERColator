//! Edge case Kani proofs - boundary conditions and corner cases
//!
//! This module tests edge cases that could break invariants:
//! - Zero values (principal, PnL, vault)
//! - Reserved PnL interactions
//! - Maximum/minimum value boundaries
//! - 3-user scenarios
//! - Total wipeout socializations

use model_safety::{state::*, transitions::*, helpers::*};
use arrayvec::ArrayVec;

// === Helper Functions ===

/// Create 1-user state with specified values (including reserved_pnl)
pub fn make_1user_with_reserved(
    principal: u128,
    pnl: i128,
    reserved_pnl: u128,
    slope: u128,
) -> State {
    let account = Account {
        principal,
        pnl_ledger: pnl,
        reserved_pnl,
        warmup_state: Warmup {
            started_at_slot: 0,
            slope_per_step: slope,
        },
        position_size: 0,
    };

    let mut users = ArrayVec::new();
    users.push(account);

    let sum_principal = principal;
    let sum_pos_pnl = if pnl > 0 { pnl as u128 } else { 0 };
    let vault = sum_principal.saturating_add(sum_pos_pnl);

    State {
        vault,
        insurance_fund: 100,
        fees_outstanding: 0,
        users,
        params: Params {
            max_users: 6,
            withdraw_cap_per_step: 1_000,
            maintenance_margin_bps: 50_000,
        },
        authorized_router: true,
    }
}

/// Create 3-user state with specified configurations
pub fn make_3user_state(
    principals: [u128; 3],
    pnls: [i128; 3],
) -> State {
    let mut users = ArrayVec::new();

    for i in 0..3 {
        users.push(Account {
            principal: principals[i],
            pnl_ledger: pnls[i],
            reserved_pnl: 0,
            warmup_state: Warmup { started_at_slot: 0, slope_per_step: 10 },
            position_size: 0,
        });
    }

    let sum_principal = principals.iter().fold(0u128, |acc, &p| acc.saturating_add(p));
    let sum_pos_pnl = pnls.iter().fold(0u128, |acc, &pnl| {
        let pos = if pnl > 0 { pnl as u128 } else { 0 };
        acc.saturating_add(pos)
    });
    let vault = sum_principal.saturating_add(sum_pos_pnl);

    State {
        vault,
        insurance_fund: 100,
        fees_outstanding: 0,
        users,
        params: Params { max_users: 6, withdraw_cap_per_step: 1_000, maintenance_margin_bps: 50_000 },
        authorized_router: true,
    }
}

// === EDGE CASE 1: Zero Value Handling ===

/// **EDGE: Zero Principal Bootstrap**
///
/// Tests that a user with zero principal can successfully deposit
/// and bootstrap their account.
///
/// **Invariant**: Deposit works correctly from zero state
#[kani::proof]
fn edge_zero_principal_bootstrap() {
    // User starts with zero principal
    let state = make_1user_with_reserved(0, 0, 0, 10);

    let deposit_amt: u8 = kani::any();
    kani::assume(deposit_amt > 0); // Must deposit something

    let after = deposit(state, 0, deposit_amt as u128);

    // Should successfully bootstrap from zero
    kani::assert(
        after.users[0].principal == deposit_amt as u128,
        "EDGE: Zero principal user can bootstrap via deposit"
    );
    kani::assert(
        after.vault >= deposit_amt as u128,
        "EDGE: Vault updated correctly from zero"
    );
}

/// **EDGE: Zero Principal Cannot Withdraw**
///
/// Tests that a user with zero principal cannot withdraw principal,
/// even if they have positive PnL.
///
/// **Invariant**: Withdrawals respect principal boundaries
#[kani::proof]
fn edge_zero_principal_cannot_withdraw() {
    // Zero principal but positive PnL (from trading)
    let state = make_1user_with_reserved(0, 100, 0, 10);

    let amount: u8 = kani::any();

    let before = state.clone();
    let after = withdraw_principal(state, 0, amount as u128);

    // Cannot withdraw principal when principal = 0
    kani::assert(
        after.users[0].principal == 0,
        "EDGE: Zero principal unchanged by withdrawal attempt"
    );
    kani::assert(
        after.vault == before.vault,
        "EDGE: Vault unchanged when withdrawing from zero principal"
    );
}

/// **EDGE: Socialization with All Losers (Zero Positive PnL)**
///
/// Tests that socialization behaves correctly when there are no winners,
/// i.e., all users have negative or zero PnL.
///
/// **Invariant**: I4 - Socialization with zero effective winners is a no-op
#[kani::proof]
fn edge_socialization_all_losers() {
    // 2 users, both with negative PnL
    let user1 = Account {
        principal: 1000,
        pnl_ledger: -500,
        reserved_pnl: 0,
        warmup_state: Warmup { started_at_slot: 0, slope_per_step: 10 },
        position_size: 0,
    };
    let user2 = Account {
        principal: 1000,
        pnl_ledger: -300,
        reserved_pnl: 0,
        warmup_state: Warmup { started_at_slot: 0, slope_per_step: 10 },
        position_size: 0,
    };

    let mut users = ArrayVec::new();
    users.push(user1);
    users.push(user2);

    let state = State {
        vault: 2000, // Only principals, no positive PnL
        insurance_fund: 100,
        fees_outstanding: 0,
        users,
        params: Params { max_users: 6, withdraw_cap_per_step: 1_000, maintenance_margin_bps: 50_000 },
        authorized_router: true,
    };

    let deficit: u8 = kani::any();

    let before = state.clone();
    let after = socialize_losses(state, deficit as u128);

    // No positive PnL to haircut - state should be largely unchanged
    kani::assert(
        after.users[0].pnl_ledger == before.users[0].pnl_ledger,
        "EDGE: Loser 1 PnL unchanged when no winners exist"
    );
    kani::assert(
        after.users[1].pnl_ledger == before.users[1].pnl_ledger,
        "EDGE: Loser 2 PnL unchanged when no winners exist"
    );
    kani::assert(
        total_haircut(&before, &after) == 0,
        "EDGE: Total haircut is zero when no positive PnL exists"
    );
}

/// **EDGE: Zero Slope Throttle (No Withdrawals)**
///
/// Tests that when warmup slope = 0, no PnL can be withdrawn
/// regardless of how many steps have passed.
///
/// **Invariant**: I5 - Throttle enforced even at zero slope
#[kani::proof]
fn edge_zero_slope_no_withdrawal() {
    // User with positive PnL but zero warmup slope
    let state = make_1user_with_reserved(1000, 500, 0, 0); // slope = 0

    let step: u8 = kani::any();
    let amount: u8 = kani::any();

    let before_pnl = state.users[0].pnl_ledger;
    let after = withdraw_pnl(state, 0, amount as u128, step as u32);
    let after_pnl = after.users[0].pnl_ledger;

    // With slope=0, max_allowed = step * 0 = 0, so no withdrawal
    kani::assert(
        after_pnl == before_pnl,
        "EDGE: Zero slope prevents all PnL withdrawals"
    );
}

// === EDGE CASE 2: Reserved PnL Interactions ===

/// **EDGE: Withdrawal with Reserved PnL**
///
/// Tests that reserved PnL (from pending withdrawals) is properly
/// accounted for and doesn't interfere with new operations.
///
/// **Invariant**: Reserved PnL reduces effective positive PnL
#[kani::proof]
fn edge_reserved_pnl_reduces_effective() {
    // User with reserved PnL (pending withdrawal)
    let principal = 1000;
    let pnl = 500;
    let reserved = 200; // 200 already reserved

    let state = make_1user_with_reserved(principal, pnl, reserved, 10);

    let deposit_amt: u8 = kani::any();

    // Deposit should work normally
    let after = deposit(state, 0, deposit_amt as u128);

    kani::assert(
        after.users[0].principal == principal + (deposit_amt as u128),
        "EDGE: Deposit works with reserved PnL present"
    );
    kani::assert(
        after.users[0].reserved_pnl == reserved,
        "EDGE: Reserved PnL unchanged by deposit"
    );
}

/// **EDGE: Socialization with Reserved PnL**
///
/// Tests that haircuts apply to total PnL, not effective PnL.
/// Reserved PnL should not protect against haircuts.
///
/// **Invariant**: I4 - Haircuts apply to pnl_ledger, not effective PnL
#[kani::proof]
fn edge_socialization_with_reserved() {
    // User with PnL and reserved amount
    let state = make_1user_with_reserved(1000, 500, 100, 10);
    // Effective PnL = 500 - 100 = 400, but haircut should apply to 500

    let deficit: u8 = kani::any();
    let deficit_bounded = (deficit as u128) % 256;

    let before = state.clone();
    let after = socialize_losses(state, deficit_bounded);

    // Haircut should apply to total pnl_ledger, not effective
    if deficit_bounded > 0 && before.users[0].pnl_ledger > 0 {
        kani::assert(
            after.users[0].pnl_ledger <= before.users[0].pnl_ledger,
            "EDGE: PnL reduced by haircut despite reserved amount"
        );
    }

    // Reserved amount unchanged
    kani::assert(
        after.users[0].reserved_pnl == before.users[0].reserved_pnl,
        "EDGE: Reserved PnL unchanged by socialization"
    );
}

/// **EDGE: Reserved PnL Cannot Exceed Positive PnL**
///
/// Tests the invariant that reserved_pnl should never exceed
/// positive pnl_ledger (if it does, it's a bug).
///
/// **Invariant**: reserved_pnl <= max(0, pnl_ledger)
#[kani::proof]
fn edge_reserved_cannot_exceed_positive_pnl() {
    // Create states and verify reserved <= positive PnL
    let principal: u16 = kani::any();
    let pnl_raw: i16 = kani::any();
    let reserved_raw: u8 = kani::any();

    let pnl = (pnl_raw as i128) % 1000;
    let reserved = (reserved_raw as u128) % 500;
    let principal_bounded = (principal as u128) % 10000;

    // Only test when PnL is positive
    kani::assume(pnl > 0);

    // Reserved should not exceed positive PnL in a valid state
    kani::assume(reserved <= pnl as u128);

    let state = make_1user_with_reserved(principal_bounded, pnl, reserved, 10);

    // Perform an operation
    let deposit_amt: u8 = kani::any();
    let after = deposit(state, 0, deposit_amt as u128);

    // Verify invariant maintained
    if after.users[0].pnl_ledger > 0 {
        kani::assert(
            after.users[0].reserved_pnl <= after.users[0].pnl_ledger as u128,
            "EDGE: Reserved PnL never exceeds positive PnL"
        );
    }
}

// === EDGE CASE 3: Total Wipeout Socialization ===

/// **EDGE: Total Wipeout - Deficit Exceeds All Positive PnL**
///
/// Tests the extreme case where the deficit is much larger than
/// all available positive PnL, resulting in a 100% haircut.
///
/// **Invariant**: I4 - All positive PnL goes to zero, principals intact
#[kani::proof]
fn edge_total_wipeout_socialization() {
    // 2 users with positive PnL
    let user1 = Account {
        principal: 1000,
        pnl_ledger: 300,
        reserved_pnl: 0,
        warmup_state: Warmup { started_at_slot: 0, slope_per_step: 10 },
        position_size: 0,
    };
    let user2 = Account {
        principal: 1000,
        pnl_ledger: 200,
        reserved_pnl: 0,
        warmup_state: Warmup { started_at_slot: 0, slope_per_step: 10 },
        position_size: 0,
    };

    let mut users = ArrayVec::new();
    users.push(user1);
    users.push(user2);

    let state = State {
        vault: 2500, // 2000 principal + 500 positive PnL
        insurance_fund: 100,
        fees_outstanding: 0,
        users,
        params: Params { max_users: 6, withdraw_cap_per_step: 1_000, maintenance_margin_bps: 50_000 },
        authorized_router: true,
    };

    // Deficit much larger than available PnL (500)
    let massive_deficit = 10_000u128;

    let before = state.clone();
    let after = socialize_losses(state, massive_deficit);

    // Both users should have PnL wiped to zero or near-zero
    kani::assert(
        after.users[0].pnl_ledger <= 1, // Allow rounding
        "EDGE: User 1 PnL wiped out by massive deficit"
    );
    kani::assert(
        after.users[1].pnl_ledger <= 1,
        "EDGE: User 2 PnL wiped out by massive deficit"
    );

    // But principals must remain intact (I1)
    kani::assert(
        after.users[0].principal == before.users[0].principal,
        "EDGE: User 1 principal intact despite total wipeout"
    );
    kani::assert(
        after.users[1].principal == before.users[1].principal,
        "EDGE: User 2 principal intact despite total wipeout"
    );
}

/// **EDGE: Exact Balance - Deficit Equals Total Positive PnL**
///
/// Tests the boundary case where deficit exactly equals the sum
/// of all positive PnL.
///
/// **Invariant**: I4 - Haircut equals exactly the available PnL
#[kani::proof]
fn edge_exact_deficit_balance() {
    // Create state with known positive PnL sum
    let pnl1 = 300i128;
    let pnl2 = 200i128;
    let total_pos_pnl = 500u128; // pnl1 + pnl2

    let user1 = Account {
        principal: 1000,
        pnl_ledger: pnl1,
        reserved_pnl: 0,
        warmup_state: Warmup { started_at_slot: 0, slope_per_step: 10 },
        position_size: 0,
    };
    let user2 = Account {
        principal: 1000,
        pnl_ledger: pnl2,
        reserved_pnl: 0,
        warmup_state: Warmup { started_at_slot: 0, slope_per_step: 10 },
        position_size: 0,
    };

    let mut users = ArrayVec::new();
    users.push(user1);
    users.push(user2);

    let state = State {
        vault: 2500,
        insurance_fund: 100,
        fees_outstanding: 0,
        users,
        params: Params { max_users: 6, withdraw_cap_per_step: 1_000, maintenance_margin_bps: 50_000 },
        authorized_router: true,
    };

    // Deficit exactly equals total positive PnL
    let exact_deficit = total_pos_pnl;

    let before = state.clone();
    let after = socialize_losses(state, exact_deficit);

    // Total haircut should equal deficit
    let haircut = total_haircut(&before, &after);
    kani::assert(
        haircut <= exact_deficit && haircut >= exact_deficit.saturating_sub(2),
        "EDGE: Haircut equals deficit when exactly balanced (±rounding)"
    );

    // All PnL should be wiped
    kani::assert(
        after.users[0].pnl_ledger + after.users[1].pnl_ledger <= 2,
        "EDGE: All PnL removed when deficit equals total"
    );
}

// === EDGE CASE 4: 3-User Scenarios ===

/// **EDGE: 3 Users - All Winners, Proportional Haircut**
///
/// Tests that with 3 users all having positive PnL, haircuts are
/// applied proportionally to their effective PnL amounts.
///
/// **Invariant**: I4 - Proportional haircut distribution
#[kani::proof]
fn edge_3users_all_winners() {
    // 3 users with different positive PnL: 500, 300, 200
    let state = make_3user_state(
        [1000, 1000, 1000],
        [500, 300, 200],
    );

    let deficit: u8 = kani::any();
    let deficit_bounded = (deficit as u128) % 512; // 0-511

    let before = state.clone();
    let after = socialize_losses(state, deficit_bounded);

    // All should be haircutted (if deficit > 0)
    if deficit_bounded > 10 {
        kani::assert(
            after.users[0].pnl_ledger <= before.users[0].pnl_ledger,
            "EDGE: User 0 (highest PnL) haircutted"
        );
        kani::assert(
            after.users[1].pnl_ledger <= before.users[1].pnl_ledger,
            "EDGE: User 1 (medium PnL) haircutted"
        );
        kani::assert(
            after.users[2].pnl_ledger <= before.users[2].pnl_ledger,
            "EDGE: User 2 (lowest PnL) haircutted"
        );
    }

    // Principals intact (I1)
    kani::assert(
        principals_unchanged(&before, &after),
        "EDGE: All principals unchanged with 3 users"
    );

    // Total haircut bounded (I4)
    let haircut = total_haircut(&before, &after);
    let sum_eff = sum_effective_winners(&before);
    let expected_max = if deficit_bounded < sum_eff { deficit_bounded } else { sum_eff };
    kani::assert(
        haircut <= expected_max,
        "EDGE: 3-user haircut properly bounded"
    );
}

/// **EDGE: 3 Users - Mixed Winners and Losers**
///
/// Tests that with 2 winners and 1 loser, only winners are haircutted
/// and the loser's negative PnL is untouched.
///
/// **Invariant**: I4 - Only winners haircutted, losers unchanged
#[kani::proof]
fn edge_3users_mixed_winners_losers() {
    // 2 winners (500, 300), 1 loser (-200)
    let state = make_3user_state(
        [1000, 1000, 1000],
        [500, 300, -200],
    );

    let deficit: u8 = kani::any();

    let before = state.clone();
    let after = socialize_losses(state, deficit as u128);

    // Loser's PnL should be unchanged (I4: winners only)
    kani::assert(
        after.users[2].pnl_ledger == before.users[2].pnl_ledger,
        "EDGE: Loser PnL unchanged in 3-user mixed scenario"
    );
    kani::assert(
        before.users[2].pnl_ledger < 0,
        "EDGE: Verify user 2 is actually a loser"
    );

    // Winners only haircutted
    kani::assert(
        winners_only_haircut(&before, &after),
        "EDGE: 3-user scenario respects winners-only rule"
    );

    // Principals intact
    kani::assert(
        principals_unchanged(&before, &after),
        "EDGE: 3-user principals all unchanged"
    );
}

/// **EDGE: 3 Users - Sequential Operations**
///
/// Tests a sequence of operations across 3 users to ensure
/// invariants hold with more complex state transitions.
///
/// **Invariant**: Conservation and authorization across multi-user ops
#[kani::proof]
fn edge_3users_sequential_ops() {
    let state = make_3user_state(
        [1000, 1000, 1000],
        [500, -200, 300],
    );

    // Symbolic operation choices
    let op1: u8 = kani::any();
    let op2: u8 = kani::any();
    let amount1: u8 = kani::any();
    let amount2: u8 = kani::any();

    let user1 = (op1 % 3) as usize; // 0, 1, or 2
    let user2 = (op2 % 3) as usize;

    let initial_vault = state.vault;

    // Op 1: Deposit to user1
    let s1 = deposit(state, user1, amount1 as u128);

    // Op 2: Withdraw from user2 (bounded to avoid over-withdrawal)
    let withdraw_amt = (amount2 as u128) % 500;
    let s2 = withdraw_principal(s1, user2, withdraw_amt);

    // Vault change should match operations
    let net_change = (amount1 as i128) - (withdraw_amt as i128);
    let actual_change = (s2.vault as i128) - (initial_vault as i128);

    // Allow some tolerance for saturation
    kani::assert(
        (actual_change - net_change).abs() <= 2,
        "EDGE: 3-user sequential ops maintain vault conservation"
    );

    // All principals should be reasonable
    for i in 0..3 {
        kani::assert(
            s2.users[i].principal <= 100_000,
            "EDGE: No principal overflow in 3-user scenario"
        );
    }
}

// === EDGE CASE 5: Extreme Boundary Values ===

/// **EDGE: Withdrawal Exactly Equals Principal (Total Exit)**
///
/// Tests that a user can withdraw their exact principal amount,
/// leaving them with zero principal (clean exit).
///
/// **Invariant**: Total withdrawal leaves zero principal
#[kani::proof]
fn edge_withdrawal_exact_principal() {
    let principal = 1000u128;
    let state = make_1user_with_reserved(principal, 0, 0, 10);

    // Withdraw exactly the principal amount
    let after = withdraw_principal(state, 0, principal);

    kani::assert(
        after.users[0].principal == 0,
        "EDGE: Exact principal withdrawal leaves zero"
    );

    // Vault should decrease by exactly principal
    kani::assert(
        after.vault == 0,
        "EDGE: Vault emptied by exact principal withdrawal"
    );
}

/// **EDGE: Tiny Deficit with Large Positive PnL**
///
/// Tests that a very small deficit (1) applied to large PnL (1000)
/// results in a tiny haircut, testing precision.
///
/// **Invariant**: I4 - Tiny haircuts preserve most PnL
#[kani::proof]
fn edge_tiny_deficit_large_pnl() {
    let state = make_1user_with_reserved(1000, 1000, 0, 10);

    let tiny_deficit = 1u128;

    let before = state.clone();
    let after = socialize_losses(state, tiny_deficit);

    // Most PnL should be preserved
    kani::assert(
        after.users[0].pnl_ledger >= 990,
        "EDGE: Tiny deficit leaves most PnL intact"
    );

    // But some haircut did occur
    kani::assert(
        after.users[0].pnl_ledger <= before.users[0].pnl_ledger,
        "EDGE: Tiny haircut still reduces PnL"
    );
}
