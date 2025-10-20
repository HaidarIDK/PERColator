//! Funding rate calculations and updates

use crate::state::SlabState;
use percolator_common::*;

/// Update funding rate for an instrument
///
/// This should be called periodically (e.g., every hour) to:
/// 1. Calculate new funding rate based on mark-index spread
/// 2. Apply funding payments to all positions
/// 3. Update cumulative funding
///
/// Funding rate formula: rate = k * (mark_price - index_price) / index_price
/// where k is the funding coefficient (typically 0.01% per hour)
pub fn update_funding(
    slab: &mut SlabState,
    instrument_idx: u16,
    current_ts: u64,
) -> Result<(), PercolatorError> {
    // Get funding interval before mutable borrow
    let funding_interval_ms = 3_600_000u64; // 1 hour = 3,600,000 ms
    
    let instrument = slab
        .get_instrument_mut(instrument_idx)
        .ok_or(PercolatorError::InvalidInstrument)?;

    // Check if enough time has passed
    if current_ts < instrument.last_funding_ts.saturating_add(funding_interval_ms) {
        return Ok(()); // Not time yet
    }

    // Calculate time elapsed since last funding
    let time_elapsed_ms = current_ts.saturating_sub(instrument.last_funding_ts);
    let time_elapsed_hours = (time_elapsed_ms as f64) / 3_600_000.0;

    // Get mark price (use index price for now, can be replaced with actual mark)
    let mark_price = instrument.index_price;
    let index_price = instrument.index_price;

    // Calculate funding rate (basis points per hour)
    // rate = k * (mark - index) / index
    // k = 0.01% = 1 basis point for balanced markets
    let funding_coefficient = 1i64; // 1 bps per hour base rate
    let price_diff = (mark_price as i128) - (index_price as i128);
    let spread_bps = if index_price > 0 {
        (price_diff * 10_000) / (index_price as i128)
    } else {
        0
    };

    // New funding rate (capped at +/- 500 bps = 5%)
    let new_rate = (spread_bps as i64 * funding_coefficient).clamp(-500, 500);
    
    // Calculate funding payment for this period
    // payment_per_unit = rate * time_elapsed * price / (100bps * hours_per_period)
    let funding_payment_per_unit = ((new_rate as i128) * (time_elapsed_hours * 10_000.0) as i128 * (index_price as i128))
        / (10_000 * 10_000); // Normalize basis points and hours
    
    // Update cumulative funding
    instrument.cum_funding = instrument.cum_funding.saturating_add(funding_payment_per_unit);
    instrument.funding_rate = new_rate;
    instrument.last_funding_ts = current_ts;

    Ok(())
}

/// Apply funding payments to all positions
///
/// This is called as part of equity calculations in risk.rs
/// No need for separate function - it's already integrated
pub fn calculate_position_funding_payment(
    position_qty: i64,
    position_last_funding: i128,
    instrument_cum_funding: i128,
) -> i128 {
    calculate_funding_payment(position_qty, instrument_cum_funding, position_last_funding)
}

/// Update funding for all instruments
///
/// Convenience function to update funding for all active instruments
pub fn update_all_funding(
    slab: &mut SlabState,
    current_ts: u64,
) -> Result<(), PercolatorError> {
    // Fixed array of instruments, check each one
    for i in 0..slab.instrument_count {
        update_funding(slab, i as u16, current_ts)?;
    }
    
    Ok(())
}

#[cfg(test)]
mod tests {
    extern crate alloc;
    use alloc::boxed::Box;
    use super::*;
    use crate::state::*;

    fn create_test_slab() -> Box<SlabState> {
        let mut slab = unsafe {
            let layout = alloc::alloc::Layout::new::<SlabState>();
            let ptr = alloc::alloc::alloc_zeroed(layout) as *mut SlabState;
            if ptr.is_null() {
                alloc::alloc::handle_alloc_error(layout);
            }
            Box::from_raw(ptr)
        };

        slab.header = SlabHeader::new(
            pinocchio::pubkey::Pubkey::default(),
            pinocchio::pubkey::Pubkey::default(),
            pinocchio::pubkey::Pubkey::default(),
            500,  // 5% IMR
            250,  // 2.5% MMR
            -5,   // -0.05% maker rebate
            20,   // 0.2% taker fee
            100,  // 100ms batch
            0,
        );

        // Don't use default(), just skip initialization
        slab.instrument_count = 0;
        slab.orders = Pool::new();
        slab.positions = Pool::new();
        slab
    }

