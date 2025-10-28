//! LP Adapter instructions for OrderBook Slab
//!
//! Implements the adapter-core interface to allow Router→Slab CPI for LP operations.

use adapter_core::*;
use crate::state::{SlabState, Side as OrderSide};
use crate::instructions::{process_place_order, process_cancel_order};
use percolator_common::PercolatorError;
use pinocchio::{account_info::AccountInfo, msg};

extern crate alloc;
use alloc::vec::Vec;

/// Process liquidity operation via adapter pattern
///
/// This is the CPI endpoint for the router to manage LP liquidity in the orderbook.
///
/// # Arguments
/// * `accounts` - [slab_account, router_signer]
/// * `intent` - The liquidity operation to perform
/// * `guard` - Risk guards for the operation
///
/// # Returns
/// * `LiquidityResult` - LP share delta, exposure delta, fee credits, PnL delta
pub fn process_adapter_liquidity(
    accounts: &[AccountInfo],
    intent: &LiquidityIntent,
    guard: &RiskGuard,
) -> Result<LiquidityResult, PercolatorError> {
    let [slab_account, router_signer] = accounts else {
        return Err(PercolatorError::InvalidAccount);
    };

    // Verify router signer
    if !router_signer.is_signer() {
        msg!("Error: Router must be signer");
        return Err(PercolatorError::Unauthorized);
    }

    // Get mutable slab state
    let data = slab_account.try_borrow_mut_data()
        .map_err(|_| PercolatorError::InvalidAccount)?;

    if data.len() != SlabState::LEN {
        msg!("Error: Slab account has incorrect size");
        return Err(PercolatorError::InvalidAccount);
    }

    let slab = unsafe { &mut *(data.as_ptr() as *mut SlabState) };

    // Verify router authority
    if &slab.header.router_id != router_signer.key() {
        msg!("Error: Invalid router signer");
        return Err(PercolatorError::Unauthorized);
    }

    // Process the liquidity operation
    match intent {
        LiquidityIntent::ObAdd {
            orders,
            post_only: _,
            reduce_only: _,
        } => {
            process_ob_add(slab, orders, guard)
        }
        LiquidityIntent::Remove { selector } => {
            process_remove(slab, selector, guard)
        }
        LiquidityIntent::AmmAdd { .. } => {
            msg!("Error: Slab does not support AMM operations");
            Err(PercolatorError::InvalidInstruction)
        }
        LiquidityIntent::Hook { .. } => {
            msg!("Error: Slab does not support custom hooks");
            Err(PercolatorError::InvalidInstruction)
        }
        LiquidityIntent::Modify { .. } => {
            msg!("Error: Slab does not support modify (use Remove + ObAdd)");
            Err(PercolatorError::InvalidInstruction)
        }
    }
}

/// Add limit orders to the orderbook
pub(crate) fn process_ob_add(
    slab: &mut SlabState,
    orders: &Vec<ObOrder>,
    _guard: &RiskGuard,
) -> Result<LiquidityResult, PercolatorError> {
    let mut total_base_delta: i128 = 0;
    let mut total_quote_delta: i128 = 0;

    // Process each order in the batch
    for order in orders {
        // Convert Q64 to i64 scale (divide by 2^64)
        let price = (order.px_q64 >> 64) as i64;
        let qty = (order.qty_q64 >> 64) as i64;

        if price <= 0 {
            msg!("Error: Price must be positive");
            return Err(PercolatorError::InvalidPrice);
        }

        if qty <= 0 {
            msg!("Error: Quantity must be positive");
            return Err(PercolatorError::InvalidQuantity);
        }

        // Convert Side to OrderSide
        let order_side = match order.side {
            Side::Bid => OrderSide::Buy,
            Side::Ask => OrderSide::Sell,
        };

        // Extract lp_owner before mutable borrow
        let lp_owner = slab.header.lp_owner;

        // Place the order
        let _order_id = process_place_order(
            slab,
            &lp_owner,
            order_side,
            price,
            qty,
        )?;

        // Calculate exposure delta for this order
        let (base_delta, quote_delta) = match order_side {
            OrderSide::Buy => {
                // Buying: negative quote (paying), positive base (receiving)
                let quote = -(price as i128 * qty as i128 / 1_000_000);
                let base = qty as i128;
                (base, quote)
            }
            OrderSide::Sell => {
                // Selling: negative base (selling), positive quote (receiving)
                let base = -(qty as i128);
                let quote = price as i128 * qty as i128 / 1_000_000;
                (base, quote)
            }
        };

        total_base_delta += base_delta;
        total_quote_delta += quote_delta;
    }

    msg!("Orderbook liquidity added successfully");

    // Return result
    Ok(LiquidityResult {
        lp_shares_delta: 0, // Orderbook doesn't use LP shares
        exposure_delta: Exposure {
            base_q64: total_base_delta << 64,
            quote_q64: total_quote_delta << 64,
        },
        maker_fee_credits: 0,
        realized_pnl_delta: 0,
    })
}

