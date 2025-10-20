//! Update funding instruction - periodic funding rate updates

use crate::state::SlabState;
use crate::matching::funding::update_funding;
use percolator_common::*;

/// Update funding rate for an instrument
///
/// This instruction should be called periodically (e.g., hourly) to:
/// 1. Calculate new funding rate based on mark-index spread
/// 2. Update cumulative funding for the instrument
/// 3. Record funding timestamp
///
/// Funding is applied to positions automatically during equity calculations
/// and position updates.
pub fn process_update_funding(
    slab: &mut SlabState,
    instrument_idx: u16,
    current_ts: u64,
) -> Result<(), PercolatorError> {
    update_funding(slab, instrument_idx, current_ts)
}

/// Update funding for all instruments at once
///
/// Convenience instruction for updating all instruments in one call
pub fn process_update_all_funding(
    slab: &mut SlabState,
    current_ts: u64,
) -> Result<(), PercolatorError> {
    crate::matching::funding::update_all_funding(slab, current_ts)
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

        slab.instrument_count = 0;
        slab.orders = Pool::new();
        slab.positions = Pool::new();
        slab
    }

    #[test]
    fn test_process_update_funding() {
        let mut slab = create_test_slab();
        
        slab.instruments[0] = Instrument {
            symbol: *b"BTC/USDC",
            contract_size: 1_000_000,
            tick: 1_000,
            lot: 1_000,
            index_price: 65_000_000_000,
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

        // Process funding update
        let result = process_update_funding(&mut slab, 0, 3_601_000);
        assert!(result.is_ok());

        let inst = &slab.instruments[0];
        assert_eq!(inst.last_funding_ts, 3_601_000);
    }

    #[test]
    fn test_process_update_all_funding() {
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

        let result = process_update_all_funding(&mut slab, 3_601_000);
        assert!(result.is_ok());

        // Both instruments should be updated
        for i in 0..2 {
            let inst = &slab.instruments[i];
            assert_eq!(inst.last_funding_ts, 3_601_000);
        }
    }
}

