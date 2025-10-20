//! Batch open instruction - promotes pending orders and sets freeze levels

use crate::matching::promote_pending;
use crate::state::SlabState;
use percolator_common::*;

/// Open a new batch window: increment epoch, promote pending orders, set freeze window
///
/// Opens a new batch epoch for the instrument, promoting all pending orders
/// to live status and setting the freeze window for anti-toxicity protection.
pub fn process_batch_open(
    slab: &mut SlabState,
    instrument_idx: u16,
    current_ts: u64,
) -> Result<(), PercolatorError> {
    // Get batch_ms first (immutable borrow)
    let batch_ms = slab.header.batch_ms;
    
    // Now do mutable operations
    let instrument = slab
        .get_instrument_mut(instrument_idx)
        .ok_or(PercolatorError::InvalidInstrument)?;

    // Increment epoch
    instrument.epoch = instrument.epoch.wrapping_add(1);
    let new_epoch = instrument.epoch;

    // Update batch open timestamp
    instrument.batch_open_ms = current_ts;

    // ANTI-TOXICITY #4: Set freeze window
    // Freeze contra orders for batch_ms duration to prevent JIT attacks
    instrument.freeze_until_ms = current_ts.saturating_add(batch_ms);

    // Promote pending orders that are now eligible
    promote_pending(slab, instrument_idx, new_epoch)?;

    // Clear old aggressor ledger entries from previous epochs
    // (Optional: keep last N epochs for analytics)
    clear_old_aggressor_entries(slab, new_epoch)?;

    Ok(())
}

/// Clear aggressor ledger entries from old epochs to free up pool space
fn clear_old_aggressor_entries(slab: &mut SlabState, current_epoch: u16) -> Result<(), PercolatorError> {
    // Keep entries from current epoch only (could keep last 2-3 epochs if needed)
    let min_epoch_to_keep = current_epoch;

    // Iterate and free old entries directly (no allocation needed)
    // We iterate multiple times to ensure all old entries are cleared
    // This is safe because freeing doesn't invalidate other indices
    loop {
        let mut found_any = false;
        
        for i in 0..slab.aggressor_ledger.items.len() {
            if let Some(entry) = slab.aggressor_ledger.get(i as u32) {
                if entry.epoch < min_epoch_to_keep {
                    slab.aggressor_ledger.free(i as u32);
                    found_any = true;
                }
            }
        }
        
        // If we didn't find any to free, we're done
        if !found_any {
            break;
        }
    }

    Ok(())
}
