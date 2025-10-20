//! Anti-toxicity mechanisms for protecting LPs
//!
//! Implements:
//! - Kill band: Reject commits if price has moved too much
//! - JIT penalty: Reduce rebates for just-in-time liquidity
//! - ARG (Aggressor Roundtrip Guard): Tax sandwich-like patterns

use crate::state::SlabState;
use percolator_common::*;

/// Check if price has moved beyond kill band threshold
///
/// Kill band protects against execution at stale prices when the market
/// has moved significantly since the reservation was made.
///
/// # Arguments
/// * `current_price` - Current index/mark price
/// * `reservation_price` - Price at reservation time
/// * `kill_band_bps` - Maximum allowed price movement in basis points
///
/// # Returns
/// * `Ok(())` if within kill band
/// * `Err(PercolatorError::KillBandExceeded)` if price moved too much
pub fn check_kill_band(
    current_price: u64,
    reservation_price: u64,
    kill_band_bps: u64,
) -> Result<(), PercolatorError> {
    if kill_band_bps == 0 {
        return Ok(()); // Kill band disabled
    }

    // Calculate price change as |current/reservation - 1| in basis points
    let price_change_bps = if current_price > reservation_price {
        let numerator = (current_price - reservation_price) as u128 * 10_000;
        let denominator = reservation_price as u128;
        (numerator / denominator) as u64
    } else {
        let numerator = (reservation_price - current_price) as u128 * 10_000;
        let denominator = reservation_price as u128;
        (numerator / denominator) as u64
    };

    if price_change_bps > kill_band_bps {
        return Err(PercolatorError::KillBandExceeded);
    }

    Ok(())
}

/// Check if an order qualifies for JIT penalty
///
/// JIT (Just-In-Time) penalty applies when makers post liquidity
/// after the batch window opens, attempting to front-run known flow.
///
/// # Arguments
/// * `order_created_ms` - Timestamp when order was created
/// * `batch_open_ms` - Timestamp when current batch opened
/// * `jit_penalty_on` - Whether JIT penalty is enabled
///
/// # Returns
/// * `true` if order should incur JIT penalty (no rebate)
/// * `false` if order was posted before batch (eligible for rebate)
#[inline]
pub fn is_jit_order(
    order_created_ms: u64,
    batch_open_ms: u64,
    jit_penalty_on: bool,
) -> bool {
    jit_penalty_on && order_created_ms >= batch_open_ms
}

/// Update aggressor ledger for ARG tracking
///
/// Tracks buy and sell activity within a batch to detect roundtrip patterns
/// that could indicate sandwich attacks.
///
/// # Arguments
/// * `slab` - Mutable slab state
/// * `account_idx` - Account making the aggressive trade
/// * `instrument_idx` - Instrument being traded
/// * `side` - Side of the trade
/// * `qty` - Quantity filled
/// * `price` - Execution price
/// * `current_epoch` - Current batch epoch
pub fn update_aggressor_ledger(
    slab: &mut SlabState,
    account_idx: u32,
    instrument_idx: u16,
    side: Side,
    qty: u64,
    price: u64,
    current_epoch: u16,
) -> Result<(), PercolatorError> {
    // Find existing entry for this (account, instrument, epoch)
    let mut found_idx = None;

    for i in 0..slab.aggressor_ledger.items.len() {
        if let Some(entry) = slab.aggressor_ledger.get(i as u32) {
            if entry.account_idx == account_idx
                && entry.instrument_idx == instrument_idx
                && entry.epoch == current_epoch
            {
                found_idx = Some(i as u32);
                break;
            }
        }
    }

    let notional = mul_u64(qty, price);

    if let Some(idx) = found_idx {
        // Update existing entry
        if let Some(entry) = slab.aggressor_ledger.get_mut(idx) {
            match side {
                Side::Buy => {
                    entry.buy_qty = entry.buy_qty.saturating_add(qty);
                    entry.buy_notional = entry.buy_notional.saturating_add(notional);
                }
                Side::Sell => {
                    entry.sell_qty = entry.sell_qty.saturating_add(qty);
                    entry.sell_notional = entry.sell_notional.saturating_add(notional);
                }
            }
        }
    } else {
        // Create new entry
        let new_idx = slab
            .aggressor_ledger
            .alloc()
            .ok_or(PercolatorError::PoolFull)?;

        if let Some(entry) = slab.aggressor_ledger.get_mut(new_idx) {
            *entry = AggressorEntry {
                account_idx,
                instrument_idx,
                epoch: current_epoch,
                buy_qty: if side == Side::Buy { qty } else { 0 },
                buy_notional: if side == Side::Buy { notional } else { 0 },
                sell_qty: if side == Side::Sell { qty } else { 0 },
                sell_notional: if side == Side::Sell { notional } else { 0 },
                used: true,
                _padding: [0; 7],
            };
        }
    }

    Ok(())
}

