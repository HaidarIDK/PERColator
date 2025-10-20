//! Commit operation - execute trades at reserved prices

use crate::state::SlabState;
use percolator_common::*;

/// Commit result
pub struct CommitResult {
    pub filled_qty: u64,
    pub avg_price: u64,
    pub total_fee: u128,
    pub total_debit: u128,
}

/// Commit a reservation and execute trades
pub fn commit(
    slab: &mut SlabState,
    hold_id: u64,
    current_ts: u64,
) -> Result<CommitResult, PercolatorError> {
    // Find reservation
    let resv_idx = find_reservation(slab, hold_id)?;

    // Validate reservation
    let resv = slab
        .reservations
        .get(resv_idx)
        .ok_or(PercolatorError::ReservationNotFound)?;

    if current_ts > resv.expiry_ms {
        return Err(PercolatorError::ReservationExpired);
    }

    if resv.committed {
        return Err(PercolatorError::InvalidReservation);
    }

    let account_idx = resv.account_idx;
    let instrument_idx = resv.instrument_idx;
    let side = resv.side;
    let slice_head = resv.slice_head;
    let reserve_oracle_px = resv.reserve_oracle_px;

    // ANTI-TOXICITY CHECK #1: Kill Band
    // Reject if oracle moved too much since reserve time
    check_kill_band(slab, instrument_idx, reserve_oracle_px)?;

    // Execute all slices
    let (filled_qty, total_notional, total_fee) =
        execute_slices(slab, slice_head, account_idx, instrument_idx, side, current_ts)?;

    // Calculate average price
    let avg_price = if filled_qty > 0 {
        calculate_vwap(total_notional, filled_qty)
    } else {
        0
    };

    let mut total_debit = total_notional.saturating_add(total_fee);

    // ANTI-TOXICITY #3 (cont): Apply ARG tax for roundtrip trades
    let arg_tax = calculate_arg_tax(slab, account_idx, instrument_idx);
    if arg_tax > 0 {
        // Debit ARG tax from taker's account
        if let Some(account) = slab.get_account_mut(account_idx) {
            account.cash = account.cash.saturating_sub(arg_tax as i128);
        }
        total_debit = total_debit.saturating_add(arg_tax);
    }

    // Mark reservation as committed
    if let Some(resv) = slab.reservations.get_mut(resv_idx) {
        resv.committed = true;
    }

    // Free slices and update reserved_qty
    free_slices(slab, slice_head)?;

    Ok(CommitResult {
        filled_qty,
        avg_price,
        total_fee: total_fee.saturating_add(arg_tax), // Include ARG tax in total fees
        total_debit,
    })
}

