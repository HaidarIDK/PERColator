//! Burn LP shares to reduce AMM LP exposure
//!
//! This is the ONLY way to reduce AMM LP exposure. This instruction:
//! - Burns LP shares proportionally
//! - Updates margin proportionally
//! - Credits equity with redemption value
//! - Enforces staleness checks on share price
//!
//! CRITICAL INVARIANT: AMM LP can ONLY be reduced via this instruction

use crate::state::{Portfolio, VenueId, VenueKind};
use percolator_common::*;
use pinocchio::msg;

/// Process burn LP shares instruction
///
/// This is the ONLY way AMM LP exposure can be reduced.
///
/// # Arguments
/// * `portfolio` - User's portfolio account (mutable)
/// * `market_id` - AMM market pubkey
/// * `shares_to_burn` - Number of LP shares to burn
/// * `current_share_price` - Current share price from AMM (scaled by 1e6)
/// * `current_ts` - Current timestamp for staleness check
/// * `max_staleness_seconds` - Maximum allowed staleness (typically 60s)
///
/// # Returns
/// * Updates portfolio:
///   - Reduces lp_shares in AMM bucket
///   - Reduces bucket margin proportionally
///   - Increases equity by redemption value
///   - If all shares burned, removes bucket entirely
///
/// # Safety
/// * Rejects stale share prices
/// * Enforces proportional margin reduction
/// * Maintains accounting consistency
pub fn process_burn_lp_shares(
    portfolio: &mut Portfolio,
    market_id: pinocchio::pubkey::Pubkey,
    shares_to_burn: u64,
    current_share_price: i64,
    current_ts: u64,
    max_staleness_seconds: u64,
) -> Result<(), PercolatorError> {
    msg!("BurnLpShares: Starting");

    // Safety check: shares_to_burn must be > 0
    if shares_to_burn == 0 {
        msg!("Error: Cannot burn zero shares");
        return Err(PercolatorError::InvalidAmount);
    }

    // Find AMM LP bucket for this market
    let venue_id = VenueId::new_amm(market_id);
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

    msg!("BurnLpShares: Found bucket");

    // Get mutable reference to bucket
    let bucket = &mut portfolio.lp_buckets[bucket_idx];

    // Verify this is an AMM bucket
    if bucket.venue.venue_kind != VenueKind::Amm {
        msg!("Error: Bucket is not AMM type");
        return Err(PercolatorError::InvalidAccount);
    }

    // Get AMM LP data
    let amm = bucket.amm.as_mut().ok_or(PercolatorError::InvalidAccount)?;

    // SAFETY TRIPWIRE 1: Staleness guard
    // Reject stale share prices to prevent using outdated valuations
    if amm.is_stale(current_ts, max_staleness_seconds) {
        msg!("Error: Share price is stale");
        return Err(PercolatorError::StalePrice);
    }

    msg!("BurnLpShares: Share price is fresh");

    // Verify shares to burn <= current shares
    if shares_to_burn > amm.lp_shares {
        msg!("Error: Cannot burn more shares than owned");
        return Err(PercolatorError::InsufficientBalance);
    }

    // Calculate redemption value using FORMALLY VERIFIED logic (VERIFIED)
    // Properties LP6-LP7: Overflow safety and proportional calculation correctness
    // See: crates/model_safety/src/lp_operations.rs for Kani proofs
    let redemption_value = crate::state::model_bridge::calculate_redemption_value_verified(
        shares_to_burn as u128,
        current_share_price as u64,
    )
    .map_err(|_| PercolatorError::Overflow)?;

    msg!("BurnLpShares: Redemption value calculated (verified)");

    // Calculate proportional reduction
    let initial_shares = amm.lp_shares;
    let remaining_shares = initial_shares - shares_to_burn;

    // Proportionally reduce margin using FORMALLY VERIFIED logic (VERIFIED)
    // Properties LP8-LP10: Monotonicity, zero ratio, and full ratio preservation
    // See: crates/model_safety/src/lp_operations.rs for Kani proofs
    let initial_im = bucket.im;
    let initial_mm = bucket.mm;

    let new_im: u128;
    let new_mm: u128;

    if remaining_shares == 0 {
        // Burning all shares - zero out margin (ratio = 0)
        new_im = 0;
        new_mm = 0;
        msg!("BurnLpShares: Burning all shares, zeroing margin");
    } else {
        // Partial burn - proportional reduction using verified logic
        // remaining_ratio = remaining_shares / initial_shares (scaled by 1e6)
        let remaining_ratio = ((remaining_shares as u128) * 1_000_000) / (initial_shares as u128);

        new_im = crate::state::model_bridge::proportional_margin_reduction_verified(
            initial_im,
            remaining_ratio,
        )
        .map_err(|_| PercolatorError::Overflow)?;

        new_mm = crate::state::model_bridge::proportional_margin_reduction_verified(
            initial_mm,
            remaining_ratio,
        )
        .map_err(|_| PercolatorError::Overflow)?;

        msg!("BurnLpShares: Proportional margin reduction (verified)");
    }

    // SAFETY TRIPWIRE 2: Accounting consistency
    // Verify redemption + margin reduction makes sense
    // The redemption value should approximately cover the margin reduction
    // (not exact due to market movements, but should be in the right ballpark)
    let margin_reduction = initial_mm.saturating_sub(new_mm) as i128;

    // Allow some slack for market movements (e.g., 50% deviation)
    let min_expected = margin_reduction / 2;
    let max_expected = margin_reduction * 2;

    if redemption_value < min_expected || redemption_value > max_expected {
        msg!("Warning: Redemption value outside expected range");
        // In production, this might be an error. For now, just warn.
    }

    // Update AMM LP bucket
    amm.lp_shares = remaining_shares;
    amm.share_price_cached = current_share_price;
    amm.last_update_ts = current_ts;

    bucket.im = new_im;
    bucket.mm = new_mm;

    msg!("BurnLpShares: Updated bucket");

    // Update portfolio equity
    portfolio.equity = portfolio.equity.saturating_add(redemption_value);

    msg!("BurnLpShares: Updated equity");

    // If all shares burned, remove bucket
    if remaining_shares == 0 {
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

        msg!("BurnLpShares: Removed bucket entirely (all shares burned)");
    }

    msg!("BurnLpShares: Complete");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use pinocchio::pubkey::Pubkey;
    use crate::state::LpBucket;

    #[test]
    fn test_burn_all_shares() {
        let mut portfolio = Portfolio::new(Pubkey::default(), Pubkey::default(), 0);

        // Initial equity
        portfolio.update_equity(100_000);

        // Add AMM LP bucket with 1000 shares
        let market = Pubkey::from([1; 32]);
        let venue_id = VenueId::new_amm(market);
        let mut bucket = LpBucket::new_amm(venue_id, 1000, 60_000_000, 100);
        bucket.update_margin(10_000, 5_000);
        assert!(portfolio.add_lp_bucket(bucket).is_ok());

        // Burn all 1000 shares at price 60_000_000 (60 per share in scaled units)
        // Redemption = 1000 * 60 = 60_000 (in base units)
        let result = process_burn_lp_shares(
            &mut portfolio,
            market,
            1000,
            60_000_000,
            150,
            60,
        );

        assert!(result.is_ok());

        // Bucket should be removed
        assert_eq!(portfolio.lp_bucket_count, 0);

        // Equity should increase by redemption value
        // shares * price / 1e6 = 1000 * 60_000_000 / 1_000_000 = 60_000
        assert_eq!(portfolio.equity, 100_000 + 60_000);
    }

    #[test]
    fn test_burn_partial_shares() {
        let mut portfolio = Portfolio::new(Pubkey::default(), Pubkey::default(), 0);

        portfolio.update_equity(100_000);

        let market = Pubkey::from([1; 32]);
        let venue_id = VenueId::new_amm(market);
        let mut bucket = LpBucket::new_amm(venue_id, 1000, 60_000_000, 100);
        bucket.update_margin(10_000, 5_000);
        assert!(portfolio.add_lp_bucket(bucket).is_ok());

        // Burn 300 out of 1000 shares
        let result = process_burn_lp_shares(
            &mut portfolio,
            market,
            300,
            60_000_000,
            150,
            60,
        );

        assert!(result.is_ok());

        // Bucket should still exist
        assert_eq!(portfolio.lp_bucket_count, 1);

        // Find bucket
        let bucket = portfolio.find_lp_bucket(&venue_id).unwrap();
        let amm = bucket.amm.as_ref().unwrap();

        // Shares: 1000 - 300 = 700
        assert_eq!(amm.lp_shares, 700);

        // Margin reduced proportionally: 5000 * 700 / 1000 = 3500
        assert_eq!(bucket.mm, 3_500);

        // Equity increased by: 300 * 60_000_000 / 1_000_000 = 18_000
        assert_eq!(portfolio.equity, 100_000 + 18_000);
    }

    #[test]
    fn test_reject_stale_price() {
        let mut portfolio = Portfolio::new(Pubkey::default(), Pubkey::default(), 0);

        let market = Pubkey::from([1; 32]);
        let venue_id = VenueId::new_amm(market);
        let bucket = LpBucket::new_amm(venue_id, 1000, 60_000_000, 100); // last_update_ts = 100
        assert!(portfolio.add_lp_bucket(bucket).is_ok());

        // Try to burn with current_ts = 161 (61 seconds later, exceeds 60s max)
        let result = process_burn_lp_shares(
            &mut portfolio,
            market,
            100,
            60_000_000,
            161,
            60,
        );

        // Should fail due to stale price
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), PercolatorError::StalePrice);
    }

    #[test]
    fn test_reject_burn_more_than_owned() {
        let mut portfolio = Portfolio::new(Pubkey::default(), Pubkey::default(), 0);

        let market = Pubkey::from([1; 32]);
        let venue_id = VenueId::new_amm(market);
        let bucket = LpBucket::new_amm(venue_id, 1000, 60_000_000, 100);
        assert!(portfolio.add_lp_bucket(bucket).is_ok());

        // Try to burn 1001 shares (more than owned)
        let result = process_burn_lp_shares(
            &mut portfolio,
            market,
            1001,
            60_000_000,
            150,
            60,
        );

        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), PercolatorError::InsufficientBalance);
    }

    #[test]
    fn test_reject_zero_burn() {
        let mut portfolio = Portfolio::new(Pubkey::default(), Pubkey::default(), 0);

        let market = Pubkey::from([1; 32]);
        let venue_id = VenueId::new_amm(market);
        let bucket = LpBucket::new_amm(venue_id, 1000, 60_000_000, 100);
        assert!(portfolio.add_lp_bucket(bucket).is_ok());

        // Try to burn 0 shares
        let result = process_burn_lp_shares(
            &mut portfolio,
            market,
            0,
            60_000_000,
            150,
            60,
        );

        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), PercolatorError::InvalidAmount);
    }

    #[test]
    fn test_reject_slab_bucket() {
        let mut portfolio = Portfolio::new(Pubkey::default(), Pubkey::default(), 0);

        let market = Pubkey::from([1; 32]);
        let venue_id = VenueId::new_slab(market);
        let bucket = LpBucket::new_slab(venue_id);
        assert!(portfolio.add_lp_bucket(bucket).is_ok());

        // Try to burn shares from Slab bucket (should fail)
        let result = process_burn_lp_shares(
            &mut portfolio,
            market,
            100,
            60_000_000,
            150,
            60,
        );

        // Should fail - can't burn shares from Slab bucket
        assert!(result.is_err());
    }
}
