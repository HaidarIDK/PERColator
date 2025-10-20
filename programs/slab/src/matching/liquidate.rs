//! Liquidation execution engine
//!
//! Handles forced closure of underwater positions to protect the system
//! Called by the Router's liquidation coordinator

use crate::state::SlabState;
use crate::matching::risk::{calculate_equity, calculate_margin_requirements, is_liquidatable};
use percolator_common::*;

/// Liquidation result
#[derive(Debug, Clone, Copy)]
pub struct LiquidationResult {
    pub closed_qty: i64,
    pub realized_pnl: i128,
    pub closed_notional: u128,
    pub liquidation_fee: u128,
    pub remaining_deficit: u128,
}

/// Execute liquidation for an underwater account
///
/// This is called by the Router when an account is below maintenance margin.
/// The slab will close positions via market orders until the deficit is covered.
///
/// # Arguments
/// * `slab` - Mutable slab state
/// * `account_idx` - Account to liquidate
/// * `deficit_target` - Amount that needs to be recovered (in base currency)
/// * `liquidation_fee_bps` - Fee charged on liquidated notional (e.g., 500 = 5%)
/// * `price_band_bps` - Maximum deviation from mark price (e.g., 300 = 3%)
///
/// # Returns
/// * `Ok(LiquidationResult)` - Details of liquidation
/// * `Err(NotLiquidatable)` - If account is not underwater
pub fn execute_liquidation(
    slab: &mut SlabState,
    account_idx: u32,
    deficit_target: u128,
    liquidation_fee_bps: u16,
    price_band_bps: u16,
) -> Result<LiquidationResult, PercolatorError> {
    // Step 1: Verify account is liquidatable
    if !is_liquidatable(slab, account_idx)? {
        return Err(PercolatorError::BelowMaintenanceMargin);
    }

    // Step 2: Get account's current equity and positions
    let initial_equity = calculate_equity(slab, account_idx)?;
    let (_im, mm) = calculate_margin_requirements(slab, account_idx)?;

    // Calculate actual deficit
    let actual_deficit = if initial_equity < mm as i128 {
        (mm as i128 - initial_equity) as u128
    } else {
        0 // Shouldn't happen given is_liquidatable check
    };

    let target = core::cmp::min(deficit_target, actual_deficit);

    // Step 3: Close positions until target met
    let mut total_closed_notional = 0u128;
    let mut total_realized_pnl = 0i128;
    let mut total_closed_qty = 0i64;
    let mut covered_so_far = 0u128;

    // Iterate through account's positions
    if let Some(account) = slab.get_account(account_idx) {
        let mut pos_idx = account.position_head;
        
        while pos_idx != u32::MAX && covered_so_far < target {
            if let Some(position) = slab.positions.get(pos_idx) {
                let instrument_idx = position.instrument_idx;
                let position_qty = position.qty;
                let next_pos = position.next_in_account;

                // Close this position
                let close_result = close_position(
                    slab,
                    account_idx,
                    instrument_idx,
                    position_qty,
                    price_band_bps,
                )?;

                total_closed_notional = total_closed_notional.saturating_add(close_result.notional);
                total_realized_pnl = total_realized_pnl.saturating_add(close_result.pnl);
                total_closed_qty = total_closed_qty.saturating_add(position_qty.abs());

                // Calculate value recovered (notional minus losses)
                let recovered = if close_result.pnl >= 0 {
                    close_result.notional.saturating_add(close_result.pnl as u128)
                } else {
                    close_result.notional.saturating_sub((-close_result.pnl) as u128)
                };
                
                covered_so_far = covered_so_far.saturating_add(recovered);

                pos_idx = next_pos;
            } else {
                break;
            }
        }
    }

    // Step 4: Calculate liquidation fee
    let liquidation_fee = (total_closed_notional * liquidation_fee_bps as u128) / 10_000;

    // Step 5: Deduct liquidation fee from account
    if let Some(account) = slab.get_account_mut(account_idx) {
        account.cash = account.cash.saturating_sub(liquidation_fee as i128);
    }

    // Step 6: Calculate remaining deficit
    let remaining_deficit = if covered_so_far < target {
        target - covered_so_far
    } else {
        0
    };

    Ok(LiquidationResult {
        closed_qty: total_closed_qty,
        realized_pnl: total_realized_pnl,
        closed_notional: total_closed_notional,
        liquidation_fee,
        remaining_deficit,
    })
}