/// Execute all slices in a reservation
fn execute_slices(
    slab: &mut SlabState,
    slice_head: u32,
    taker_account_idx: u32,
    instrument_idx: u16,
    side: Side,
    current_ts: u64,
) -> Result<(u64, u128, u128), PercolatorError> {
    let mut curr_slice_idx = slice_head;
    let mut total_qty = 0u64;
    let mut total_notional = 0u128;
    let mut total_fee = 0u128;

    while curr_slice_idx != u32::MAX {
        let slice = slab
            .slices
            .get(curr_slice_idx)
            .ok_or(PercolatorError::InvalidReservation)?;

        let order_idx = slice.order_idx;
        let qty = slice.qty;
        let next_slice = slice.next;

        // Get order data
        let order = slab
            .orders
            .get(order_idx)
            .ok_or(PercolatorError::OrderNotFound)?;

        let maker_account_idx = order.account_idx;
        let price = order.price;
        let order_created_ms = order.created_ms;
        let order_id = order.order_id;

        // Get batch_open_ms for JIT penalty check
        let batch_open_ms = slab
            .get_instrument(instrument_idx)
            .ok_or(PercolatorError::InvalidInstrument)?
            .batch_open_ms;

        // Execute trade
        execute_trade(
            slab,
            taker_account_idx,
            maker_account_idx,
            instrument_idx,
            side,
            qty,
            price,
            order_id,
            current_ts,
        )?;

        // Calculate fees
        let notional = mul_u64(qty, price);
        let taker_fee = calculate_fee(notional, slab.header.taker_fee as i64);
        
        // ANTI-TOXICITY #2: Apply JIT penalty to maker fee
        let base_maker_fee_bps = slab.header.maker_fee;
        let adjusted_maker_fee_bps = apply_jit_penalty(
            slab,
            order_created_ms,
            batch_open_ms,
            base_maker_fee_bps,
        );
        let maker_fee = calculate_fee(notional, adjusted_maker_fee_bps);

        // ANTI-TOXICITY #3: Track aggressor activity for ARG
        update_aggressor_ledger(slab, taker_account_idx, instrument_idx, side, qty, notional)?;

        total_qty = total_qty.saturating_add(qty);
        total_notional = total_notional.saturating_add(notional);
        total_fee = total_fee.saturating_add(taker_fee);

        // Update maker's cash (subtract maker fee, can be negative for rebate)
        if let Some(maker) = slab.get_account_mut(maker_account_idx) {
            if adjusted_maker_fee_bps >= 0 {
                maker.cash = maker.cash.saturating_sub(maker_fee as i128);
            } else {
                // Negative fee = rebate (but may be zero due to JIT penalty)
                maker.cash = maker.cash.saturating_add(maker_fee as i128);
            }
        }

        // Update order quantity
        if let Some(order) = slab.orders.get_mut(order_idx) {
            order.qty = order.qty.saturating_sub(qty);

            // If fully filled, remove from book
            if order.qty == 0 {
                remove_order_from_book(slab, instrument_idx, order_idx)?;
                slab.orders.free(order_idx);
            }
        }

        curr_slice_idx = next_slice;
    }

    Ok((total_qty, total_notional, total_fee))
}

/// Execute a single trade and update positions
fn execute_trade(
    slab: &mut SlabState,
    taker_account_idx: u32,
    maker_account_idx: u32,
    instrument_idx: u16,
    side: Side,
    qty: u64,
    price: u64,
    maker_order_id: u64,
    current_ts: u64,
) -> Result<(), PercolatorError> {
    // Get cum_funding before any mutable borrows
    let cum_funding = slab
        .get_instrument(instrument_idx)
        .ok_or(PercolatorError::InvalidInstrument)?
        .cum_funding;

    // Update/create taker position
    let taker_qty = match side {
        Side::Buy => qty as i64,
        Side::Sell => -(qty as i64),
    };
    update_position(
        slab,
        taker_account_idx,
        instrument_idx,
        taker_qty,
        price,
        cum_funding,
    )?;

    // Update/create maker position (opposite side)
    let maker_qty = -taker_qty;
    update_position(
        slab,
        maker_account_idx,
        instrument_idx,
        maker_qty,
        price,
        cum_funding,
    )?;

    // Record trade
    let trade = Trade {
        ts: current_ts,
        order_id_maker: maker_order_id,
        order_id_taker: 0, // Route ID from taker
        instrument_idx,
        side,
        _padding: [0; 5],
        price,
        qty,
        hash: [0; 32],
        reveal_ms: current_ts,
    };
    slab.record_trade(trade);

    Ok(())
}

