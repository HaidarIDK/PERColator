//! CancelOrder instruction - v1 orderbook interaction
//!
//! Allows users to cancel their resting limit orders from the orderbook

use crate::state::{SlabState, model_bridge};
use percolator_common::PercolatorError;
use pinocchio::{msg, pubkey::Pubkey};

/// Process cancel_order instruction
///
/// Cancels (removes) a limit order from the orderbook.
/// Only the order owner can cancel their own orders.
///
/// # Arguments
/// * `slab` - The slab state account (mut)
/// * `owner` - The order owner's public key (must be signer)
/// * `order_id` - The unique ID of the order to cancel
///
/// # Returns
/// * Ok(()) on success
///
/// # Errors
/// * OrderNotFound - Order ID does not exist in the book
/// * Unauthorized - Signer is not the owner of the order
pub fn process_cancel_order(
    slab: &mut SlabState,
    owner: &Pubkey,
    order_id: u64,
) -> Result<(), PercolatorError> {
    // First, find the order to verify ownership before removing
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

    // Remove the order from the book using FORMALLY VERIFIED orderbook logic
    // This call ensures property O2 (no double-execution) is maintained
    // See: crates/model_safety/src/orderbook.rs for Kani proofs
    model_bridge::remove_order_verified(&mut slab.book, order_id).map_err(|_| {
        msg!("Error: Order not found during removal");
        PercolatorError::OrderNotFound
    })?;

    // Increment seqno (book state changed)
    slab.header.increment_seqno();

    msg!("CancelOrder executed");

    Ok(())
}
