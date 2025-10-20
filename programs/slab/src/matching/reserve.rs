//! Reserve operation - walk book and lock slices without executing

use crate::state::SlabState;
use percolator_common::*;

/// Reserve result
pub struct ReserveResult {
    pub hold_id: u64,
    pub vwap_px: u64,
    pub worst_px: u64,
    pub max_charge: u128,
    pub expiry_ms: u64,
    pub book_seqno: u64,
    pub filled_qty: u64,
}

/// Reserve liquidity from the book
pub fn reserve(
    slab: &mut SlabState,
    account_idx: u32,
    instrument_idx: u16,
    side: Side,
    qty: u64,
    limit_px: u64,
    ttl_ms: u64,
    commitment_hash: [u8; 32],
    route_id: u64,
) -> Result<ReserveResult, PercolatorError> {
    // Validate instrument and get needed values
    let (tick, lot, contract_size, index_price, freeze_until_ms) = {
        let instrument = slab
            .get_instrument(instrument_idx)
            .ok_or(PercolatorError::InvalidInstrument)?;

        (instrument.tick, instrument.lot, instrument.contract_size, instrument.index_price, instrument.freeze_until_ms)
    };

    // ANTI-TOXICITY #4: Freeze Window Enforcement
    // During freeze window, only DLP accounts can take contra liquidity
    // This prevents JIT liquidity from sniping during batch execution
    let current_ts = slab.header.current_ts;
    if current_ts < freeze_until_ms {
        // Freeze is active - check if account is DLP
        if !slab.is_dlp(account_idx) {
            // Non-DLP accounts cannot reserve during freeze window
            return Err(PercolatorError::OrderFrozen);
        }
    }

    // Check alignment
    if !is_tick_aligned(limit_px, tick) {
        return Err(PercolatorError::PriceNotAligned);
    }
    if !is_lot_aligned(qty, lot) {
        return Err(PercolatorError::QuantityNotAligned);
    }

    // Allocate reservation
    let resv_idx = slab
        .reservations
        .alloc()
        .ok_or(PercolatorError::PoolFull)?;

    let hold_id = slab.header.next_hold_id();

    // Walk contra book and reserve slices
    let contra_side = match side {
        Side::Buy => Side::Sell,
        Side::Sell => Side::Buy,
    };

    let (filled_qty, total_notional, worst_px, slice_head) =
        walk_and_reserve(slab, account_idx, instrument_idx, contra_side, qty, limit_px, resv_idx)?;

    // Calculate VWAP
    let vwap_px = if filled_qty > 0 {
        calculate_vwap(total_notional, filled_qty)
    } else {
        limit_px
    };

    // Calculate max charge (notional + fees)
    let taker_fee = slab.header.taker_fee;
    let max_charge = calculate_max_charge(filled_qty, worst_px, contract_size, taker_fee);

    // Create reservation
    let book_seqno = slab.header.book_seqno;
    let current_ts = slab.header.current_ts;
    let expiry_ms = current_ts.saturating_add(ttl_ms);

    if let Some(resv) = slab.reservations.get_mut(resv_idx) {
        *resv = Reservation {
            hold_id,
            route_id,
            account_idx,
            instrument_idx,
            side,
            _padding: 0,
            qty: filled_qty,
            vwap_px,
            worst_px,
            max_charge,
            commitment_hash,
            salt: [0; 16], // Will be revealed at commit
            book_seqno,
            expiry_ms,
            reserve_oracle_px: index_price, // Capture oracle price for kill band check
            slice_head,
            index: resv_idx,
            used: true,
            committed: false,
            _padding2: [0; 6],
        };
    }

    Ok(ReserveResult {
        hold_id,
        vwap_px,
        worst_px,
        max_charge,
        expiry_ms,
        book_seqno,
        filled_qty,
    })
}

