//! Invariant checking helpers

use crate::state::*;
use crate::math::*;

/// I2: Conservation - vault balance equals sum of principals + insurance - fees
pub fn conservation_ok(s: &State) -> bool {
    let sum_principal = s.users.iter().fold(0u128, |acc, u| add_u128(acc, u.principal));

    // vault should equal: principals + insurance - fees
    // But we also need to account for PnL in the vault
    // Simplified model: vault == sum(principal) + insurance - fees + sum(positive_pnl)
    let sum_pos_pnl = s.users.iter().fold(0u128, |acc, u| {
        let pos_pnl = clamp_pos_i128(u.pnl_ledger);
        add_u128(acc, pos_pnl)
    });

    let expected_vault = add_u128(
        add_u128(sum_principal, s.insurance_fund),
        sum_pos_pnl
    );
    let expected_vault = sub_u128(expected_vault, s.fees_outstanding);

    s.vault == expected_vault
}

/// I1: Principals unchanged between two states
pub fn principals_unchanged(before: &State, after: &State) -> bool {
    if before.users.len() != after.users.len() {
        return false;
    }
    before.users.iter().zip(after.users.iter())
        .all(|(a, b)| a.principal == b.principal)
}

/// I4: Haircut only hits winners (positive PnL accounts)
pub fn winners_only_haircut(before: &State, after: &State) -> bool {
    if before.users.len() != after.users.len() {
        return false;
    }
    before.users.iter().zip(after.users.iter()).all(|(a, b)| {
        if b.pnl_ledger < a.pnl_ledger {
            // PnL decreased - must have been positive before
            a.pnl_ledger > 0
        } else {
            true
        }
    })
}

/// I4: Calculate total haircut applied
pub fn total_haircut(before: &State, after: &State) -> u128 {
    if before.users.len() != after.users.len() {
        return 0;
    }
    before.users.iter().zip(after.users.iter()).fold(0u128, |acc, (a, b)| {
        if b.pnl_ledger < a.pnl_ledger {
            let cut = sub_i128(a.pnl_ledger, b.pnl_ledger);
            add_u128(acc, clamp_pos_i128(cut))
        } else {
            acc
        }
    })
}

/// I4: Calculate sum of effective positive PnL across all users
pub fn sum_effective_winners(s: &State) -> u128 {
    s.users.iter().fold(0u128, |acc, u| {
        let eff = crate::warmup::effective_positive_pnl(u);
        add_u128(acc, eff)
    })
}

/// I6: Balances unchanged (vault and all user balances)
pub fn balances_unchanged(before: &State, after: &State) -> bool {
    if before.vault != after.vault {
        return false;
    }
    if before.users.len() != after.users.len() {
        return false;
    }
    before.users.iter().zip(after.users.iter()).all(|(a, b)| {
        a.principal == b.principal && a.pnl_ledger == b.pnl_ledger
    })
}

// ============================================================================
// Liquidation Helpers
// ============================================================================

use crate::state::Prices;

/// Check if an account is liquidatable
/// An account is liquidatable if:
///   collateral_value < position_size * maintenance_margin
/// Where collateral_value = principal + max(0, pnl_ledger)
pub fn is_liquidatable(acc: &Account, _prices: &Prices, params: &Params) -> bool {
    // Calculate collateral value
    let collateral = add_u128(acc.principal, clamp_pos_i128(acc.pnl_ledger));

    // Calculate required margin: position_size * maintenance_margin_bps / 1_000_000
    // Using safe math to avoid overflow
    let position_value = acc.position_size;

    // If no position, not liquidatable
    if position_value == 0 {
        return false;
    }

    // Required margin = position * margin_bps / 1_000_000
    // To avoid overflow, check: collateral * 1_000_000 < position * margin_bps
    let collateral_scaled = mul_u128(collateral, 1_000_000);
    let required_margin_scaled = mul_u128(position_value, params.maintenance_margin_bps as u128);

    collateral_scaled < required_margin_scaled
}

/// Count liquidatable accounts in the state
pub fn liquidatable_count(s: &State, prices: &Prices) -> u8 {
    let mut count = 0u8;
    for acc in s.users.iter() {
        if is_liquidatable(acc, prices, &s.params) {
            count = count.saturating_add(1);
        }
    }
    count
}

/// Check if state is valid for liquidation operations
/// (auth enabled, compute units fit, etc.)
pub fn valid_for_liquidation(s: &State, _prices: &Prices) -> bool {
    // In the simplified model, just check authorization
    s.authorized_router
}

/// Choose a liquidatable account index (simple strategy: first liquidatable)
pub fn choose_liquidatable_index(s: &State, prices: &Prices) -> usize {
    for (i, acc) in s.users.iter().enumerate() {
        if is_liquidatable(acc, prices, &s.params) {
            return i;
        }
    }
    // Fallback to 0 if none found (caller should check count > 0 first)
    0
}
