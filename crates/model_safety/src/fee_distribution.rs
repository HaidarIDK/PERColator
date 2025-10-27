//! Index-based fee distribution system
//!
//! This module implements a scan-free, O(1) fee distribution algorithm that:
//! - Loss-first: Fees offset socialized losses before distribution
//! - Winners-only: Only accounts with positive vested PnL receive fees
//! - Index-based: Uses global fee_index to track fees per unit vested PnL
//! - Lazy reconciliation: Global sum updated incrementally, no scans required
//!
//! Fixed-point scaling: All index values scaled by FEE_SCALE (1e6) for precision

use crate::state::*;
use crate::math::*;
use crate::warmup::effective_positive_pnl;

/// Fixed-point scaling factor for fee index (1e6)
pub const FEE_SCALE: u128 = 1_000_000;

/// Calculate vested positive PnL for an account
/// Uses effective_positive_pnl which already handles warmup logic
#[inline]
pub fn vested_positive_pnl(acc: &Account) -> u128 {
    effective_positive_pnl(acc)
}

/// Process incoming fees: offset losses first, then distribute remainder
///
/// This is an O(1) operation that:
/// 1. Covers socialized losses first (loss_accum)
/// 2. Distributes remaining fees by updating the global fee_index
/// 3. Handles edge case when no winners exist (carries fees forward)
///
/// # Arguments
/// * `s` - Global state (contains loss_accum, fee_index, sum_vested_pos_pnl, fee_carry)
/// * `fees` - Incoming fee amount (in base units)
///
/// # Returns
/// (covered, distributable) - Amount used to cover losses and amount distributed
pub fn on_fees(s: &mut State, fees: u128) -> (u128, u128) {
    if fees == 0 {
        return (0, 0);
    }

    // Step 1: Cover losses first (loss-first property)
    let cover = if fees > s.loss_accum {
        s.loss_accum
    } else {
        fees
    };
    s.loss_accum = sub_u128(s.loss_accum, cover);

    // Step 2: Calculate distributable amount
    let distributable = sub_u128(fees, cover);

    if distributable > 0 {
        // Add any carried fees from previous rounds
        let total_distributable = add_u128(distributable, s.fee_carry);
        s.fee_carry = 0;

        if s.sum_vested_pos_pnl > 0 {
            // Update global fee index (fees per unit vested PnL)
            // index_delta = distributable / sum_vested_pos_pnl (scaled by FEE_SCALE)
            // Using checked division to prevent overflow
            let numerator = mul_u128(total_distributable, FEE_SCALE);
            let index_delta = div_u128(numerator, s.sum_vested_pos_pnl);
            s.fee_index = add_u128(s.fee_index, index_delta);
        } else {
            // No winners yet; carry forward for next round
            s.fee_carry = total_distributable;
        }
    }

    (cover, distributable)
}

/// Touch an account: accrue fees from index delta and reconcile global sum
///
/// This is an O(1) operation that:
/// 1. Accrues fees based on the delta between global and user fee_index
/// 2. Updates the user's fee_index_user snapshot
/// 3. Lazily reconciles the user's contribution to sum_vested_pos_pnl
///
/// # Arguments
/// * `s` - Global state
/// * `acc` - Account to touch
///
/// # Returns
/// Amount of fees credited to the account in this touch
pub fn on_touch(s: &mut State, acc: &mut Account) -> u128 {
    let vested_now = vested_positive_pnl(acc);

    // Step 1: Accrue fees from index delta (winners-only property)
    let mut credited = 0u128;
    if vested_now > 0 {
        let index_delta = sub_u128(s.fee_index, acc.fee_index_user);
        if index_delta > 0 {
            // Calculate raw fee accrual: delta * vested_pnl / FEE_SCALE
            let raw = mul_u128(index_delta, vested_now);
            let credit = div_u128(raw, FEE_SCALE);

            if credit > 0 {
                acc.fee_accrued = add_u128(acc.fee_accrued, credit);
                credited = credit;
            }
        }
    }

    // Update user's index snapshot
    acc.fee_index_user = s.fee_index;

    // Step 2: Reconcile global sum lazily (no-subsidy property)
    let prev_snapshot = acc.vested_pos_snapshot;
    if vested_now != prev_snapshot {
        if vested_now > prev_snapshot {
            // Vested PnL increased
            let delta = sub_u128(vested_now, prev_snapshot);
            s.sum_vested_pos_pnl = add_u128(s.sum_vested_pos_pnl, delta);
        } else {
            // Vested PnL decreased (saturating subtraction to handle edge cases)
            let delta = sub_u128(prev_snapshot, vested_now);
            s.sum_vested_pos_pnl = sub_u128(s.sum_vested_pos_pnl, delta);
        }
        acc.vested_pos_snapshot = vested_now;
    }

    credited
}

