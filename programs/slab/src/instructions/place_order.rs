//! PlaceOrder instruction - v1 orderbook interaction
//!
//! Allows users to place resting limit orders in the orderbook

use crate::state::{SlabState, Side as OrderSide, model_bridge};
use percolator_common::PercolatorError;
use pinocchio::{msg, pubkey::Pubkey, sysvars::{clock::Clock, Sysvar}};

/// Process place_order instruction
///
/// Places a limit order in the orderbook that rests until filled or cancelled.
///
/// # Arguments
/// * `slab` - The slab state account (mut)
/// * `owner` - The order owner's public key (must be signer)
/// * `side` - Buy or Sell
/// * `price` - Limit price (1e6 scale, positive)
/// * `qty` - Order quantity (1e6 scale, positive)
///
/// # Returns
/// * Order ID of the placed order
///
/// # Errors
/// * InvalidPrice - Price must be positive
/// * InvalidQuantity - Quantity must be positive
/// * OrderBookFull - Book has reached capacity
pub fn process_place_order(
    slab: &mut SlabState,
    owner: &Pubkey,
    side: OrderSide,
    price: i64,
    qty: i64,
) -> Result<u64, PercolatorError> {
    // Validate order parameters
    if price <= 0 {
        msg!("Error: Price must be positive");
        return Err(PercolatorError::InvalidPrice);
    }
    if qty <= 0 {
        msg!("Error: Quantity must be positive");
        return Err(PercolatorError::InvalidQuantity);
    }

    // Get current timestamp from Clock sysvar
    // In BPF, this would use get_clock_sysvar(); for testing we use a default
    let timestamp = Clock::get().map(|c| c.unix_timestamp as u64).unwrap_or(0);

    // Insert order using FORMALLY VERIFIED orderbook logic
    // This call ensures property O1 (sorted price-time priority) is maintained
    // See: crates/model_safety/src/orderbook.rs for Kani proofs
    let order_id = model_bridge::insert_order_verified(
        &mut slab.book,
        *owner,
        side,
        price,
        qty,
        timestamp,
    ).map_err(|e| {
        match e {
            "Invalid price" => {
                msg!("Error: Price must be positive");
                PercolatorError::InvalidPrice
            }
            "Invalid quantity" => {
                msg!("Error: Quantity must be positive");
                PercolatorError::InvalidQuantity
            }
            "Order book full" => {
                msg!("Error: Order book full");
                PercolatorError::PoolFull
            }
            _ => {
                msg!("Error: Failed to insert order");
                PercolatorError::PoolFull
            }
        }
    })?;

    // Increment seqno (book state changed)
    slab.header.increment_seqno();

    msg!("PlaceOrder executed");

    Ok(order_id)
}
