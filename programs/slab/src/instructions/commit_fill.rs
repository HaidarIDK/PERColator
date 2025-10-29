//! Commit fill instruction - v1 orderbook matching

use crate::state::{SlabState, FillReceipt, model_bridge, Side};
use percolator_common::*;
use pinocchio::{account_info::AccountInfo, msg, pubkey::Pubkey};

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

    // v1 Matching: Match against real orderbook using FORMALLY VERIFIED logic
    // Properties verified by Kani:
    // - O3: Fill quantities never exceed order quantities
    // - O4: VWAP calculation is monotonic and bounded
    // - O6: Fee arithmetic is conservative (no overflow)
    // See: crates/model_safety/src/orderbook.rs for Kani proofs
    let match_result = model_bridge::match_orders_verified(&mut slab.book, side, qty, limit_px)
        .map_err(|e| {
            match e {
                "Invalid price" => {
                    msg!("Error: Invalid price");
                    PercolatorError::InvalidPrice
                }
                "Invalid quantity" => {
                    msg!("Error: Invalid quantity");
                    PercolatorError::InvalidQuantity
                }
                "No liquidity" => {
                    msg!("Error: No liquidity available at limit price");
                    PercolatorError::InsufficientLiquidity
                }
                _ => {
                    msg!("Error: Order matching failed");
                    PercolatorError::InsufficientLiquidity
                }
            }
        })?;

    let filled_qty = match_result.filled_qty;
    let vwap_px = match_result.vwap_px;
    let notional = match_result.notional; // Already computed by verified logic

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