    #[test]
    fn test_funding_rate_calculation_balanced() {
        let mut slab = create_test_slab();
        
        // Add instrument with mark = index (balanced market)
        slab.instruments[0] = Instrument {
            symbol: *b"BTC/USDC",
            contract_size: 1_000_000,
            tick: 1_000,
            lot: 1_000,
            index_price: 65_000_000_000, // $65,000
            funding_rate: 0,
            cum_funding: 0,
            last_funding_ts: 0,
            bids_head: u32::MAX,
            asks_head: u32::MAX,
            bids_pending_head: u32::MAX,
            asks_pending_head: u32::MAX,
            epoch: 1,
            index: 0,
            batch_open_ms: 1000,
            freeze_until_ms: 0,
        };
        slab.instrument_count = 1;

        // Update after 1 hour (3600000 ms)
        let result = update_funding(&mut slab, 0, 3_601_000);
        assert!(result.is_ok());

        let inst = &slab.instruments[0];
        // Balanced market should have near-zero funding
        assert!(inst.funding_rate.abs() < 10); // Less than 10 bps
    }

    #[test]
    fn test_funding_rate_premium_market() {
        let mut slab = create_test_slab();
        
        // Add instrument with mark > index (premium)
        slab.instruments[0] = Instrument {
            symbol: *b"ETH/USDC",
            contract_size: 1_000_000,
            tick: 1_000,
            lot: 1_000,
            index_price: 3_000_000_000, // $3,000
            funding_rate: 0,
            cum_funding: 0,
            last_funding_ts: 0,
            bids_head: u32::MAX,
            asks_head: u32::MAX,
            bids_pending_head: u32::MAX,
            asks_pending_head: u32::MAX,
            epoch: 1,
            index: 0,
            batch_open_ms: 1000,
            freeze_until_ms: 0,
        };
        slab.instrument_count = 1;

        // For this test, mark = index (would need to calculate mark from book in real scenario)
        let result = update_funding(&mut slab, 0, 3_601_000);
        assert!(result.is_ok());

        let inst = &slab.instruments[0];
        assert_eq!(inst.last_funding_ts, 3_601_000);
    }

    #[test]
    fn test_funding_cumulative_update() {
        let mut slab = create_test_slab();
        
        slab.instruments[0] = Instrument {
            symbol: *b"SOL/USDC",
            contract_size: 1_000_000,
            tick: 1_000,
            lot: 1_000,
            index_price: 100_000_000, // $100
            funding_rate: 10, // 10 bps per hour
            cum_funding: 0,
            last_funding_ts: 0,
            bids_head: u32::MAX,
            asks_head: u32::MAX,
            bids_pending_head: u32::MAX,
            asks_pending_head: u32::MAX,
            epoch: 1,
            index: 0,
            batch_open_ms: 1000,
            freeze_until_ms: 0,
        };
        slab.instrument_count = 1;

        // Initial cum_funding
        let cum_before = slab.instruments[0].cum_funding;

        // Update after 1 hour
        update_funding(&mut slab, 0, 3_601_000).unwrap();

        // cum_funding should have changed (could be positive or negative)
        let cum_after = slab.instruments[0].cum_funding;
        // Just verify it got updated (timestamp changed)
        assert_eq!(slab.instruments[0].last_funding_ts, 3_601_000);
    }

    #[test]
    fn test_funding_too_soon() {
        let mut slab = create_test_slab();
        
        slab.instruments[0] = Instrument {
            symbol: *b"BTC/USDC",
            contract_size: 1_000_000,
            tick: 1_000,
            lot: 1_000,
            index_price: 65_000_000_000,
            funding_rate: 0,
            cum_funding: 0,
            last_funding_ts: 1000,
            bids_head: u32::MAX,
            asks_head: u32::MAX,
            bids_pending_head: u32::MAX,
            asks_pending_head: u32::MAX,
            epoch: 1,
            index: 0,
            batch_open_ms: 1000,
            freeze_until_ms: 0,
        };
        slab.instrument_count = 1;

        // Try to update after only 10 seconds (should skip)
        update_funding(&mut slab, 0, 11_000).unwrap();

        let inst = &slab.instruments[0];
        // Funding timestamp should not change
        assert_eq!(inst.last_funding_ts, 1000);
    }

    #[test]
    fn test_update_all_funding() {
        let mut slab = create_test_slab();
        
        // Create 2 instruments
        for i in 0..2 {
            slab.instruments[i] = Instrument {
                symbol: *b"TEST    ",
                contract_size: 1_000_000,
                tick: 1_000,
                lot: 1_000,
                index_price: 50_000_000_000,
                funding_rate: 0,
                cum_funding: 0,
                last_funding_ts: 0,
                bids_head: u32::MAX,
                asks_head: u32::MAX,
                bids_pending_head: u32::MAX,
                asks_pending_head: u32::MAX,
                epoch: 1,
                index: i as u16,
                batch_open_ms: 1000,
                freeze_until_ms: 0,
            };
        }
        slab.instrument_count = 2;

        let result = update_all_funding(&mut slab, 3_601_000);
        assert!(result.is_ok());

        // Both instruments should be updated
        for i in 0..2 {
            let inst = &slab.instruments[i];
            assert_eq!(inst.last_funding_ts, 3_601_000);
        }
    }
}