/// Update or create position with VWAP logic
fn update_position(
    slab: &mut SlabState,
    account_idx: u32,
    instrument_idx: u16,
    qty_delta: i64,
    price: u64,
    cum_funding: i128,
) -> Result<(), PercolatorError> {
    // Find existing position (immutable pass)
    let position_head = slab
        .get_account(account_idx)
        .ok_or(PercolatorError::InvalidAccount)?
        .position_head;

    let mut position_idx = position_head;
    let mut found = None;

    while position_idx != u32::MAX {
        let pos = slab
            .positions
            .get(position_idx)
            .ok_or(PercolatorError::PositionNotFound)?;

        if pos.instrument_idx == instrument_idx {
            found = Some(position_idx);
            break;
        }

        position_idx = pos.next_in_account;
    }

    if let Some(pos_idx) = found {
        // Get position data before any mutable borrows
        let (old_qty, old_entry_px) = {
            let pos = slab.positions.get(pos_idx).unwrap();
            (pos.qty, pos.entry_px)
        };

        let new_qty = old_qty + qty_delta;

        if new_qty == 0 {
            // Position closed - realize PnL
            let pnl = calculate_pnl(old_qty, old_entry_px, price);
            if let Some(account) = slab.get_account_mut(account_idx) {
                account.cash = account.cash.saturating_add(pnl);
            }

            // Remove position
            remove_position(slab, account_idx, pos_idx)?;
        } else if (old_qty > 0 && new_qty > 0) || (old_qty < 0 && new_qty < 0) {
            // Same direction - update VWAP
            let abs_old = old_qty.abs() as u64;
            let abs_delta = qty_delta.abs() as u64;
            let old_notional = mul_u64(abs_old, old_entry_px);
            let delta_notional = mul_u64(abs_delta, price);
            let new_notional = old_notional.saturating_add(delta_notional);
            let new_entry_px = calculate_vwap(new_notional, abs_old + abs_delta);

            // Now mutably update position
            if let Some(pos) = slab.positions.get_mut(pos_idx) {
                pos.entry_px = new_entry_px;
                pos.qty = new_qty;
            }
        } else {
            // Flipped - realize partial PnL
            let pnl = calculate_pnl(old_qty, old_entry_px, price);
            if let Some(account) = slab.get_account_mut(account_idx) {
                account.cash = account.cash.saturating_add(pnl);
            }

            // Set new position
            if let Some(pos) = slab.positions.get_mut(pos_idx) {
                pos.qty = new_qty;
                pos.entry_px = price;
                pos.last_funding = cum_funding;
            }
        }
    } else if qty_delta != 0 {
        // Create new position
        let pos_idx = slab
            .positions
            .alloc()
            .ok_or(PercolatorError::PoolFull)?;

        // Get position_head value before creating position
        let pos_head = slab.get_account(account_idx).unwrap().position_head;

        if let Some(pos) = slab.positions.get_mut(pos_idx) {
            *pos = Position {
                account_idx,
                instrument_idx,
                _padding: 0,
                qty: qty_delta,
                entry_px: price,
                last_funding: cum_funding,
                next_in_account: pos_head,
                index: pos_idx,
                used: true,
                _padding2: [0; 7],
            };
        }

        // Update account position head
        if let Some(account) = slab.get_account_mut(account_idx) {
            account.position_head = pos_idx;
        }
    }

    Ok(())
}

/// Remove position from account's linked list
fn remove_position(
    slab: &mut SlabState,
    account_idx: u32,
    position_idx: u32,
) -> Result<(), PercolatorError> {
    let account = slab
        .get_account(account_idx)
        .ok_or(PercolatorError::InvalidAccount)?;

    let mut curr = account.position_head;
    let mut prev = u32::MAX;

    while curr != u32::MAX {
        if curr == position_idx {
            let pos = slab
                .positions
                .get(curr)
                .ok_or(PercolatorError::PositionNotFound)?;
            let next = pos.next_in_account;

            if prev == u32::MAX {
                // Removing head
                if let Some(account) = slab.get_account_mut(account_idx) {
                    account.position_head = next;
                }
            } else if let Some(prev_pos) = slab.positions.get_mut(prev) {
                prev_pos.next_in_account = next;
            }

            slab.positions.free(position_idx);
            return Ok(());
        }

        if let Some(pos) = slab.positions.get(curr) {
            prev = curr;
            curr = pos.next_in_account;
        } else {
            break;
        }
    }

    Ok(())
}

