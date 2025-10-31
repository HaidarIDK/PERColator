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
/// * `post_only` - If true, reject order if it would cross immediately
/// * `reduce_only` - If true, order can only reduce existing position
///
/// # Returns
/// * Order ID of the placed order
///
/// # Errors
/// * InvalidPrice - Price must be positive
/// * InvalidQuantity - Quantity must be positive
/// * InvalidTickSize - Price not aligned to tick size
/// * InvalidLotSize - Quantity not aligned to lot size
/// * OrderTooSmall - Quantity below minimum order size
/// * WouldCross - Post-only order would cross
/// * OrderBookFull - Book has reached capacity
pub fn process_place_order(
    slab: &mut SlabState,
    owner: &Pubkey,
    side: OrderSide,
    price: i64,
    qty: i64,
    post_only: bool,
    reduce_only: bool,
) -> Result<u64, PercolatorError> {
    // Check if trading is halted
    if slab.header.is_trading_halted() {
        msg!("Error: Trading is halted");
        return Err(PercolatorError::TradingHalted);
    }

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

    // Insert order using FORMALLY VERIFIED orderbook logic with extended validation
    // This call ensures properties O1 (sorted price-time priority), O7 (tick size),
    // O8 (lot/min size), and O9 (post-only) are maintained
    // See: crates/model_safety/src/orderbook.rs for Kani proofs
    let order_id = model_bridge::insert_order_extended_verified(
        &mut slab.book,
        *owner,
        side,
        price,
        qty,
        timestamp,
        slab.header.tick,           // tick_size
        slab.header.lot,            // lot_size
        slab.header.min_order_size, // min_order_size
        post_only,
        reduce_only,
    ).map_err(|e| {
        match e {
            "Invalid tick size" => {
                msg!("Error: Price not aligned to tick size");
                PercolatorError::InvalidTickSize
            }
            "Invalid lot size" => {
                msg!("Error: Quantity not aligned to lot size");
                PercolatorError::InvalidLotSize
            }
            "Order too small" => {
                msg!("Error: Order quantity below minimum");
                PercolatorError::OrderTooSmall
            }
            "Post-only order would cross" => {
                msg!("Error: Post-only order would cross existing orders");
                PercolatorError::WouldCross
            }
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