/// Walk book and create reservation slices
fn walk_and_reserve(
    slab: &mut SlabState,
    account_idx: u32,
    instrument_idx: u16,
    side: Side,
    qty: u64,
    limit_px: u64,
    _resv_idx: u32,
) -> Result<(u64, u128, u64, u32), PercolatorError> {
    let head = {
        let instrument = slab
            .get_instrument(instrument_idx)
            .ok_or(PercolatorError::InvalidInstrument)?;

        match side {
            Side::Buy => instrument.bids_head,
            Side::Sell => instrument.asks_head,
        }
    };

    // Get freeze parameters for Top-K freeze logic
    let (freeze_until_ms, freeze_levels) = {
        let instrument = slab.get_instrument(instrument_idx).unwrap();
        (instrument.freeze_until_ms, slab.header.freeze_levels)
    };
    let current_ts = slab.header.current_ts;
    let freeze_active = current_ts < freeze_until_ms && freeze_levels > 0;
    let is_dlp_account = slab.is_dlp(account_idx);

    let mut curr_idx = head;
    let mut qty_left = qty;
    let mut total_notional: u128 = 0;
    let mut worst_px = limit_px;
    let mut slice_head = u32::MAX;
    let mut slice_tail = u32::MAX;
    let mut price_level_count = 0u16;
    let mut last_price = None;

    while curr_idx != u32::MAX && qty_left > 0 {
        // Get order info (immutable borrow)
        let (order_price, order_qty, order_reserved_qty, order_next) = {
            let order = slab
                .orders
                .get(curr_idx)
                .ok_or(PercolatorError::OrderNotFound)?;

            (order.price, order.qty, order.reserved_qty, order.next)
        };

        // ANTI-TOXICITY #5: Top-K Freeze Logic
        // Track price levels and enforce freeze on top K levels for non-DLP
        if last_price != Some(order_price) {
            price_level_count += 1;
            last_price = Some(order_price);
        }
        
        // If freeze is active, non-DLP accounts cannot access top K levels
        if freeze_active && !is_dlp_account && price_level_count <= freeze_levels {
            // Skip orders in frozen levels for non-DLP accounts
            curr_idx = order_next;
            continue;
        }

        // Check price limit
        let crosses = match side {
            Side::Buy => order_price <= limit_px,
            Side::Sell => order_price >= limit_px,
        };

        if !crosses {
            break;
        }

        // Calculate available quantity
        let available = order_qty.saturating_sub(order_reserved_qty);
        if available == 0 {
            curr_idx = order_next;
            continue;
        }

        let take_qty = core::cmp::min(qty_left, available);

        // Allocate slice
        let slice_idx = slab.slices.alloc().ok_or(PercolatorError::PoolFull)?;

        // Create slice
        if let Some(slice) = slab.slices.get_mut(slice_idx) {
            *slice = Slice {
                order_idx: curr_idx,
                qty: take_qty,
                next: u32::MAX,
                index: slice_idx,
                used: true,
                _padding: [0; 7],
            };

            // Link slice
            if slice_head == u32::MAX {
                slice_head = slice_idx;
            } else if let Some(tail) = slab.slices.get_mut(slice_tail) {
                tail.next = slice_idx;
            }
            slice_tail = slice_idx;
        }

        // Update order reserved quantity
        if let Some(order) = slab.orders.get_mut(curr_idx) {
            order.reserved_qty = order.reserved_qty.saturating_add(take_qty);
        }

        // Update totals
        qty_left = qty_left.saturating_sub(take_qty);
        total_notional = total_notional.saturating_add(mul_u64(take_qty, order_price));
        worst_px = order_price;

        curr_idx = order_next;
    }

    let filled_qty = qty.saturating_sub(qty_left);

    Ok((filled_qty, total_notional, worst_px, slice_head))
}

/// Calculate maximum charge including fees
fn calculate_max_charge(filled_qty: u64, price: u64, contract_size: u64, taker_fee_bps: u64) -> u128 {
    let notional = mul_u64(filled_qty, contract_size);
    let value = mul_u64_u128(price, notional);
    let fee = (value * (taker_fee_bps as u128)) / 10_000;
    value.saturating_add(fee)
}

#[cfg(test)]
mod tests {
    extern crate alloc;
    use alloc::boxed::Box;
    use super::*;
    use crate::state::*;
    use crate::matching::insert_order;

    #[test]
    fn test_max_charge_calculation() {
        // 100 contracts at 50,000 price, 0.001 contract size, 0.1% taker fee
        let max_charge = calculate_max_charge(100, 50_000, 1000, 10);

        // Notional = 100 * 1000 = 100,000
        // Value = 100,000 * 50,000 = 5,000,000,000
        // Fee = 5,000,000,000 * 0.001 = 5,000,000
        // Total = 5,005,000,000
        assert_eq!(max_charge, 5_005_000_000);
    }

    /// Helper to create and insert an order for tests
    fn create_and_insert_order(
        slab: &mut SlabState,
        account_idx: u32,
        instrument_idx: u16,
        side: Side,
        price: u64,
        qty: u64,
        created_ms: u64,
    ) -> Result<u32, PercolatorError> {
        let order_idx = slab.orders.alloc().ok_or(PercolatorError::PoolFull)?;
        if let Some(order) = slab.orders.get_mut(order_idx) {
            *order = Order {
                order_id: slab.header.next_order_id(),
                account_idx,
                instrument_idx,
                side,
                tif: TimeInForce::GTC,
                maker_class: MakerClass::DLP,
                state: OrderState::LIVE,
                eligible_epoch: 1,
                created_ms,
                price,
                qty,
                reserved_qty: 0,
                qty_orig: qty,
                next: u32::MAX,
                prev: u32::MAX,
                next_free: u32::MAX,
                used: true,
                _padding: [0; 3],
            };
        }
        insert_order(slab, instrument_idx, order_idx, side, price, OrderState::LIVE)?;
        Ok(order_idx)
    }

