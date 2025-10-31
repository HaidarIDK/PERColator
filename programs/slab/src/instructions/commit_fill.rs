//! Commit fill instruction - v1 orderbook matching

use crate::state::{SlabState, FillReceipt, model_bridge, Side};
use crate::state::model_bridge::{TimeInForce, SelfTradePrevent};
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
/// * `expected_seqno` - Expected seqno for TOCTOU protection
/// * `taker_owner` - The taker's public key (for self-trade prevention)
/// * `side` - Buy or Sell
/// * `qty` - Desired quantity (1e6 scale, positive)
/// * `limit_px` - Worst acceptable price (1e6 scale)
/// * `time_in_force` - GTC/IOC/FOK enforcement
/// * `self_trade_prevention` - Self-trade prevention policy
///
/// # Returns
/// * Writes FillReceipt to receipt_account
/// * Updates slab state (book, seqno, quote_cache)
pub fn process_commit_fill(
    slab: &mut SlabState,
    receipt_account: &AccountInfo,
    router_signer: &Pubkey,
    expected_seqno: u32,
    taker_owner: &Pubkey,
    side: Side,
    qty: i64,
    limit_px: i64,
    time_in_force: TimeInForce,
    self_trade_prevention: SelfTradePrevent,
) -> Result<(), PercolatorError> {
    // Verify router authority
    if &slab.header.router_id != router_signer {
        msg!("Error: Invalid router signer");
        return Err(PercolatorError::Unauthorized);
    }

    // Check if trading is halted
    if slab.header.is_trading_halted() {
        msg!("Error: Trading is halted");
        return Err(PercolatorError::TradingHalted);
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

    // v1 Matching: Match against real orderbook using FORMALLY VERIFIED logic with TIF+STPF
    // Properties verified by Kani:
    // - O3: Fill quantities never exceed order quantities
    // - O4: VWAP calculation is monotonic and bounded
    // - O6: Fee arithmetic is conservative (no overflow)
    // - O11: TimeInForce semantics (GTC/IOC/FOK)
    // - O12: Self-trade prevention policies
    // See: crates/model_safety/src/orderbook.rs for Kani proofs
    let match_result = model_bridge::match_orders_with_tif_verified(
        &mut slab.book,
        *taker_owner,
        side,
        qty,
        limit_px,
        time_in_force,
        self_trade_prevention,
    )
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
            "Cannot fill completely (FOK)" => {
                msg!("Error: Cannot fill completely (FOK requirement)");
                PercolatorError::CannotFillCompletely
            }
            "Self trade detected" => {
                msg!("Error: Self trade detected");
                PercolatorError::SelfTrade
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