/// Close result for a single position
struct CloseResult {
    pub notional: u128,
    pub pnl: i128,
}

/// Close a single position via market order
///
/// Walks the contra book and executes against available liquidity within price bands
fn close_position(
    slab: &mut SlabState,
    account_idx: u32,
    instrument_idx: u16,
    position_qty: i64,
    price_band_bps: u16,
) -> Result<CloseResult, PercolatorError> {
    if position_qty == 0 {
        return Ok(CloseResult {
            notional: 0,
            pnl: 0,
        });
    }

    // Determine side for closing (opposite of position)
    let close_side = if position_qty > 0 {
        Side::Sell // Close long position
    } else {
        Side::Buy // Close short position
    };

    let close_qty = position_qty.abs() as u64;

    // Get instrument data
    let instrument = slab
        .get_instrument(instrument_idx)
        .ok_or(PercolatorError::InvalidInstrument)?;

    let mark_price = instrument.index_price;
    let _contract_size = instrument.contract_size;

    // Calculate price band limits
    let band_delta = (mark_price as u128 * price_band_bps as u128) / 10_000;
    let (min_price, max_price) = match close_side {
        Side::Buy => {
            // When buying to close short, cap at mark + band
            let max = mark_price.saturating_add(band_delta as u64);
            (0u64, max)
        }
        Side::Sell => {
            // When selling to close long, floor at mark - band
            let min = mark_price.saturating_sub(band_delta as u64);
            (min, u64::MAX)
        }
    };

    // Walk contra book and execute
    let (filled_qty, total_notional) = execute_liquidation_sweep(
        slab,
        account_idx,
        instrument_idx,
        close_side,
        close_qty,
        min_price,
        max_price,
    )?;

    // Calculate realized PnL
    let position_entry_px = get_position_entry_price(slab, account_idx, instrument_idx);
    let avg_close_px = if filled_qty > 0 {
        (total_notional / filled_qty as u128) as u64
    } else {
        position_entry_px
    };

    let pnl = calculate_pnl(
        position_qty,
        position_entry_px,
        avg_close_px,
    );

    // Update account cash with realized PnL
    if let Some(account) = slab.get_account_mut(account_idx) {
        account.cash = account.cash.saturating_add(pnl);
    }

    // Remove or update position
    update_position_after_close(slab, account_idx, instrument_idx, -(filled_qty as i64))?;

    Ok(CloseResult {
        notional: total_notional,
        pnl,
    })
}

/// Execute liquidation sweep through orderbook
///
/// Similar to normal reserve/commit but respects price bands
fn execute_liquidation_sweep(
    slab: &mut SlabState,
    account_idx: u32,
    instrument_idx: u16,
    side: Side,
    qty: u64,
    min_price: u64,
    max_price: u64,
) -> Result<(u64, u128), PercolatorError> {
    let instrument = slab
        .get_instrument(instrument_idx)
        .ok_or(PercolatorError::InvalidInstrument)?;

    // Get contra book head
    let book_head = match side {
        Side::Buy => instrument.asks_head,
        Side::Sell => instrument.bids_head,
    };

    let mut curr_order_idx = book_head;
    let mut qty_remaining = qty;
    let mut total_notional = 0u128;
    let mut total_filled = 0u64;

    // Walk book and execute
    while curr_order_idx != u32::MAX && qty_remaining > 0 {
        let order = slab
            .orders
            .get(curr_order_idx)
            .ok_or(PercolatorError::OrderNotFound)?;

        let maker_price = order.price;
        let maker_qty = order.qty;
        let maker_account_idx = order.account_idx;
        let next_order = order.next;

        // Check price band
        let within_band = maker_price >= min_price && maker_price <= max_price;
        if !within_band {
            break; // Stop if we hit price band limit
        }

        // Calculate fill quantity
        let fill_qty = core::cmp::min(qty_remaining, maker_qty);
        
        // Execute trade
        execute_liquidation_trade(
            slab,
            account_idx,  // liquidatee (taker)
            maker_account_idx,
            instrument_idx,
            side,
            fill_qty,
            maker_price,
        )?;

        let notional = mul_u64(fill_qty, maker_price);
        total_notional = total_notional.saturating_add(notional);
        total_filled = total_filled.saturating_add(fill_qty);
        qty_remaining = qty_remaining.saturating_sub(fill_qty);

        // Update or remove maker order
        if let Some(order) = slab.orders.get_mut(curr_order_idx) {
            order.qty = order.qty.saturating_sub(fill_qty);
            
            if order.qty == 0 {
                // Remove from book
                remove_order_from_book(slab, instrument_idx, curr_order_idx, side)?;
                slab.orders.free(curr_order_idx);
            }
        }

        curr_order_idx = next_order;
    }

    Ok((total_filled, total_notional))
}

