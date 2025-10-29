//! Update funding instruction - periodic funding rate calculation

use crate::state::SlabState;
use percolator_common::*;
use pinocchio::{account_info::AccountInfo, msg, pubkey::Pubkey, sysvars::{clock::Clock, Sysvar}};

/// Process update_funding instruction
///
/// Updates the cumulative funding index based on mark-oracle price deviation.
/// This should be called periodically (e.g., every hour) to update funding rates.
///
/// Uses FORMALLY VERIFIED funding logic from model_safety::funding.
///
/// # Arguments
/// * `slab` - The slab state account
/// * `authority` - LP owner (must match slab.header.lp_owner)
/// * `oracle_price` - Current oracle reference price (1e6 scale)
///
/// # Properties (Proven with Kani in model_safety::funding)
/// - F4: Overflow safety - no overflow on realistic inputs
/// - F5: Sign correctness - longs pay when mark > oracle
///
/// # Returns
/// * Updates slab.header.cum_funding and slab.header.last_funding_ts
pub fn process_update_funding(
    slab: &mut SlabState,
    authority: &Pubkey,
    oracle_price: i64,
) -> Result<(), PercolatorError> {
    // Verify authority (only LP owner can update funding)
    if &slab.header.lp_owner != authority {
        msg!("Error: Invalid authority for update_funding");
        return Err(PercolatorError::Unauthorized);
    }

    // Validate oracle price
    if oracle_price <= 0 {
        msg!("Error: Invalid oracle price");
        return Err(PercolatorError::InvalidPrice);
    }

    // Get current timestamp
    let current_ts = Clock::get()
        .map(|clock| clock.unix_timestamp as u64)
        .unwrap_or(slab.header.last_funding_ts);

    // Calculate time delta since last funding update
    let dt_seconds = if current_ts > slab.header.last_funding_ts {
        current_ts - slab.header.last_funding_ts
    } else {
        msg!("Warning: Clock regression detected, skipping funding update");
        return Ok(()); // Skip update on clock regression
    };

    // Skip update if too soon (less than 60 seconds)
    if dt_seconds < 60 {
        msg!("Funding update too frequent, skipping");
        return Ok(());
    }

    // Use mark price from SlabHeader
    let mark_price = slab.header.mark_px;

    // Funding sensitivity: 8 bps per hour = 800 (1e6 scaled)
    // This means if mark is 1% above oracle, funding rate is 0.08% per hour
    const FUNDING_SENSITIVITY: i64 = 800;

    // Call FORMALLY VERIFIED funding index update
    // Properties F4 and F5 proven with Kani
    let mut market = model_safety::funding::MarketFunding {
        cumulative_funding_index: slab.header.cum_funding,
    };

    model_safety::funding::update_funding_index(
        &mut market,
        mark_price,
        oracle_price,
        FUNDING_SENSITIVITY,
        dt_seconds,
    )
    .map_err(|_e| {
        msg!("Error: Funding update failed");
        PercolatorError::InvalidPrice
    })?;

    // Calculate the change in funding rate for logging
    let old_cum_funding = slab.header.cum_funding;
    let new_cum_funding = market.cumulative_funding_index;
    let funding_delta = new_cum_funding - old_cum_funding;

    // Update slab header
    slab.header.cum_funding = new_cum_funding;
    slab.header.last_funding_ts = current_ts;

    // Update funding_rate field for display (funding per hour in bps)
    // funding_delta is the total funding accrued over dt_seconds
    // Convert to per-hour rate: (delta * 3600 / dt_seconds)
    let funding_rate_per_hour = if dt_seconds > 0 {
        ((funding_delta as i128) * 3600 / (dt_seconds as i128)) as i64
    } else {
        0
    };
    slab.header.funding_rate = funding_rate_per_hour;

    msg!("Funding updated successfully");

    Ok(())
}

#[cfg(all(test, not(target_os = "solana")))]
mod tests {
    use super::*;

    #[test]
    fn test_update_funding_basic() {
        // This is a placeholder test - real tests would require Clock mock
        // In production, use integration tests with solana-test-validator
    }

    #[test]
    fn test_funding_sign_correctness() {
        // Property F5: When mark > oracle, longs pay shorts
        // This is verified by Kani in model_safety::funding
        // Test here would verify integration only
    }
}
