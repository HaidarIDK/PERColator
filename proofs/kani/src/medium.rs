//! Medium-complexity Kani proofs using parameterized concrete scenarios
//! Fixed state structures with small symbolic inputs for tractable verification

use model_safety::{state::*, transitions::*, helpers::*};
use arrayvec::ArrayVec;

// === Helper Functions: Build Concrete States ===

/// Create 1-user state with specified values
pub fn make_1user_state(principal: u128, pnl: i128, slope: u128) -> State {
    let account = Account {
        principal,
        pnl_ledger: pnl,
        reserved_pnl: 0,
        warmup_state: Warmup {
            started_at_slot: 0,
            slope_per_step: slope,
        },
        position_size: 0,
    };

    let mut users = ArrayVec::new();
    users.push(account);

    // Set vault to maintain conservation
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

/// Create 2-user state: winner + loser
pub fn make_2user_winner_loser(
    principal1: u128,
    pnl_winner: i128,
    principal2: u128,
    pnl_loser: i128,
) -> State {
    let user1 = Account {
        principal: principal1,
        pnl_ledger: pnl_winner,
        reserved_pnl: 0,
        warmup_state: Warmup { started_at_slot: 0, slope_per_step: 10 },
        position_size: 0,
    };

    let user2 = Account {
        principal: principal2,
        pnl_ledger: pnl_loser,
        reserved_pnl: 0,
        warmup_state: Warmup { started_at_slot: 0, slope_per_step: 10 },
        position_size: 0,
    };

    let mut users = ArrayVec::new();
    users.push(user1);
    users.push(user2);

    // Calculate vault for conservation
    let sum_principal = principal1.saturating_add(principal2);
    let sum_pos_pnl = {
        let p1 = if pnl_winner > 0 { pnl_winner as u128 } else { 0 };
        let p2 = if pnl_loser > 0 { pnl_loser as u128 } else { 0 };
        p1.saturating_add(p2)
    };
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

/// Create 2-user state: both winners with different PnL
pub fn make_2user_both_winners(principal1: u128, pnl1: i128, principal2: u128, pnl2: i128) -> State {
    make_2user_winner_loser(principal1, pnl1, principal2, pnl2)
}

/// Apply bounded adversarial step (simplified, deterministic based on op)
pub fn bounded_adversary_step(mut state: State, op: u8) -> State {
    if state.users.is_empty() {
        return state;
    }

    let choice = op % 6;
    let uid = (op as usize) % state.users.len();
    let amount = (op as u128) * 10; // Small amounts

    match choice {
        0 => deposit(state, uid, amount),
        1 => withdraw_principal(state, uid, amount),
        2 => withdraw_pnl(state, uid, amount, op as u32),
        3 => socialize_losses(state, amount),
        4 => {
            // Trade settle (add PnL)
            if uid < state.users.len() {
                state.users[uid].pnl_ledger = state.users[uid].pnl_ledger.saturating_add(amount as i128);
            }
            state
        }
        5 => matcher_noise(state),
        _ => state,
    }
}

/// Calculate how much was withdrawn (for throttle checks)
pub fn calculate_withdrawn(before: &State, after: &State, uid: usize) -> u128 {
    if uid >= before.users.len() || uid >= after.users.len() {
        return 0;
    }

    let before_pnl = before.users[uid].pnl_ledger;
    let after_pnl = after.users[uid].pnl_ledger;

    if before_pnl > after_pnl {
        (before_pnl - after_pnl) as u128
    } else {
        0
    }
}

// === Medium Complexity Proofs ===

/// I2: Conservation with 2 users, deposit then withdraw
#[kani::proof]
fn i2_conservation_2users_deposit_withdraw() {
    // Concrete: 2 users (winner + loser)
    let state = make_2user_winner_loser(1000, 500, 1000, -200);

    // Symbolic amounts
    let deposit_amt: u8 = kani::any();
    let withdraw_amt: u8 = kani::any();

    let initial_vault = state.vault;

    // User 0 deposits
    let s1 = deposit(state, 0, deposit_amt as u128);

    // User 1 withdraws (bounded)
    let withdraw_bounded = (withdraw_amt as u128) % 500; // Don't exceed principal
    let s2 = withdraw_principal(s1, 1, withdraw_bounded);

    // Basic sanity: no overflow
    kani::assert(s2.vault < u128::MAX, "I2: Vault must not overflow");

    // Vault change should match operations
    let expected_change = (deposit_amt as i128) - (withdraw_bounded as i128);
    let actual_change = (s2.vault as i128) - (initial_vault as i128);

    // Should be close (accounting for saturation)
    kani::assert(
        (actual_change - expected_change).abs() <= 1,
        "I2: Vault change matches operations"
    );
}

/// I2: Conservation with deposit, socialization, withdrawal
#[kani::proof]
fn i2_conservation_deposit_socialize_withdraw() {
    let state = make_1user_state(1000, 500, 10);

    let deposit_amt: u8 = kani::any();
    let deficit: u8 = kani::any();
    let withdraw_amt: u8 = kani::any();

    let initial_vault = state.vault;

    // Deposit
    let s1 = deposit(state, 0, deposit_amt as u128);

    // Socialize losses (haircut PnL)
    let s2 = socialize_losses(s1, deficit as u128);

    // Withdraw principal (bounded)
    let withdraw_bounded = (withdraw_amt as u128) % 500;
    let s3 = withdraw_principal(s2, 0, withdraw_bounded);

    // Basic sanity checks
    kani::assert(s3.vault < u128::MAX, "I2: No overflow");
    kani::assert(s3.vault < 10_000_000, "I2: Vault stays bounded");

    // Vault shouldn't decrease by more than we withdrew
    if s3.vault < initial_vault {
        let decrease = initial_vault - s3.vault;
        kani::assert(
            decrease <= withdraw_bounded + deficit as u128,
            "I2: Vault decrease bounded by operations"
        );
    }
}

/// I4: Bounded socialization with 2 users, symbolic deficit
#[kani::proof]
fn i4_socialization_2users_symbolic_deficit() {
    // Concrete: winner (500 PnL) + loser (-200 PnL)
    let state = make_2user_winner_loser(1000, 500, 1000, -200);

    // Symbolic deficit (0-1023)
    let deficit_raw: u16 = kani::any();
    let deficit = (deficit_raw % 1024) as u128;

    let before = state.clone();
    let after = socialize_losses(state, deficit);

    // I4: Winners only get haircutted
    kani::assert(winners_only_haircut(&before, &after), "I4: Only winners haircutted");

    // I4: Total haircut bounded by min(deficit, sum_effective_winners)
    let sum_eff = sum_effective_winners(&before);
    let haircut = total_haircut(&before, &after);
    let expected_max = if deficit < sum_eff { deficit } else { sum_eff };

    kani::assert(haircut <= expected_max, "I4: Total haircut bounded correctly");

    // I1: Principals unchanged
    kani::assert(principals_unchanged(&before, &after), "I1: Principals intact during socialization");
}

/// I4: Socialization with both winners (different PnL)
#[kani::proof]
fn i4_socialization_both_winners() {
    // Two winners: user0 has 500 PnL, user1 has 300 PnL
    let state = make_2user_both_winners(1000, 500, 1000, 300);

    let deficit_raw: u8 = kani::any();
    let deficit = (deficit_raw as u128) * 5; // 0-1275

    let before = state.clone();
    let after = socialize_losses(state, deficit);

    kani::assert(winners_only_haircut(&before, &after), "I4: Winners only");

    // Both should be haircutted proportionally (or up to their effective PnL)
    let haircut = total_haircut(&before, &after);
    let sum_eff = sum_effective_winners(&before);
    let expected_max = if deficit < sum_eff { deficit } else { sum_eff };

    kani::assert(haircut <= expected_max, "I4: Haircut bounded");
}

/// I5: Withdraw throttle with symbolic step and amount
#[kani::proof]
fn i5_throttle_symbolic_step_and_amount() {
    // Concrete 1-user state with warmup slope=10
    let state = make_1user_state(1000, 500, 10);

    // Symbolic: withdrawal step (0-15) and amount (0-255)
    let step_raw: u8 = kani::any();
    let step = (step_raw % 16) as u32; // 0-15
    let amount: u8 = kani::any();

    let before = state.clone();
    let after = withdraw_pnl(state, 0, amount as u128, step);

    // Calculate allowed withdrawal based on warmup
    let max_allowed = (step as u128).saturating_mul(10); // slope=10

    // Calculate actual withdrawal
    let withdrawn = calculate_withdrawn(&before, &after, 0);

    // I5: Withdrawal must respect warm-up throttle (allow +1 for rounding)
    kani::assert(
        withdrawn <= max_allowed.saturating_add(1),
        "I5: Withdrawal respects throttle"
    );

    // I5: Vault decreases (or stays same)
    kani::assert(after.vault <= before.vault, "I5: Vault decreases on withdrawal");
}

/// I5: Throttle with larger steps
#[kani::proof]
fn i5_throttle_larger_steps() {
    let state = make_1user_state(1000, 500, 20); // Higher slope

    let step_raw: u8 = kani::any();
    let step = (step_raw % 32) as u32; // 0-31
    let amount_raw: u8 = kani::any();
    let amount = (amount_raw as u128) * 2; // 0-510

    let before = state.clone();
    let after = withdraw_pnl(state, 0, amount, step);

    let max_allowed = (step as u128).saturating_mul(20); // slope=20
    let withdrawn = calculate_withdrawn(&before, &after, 0);

    kani::assert(
        withdrawn <= max_allowed.saturating_add(1),
        "I5: Throttle with slope=20"
    );
}

/// Deposit with 2 users, symbolic amounts
#[kani::proof]
fn deposit_2users_symbolic() {
    let state = make_2user_winner_loser(1000, 500, 1000, -200);

    let user_id: u8 = kani::any();
    let uid = (user_id % 2) as usize; // 0 or 1
    let amount: u8 = kani::any();

    let before_principal = state.users[uid].principal;
    let before_vault = state.vault;

    let after = deposit(state, uid, amount as u128);

    // Monotonicity
    kani::assert(
        after.users[uid].principal >= before_principal,
        "Deposit must not decrease principal"
    );
    kani::assert(after.vault >= before_vault, "Deposit must not decrease vault");

    // Exact equality (when no saturation)
    if before_principal < u128::MAX - (amount as u128) {
        kani::assert(
            after.users[uid].principal == before_principal + (amount as u128),
            "Deposit increases principal by exact amount"
        );
    }
}

/// Withdrawal with 2 users, symbolic amounts
#[kani::proof]
fn withdrawal_2users_symbolic() {
    let state = make_2user_winner_loser(1000, 500, 1000, -200);

    let user_id: u8 = kani::any();
    let uid = (user_id % 2) as usize;
    let amount_raw: u8 = kani::any();
    let amount = (amount_raw as u128) % 500; // Don't exceed principal

    let before_principal = state.users[uid].principal;
    let before_vault = state.vault;

    let after = withdraw_principal(state, uid, amount);

    let after_principal = after.users[uid].principal;
    let after_vault = after.vault;

    // Principal decreases
    kani::assert(after_principal <= before_principal, "Withdrawal decreases principal");

    // Vault decreases by same amount as principal
    let principal_withdrawn = before_principal.saturating_sub(after_principal);
    let vault_decrease = before_vault.saturating_sub(after_vault);

    kani::assert(
        vault_decrease == principal_withdrawn,
        "Vault decreases by withdrawn principal amount"
    );
}

/// I3: Multi-user unauthorized operations
#[kani::proof]
fn i3_multiuser_unauthorized() {
    let mut state = make_2user_winner_loser(1000, 500, 1000, -200);
    state.authorized_router = false; // Disable auth

    let before = state.clone();

    // Try various operations on both users
    let uid: u8 = kani::any();
    let user_idx = (uid % 2) as usize;
    let amount: u8 = kani::any();

    // Try deposit - should fail
    let after_deposit = deposit(state.clone(), user_idx, amount as u128);
    kani::assert(
        balances_unchanged(&before, &after_deposit),
        "I3: Unauthorized deposit cannot change balances"
    );

    // Try withdrawal - should fail
    let after_withdraw = withdraw_principal(state.clone(), user_idx, amount as u128);
    kani::assert(
        balances_unchanged(&before, &after_withdraw),
        "I3: Unauthorized withdrawal cannot change balances"
    );

    // Try socialization - should fail
    let after_socialize = socialize_losses(state, amount as u128);
    kani::assert(
        balances_unchanged(&before, &after_socialize),
        "I3: Unauthorized socialization cannot change balances"
    );
}

/// I1: Principal inviolability across multiple operations
#[kani::proof]
fn i1_principal_inviolability_multi_ops() {
    let state = make_2user_winner_loser(1000, 500, 1000, -200);

    let deficit1: u8 = kani::any();
    let deficit2: u8 = kani::any();

    let before = state.clone();

    // Apply two socializations
    let s1 = socialize_losses(state, deficit1 as u128);
    let s2 = socialize_losses(s1, deficit2 as u128);

    // Principals must be unchanged after both
    kani::assert(
        principals_unchanged(&before, &s2),
        "I1: Principals unchanged after multiple socializations"
    );
}

/// I6: Matcher cannot move funds with symbolic noise
#[kani::proof]
fn i6_matcher_symbolic_2users() {
    let state = make_2user_winner_loser(1000, 500, 1000, -200);

    let before = state.clone();
    let after = matcher_noise(state);

    kani::assert(
        balances_unchanged(&before, &after),
        "I6: Matcher cannot move funds"
    );

    // Also check principals specifically
    kani::assert(
        principals_unchanged(&before, &after),
        "I6: Matcher cannot change principals"
    );
}