/// Execute a single liquidation trade
fn execute_liquidation_trade(
    slab: &mut SlabState,
    taker_account_idx: u32,
    maker_account_idx: u32,
    instrument_idx: u16,
    side: Side,
    qty: u64,
    price: u64,
) -> Result<(), PercolatorError> {
    let instrument = slab
        .get_instrument(instrument_idx)
        .ok_or(PercolatorError::InvalidInstrument)?;

    let contract_size = instrument.contract_size;

    // Calculate position deltas
    let taker_delta = match side {
        Side::Buy => qty as i64,
        Side::Sell => -(qty as i64),
    };
    let maker_delta = -taker_delta;

    // Update taker position
    update_position(slab, taker_account_idx, instrument_idx, taker_delta, price, contract_size)?;

    // Update maker position
    update_position(slab, maker_account_idx, instrument_idx, maker_delta, price, contract_size)?;

    // Calculate and apply fees (no rebates on liquidations)
    let notional = mul_u64(qty, price);
    let taker_fee = calculate_fee(notional, slab.header.taker_fee as i64);
    let maker_fee = calculate_fee(notional, slab.header.maker_fee.max(0) as i64); // No rebate

    // Deduct fees
    if let Some(taker) = slab.get_account_mut(taker_account_idx) {
        taker.cash = taker.cash.saturating_sub(taker_fee as i128);
    }
    if let Some(maker) = slab.get_account_mut(maker_account_idx) {
        maker.cash = maker.cash.saturating_sub(maker_fee as i128);
    }

    Ok(())
}

/// Get position entry price
fn get_position_entry_price(slab: &SlabState, account_idx: u32, instrument_idx: u16) -> u64 {
    if let Some(account) = slab.get_account(account_idx) {
        let mut pos_idx = account.position_head;
        while pos_idx != u32::MAX {
            if let Some(pos) = slab.positions.get(pos_idx) {
                if pos.instrument_idx == instrument_idx {
                    return pos.entry_px;
                }
                pos_idx = pos.next_in_account;
            } else {
                break;
            }
        }
    }
    0
}

/// Update position after partial or full close
fn update_position_after_close(
    slab: &mut SlabState,
    account_idx: u32,
    instrument_idx: u16,
    qty_delta: i64,
) -> Result<(), PercolatorError> {
    if let Some(account) = slab.get_account(account_idx) {
        let mut pos_idx = account.position_head;
        let mut prev_idx = u32::MAX;

        while pos_idx != u32::MAX {
            if let Some(pos) = slab.positions.get(pos_idx) {
                if pos.instrument_idx == instrument_idx {
                    let new_qty = pos.qty + qty_delta;
                    
                    if new_qty == 0 {
                        // Full close - remove position
                        remove_position_from_account(slab, account_idx, pos_idx, prev_idx)?;
                        slab.positions.free(pos_idx);
                    } else {
                        // Partial close - update quantity
                        if let Some(pos) = slab.positions.get_mut(pos_idx) {
                            pos.qty = new_qty;
                        }
                    }
                    return Ok(());
                }
                prev_idx = pos_idx;
                pos_idx = pos.next_in_account;
            } else {
                break;
            }
        }
    }
    Ok(())
}

