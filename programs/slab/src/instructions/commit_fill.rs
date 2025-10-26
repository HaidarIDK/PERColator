//! Commit fill instruction - v1 orderbook matching

use crate::state::{SlabState, FillReceipt};
use percolator_common::*;
use pinocchio::{account_info::AccountInfo, msg, pubkey::Pubkey};

/// Side of the order
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    Buy = 0,
    Sell = 1,
}

/// Result of matching against the orderbook
struct MatchResult {
    /// Quantity filled (1e6 scale)
    filled_qty: i64,
    /// Volume-weighted average price (1e6 scale)
    vwap_px: i64,
}

/// Match an incoming order against the orderbook
///
/// This function walks the book and consumes liquidity up to the limit price.
/// Orders are filled at their resting price (price-time priority).
///
/// # Arguments
/// * `slab` - The slab state (will be mutated as orders are filled)
/// * `side` - Buy or Sell (determines which side of book to match against)
/// * `qty` - Desired quantity to fill (1e6 scale)
/// * `limit_px` - Worst acceptable price (1e6 scale)
///
/// # Returns
/// * `MatchResult` with filled_qty and vwap_px
///
/// # Matching Logic
/// - Buy orders match against asks (lowest price first)
/// - Sell orders match against bids (highest price first)
/// - Stop when limit price is reached or book is exhausted
/// - VWAP = sum(qty_i * price_i) / sum(qty_i)
fn match_against_book(
    slab: &mut SlabState,
    side: Side,
    qty: i64,
    limit_px: i64,
) -> MatchResult {
    let mut remaining = qty;
    let mut total_notional: i128 = 0; // Use i128 to prevent overflow
    let mut total_filled: i64 = 0;

    // Determine which side of the book to match against
    let (orders, count) = match side {
        Side::Buy => {
            // Buy matches against asks (ascending price)
            (&mut slab.book.asks[..], slab.book.num_asks)
        }
        Side::Sell => {
            // Sell matches against bids (descending price)
            (&mut slab.book.bids[..], slab.book.num_bids)
        }
    };

    let mut orders_to_remove: [u64; 19] = [0; 19]; // Max orders per side
    let mut remove_count: usize = 0;

    // Walk the book and fill orders
    for i in 0..(count as usize) {
        if remaining <= 0 {
            break;
        }

        let order = &mut orders[i];

        // Check if price crosses the limit
        let price_acceptable = match side {
            Side::Buy => order.price <= limit_px,   // Buy: ask price must be <= limit
            Side::Sell => order.price >= limit_px,  // Sell: bid price must be >= limit
        };

        if !price_acceptable {
            break; // Stop matching, price too unfavorable
        }

        // Calculate fill quantity for this order
        let fill_qty = remaining.min(order.qty);

        // Update accounting
        total_notional += (fill_qty as i128) * (order.price as i128);
        total_filled += fill_qty;
        remaining -= fill_qty;

        // Update order quantity
        order.qty -= fill_qty;

        // Mark for removal if fully filled
        if order.qty == 0 && remove_count < 19 {
            orders_to_remove[remove_count] = order.order_id;
            remove_count += 1;
        }
    }

    // Remove fully filled orders from the book
    for i in 0..remove_count {
        let order_id = orders_to_remove[i];
        // Ignore errors - order might already be removed
        let _ = slab.book.remove_order(order_id);
    }

    // Calculate VWAP
    let vwap_px = if total_filled > 0 {
        // VWAP = total_notional / total_filled (both in 1e6 scale)
        (total_notional / total_filled as i128) as i64
    } else {
        0 // No fill
    };

    MatchResult {
        filled_qty: total_filled,
        vwap_px,
    }
}

/// Process commit_fill instruction (v0 - atomic fill)
///
/// This is the single CPI endpoint for v0. Router calls this to fill orders.
///
/// # Arguments
/// * `slab` - The slab state account
/// * `receipt_account` - Account to write fill receipt
/// * `router_signer` - Router authority (must match slab.header.router_id)
/// * `side` - Buy or Sell
/// * `qty` - Desired quantity (1e6 scale, positive)
/// * `limit_px` - Worst acceptable price (1e6 scale)
///
/// # Returns
/// * Writes FillReceipt to receipt_account
/// * Updates slab state (book, seqno, quote_cache)
pub fn process_commit_fill(
    slab: &mut SlabState,
    receipt_account: &AccountInfo,
    router_signer: &Pubkey,
    expected_seqno: u32,
    side: Side,
    qty: i64,
    limit_px: i64,
) -> Result<(), PercolatorError> {
    // Verify router authority
    if &slab.header.router_id != router_signer {
        msg!("Error: Invalid router signer");
        return Err(PercolatorError::Unauthorized);
    }

    // TOCTOU Protection: Validate seqno hasn't changed
    if slab.header.seqno != expected_seqno {
        msg!("Error: Seqno mismatch - book changed since read");
        return Err(PercolatorError::SeqnoMismatch);
    }

    // Validate order parameters
    if qty <= 0 {
        msg!("Error: Quantity must be positive");
        return Err(PercolatorError::InvalidQuantity);
    }
    if limit_px <= 0 {
        msg!("Error: Limit price must be positive");
        return Err(PercolatorError::InvalidPrice);
    }

    // Capture seqno at start
    let seqno_start = slab.header.seqno;

    // v1 Matching: Match against real orderbook
    let match_result = match_against_book(slab, side, qty, limit_px);
    let filled_qty = match_result.filled_qty;
    let vwap_px = match_result.vwap_px;

    // Check if any liquidity was available
    if filled_qty == 0 {
        msg!("Error: No liquidity available at limit price");
        return Err(PercolatorError::InsufficientLiquidity);
    }

    // Calculate notional: filled_qty * vwap_px / 1e6
    // Both values are in 1e6 scale, so we divide by 1e6
    let notional = (filled_qty as i128 * vwap_px as i128 / 1_000_000) as i64;

    // Calculate fee: notional * taker_fee_bps / 10000
    let fee = (notional as i128 * slab.header.taker_fee_bps as i128 / 10_000) as i64;

    // Write receipt
    let receipt = unsafe { percolator_common::borrow_account_data_mut::<FillReceipt>(receipt_account)? };
    receipt.write(seqno_start, filled_qty, vwap_px, notional, fee);

    // Increment seqno (book changed - orders were filled/removed)
    slab.header.increment_seqno();

    msg!("CommitFill executed successfully");
    Ok(())
}
