//! ModifyOrder instruction
//!
//! Allows users to modify their resting limit orders (change price/qty)
//! while preserving time priority if only size changes.

use crate::state::{SlabState, model_bridge};
use model_safety::orderbook::OrderbookError;
use percolator_common::PercolatorError;
use pinocchio::{msg, pubkey::Pubkey, sysvars::{clock::Clock, Sysvar}};

/// Process modify_order instruction
///
/// Modifies a limit order's price and/or quantity.
/// Time priority semantics:
/// - Same price: preserves timestamp (keeps time priority)
/// - Different price: uses current timestamp (loses priority)
///
/// Only the order owner can modify their own orders.
///
/// # Arguments
/// * `slab` - The slab state account (mut)
/// * `owner` - The order owner's public key (must be signer)
/// * `order_id` - The unique ID of the order to modify
/// * `new_price` - New limit price (1e6 scale, positive)
/// * `new_qty` - New quantity (1e6 scale, positive)
///
/// # Returns
/// * Ok(()) on success
///
/// # Errors
/// * OrderNotFound - Order ID does not exist in the book
/// * Unauthorized - Signer is not the owner of the order
/// * InvalidPrice - New price is invalid or doesn't meet tick size
/// * InvalidQuantity - New qty is invalid or doesn't meet lot/min size
/// * TradingHalted - Trading is currently halted
pub fn process_modify_order(
    slab: &mut SlabState,
    owner: &Pubkey,
    order_id: u64,
    new_price: i64,
    new_qty: i64,
) -> Result<(), PercolatorError> {
    // Check if trading is halted
    if slab.header.is_trading_halted() {
        msg!("Error: Trading is halted");
        return Err(PercolatorError::TradingHalted);
    }

    // Find the order to verify ownership
    let order = slab.book.find_order(order_id)
        .ok_or_else(|| {
            msg!("Error: Order not found");
            PercolatorError::OrderNotFound
        })?;

    // Verify the signer owns this order
    if order.owner != *owner {
        msg!("Error: Unauthorized");
        return Err(PercolatorError::Unauthorized);
    }

    // Get current timestamp for price changes
    let clock = Clock::get().map_err(|_| PercolatorError::InvalidInstruction)?;
    let current_timestamp = clock.unix_timestamp as u64;

    // Modify the order using FORMALLY VERIFIED orderbook logic
    // This call ensures proper time priority semantics:
    // - Same price: preserves original timestamp
    // - Different price: uses current timestamp
    // See: crates/model_safety/src/orderbook.rs for implementation
    model_bridge::modify_order_verified(
        &mut slab.book,
        order_id,
        new_price,
        new_qty,
        current_timestamp,
    ).map_err(|e| {
        msg!("Error: Modify order failed");
        match e {
            OrderbookError::InvalidPrice => PercolatorError::InvalidPrice,
            OrderbookError::InvalidQuantity => PercolatorError::InvalidQuantity,
            OrderbookError::OrderNotFound => PercolatorError::OrderNotFound,
            _ => PercolatorError::OrderNotFound,
        }
    })?;

    // Increment seqno (book state changed)
    slab.header.increment_seqno();

    // Refresh quote cache to maintain snapshot consistency (Scenario 21)
    slab.refresh_quote_cache();

    msg!("ModifyOrder executed");

    Ok(())
}
