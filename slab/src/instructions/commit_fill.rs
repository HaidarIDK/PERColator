//! Commit fill instruction - v0 single-instruction orderbook interaction

use crate::state::{SlabState, FillReceipt, QuoteCache, QuoteLevel};
use percolator_common::*;
use pinocchio::{account_info::AccountInfo, msg, pubkey::Pubkey};

/// Side of the order
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    Buy = 0,
    Sell = 1,
}

/// Update quote cache after a fill (v0 stub)
/// In v1, this will reflect actual book state after matching
fn update_quote_cache_after_fill(
    cache: &mut QuoteCache,
    seqno: u32,
    side: Side,
    px: i64,
    qty: i64,
) {
    // For v0, simulate liquidity by adding fill as a quote level
    // This proves the cache update mechanism works
    let level = QuoteLevel { px, avail_qty: qty };
    match side {
        Side::Buy => {
            // Buy removes ask liquidity, add to bids
            cache.update(seqno, &[level], &[]);
        }
        Side::Sell => {
            // Sell removes bid liquidity, add to asks
            cache.update(seqno, &[], &[level]);
        }
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

    // v0 Matching: Simulate instant fill at limit price
    // In v1, this will match against real book liquidity
    let filled_qty = qty;
    let vwap_px = limit_px;

    // Calculate notional: qty * contract_size * price / 1e6
    // For v0, simplified: qty * price / 1e6 (assuming contract_size normalized)
    let notional = (filled_qty as i128 * limit_px as i128 / 1_000_000) as i64;

    // Calculate fee: notional * taker_fee_bps / 10000
    let fee = (notional as i128 * slab.header.taker_fee_bps as i128 / 10_000) as i64;

    // Update quote cache to reflect this fill
    // For v0, add this as liquidity at the fill price
    update_quote_cache_after_fill(&mut slab.quote_cache, slab.header.seqno + 1, side, limit_px, filled_qty);

    // Write receipt
    let receipt = unsafe { percolator_common::borrow_account_data_mut::<FillReceipt>(receipt_account)? };
    receipt.write(seqno_start, filled_qty, vwap_px, notional, fee);

    // Increment seqno (book changed)
    slab.header.increment_seqno();

    msg!("CommitFill executed successfully");
    Ok(())
}
