//! State transition functions - all total, no panics

use crate::state::*;
use crate::math::*;
use crate::warmup::*;

/// Deposit funds (increases principal and vault)
pub fn deposit(mut s: State, uid: usize, amount: u128) -> State {
    // I3: Check authorization
    if !s.authorized_router {
        return s;
    }

    if uid >= s.users.len() {
        return s;
    }

    // Update principal
    s.users[uid].principal = add_u128(s.users[uid].principal, amount);

    // Update vault to maintain conservation
    s.vault = add_u128(s.vault, amount);

    s
}

/// Trade settlement (updates PnL, maintains conservation)
pub fn trade_settle(mut s: State, uid: usize, realized: i128) -> State {
    // I3: Check authorization
    if !s.authorized_router {
        return s;
    }

    if uid >= s.users.len() {
        return s;
    }

    // Update PnL
    s.users[uid].pnl_ledger = add_i128(s.users[uid].pnl_ledger, realized);

    // Update vault: if positive PnL, vault increases; if negative, vault decreases
    if realized > 0 {
        s.vault = add_u128(s.vault, clamp_pos_i128(realized));
    } else {
        let loss = clamp_pos_i128(sub_i128(0, realized));
        s.vault = sub_u128(s.vault, loss);
    }

    s
}

/// Mark a deficit for socialization (triggers loss event)
pub fn loss_event(s: State, deficit: u128) -> State {
    // Immediately socialize the deficit
    socialize_losses(s, deficit)
}

/// Socialize losses across winners (I1, I4, I2)
pub fn socialize_losses(mut s: State, deficit: u128) -> State {
    // I3: Check authorization
    if !s.authorized_router {
        return s;
    }

    if deficit == 0 {
        return s;
    }

    // Calculate sum of effective positive PnL (I4: cap)
    let total_eff_winners = sum_effective_winners(&s);

    if total_eff_winners == 0 {
        // No winners to socialize to
        return s;
    }

    // Actual haircut is min(deficit, total_eff_winners) (I4: bounded)
    let actual_haircut = min_u128(deficit, total_eff_winners);

    // Distribute haircut proportionally across winners
    let mut remaining = actual_haircut;

    for user in s.users.iter_mut() {
        if user.pnl_ledger <= 0 {
            // I4: Skip losers/zero PnL accounts
            continue;
        }

        let eff_pos = effective_positive_pnl(user);
        if eff_pos == 0 {
            continue;
        }

        // Calculate this user's share: (eff_pos / total_eff) * actual_haircut
        // To avoid overflow: share = (eff_pos * actual_haircut) / total_eff
        let share = if total_eff_winners > 0 {
            div_u128(mul_u128(eff_pos, actual_haircut), total_eff_winners)
        } else {
            0
        };

        let share = min_u128(share, remaining);
        let share = min_u128(share, eff_pos); // Can't take more than they have

        // Apply haircut to PnL (I1: principal untouched!)
        let share_i128 = u128_to_i128(share);
        user.pnl_ledger = sub_i128(user.pnl_ledger, share_i128);

        remaining = sub_u128(remaining, share);

        // Update vault to reflect loss socialization (I2: conservation)
        s.vault = sub_u128(s.vault, share);
    }

    s
}

/// Withdraw principal (I1: allowed, maintains conservation)
pub fn withdraw_principal(mut s: State, uid: usize, amount: u128) -> State {
    // I3: Check authorization
    if !s.authorized_router {
        return s;
    }

    if uid >= s.users.len() {
        return s;
    }

    // Can only withdraw up to principal balance
    let actual_withdraw = min_u128(amount, s.users[uid].principal);

    // Update principal
    s.users[uid].principal = sub_u128(s.users[uid].principal, actual_withdraw);

    // Update vault (I2: conservation)
    s.vault = sub_u128(s.vault, actual_withdraw);

    s
}