    /// Helper to create a test slab (same as in commit.rs)
    fn create_test_slab() -> Box<SlabState> {
        // Allocate on heap using alloc_zeroed to avoid stack overflow
        let mut slab = unsafe {
            let layout = alloc::alloc::Layout::new::<SlabState>();
            let ptr = alloc::alloc::alloc_zeroed(layout) as *mut SlabState;
            if ptr.is_null() {
                alloc::alloc::handle_alloc_error(layout);
            }
            Box::from_raw(ptr)
        };
        
        // Initialize header
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
        
        // Initialize first instrument
        slab.instruments[0] = Instrument {
            symbol: *b"BTC-PERP",
            contract_size: 1000,
            tick: 100,
            lot: 1,
            index_price: 50_000_000, // $50k with 6 decimals
            funding_rate: 0,
            cum_funding: 0,
            last_funding_ts: 0,
            bids_head: u32::MAX,
            asks_head: u32::MAX,
            bids_pending_head: u32::MAX,
            asks_pending_head: u32::MAX,
            epoch: 1,
            index: 0,
            batch_open_ms: 0,
            freeze_until_ms: 0,
        };
        slab.instrument_count = 1;
        
        // Initialize pools
        slab.orders = Pool::new();
        slab.positions = Pool::new();
        slab.reservations = Pool::new();
        slab.slices = Pool::new();
        slab.aggressor_ledger = Pool::new();
        
        slab
    }

    #[test]
    fn test_freeze_window_rejects_non_dlp() {
        let mut slab = create_test_slab();
        
        // Create a non-DLP account
        let account_idx = 0u32;
        slab.accounts[account_idx as usize].active = true;
        slab.accounts[account_idx as usize].index = account_idx;
        slab.accounts[account_idx as usize].cash = 1_000_000_000;
        
        // Set freeze window active
        slab.header.current_ts = 1000;
        slab.instruments[0].freeze_until_ms = 1100; // Frozen until 1100ms
        slab.instruments[0].batch_open_ms = 1000;
        
        // Add a maker order
        let maker_idx = 1u32;
        slab.accounts[maker_idx as usize].active = true;
        slab.accounts[maker_idx as usize].index = maker_idx;
        
        create_and_insert_order(&mut slab, maker_idx, 0, Side::Sell, 50_000_000, 10, 1000).unwrap();
        
        // Try to reserve as non-DLP during freeze - should fail
        let result = reserve(
            &mut slab,
            account_idx,
            0,
            Side::Buy,
            5,
            50_000_000,
            10_000,
            [0u8; 32],
            12345,
        );
        
        assert!(matches!(result, Err(PercolatorError::OrderFrozen)));
    }

    #[test]
    fn test_freeze_window_allows_dlp() {
        let mut slab = create_test_slab();
        
        // Create a DLP account
        let account_idx = 0u32;
        slab.accounts[account_idx as usize].active = true;
        slab.accounts[account_idx as usize].index = account_idx;
        slab.accounts[account_idx as usize].cash = 1_000_000_000;
        
        // Add as DLP
        slab.add_dlp(account_idx).unwrap();
        
        // Set freeze window active
        slab.header.current_ts = 1000;
        slab.instruments[0].freeze_until_ms = 1100;
        slab.instruments[0].batch_open_ms = 1000;
        
        // Add a maker order
        let maker_idx = 1u32;
        slab.accounts[maker_idx as usize].active = true;
        slab.accounts[maker_idx as usize].index = maker_idx;
        
        create_and_insert_order(&mut slab, maker_idx, 0, Side::Sell, 50_000_000, 10, 1000).unwrap();
        
        // Try to reserve as DLP during freeze - should succeed
        let result = reserve(
            &mut slab,
            account_idx,
            0,
            Side::Buy,
            5,
            50_000_000,
            10_000,
            [0u8; 32],
            12345,
        );
        
        assert!(result.is_ok());
    }