/// Cancel a reservation and release slices
pub fn cancel(slab: &mut SlabState, hold_id: u64) -> Result<(), PercolatorError> {
    let resv_idx = find_reservation(slab, hold_id)?;

    let resv = slab
        .reservations
        .get(resv_idx)
        .ok_or(PercolatorError::ReservationNotFound)?;

    if resv.committed {
        return Err(PercolatorError::InvalidReservation);
    }

    let slice_head = resv.slice_head;

    // Free slices and unreserve quantities
    free_slices(slab, slice_head)?;

    // Free reservation
    slab.reservations.free(resv_idx);

    Ok(())
}

/// Free slices and update order reserved quantities
fn free_slices(slab: &mut SlabState, slice_head: u32) -> Result<(), PercolatorError> {
    let mut curr_idx = slice_head;

    while curr_idx != u32::MAX {
        let slice = slab
            .slices
            .get(curr_idx)
            .ok_or(PercolatorError::InvalidReservation)?;

        let order_idx = slice.order_idx;
        let qty = slice.qty;
        let next = slice.next;

        // Unreserve quantity in order
        if let Some(order) = slab.orders.get_mut(order_idx) {
            order.reserved_qty = order.reserved_qty.saturating_sub(qty);
        }

        // Free slice
        slab.slices.free(curr_idx);

        curr_idx = next;
    }

    Ok(())
}

/// Find reservation by hold_id
fn find_reservation(slab: &SlabState, hold_id: u64) -> Result<u32, PercolatorError> {
    // Linear search through reservations
    // Could be optimized with a hashmap, but keeping simple for now
    for i in 0..slab.reservations.items.len() {
        if let Some(resv) = slab.reservations.get(i as u32) {
            if resv.hold_id == hold_id {
                return Ok(i as u32);
            }
        }
    }

    Err(PercolatorError::ReservationNotFound)
}

/// Remove order from book (internal helper)
fn remove_order_from_book(
    slab: &mut SlabState,
    instrument_idx: u16,
    order_idx: u32,
) -> Result<(), PercolatorError> {
    crate::matching::book::remove_order(slab, instrument_idx, order_idx)
}

/// Calculate fee (can be negative for maker rebate)
fn calculate_fee(notional: u128, fee_bps: i64) -> u128 {
    if fee_bps >= 0 {
        (notional * (fee_bps as u128)) / 10_000
    } else {
        // Negative fee handled by caller
        (notional * ((-fee_bps) as u128)) / 10_000
    }
}

// ============================================================================
// ANTI-TOXICITY ENFORCEMENT
// ============================================================================

/// ANTI-TOXICITY CHECK #1: Kill Band
/// Reject commit if oracle price moved more than kill_band_bps since reserve
fn check_kill_band(
    slab: &SlabState,
    instrument_idx: u16,
    reserve_oracle_px: u64,
) -> Result<(), PercolatorError> {
    let kill_band_bps = slab.header.kill_band_bps;
    
    // If kill band is 0, skip check
    if kill_band_bps == 0 {
        return Ok(());
    }

    let current_oracle_px = slab
        .get_instrument(instrument_idx)
        .ok_or(PercolatorError::InvalidInstrument)?
        .index_price;

    // Calculate percentage change in basis points
    let price_change_bps = if current_oracle_px > reserve_oracle_px {
        let delta = current_oracle_px - reserve_oracle_px;
        ((delta as u128) * 10_000) / (reserve_oracle_px as u128)
    } else {
        let delta = reserve_oracle_px - current_oracle_px;
        ((delta as u128) * 10_000) / (reserve_oracle_px as u128)
    };

    // Reject if price moved too much
    if price_change_bps > (kill_band_bps as u128) {
        return Err(PercolatorError::KillBandExceeded);
    }

    Ok(())
}

/// ANTI-TOXICITY CHECK #2: JIT Penalty
/// Returns the actual maker fee to apply (zero if JIT penalty applies)
fn apply_jit_penalty(
    slab: &SlabState,
    order_created_ms: u64,
    batch_open_ms: u64,
    base_maker_fee_bps: i64,
) -> i64 {
    // If JIT penalty is off, or order was created before batch opened, use base fee
    if !slab.header.jit_penalty_on || order_created_ms < batch_open_ms {
        return base_maker_fee_bps;
    }

    // JIT order - zero out maker rebate
    // If base fee is negative (rebate), make it zero
    // If base fee is positive (maker pays), keep it
    if base_maker_fee_bps < 0 {
        0 // No rebate for JIT orders
    } else {
        base_maker_fee_bps
    }
}