/// Claim accrued fees for an account
///
/// Transfers fee_accrued from the account to their principal.
/// This is typically called during withdrawal or position closure.
///
/// # Arguments
/// * `s` - Global state (for potential future checks)
/// * `acc` - Account claiming fees
///
/// # Returns
/// Amount of fees claimed
pub fn claim_fees(s: &State, acc: &mut Account) -> u128 {
    let claimed = acc.fee_accrued;
    if claimed > 0 {
        acc.fee_accrued = 0;
        // In production, this would transfer to principal or vault
        // For the model, we just return the amount
    }
    claimed
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vested_positive_pnl_zero_for_negative() {
        let mut acc = Account::default();
        acc.pnl_ledger = -1000;
        assert_eq!(vested_positive_pnl(&acc), 0);
    }

    #[test]
    fn test_vested_positive_pnl_positive() {
        let mut acc = Account::default();
        acc.pnl_ledger = 1000;
        assert_eq!(vested_positive_pnl(&acc), 1000);
    }

    #[test]
    fn test_on_fees_loss_first() {
        let mut s = State::default();
        s.loss_accum = 500;
        s.sum_vested_pos_pnl = 1000;

        let (covered, distributable) = on_fees(&mut s, 300);

        assert_eq!(covered, 300); // All fees went to covering losses
        assert_eq!(distributable, 0);
        assert_eq!(s.loss_accum, 200); // 500 - 300
        assert_eq!(s.fee_index, 0); // No index update since no distributable
    }

    #[test]
    fn test_on_fees_with_distribution() {
        let mut s = State::default();
        s.loss_accum = 100;
        s.sum_vested_pos_pnl = 1_000_000; // 1 unit at FEE_SCALE

        let (covered, distributable) = on_fees(&mut s, 600);

        assert_eq!(covered, 100);
        assert_eq!(distributable, 500);
        assert_eq!(s.loss_accum, 0);
        // fee_index should increase by (500 * FEE_SCALE) / 1_000_000 = 500
        assert_eq!(s.fee_index, 500);
    }

    #[test]
    fn test_on_touch_accrues_fees() {
        let mut s = State::default();
        s.fee_index = 1000; // Global index at 1000
        s.sum_vested_pos_pnl = 0; // Will be updated by on_touch

        let mut acc = Account::default();
        acc.pnl_ledger = 1_000_000; // 1 unit vested PnL
        acc.fee_index_user = 500; // User last synced at 500

        let credited = on_touch(&mut s, &mut acc);

        // Index delta = 1000 - 500 = 500
        // Credit = (500 * 1_000_000) / FEE_SCALE = 500
        assert_eq!(credited, 500);
        assert_eq!(acc.fee_accrued, 500);
        assert_eq!(acc.fee_index_user, 1000);
        assert_eq!(s.sum_vested_pos_pnl, 1_000_000);
    }

    #[test]
    fn test_on_touch_no_subsidy_to_losers() {
        let mut s = State::default();
        s.fee_index = 1000;

        let mut acc = Account::default();
        acc.pnl_ledger = -500; // Loser
        acc.fee_index_user = 0;

        let credited = on_touch(&mut s, &mut acc);

        assert_eq!(credited, 0); // Losers get no fees
        assert_eq!(acc.fee_accrued, 0);
    }
}
