//! Ultra-minimal Kani proofs using concrete values
//! Start with specific test cases, then gradually increase generality

use model_safety::{state::*, transitions::*, helpers::*};

// === Level 1: Concrete single-user tests ===

/// Minimal I1: Principal unchanged by socialization (concrete 1-user case)
#[kani::proof]
fn i1_concrete_single_user() {
    let account = Account {
        principal: 1000,
        pnl_ledger: 500,  // Positive PnL
        reserved_pnl: 0,
        warmup_state: Warmup { started_at_slot: 0, slope_per_step: 10 },
        position_size: 0,
    };

    let mut users = arrayvec::ArrayVec::<Account, 6>::new();
    users.push(account);

    let state = State {
        vault: 1500,
        insurance_fund: 0,
        fees_outstanding: 0,
        users,
        params: Params { max_users: 6, withdraw_cap_per_step: 1000, maintenance_margin_bps: 50_000 },
        authorized_router: true,
    };

    let before = state.clone();
    let after = socialize_losses(state, 100);

    // Principal must be unchanged
    assert_eq!(before.users[0].principal, after.users[0].principal,
        "I1: Principal must never change during socialization");
}

/// Minimal I3: Unauthorized operations cannot mutate (concrete case)
#[kani::proof]
fn i3_concrete_unauthorized() {
    let account = Account {
        principal: 1000,
        pnl_ledger: 500,
        reserved_pnl: 0,
        warmup_state: Warmup { started_at_slot: 0, slope_per_step: 10 },
        position_size: 0,
    };

    let mut users = arrayvec::ArrayVec::<Account, 6>::new();
    users.push(account);

    let state = State {
        vault: 1500,
        insurance_fund: 0,
        fees_outstanding: 0,
        users,
        params: Params { max_users: 6, withdraw_cap_per_step: 1000, maintenance_margin_bps: 50_000 },
        authorized_router: false,  // NOT authorized
    };

    let before = state.clone();

    // Try deposit - should fail
    let after = deposit(state.clone(), 0, 100);
    assert_eq!(before.users[0].principal, after.users[0].principal,
        "I3: Unauthorized deposit must not change principal");
    assert_eq!(before.vault, after.vault,
        "I3: Unauthorized deposit must not change vault");

    // Try withdrawal - should fail
    let after = withdraw_principal(state, 0, 100);
    assert_eq!(before.users[0].principal, after.users[0].principal,
        "I3: Unauthorized withdrawal must not change principal");
    assert_eq!(before.vault, after.vault,
        "I3: Unauthorized withdrawal must not change vault");
}

/// Minimal I6: Matcher cannot move funds (concrete case)
#[kani::proof]
fn i6_concrete_matcher() {
    let account = Account {
        principal: 1000,
        pnl_ledger: 500,
        reserved_pnl: 0,
        warmup_state: Warmup { started_at_slot: 0, slope_per_step: 10 },
        position_size: 0,
    };

    let mut users = arrayvec::ArrayVec::<Account, 6>::new();
    users.push(account);

    let state = State {
        vault: 1500,
        insurance_fund: 0,
        fees_outstanding: 0,
        users,
        params: Params { max_users: 6, withdraw_cap_per_step: 1000, maintenance_margin_bps: 50_000 },
        authorized_router: true,
    };

    let before = state.clone();
    let after = matcher_noise(state);

    assert!(balances_unchanged(&before, &after),
        "I6: Matcher cannot move funds");
}

/// Minimal deposit test: increases principal and vault
#[kani::proof]
fn deposit_concrete() {
    let account = Account {
        principal: 1000,
        pnl_ledger: 0,
        reserved_pnl: 0,
        warmup_state: Warmup { started_at_slot: 0, slope_per_step: 10 },
        position_size: 0,
    };

    let mut users = arrayvec::ArrayVec::<Account, 6>::new();
    users.push(account);

    let state = State {
        vault: 1000,
        insurance_fund: 0,
        fees_outstanding: 0,
        users,
        params: Params { max_users: 6, withdraw_cap_per_step: 1000, maintenance_margin_bps: 50_000 },
        authorized_router: true,
    };

    let before_principal = state.users[0].principal;
    let before_vault = state.vault;

    let after = deposit(state, 0, 500);

    assert_eq!(after.users[0].principal, before_principal + 500,
        "Deposit must increase principal by deposit amount");
    assert_eq!(after.vault, before_vault + 500,
        "Deposit must increase vault by deposit amount");
}

/// Minimal withdrawal test: decreases principal and vault
#[kani::proof]
fn withdrawal_concrete() {
    let account = Account {
        principal: 1000,
        pnl_ledger: 0,
        reserved_pnl: 0,
        warmup_state: Warmup { started_at_slot: 0, slope_per_step: 10 },
        position_size: 0,
    };

    let mut users = arrayvec::ArrayVec::<Account, 6>::new();
    users.push(account);

    let state = State {
        vault: 1000,
        insurance_fund: 0,
        fees_outstanding: 0,
        users,
        params: Params { max_users: 6, withdraw_cap_per_step: 1000, maintenance_margin_bps: 50_000 },
        authorized_router: true,
    };

    let before_principal = state.users[0].principal;
    let before_vault = state.vault;

    let after = withdraw_principal(state, 0, 300);

    assert_eq!(after.users[0].principal, before_principal - 300,
        "Withdrawal must decrease principal by withdrawn amount");
    assert_eq!(after.vault, before_vault - 300,
        "Withdrawal must decrease vault by withdrawn amount");
}

// === Level 2: Small bounded symbolic tests ===

/// I1 with bounded symbolic deficit
#[kani::proof]
fn i1_bounded_deficit() {
    let account = Account {
        principal: 1000,
        pnl_ledger: 500,
        reserved_pnl: 0,
        warmup_state: Warmup { started_at_slot: 0, slope_per_step: 10 },
        position_size: 0,
    };

    let mut users = arrayvec::ArrayVec::<Account, 6>::new();
    users.push(account);

    let state = State {
        vault: 1500,
        insurance_fund: 0,
        fees_outstanding: 0,
        users,
        params: Params { max_users: 6, withdraw_cap_per_step: 1000, maintenance_margin_bps: 50_000 },
        authorized_router: true,
    };

    // Bounded symbolic deficit (0-255)
    let deficit: u8 = kani::any();

    let before = state.clone();
    let after = socialize_losses(state, deficit as u128);

    assert_eq!(before.users[0].principal, after.users[0].principal,
        "I1: Principal must never change regardless of deficit amount");
}

/// Deposit with bounded symbolic amount
#[kani::proof]
fn deposit_bounded_amount() {
    let account = Account {
        principal: 1000,
        pnl_ledger: 0,
        reserved_pnl: 0,
        warmup_state: Warmup { started_at_slot: 0, slope_per_step: 10 },
        position_size: 0,
    };

    let mut users = arrayvec::ArrayVec::<Account, 6>::new();
    users.push(account);

    let state = State {
        vault: 1000,
        insurance_fund: 0,
        fees_outstanding: 0,
        users,
        params: Params { max_users: 6, withdraw_cap_per_step: 1000, maintenance_margin_bps: 50_000 },
        authorized_router: true,
    };

    // Bounded symbolic amount (0-255)
    let amount: u8 = kani::any();

    let before_principal = state.users[0].principal;
    let before_vault = state.vault;

    let after = deposit(state, 0, amount as u128);

    // Monotonicity checks
    kani::assert(after.users[0].principal >= before_principal,
        "Deposit must not decrease principal");
    kani::assert(after.vault >= before_vault,
        "Deposit must not decrease vault");
}