/// ANTI-TOXICITY CHECK #3: Aggressor Roundtrip Guard (ARG)
/// Track aggressive activity and detect roundtrips within a batch
fn update_aggressor_ledger(
    slab: &mut SlabState,
    account_idx: u32,
    instrument_idx: u16,
    side: Side,
    qty: u64,
    notional: u128,
) -> Result<(), PercolatorError> {
    let current_epoch = slab
        .get_instrument(instrument_idx)
        .ok_or(PercolatorError::InvalidInstrument)?
        .epoch;

    // Find or create aggressor entry for this (account, instrument, epoch)
    let entry_idx = find_or_create_aggressor_entry(slab, account_idx, instrument_idx, current_epoch)?;

    if let Some(entry) = slab.aggressor_ledger.get_mut(entry_idx) {
        match side {
            Side::Buy => {
                entry.buy_qty = entry.buy_qty.saturating_add(qty);
                entry.buy_notional = entry.buy_notional.saturating_add(notional);
            }
            Side::Sell => {
                entry.sell_qty = entry.sell_qty.saturating_add(qty);
                entry.sell_notional = entry.sell_notional.saturating_add(notional);
            }
        }
    }

    Ok(())
}

/// Find existing or create new aggressor entry
fn find_or_create_aggressor_entry(
    slab: &mut SlabState,
    account_idx: u32,
    instrument_idx: u16,
    epoch: u16,
) -> Result<u32, PercolatorError> {
    // First pass: find existing entry
    for i in 0..slab.aggressor_ledger.items.len() {
        if let Some(entry) = slab.aggressor_ledger.get(i as u32) {
            if entry.account_idx == account_idx
                && entry.instrument_idx == instrument_idx
                && entry.epoch == epoch
            {
                return Ok(i as u32);
            }
        }
    }

    // Not found - allocate new entry
    let entry_idx = slab
        .aggressor_ledger
        .alloc()
        .ok_or(PercolatorError::PoolFull)?;

    if let Some(entry) = slab.aggressor_ledger.get_mut(entry_idx) {
        *entry = AggressorEntry {
            account_idx,
            instrument_idx,
            epoch,
            buy_qty: 0,
            buy_notional: 0,
            sell_qty: 0,
            sell_notional: 0,
            used: true,
            _padding: [0; 7],
        };
    }

    Ok(entry_idx)
}

/// Calculate ARG tax for roundtrip trades
/// If user bought and sold within same batch, tax the overlapping notional
fn calculate_arg_tax(
    slab: &SlabState,
    account_idx: u32,
    instrument_idx: u16,
) -> u128 {
    let current_epoch = match slab.get_instrument(instrument_idx) {
        Some(inst) => inst.epoch,
        None => return 0,
    };

    // Find aggressor entry
    for i in 0..slab.aggressor_ledger.items.len() {
        if let Some(entry) = slab.aggressor_ledger.get(i as u32) {
            if entry.account_idx == account_idx
                && entry.instrument_idx == instrument_idx
                && entry.epoch == current_epoch
            {
                // Calculate overlap - minimum of buy and sell notional
                let overlap = core::cmp::min(entry.buy_notional, entry.sell_notional);
                
                if overlap > 0 {
                    // Apply anti-sandwich fee
                    let as_fee_k = slab.header.as_fee_k;
                    return (overlap * (as_fee_k as u128)) / 10_000;
                }
            }
        }
    }

    0
}

#[cfg(test)]
mod tests {
    extern crate alloc;
    use alloc::boxed::Box;
    use super::*;
    use crate::state::*;