/// Withdraw PnL with warm-up throttle (I5: respects warm-up cap)
pub fn withdraw_pnl(mut s: State, uid: usize, amount: u128, current_step: u32) -> State {
    // I3: Check authorization
    if !s.authorized_router {
        return s;
    }

    if uid >= s.users.len() {
        return s;
    }

    let user = &mut s.users[uid];

    // I5: Calculate warm-up cap
    let steps_elapsed = current_step.saturating_sub(user.warmup_state.started_at_slot as u32);
    let max_withdrawable = withdrawable_pnl(user, steps_elapsed, user.warmup_state.slope_per_step);

    // L13: Also limit by margin health - don't allow withdrawals that would trigger liquidation
    let margin_limited_withdraw = if user.position_size > 0 {
        // Calculate current collateral
        let current_collateral = add_u128(user.principal, clamp_pos_i128(user.pnl_ledger));

        // We need to ensure: (current_collateral - withdraw) * 1_000_000 >= position * margin_bps
        // Solving for withdraw: withdraw <= current_collateral - (position * margin_bps / 1_000_000)
        // But we need to be careful with rounding! Use scaled arithmetic like is_liquidatable.

        // Required: collateral_after * 1_000_000 >= position * margin_bps
        // Where: collateral_after = current_collateral - withdraw

        // Safe maximum withdraw that keeps: (collateral - w) * 1M >= pos * bps
        // Rearrange: collateral * 1M - w * 1M >= pos * bps
        // Therefore: w * 1M <= collateral * 1M - pos * bps
        // So: w <= (collateral * 1M - pos * bps) / 1M

        let collateral_scaled = mul_u128(current_collateral, 1_000_000);
        let required_margin_scaled = mul_u128(user.position_size, s.params.maintenance_margin_bps as u128);

        if collateral_scaled > required_margin_scaled {
            // Safe withdraw amount = (collateral_scaled - required_margin_scaled) / 1_000_000
            div_u128(sub_u128(collateral_scaled, required_margin_scaled), 1_000_000)
        } else {
            // Already at or below margin requirement, no withdrawal allowed
            0
        }
    } else {
        // No position, no margin requirement
        max_withdrawable
    };

    // Actual withdrawal is min of requested, warmup-allowed, and margin-safe
    let actual_withdraw = min_u128(min_u128(amount, max_withdrawable), margin_limited_withdraw);

    if actual_withdraw == 0 {
        return s;
    }

    // Reduce PnL (I2: conservation maintained)
    let withdraw_i128 = u128_to_i128(actual_withdraw);
    user.pnl_ledger = sub_i128(user.pnl_ledger, withdraw_i128);

    // Update vault
    s.vault = sub_u128(s.vault, actual_withdraw);

    s
}

/// Tick warm-up state (monotonically increases withdrawal caps)
pub fn tick_warmup(mut s: State, steps: u32) -> State {
    // I3: Check authorization
    if !s.authorized_router {
        return s;
    }

    // Advance all user warm-up states
    for user in s.users.iter_mut() {
        user.warmup_state.started_at_slot =
            user.warmup_state.started_at_slot.saturating_add(steps as u64);
    }

    s
}

/// Matcher noise (I6: cannot move funds)
pub fn matcher_noise(s: State) -> State {
    // By construction, this does nothing to balances
    s
}

// ============================================================================
// Liquidation Transitions
// ============================================================================

use crate::state::Prices;
use crate::helpers::{is_liquidatable, choose_liquidatable_index};

/// Liquidate one account (router chooses first liquidatable)
/// If no accounts are liquidatable, this is a no-op
pub fn liquidate_one(s: State, prices: &Prices) -> State {
    // I3: Check authorization
    if !s.authorized_router {
        return s;
    }

    // Find a liquidatable account
    let uid = choose_liquidatable_index(&s, prices);

    // Verify it's actually liquidatable
    if uid >= s.users.len() || !is_liquidatable(&s.users[uid], prices, &s.params) {
        return s; // No-op if none found
    }

    // Liquidate the account
    liquidate_account(s, uid, prices)
}

/// Liquidate a specific account by index
/// If the account is not liquidatable, this is a no-op
pub fn liquidate_account(mut s: State, uid: usize, prices: &Prices) -> State {
    // I3: Check authorization
    if !s.authorized_router {
        return s;
    }

    if uid >= s.users.len() {
        return s;
    }

    // Check if account is actually liquidatable
    if !is_liquidatable(&s.users[uid], prices, &s.params) {
        return s; // No-op
    }

    // Simplified liquidation logic:
    // 1. Close position (set position_size to 0)
    // 2. Realize the loss from PnL if negative
    // 3. Deduct from insurance fund or socialize if needed

    let user = &mut s.users[uid];

    // Close position
    user.position_size = 0;

    // If user has negative PnL, realize the loss
    if user.pnl_ledger < 0 {
        let loss = clamp_pos_i128(sub_i128(0, user.pnl_ledger));

        // Try to cover from insurance fund first
        if s.insurance_fund >= loss {
            s.insurance_fund = sub_u128(s.insurance_fund, loss);
            // Set PnL to 0 (loss covered)
            user.pnl_ledger = 0;
        } else {
            // Insurance fund depleted, socialize remaining loss
            let covered = s.insurance_fund;
            s.insurance_fund = 0;

            // Partially reduce this user's negative PnL by covered amount
            user.pnl_ledger = add_i128(user.pnl_ledger, u128_to_i128(covered));

            // Note: remaining loss would be socialized, but we leave PnL negative for now
            // In a full model, we'd call socialize_losses here
        }
    }

    s
}

/// Liquidate with unauthorized router (should be no-op for L6)
pub fn liquidate_one_unauthorized(mut s: State, prices: &Prices) -> State {
    s.authorized_router = false;
    liquidate_one(s, prices)
}

// Re-export helpers for use in transitions
use crate::helpers::sum_effective_winners;
