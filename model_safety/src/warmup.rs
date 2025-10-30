//! PnL withdrawal warm-up logic

use crate::state::*;
use crate::math::*;

/// Calculate withdrawable PnL based on warm-up period (I5)
pub fn withdrawable_pnl(acc: &Account, steps_since_start: u32, per_step: u128) -> u128 {
    let cap = mul_u128(steps_since_start as u128, per_step);
    min_u128(cap, effective_positive_pnl(acc))
}

/// Calculate effective positive PnL (positive PnL minus reserved)
pub fn effective_positive_pnl(acc: &Account) -> u128 {
    let pos = clamp_pos_i128(acc.pnl_ledger);
    sub_u128(pos, acc.reserved_pnl)
}