    /// Helper to create a minimal slab for testing
    fn create_test_slab() -> Box<SlabState> {
        // Allocate on heap using Box::new_uninit to avoid stack overflow
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
        
        // Initialize pools (already done by alloc_zeroed)
        slab.orders = Pool::new();
        slab.positions = Pool::new();
        slab.reservations = Pool::new();
        slab.slices = Pool::new();
        slab.aggressor_ledger = Pool::new();
        
        slab
    }

    #[test]
    fn test_kill_band_within_threshold() {
        let slab = create_test_slab();
        
        // Price at reserve: 50,000
        // Current price: 50,500 (1% move)
        // Kill band: 100 bps (1%)
        // Should pass
        
        let result = check_kill_band(&slab, 0, 50_000_000);
        assert!(result.is_ok());
    }

    #[test]
    fn test_kill_band_exceeded() {
        let mut slab = create_test_slab();
        
        // Set kill band to 50 bps (0.5%)
        slab.header.kill_band_bps = 50;
        
        // Update current price to 51,000 (2% move from 50,000)
        slab.instruments[0].index_price = 51_000_000;
        
        // Reserve price was 50,000
        let result = check_kill_band(&slab, 0, 50_000_000);
        assert!(matches!(result, Err(PercolatorError::KillBandExceeded)));
    }

    #[test]
    fn test_kill_band_disabled() {
        let mut slab = create_test_slab();
        
        // Disable kill band
        slab.header.kill_band_bps = 0;
        
        // Extreme price move - should still pass
        slab.instruments[0].index_price = 100_000_000; // 100% move
        
        let result = check_kill_band(&slab, 0, 50_000_000);
        assert!(result.is_ok());
    }

    #[test]
    fn test_jit_penalty_applied() {
        let mut slab = create_test_slab();
        slab.header.jit_penalty_on = true;
        
        let batch_open_ms = 1000;
        let order_created_ms = 1050; // Created after batch opened
        let base_maker_fee = -5; // Negative = rebate
        
        let adjusted = apply_jit_penalty(&slab, order_created_ms, batch_open_ms, base_maker_fee);
        
        // JIT order should get no rebate
        assert_eq!(adjusted, 0);
    }

    #[test]
    fn test_jit_penalty_not_applied_early_order() {
        let mut slab = create_test_slab();
        slab.header.jit_penalty_on = true;
        
        let batch_open_ms = 1000;
        let order_created_ms = 950; // Created before batch opened
        let base_maker_fee = -5; // Negative = rebate
        
        let adjusted = apply_jit_penalty(&slab, order_created_ms, batch_open_ms, base_maker_fee);
        
        // Early order keeps rebate
        assert_eq!(adjusted, -5);
    }

    #[test]
    fn test_jit_penalty_disabled() {
        let mut slab = create_test_slab();
        slab.header.jit_penalty_on = false;
        
        let batch_open_ms = 1000;
        let order_created_ms = 1050; // Created after batch opened
        let base_maker_fee = -5; // Negative = rebate
        
        let adjusted = apply_jit_penalty(&slab, order_created_ms, batch_open_ms, base_maker_fee);
        
        // JIT penalty off, keep rebate
        assert_eq!(adjusted, -5);
    }

    #[test]
    fn test_jit_penalty_positive_maker_fee() {
        let mut slab = create_test_slab();
        slab.header.jit_penalty_on = true;
        
        let batch_open_ms = 1000;
        let order_created_ms = 1050; // Created after batch opened
        let base_maker_fee = 10; // Positive = maker pays
        
        let adjusted = apply_jit_penalty(&slab, order_created_ms, batch_open_ms, base_maker_fee);
        
        // Positive fees are not affected
        assert_eq!(adjusted, 10);
    }