/// Remove liquidity from orderbook
pub(crate) fn process_remove(
    slab: &mut SlabState,
    selector: &RemoveSel,
    _guard: &RiskGuard,
) -> Result<LiquidityResult, PercolatorError> {
    match selector {
        RemoveSel::ObByIds { ids } => {
            process_ob_remove_by_ids(slab, ids)
        }
        RemoveSel::ObAll => {
            process_ob_remove_all(slab)
        }
        RemoveSel::AmmByShares { .. } => {
            msg!("Error: Slab does not support AMM removal");
            Err(PercolatorError::InvalidInstruction)
        }
    }
}

/// Remove orders by ID
fn process_ob_remove_by_ids(
    slab: &mut SlabState,
    ids: &[u128],
) -> Result<LiquidityResult, PercolatorError> {
    let mut total_base_delta: i128 = 0;
    let mut total_quote_delta: i128 = 0;

    for &order_id_u128 in ids {
        // Convert u128 to u64 (slab uses u64 for order IDs)
        let order_id = order_id_u128 as u64;

        // Get order details before canceling to calculate exposure
        if let Some(order) = slab.book.find_order(order_id) {
            let qty = order.qty as i128;
            let price = order.price as i128;

            // Calculate exposure delta for this order
            // Note: order.side is a u8, need to convert for match
            let side = if order.side == 0 {
                OrderSide::Buy
            } else {
                OrderSide::Sell
            };

            let (base_delta, quote_delta) = match side {
                OrderSide::Buy => {
                    // Canceling buy order: return quote, remove base obligation
                    let quote = price * qty / 1_000_000;
                    let base = -qty;
                    (base, quote)
                }
                OrderSide::Sell => {
                    // Canceling sell order: return base, remove quote expectation
                    let base = qty;
                    let quote = -(price * qty / 1_000_000);
                    (base, quote)
                }
            };

            total_base_delta += base_delta;
            total_quote_delta += quote_delta;

            // Extract lp_owner before mutable borrow
            let lp_owner = slab.header.lp_owner;

            // Cancel the order
            process_cancel_order(slab, &lp_owner, order_id)?;
        } else {
            msg!("Warning: Order ID not found, skipping");
        }
    }

    msg!("Orderbook liquidity removed successfully");

    Ok(LiquidityResult {
        lp_shares_delta: 0,
        exposure_delta: Exposure {
            base_q64: total_base_delta << 64,
            quote_q64: total_quote_delta << 64,
        },
        maker_fee_credits: 0,
        realized_pnl_delta: 0,
    })
}

/// Remove all orders for the LP owner
fn process_ob_remove_all(
    slab: &mut SlabState,
) -> Result<LiquidityResult, PercolatorError> {
    // Collect all order IDs for the LP owner
    let mut order_ids = Vec::new();

    // Iterate through bids
    for i in 0..slab.book.num_bids as usize {
        let order = &slab.book.bids[i];
        if order.owner == slab.header.lp_owner {
            order_ids.push(order.order_id as u128);
        }
    }

    // Iterate through asks
    for i in 0..slab.book.num_asks as usize {
        let order = &slab.book.asks[i];
        if order.owner == slab.header.lp_owner {
            order_ids.push(order.order_id as u128);
        }
    }

    // Cancel all collected orders
    process_ob_remove_by_ids(slab, &order_ids)
}