    #[test]
    fn test_freeze_window_expired_allows_non_dlp() {
        let mut slab = create_test_slab();
        
        // Create a non-DLP account
        let account_idx = 0u32;
        slab.accounts[account_idx as usize].active = true;
        slab.accounts[account_idx as usize].index = account_idx;
        slab.accounts[account_idx as usize].cash = 1_000_000_000;
        
        // Set freeze window expired
        slab.header.current_ts = 1200; // After freeze
        slab.instruments[0].freeze_until_ms = 1100;
        slab.instruments[0].batch_open_ms = 1000;
        
        // Add a maker order
        let maker_idx = 1u32;
        slab.accounts[maker_idx as usize].active = true;
        slab.accounts[maker_idx as usize].index = maker_idx;
        
        create_and_insert_order(&mut slab, maker_idx, 0, Side::Sell, 50_000_000, 10, 1200).unwrap();
        
        // Try to reserve as non-DLP after freeze - should succeed
        let result = reserve(
            &mut slab,
            account_idx,
            0,
            Side::Buy,
            5,
            50_000_000,
            10_000,
            [0u8; 32],
            12345,
        );
        
        assert!(result.is_ok());
    }

    #[test]
    fn test_top_k_freeze_blocks_best_levels() {
        let mut slab = create_test_slab();
        
        // Create non-DLP account
        let account_idx = 0u32;
        slab.accounts[account_idx as usize].active = true;
        slab.accounts[account_idx as usize].index = account_idx;
        slab.accounts[account_idx as usize].cash = 1_000_000_000;
        
        // Set freeze window and freeze top 2 levels
        slab.header.current_ts = 1000;
        slab.header.freeze_levels = 2; // Freeze top 2 price levels
        slab.instruments[0].freeze_until_ms = 1100;
        slab.instruments[0].batch_open_ms = 1000;
        
        // Add maker orders at different price levels
        let maker_idx = 1u32;
        slab.accounts[maker_idx as usize].active = true;
        slab.accounts[maker_idx as usize].index = maker_idx;
        
        // Level 1: Best price
        create_and_insert_order(&mut slab, maker_idx, 0, Side::Sell, 50_000_000, 10, 900).unwrap();
        // Level 2: Second best
        create_and_insert_order(&mut slab, maker_idx, 0, Side::Sell, 50_100_000, 10, 900).unwrap();
        // Level 3: Third best (not frozen)
        create_and_insert_order(&mut slab, maker_idx, 0, Side::Sell, 50_200_000, 10, 900).unwrap();
        
        // Non-DLP reserve should skip top 2 levels and only get level 3
        let result = reserve(
            &mut slab,
            account_idx,
            0,
            Side::Buy,
            100, // Try to buy 100
            50_500_000, // Willing to pay up to 50.5
            10_000,
            [0u8; 32],
            12345,
        );
        
        if let Err(e) = result {
            panic!("Reserve failed with error: {:?}", e);
        }
        let reserve_result = result.unwrap();
        
        // Should have filled from level 3 only (price 50.2)
        // Note: If nothing fills, worst_px stays at limit_px and filled_qty = 0
        if reserve_result.filled_qty == 0 {
            // This means freeze logic blocked everything or no liquidity
            panic!("No liquidity filled! worst_px={}, filled_qty={}", 
                   reserve_result.worst_px, reserve_result.filled_qty);
        }
        assert_eq!(reserve_result.worst_px, 50_200_000);
        assert_eq!(reserve_result.filled_qty, 10); // Only got qty from level 3
    }

    #[test]
    fn test_top_k_freeze_allows_dlp_all_levels() {
        let mut slab = create_test_slab();
        
        // Create DLP account
        let account_idx = 0u32;
        slab.accounts[account_idx as usize].active = true;
        slab.accounts[account_idx as usize].index = account_idx;
        slab.accounts[account_idx as usize].cash = 1_000_000_000;
        slab.add_dlp(account_idx).unwrap();
        
        // Set freeze window and freeze top 2 levels
        slab.header.current_ts = 1000;
        slab.header.freeze_levels = 2;
        slab.instruments[0].freeze_until_ms = 1100;
        slab.instruments[0].batch_open_ms = 1000;
        
        // Add maker orders at different price levels
        let maker_idx = 1u32;
        slab.accounts[maker_idx as usize].active = true;
        slab.accounts[maker_idx as usize].index = maker_idx;
        
        // Level 1: Best price
        create_and_insert_order(&mut slab, maker_idx, 0, Side::Sell, 50_000_000, 10, 900).unwrap();
        // Level 2: Second best
        create_and_insert_order(&mut slab, maker_idx, 0, Side::Sell, 50_100_000, 10, 900).unwrap();
        
        // DLP reserve should access all levels including top 2
        let result = reserve(
            &mut slab,
            account_idx,
            0,
            Side::Buy,
            100,
            50_500_000,
            10_000,
            [0u8; 32],
            12345,
        );
        
        assert!(result.is_ok());
        let reserve_result = result.unwrap();
        
        // Should have filled from level 1 and 2 (best prices)
        assert_eq!(reserve_result.worst_px, 50_100_000);
        assert_eq!(reserve_result.filled_qty, 20); // Got qty from both levels
    }
}
