//! Cancel LP orders to reduce Slab LP exposure
//!
//! This is the ONLY way to reduce Slab LP exposure. This instruction:
//! - Cancels resting orders on the slab
//! - Frees reserved quote/base
//! - Updates margin proportionally
//! - Maintains precise reservation accounting
//!
//! CRITICAL INVARIANT: Slab LP can ONLY be reduced via this instruction

use crate::state::{Portfolio, VenueId, VenueKind};
use percolator_common::*;
use pinocchio::msg;

/// Process cancel LP orders instruction
///
/// This is the ONLY way Slab LP exposure can be reduced.
///
/// # Arguments
/// * `portfolio` - User's portfolio account (mutable)
/// * `market_id` - Slab market pubkey
/// * `order_ids` - Array of order IDs to cancel
/// * `order_count` - Number of orders to cancel
/// * `freed_quote` - Total quote freed by cancellations
/// * `freed_base` - Total base freed by cancellations
///
/// # Returns
/// * Updates portfolio:
///   - Removes order IDs from bucket
///   - Reduces reserved_quote and reserved_base
///   - Reduces bucket margin proportionally
///   - If no orders remain, removes bucket entirely
///
/// # Safety
/// * Enforces exact reservation accounting
/// * Verifies all order IDs exist before canceling
/// * Maintains proportional margin reduction
pub fn process_cancel_lp_orders(
    portfolio: &mut Portfolio,
    market_id: pinocchio::pubkey::Pubkey,
    order_ids: &[u64],
    order_count: usize,
    freed_quote: u128,
    freed_base: u128,
) -> Result<(), PercolatorError> {
    msg!("CancelLpOrders: Starting");

    // Safety check: must cancel at least one order
    if order_count == 0 || order_ids.is_empty() {
        msg!("Error: Must cancel at least one order");
        return Err(PercolatorError::InvalidAmount);
    }

    // Find Slab LP bucket for this market
    let venue_id = VenueId::new_slab(market_id);
    let bucket_idx = {
        let mut idx: Option<usize> = None;
        for i in 0..portfolio.lp_bucket_count as usize {
            if portfolio.lp_buckets[i].active && &portfolio.lp_buckets[i].venue == &venue_id {
                idx = Some(i);
                break;
            }
        }
        idx.ok_or(PercolatorError::InvalidAccount)?
    };

    msg!("CancelLpOrders: Found bucket");

    // Get mutable reference to bucket
    let bucket = &mut portfolio.lp_buckets[bucket_idx];

    // Verify this is a Slab bucket
    if bucket.venue.venue_kind != VenueKind::Slab {
        msg!("Error: Bucket is not Slab type");
        return Err(PercolatorError::InvalidAccount);
    }

    // Get Slab LP data
    let slab = bucket.slab.as_mut().ok_or(PercolatorError::InvalidAccount)?;

    msg!("CancelLpOrders: Initial state");

    // Store initial state for proportional calculation
    let initial_reserved_quote = slab.reserved_quote;
    let initial_reserved_base = slab.reserved_base;
    let initial_im = bucket.im;
    let initial_mm = bucket.mm;

    // Remove each order
    for i in 0..order_count.min(order_ids.len()) {
        let order_id = order_ids[i];

        // Find and remove order (note: we don't need to know the exact quote/base per order
        // since the caller provides the total freed amounts)
        let mut found = false;
        for j in 0..slab.open_order_count as usize {
            if slab.open_order_ids[j] == order_id {
                // Remove this order ID (swap with last)
                let last_idx = (slab.open_order_count - 1) as usize;
                if j != last_idx {
                    slab.open_order_ids[j] = slab.open_order_ids[last_idx];
                }
                slab.open_order_ids[last_idx] = 0;
                slab.open_order_count -= 1;
                found = true;
                break;
            }
        }

        if !found {
            msg!("Error: Order ID not found");
            return Err(PercolatorError::InvalidAccount);
        }
    }

    msg!("CancelLpOrders: Removed orders");

    // SAFETY TRIPWIRE 1: Exact reservation accounting
    // Verify freed amounts don't exceed current reservations
    if freed_quote > slab.reserved_quote {
        msg!("Error: Freed quote exceeds reserved");
        return Err(PercolatorError::InvalidAmount);
    }

    if freed_base > slab.reserved_base {
        msg!("Error: Freed base exceeds reserved");
        return Err(PercolatorError::InvalidAmount);
    }

    // Update reservations
    slab.reserved_quote = slab.reserved_quote.saturating_sub(freed_quote);
    slab.reserved_base = slab.reserved_base.saturating_sub(freed_base);

    msg!("CancelLpOrders: Updated reservations");

    // Calculate proportional margin reduction
    // We use the maximum of (quote_freed / initial_quote, base_freed / initial_base)
    // to determine the reduction ratio
    let quote_ratio = if initial_reserved_quote > 0 {
        ((initial_reserved_quote - slab.reserved_quote) as u128 * 1_000_000) / (initial_reserved_quote as u128)
    } else {
        0
    };

    let base_ratio = if initial_reserved_base > 0 {
        ((initial_reserved_base - slab.reserved_base) as u128 * 1_000_000) / (initial_reserved_base as u128)
    } else {
        0
    };

    // Use the larger ratio (more conservative)
    let reduction_ratio = quote_ratio.max(base_ratio);

    msg!("CancelLpOrders: Reduction ratio calculated");

    // Calculate new margin
    let new_im: u128;
    let new_mm: u128;

    if slab.open_order_count == 0 {
        // All orders canceled - zero out margin
        new_im = 0;
        new_mm = 0;
        msg!("CancelLpOrders: All orders canceled, zeroing margin");
    } else {
        // Proportional reduction based on freed reservations
        new_im = initial_im - ((initial_im * reduction_ratio as u128) / 1_000_000);
        new_mm = initial_mm - ((initial_mm * reduction_ratio as u128) / 1_000_000);
        msg!("CancelLpOrders: Proportional margin reduction");
    }

    // Update bucket margin
    bucket.im = new_im;
    bucket.mm = new_mm;

    msg!("CancelLpOrders: Updated margin");

    // If no orders remain, remove bucket
    if slab.open_order_count == 0 {
        // Deactivate bucket
        bucket.active = false;

        // Swap with last bucket and decrement count
        let last_idx = (portfolio.lp_bucket_count - 1) as usize;
        if bucket_idx != last_idx {
            portfolio.lp_buckets[bucket_idx] = portfolio.lp_buckets[last_idx];
        }

        // Zero out last bucket
        unsafe {
            core::ptr::write_bytes(
                &mut portfolio.lp_buckets[last_idx] as *mut _,
                0,
                1,
            );
        }

        portfolio.lp_bucket_count -= 1;

        msg!("CancelLpOrders: Removed bucket entirely (all orders canceled)");
    }

    msg!("CancelLpOrders: Complete");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use pinocchio::pubkey::Pubkey;
    use crate::state::LpBucket;

    #[test]
    fn test_cancel_all_orders() {
        let mut portfolio = Portfolio::new(Pubkey::default(), Pubkey::default(), 0);

        let market = Pubkey::from([1; 32]);
        let venue_id = VenueId::new_slab(market);
        let mut bucket = LpBucket::new_slab(venue_id);
        bucket.update_margin(10_000, 5_000);

        // Add 3 orders
        if let Some(ref mut slab) = bucket.slab {
            assert!(slab.add_reservation(1001, 1000, 500).is_ok());
            assert!(slab.add_reservation(1002, 2000, 1000).is_ok());
            assert!(slab.add_reservation(1003, 1500, 750).is_ok());
        }

        assert!(portfolio.add_lp_bucket(bucket).is_ok());

        // Cancel all 3 orders
        let order_ids = [1001, 1002, 1003];
        let result = process_cancel_lp_orders(
            &mut portfolio,
            market,
            &order_ids,
            3,
            4500,  // Total freed quote
            2250,  // Total freed base
        );

        assert!(result.is_ok());

        // Bucket should be removed
        assert_eq!(portfolio.lp_bucket_count, 0);
    }

    #[test]
    fn test_cancel_partial_orders() {
        let mut portfolio = Portfolio::new(Pubkey::default(), Pubkey::default(), 0);

        let market = Pubkey::from([1; 32]);
        let venue_id = VenueId::new_slab(market);
        let mut bucket = LpBucket::new_slab(venue_id);
        bucket.update_margin(10_000, 5_000);

        // Add 3 orders
        if let Some(ref mut slab) = bucket.slab {
            assert!(slab.add_reservation(1001, 1000, 500).is_ok());
            assert!(slab.add_reservation(1002, 2000, 1000).is_ok());
            assert!(slab.add_reservation(1003, 1500, 750).is_ok());
        }

        assert!(portfolio.add_lp_bucket(bucket).is_ok());

        // Cancel 1 order (the middle one)
        let order_ids = [1002];
        let result = process_cancel_lp_orders(
            &mut portfolio,
            market,
            &order_ids,
            1,
            2000,  // Freed quote
            1000,  // Freed base
        );

        assert!(result.is_ok());

        // Bucket should still exist
        assert_eq!(portfolio.lp_bucket_count, 1);

        // Find bucket
        let bucket = portfolio.find_lp_bucket(&venue_id).unwrap();
        let slab = bucket.slab.as_ref().unwrap();

        // Should have 2 orders left
        assert_eq!(slab.open_order_count, 2);

        // Reservations: 4500 - 2000 = 2500 quote, 2250 - 1000 = 1250 base
        assert_eq!(slab.reserved_quote, 2500);
        assert_eq!(slab.reserved_base, 1250);

        // Margin should be reduced proportionally
        // Freed 2000/4500 = 44.4% of quote, or 1000/2250 = 44.4% of base
        // So margin should be reduced by ~44.4%
        // 5000 - (5000 * 0.444) = ~2780
        // Due to integer math, should be close to this
        assert!(bucket.mm < 5000);
        assert!(bucket.mm > 2500);
    }

    #[test]
    fn test_reject_freed_exceeds_reserved() {
        let mut portfolio = Portfolio::new(Pubkey::default(), Pubkey::default(), 0);

        let market = Pubkey::from([1; 32]);
        let venue_id = VenueId::new_slab(market);
        let mut bucket = LpBucket::new_slab(venue_id);

        if let Some(ref mut slab) = bucket.slab {
            assert!(slab.add_reservation(1001, 1000, 500).is_ok());
        }

        assert!(portfolio.add_lp_bucket(bucket).is_ok());

        // Try to free more than reserved
        let order_ids = [1001];
        let result = process_cancel_lp_orders(
            &mut portfolio,
            market,
            &order_ids,
            1,
            2000,  // More than 1000 reserved
            500,
        );

        // Should fail
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), PercolatorError::InvalidAmount);
    }

    #[test]
    fn test_reject_order_not_found() {
        let mut portfolio = Portfolio::new(Pubkey::default(), Pubkey::default(), 0);

        let market = Pubkey::from([1; 32]);
        let venue_id = VenueId::new_slab(market);
        let mut bucket = LpBucket::new_slab(venue_id);

        if let Some(ref mut slab) = bucket.slab {
            assert!(slab.add_reservation(1001, 1000, 500).is_ok());
        }

        assert!(portfolio.add_lp_bucket(bucket).is_ok());

        // Try to cancel non-existent order
        let order_ids = [9999];
        let result = process_cancel_lp_orders(
            &mut portfolio,
            market,
            &order_ids,
            1,
            1000,
            500,
        );

        // Should fail
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), PercolatorError::InvalidAccount);
    }

    #[test]
    fn test_reject_zero_orders() {
        let mut portfolio = Portfolio::new(Pubkey::default(), Pubkey::default(), 0);

        let market = Pubkey::from([1; 32]);
        let venue_id = VenueId::new_slab(market);
        let bucket = LpBucket::new_slab(venue_id);
        assert!(portfolio.add_lp_bucket(bucket).is_ok());

        // Try to cancel zero orders
        let order_ids: [u64; 0] = [];
        let result = process_cancel_lp_orders(
            &mut portfolio,
            market,
            &order_ids,
            0,
            0,
            0,
        );

        // Should fail
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), PercolatorError::InvalidAmount);
    }

    #[test]
    fn test_reject_amm_bucket() {
        let mut portfolio = Portfolio::new(Pubkey::default(), Pubkey::default(), 0);

        let market = Pubkey::from([1; 32]);
        let venue_id = VenueId::new_amm(market);
        let bucket = LpBucket::new_amm(venue_id, 1000, 60_000_000, 100);
        assert!(portfolio.add_lp_bucket(bucket).is_ok());

        // Try to cancel orders from AMM bucket (should fail)
        let order_ids = [1001];
        let result = process_cancel_lp_orders(
            &mut portfolio,
            market,
            &order_ids,
            1,
            1000,
            500,
        );

        // Should fail - can't cancel orders from AMM bucket
        assert!(result.is_err());
    }
}