    #[test]
    fn test_aggressor_ledger_tracking() {
        let mut slab = create_test_slab();
        
        // Activate account
        slab.accounts[0].active = true;
        slab.accounts[0].index = 0;
        
        let account_idx = 0;
        let instrument_idx = 0;
        
        // First buy
        let result = update_aggressor_ledger(&mut slab, account_idx, instrument_idx, Side::Buy, 10, 500_000);
        assert!(result.is_ok());
        
        // Second buy
        let result = update_aggressor_ledger(&mut slab, account_idx, instrument_idx, Side::Buy, 5, 250_000);
        assert!(result.is_ok());
        
        // Verify ledger updated
        let entry = slab.aggressor_ledger.get(0).unwrap();
        assert_eq!(entry.buy_qty, 15);
        assert_eq!(entry.buy_notional, 750_000);
        assert_eq!(entry.sell_qty, 0);
        assert_eq!(entry.sell_notional, 0);
    }

    #[test]
    fn test_arg_tax_calculation_no_roundtrip() {
        let mut slab = create_test_slab();
        slab.header.as_fee_k = 50; // 0.5% ARG tax
        
        // Activate account
        slab.accounts[0].active = true;
        slab.accounts[0].index = 0;
        
        let account_idx = 0;
        let instrument_idx = 0;
        
        // Only buy, no sell
        update_aggressor_ledger(&mut slab, account_idx, instrument_idx, Side::Buy, 10, 500_000).unwrap();
        
        let tax = calculate_arg_tax(&slab, account_idx, instrument_idx);
        
        // No roundtrip, no tax
        assert_eq!(tax, 0);
    }

    #[test]
    fn test_arg_tax_calculation_with_roundtrip() {
        let mut slab = create_test_slab();
        slab.header.as_fee_k = 50; // 0.5% ARG tax
        
        // Activate account
        slab.accounts[0].active = true;
        slab.accounts[0].index = 0;
        
        let account_idx = 0;
        let instrument_idx = 0;
        
        // Buy then sell (roundtrip)
        update_aggressor_ledger(&mut slab, account_idx, instrument_idx, Side::Buy, 10, 500_000).unwrap();
        update_aggressor_ledger(&mut slab, account_idx, instrument_idx, Side::Sell, 8, 400_000).unwrap();
        
        let tax = calculate_arg_tax(&slab, account_idx, instrument_idx);
        
        // Overlap = min(500k, 400k) = 400k
        // Tax = 400k * 0.005 = 2,000
        assert_eq!(tax, 2_000);
    }

    #[test]
    fn test_arg_tax_full_roundtrip() {
        let mut slab = create_test_slab();
        slab.header.as_fee_k = 100; // 1% ARG tax
        
        // Activate account
        slab.accounts[0].active = true;
        slab.accounts[0].index = 0;
        
        let account_idx = 0;
        let instrument_idx = 0;
        
        // Equal buy and sell
        update_aggressor_ledger(&mut slab, account_idx, instrument_idx, Side::Buy, 10, 500_000).unwrap();
        update_aggressor_ledger(&mut slab, account_idx, instrument_idx, Side::Sell, 10, 500_000).unwrap();
        
        let tax = calculate_arg_tax(&slab, account_idx, instrument_idx);
        
        // Full overlap = 500k
        // Tax = 500k * 0.01 = 5,000
        assert_eq!(tax, 5_000);
    }

    #[test]
    fn test_batch_open_increments_epoch() {
        let mut slab = create_test_slab();
        let initial_epoch = slab.instruments[0].epoch;
        
        let result = super::super::super::instructions::batch_open::process_batch_open(&mut slab, 0, 1000);
        assert!(result.is_ok());
        
        assert_eq!(slab.instruments[0].epoch, initial_epoch.wrapping_add(1));
    }

    #[test]
    fn test_batch_open_sets_freeze_window() {
        let mut slab = create_test_slab();
        slab.header.batch_ms = 100;
        
        let current_ts = 1000;
        let result = super::super::super::instructions::batch_open::process_batch_open(&mut slab, 0, current_ts);
        assert!(result.is_ok());
        
        assert_eq!(slab.instruments[0].freeze_until_ms, 1100);
        assert_eq!(slab.instruments[0].batch_open_ms, 1000);
    }
}