/// Calculate ARG sandwich tax if applicable
///
/// ARG (Aggressor Roundtrip Guard) detects when an account buys and sells
/// within the same batch, potentially indicating a sandwich attack.
/// If detected, a tax is applied to the overlapping portion.
///
/// # Arguments
/// * `slab` - Slab state (for ledger lookup)
/// * `account_idx` - Account being checked
/// * `instrument_idx` - Instrument being traded
/// * `current_epoch` - Current batch epoch
/// * `as_fee_k` - Anti-sandwich fee factor in basis points
///
/// # Returns
/// * Sandwich tax amount (0 if no roundtrip detected)
pub fn calculate_arg_tax(
    slab: &SlabState,
    account_idx: u32,
    instrument_idx: u16,
    current_epoch: u16,
    as_fee_k: u64,
) -> u128 {
    if as_fee_k == 0 {
        return 0; // ARG disabled
    }

    // Find aggressor entry for this account/instrument/epoch
    for i in 0..slab.aggressor_ledger.items.len() {
        if let Some(entry) = slab.aggressor_ledger.get(i as u32) {
            if entry.account_idx == account_idx
                && entry.instrument_idx == instrument_idx
                && entry.epoch == current_epoch
            {
                // Check if there's roundtrip activity (both buy and sell)
                if entry.buy_qty > 0 && entry.sell_qty > 0 {
                    // Calculate overlap
                    let overlap_qty = core::cmp::min(entry.buy_qty, entry.sell_qty);

                    // Calculate average price for overlap
                    let buy_avg = if entry.buy_qty > 0 {
                        entry.buy_notional / (entry.buy_qty as u128)
                    } else {
                        0
                    };

                    let sell_avg = if entry.sell_qty > 0 {
                        entry.sell_notional / (entry.sell_qty as u128)
                    } else {
                        0
                    };

                    // Tax based on the notional of the overlap
                    let overlap_notional = (overlap_qty as u128) * core::cmp::max(buy_avg, sell_avg);
                    let tax = (overlap_notional * (as_fee_k as u128)) / 10_000;

                    return tax;
                }
                break;
            }
        }
    }

    0
}

/// Clean up stale aggressor ledger entries from old epochs
///
/// Should be called at batch_open to free up pool space
pub fn cleanup_stale_aggressor_entries(
    slab: &mut SlabState,
    current_epoch: u16,
) -> Result<(), PercolatorError> {
    // Free entries from epochs before current_epoch
    let len = slab.aggressor_ledger.items.len();
    for i in (0..len).rev() {
        let should_free = if let Some(entry) = slab.aggressor_ledger.get(i as u32) {
            entry.epoch < current_epoch
        } else {
            false
        };

        if should_free {
            slab.aggressor_ledger.free(i as u32);
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kill_band_within_threshold() {
        // 1% move should pass with 100 bps kill band
        assert!(check_kill_band(50_500, 50_000, 100).is_ok());

        // 0.5% move should pass
        assert!(check_kill_band(50_250, 50_000, 100).is_ok());
    }

    #[test]
    fn test_kill_band_exceeded() {
        // 2% move should fail with 50 bps (0.5%) kill band
        assert!(check_kill_band(51_000, 50_000, 50).is_err());

        // Downward move should also be checked
        assert!(check_kill_band(49_000, 50_000, 50).is_err());
    }

    #[test]
    fn test_kill_band_disabled() {
        // Should always pass when kill_band_bps = 0
        assert!(check_kill_band(100_000, 50_000, 0).is_ok());
    }

    #[test]
    fn test_jit_order_detection() {
        // Order created before batch_open should not be JIT
        assert!(!is_jit_order(1000, 2000, true));

        // Order created at or after batch_open should be JIT
        assert!(is_jit_order(2000, 2000, true));
        assert!(is_jit_order(2001, 2000, true));

        // JIT penalty disabled
        assert!(!is_jit_order(2001, 2000, false));
    }
}