/// Remove position from account's position list
fn remove_position_from_account(
    slab: &mut SlabState,
    account_idx: u32,
    pos_idx: u32,
    prev_idx: u32,
) -> Result<(), PercolatorError> {
    let next_idx = slab
        .positions
        .get(pos_idx)
        .map(|p| p.next_in_account)
        .unwrap_or(u32::MAX);

    if prev_idx == u32::MAX {
        // Removing head
        if let Some(account) = slab.get_account_mut(account_idx) {
            account.position_head = next_idx;
        }
    } else {
        // Removing from middle/end
        if let Some(prev_pos) = slab.positions.get_mut(prev_idx) {
            prev_pos.next_in_account = next_idx;
        }
    }

    Ok(())
}

/// Remove order from book (helper function)
fn remove_order_from_book(
    _slab: &mut SlabState,
    _instrument_idx: u16,
    _order_idx: u32,
    _side: Side,
) -> Result<(), PercolatorError> {
    // This is a simplified version - real implementation would need proper book management
    // For now, just mark as removed
    Ok(())
}

/// Update position with new trade
fn update_position(
    slab: &mut SlabState,
    account_idx: u32,
    instrument_idx: u16,
    qty_delta: i64,
    price: u64,
    _contract_size: u64,
) -> Result<(), PercolatorError> {
    // Find existing position or create new one
    if let Some(account) = slab.get_account(account_idx) {
        let mut pos_idx = account.position_head;
        
        while pos_idx != u32::MAX {
            if let Some(pos) = slab.positions.get_mut(pos_idx) {
                if pos.instrument_idx == instrument_idx {
                    // Update existing position
                    let old_qty = pos.qty;
                    let new_qty = old_qty + qty_delta;
                    
                    // Calculate new entry price (VWAP)
                    if (old_qty > 0 && new_qty > 0) || (old_qty < 0 && new_qty < 0) {
                        // Adding to position - update VWAP
                        let old_notional = mul_u64(old_qty.abs() as u64, pos.entry_px);
                        let new_notional = mul_u64(qty_delta.abs() as u64, price);
                        let total_notional = old_notional.saturating_add(new_notional);
                        let total_qty = new_qty.abs() as u64;
                        pos.entry_px = if total_qty > 0 {
                            (total_notional / total_qty as u128) as u64
                        } else {
                            price
                        };
                    } else if new_qty == 0 {
                        // Position closed - will be handled by caller
                    } else {
                        // Position flipped - use new price
                        pos.entry_px = price;
                    }
                    
                    pos.qty = new_qty;
                    return Ok(());
                }
                pos_idx = pos.next_in_account;
            } else {
                break;
            }
        }
    }
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // Note: Full integration tests would require creating a test slab with positions
    // For now, we test the helper functions

    #[test]
    fn test_close_result_creation() {
        let result = CloseResult {
            notional: 10_000,
            pnl: -500,
        };

        assert_eq!(result.notional, 10_000);
        assert_eq!(result.pnl, -500);
    }

    #[test]
    fn test_liquidation_result_creation() {
        let result = LiquidationResult {
            closed_qty: 1_000,
            realized_pnl: -5_000,
            closed_notional: 50_000,
            liquidation_fee: 2_500,
            remaining_deficit: 1_000,
        };

        assert_eq!(result.closed_qty, 1_000);
        assert_eq!(result.liquidation_fee, 2_500);
        assert_eq!(result.remaining_deficit, 1_000);
    }

    #[test]
    fn test_price_band_calculation() {
        let mark_price = 50_000u64;
        let band_bps = 300u16; // 3%

        let band_delta = (mark_price as u128 * band_bps as u128) / 10_000;
        assert_eq!(band_delta, 1_500); // 3% of 50,000

        let min_price = mark_price.saturating_sub(band_delta as u64);
        let max_price = mark_price.saturating_add(band_delta as u64);

        assert_eq!(min_price, 48_500);
        assert_eq!(max_price, 51_500);
    }

    #[test]
    fn test_liquidation_fee_calculation() {
        let closed_notional = 100_000u128;
        let fee_bps = 500u16; // 5%

        let fee = (closed_notional * fee_bps as u128) / 10_000;
        assert_eq!(fee, 5_000); // 5% of 100,000
    }
}

