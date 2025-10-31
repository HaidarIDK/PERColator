//! Slab state - v1 orderbook implementation

use super::{BookArea, SlabHeader, QuoteCache};

/// Main slab state - v0 minimal structure (~4KB)
/// Layout: Header (256B) + QuoteCache (256B) + BookArea (3KB)
#[repr(C)]
pub struct SlabState {
    /// Header with metadata and offsets
    pub header: SlabHeader,
    /// Quote cache (router-readable)
    pub quote_cache: QuoteCache,
    /// Book area (price-time queues)
    pub book: BookArea,
}

impl SlabState {
    /// Size of the slab state
    pub const LEN: usize = core::mem::size_of::<Self>();

    /// Create new slab state
    pub fn new(header: SlabHeader) -> Self {
        Self {
            header,
            quote_cache: QuoteCache::new(),
            book: BookArea::new(),
        }
    }

    /// Refresh quote cache from current orderbook state
    ///
    /// This should be called after any operation that modifies the orderbook
    /// (PlaceOrder, CancelOrder, ModifyOrder, CommitFill)  to ensure the
    /// quote cache snapshot stays consistent.
    ///
    /// Scenario 21: Snapshot consistency - QuoteCache provides router-readable
    /// snapshot of best 4 bid/ask levels with seqno versioning
    pub fn refresh_quote_cache(&mut self) {
        use percolator_common::QuoteLevel;

        // Extract top 4 bids (already sorted descending by price)
        let mut best_bids = [QuoteLevel::default(); 4];
        for i in 0..4.min(self.book.num_bids as usize) {
            let order = &self.book.bids[i];
            best_bids[i] = QuoteLevel {
                px: order.price,
                avail_qty: order.qty,
            };
        }

        // Extract top 4 asks (already sorted ascending by price)
        let mut best_asks = [QuoteLevel::default(); 4];
        for i in 0..4.min(self.book.num_asks as usize) {
            let order = &self.book.asks[i];
            best_asks[i] = QuoteLevel {
                px: order.price,
                avail_qty: order.qty,
            };
        }

        // Update quote cache with current seqno
        self.quote_cache.update(self.header.seqno, &best_bids, &best_asks);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pinocchio::pubkey::Pubkey;

    #[test]
    fn test_slab_size() {
        use core::mem::size_of;

        // Calculate component sizes
        let header_size = size_of::<SlabHeader>();
        let quote_cache_size = size_of::<QuoteCache>();
        let book_area_size = size_of::<BookArea>();
        let total_size = size_of::<SlabState>();

        // Should be around 4KB for v0
        assert!(total_size < 5000, "SlabState is {} bytes, should be < 5KB", total_size);
        assert!(total_size > 3000, "SlabState is {} bytes, should be > 3KB", total_size);

        // Verify it matches the LEN constant
        assert_eq!(total_size, SlabState::LEN, "size_of differs from LEN constant");

        // Verify component sizes sum correctly (accounting for padding)
        let expected_min = header_size + quote_cache_size + book_area_size;
        assert!(total_size >= expected_min,
                "Total size {} should be >= sum of components {}",
                total_size, expected_min);
    }

    #[test]
    fn test_slab_creation() {
        let header = SlabHeader::new(
            Pubkey::default(),
            Pubkey::default(),
            Pubkey::default(),
            Pubkey::default(),
            50_000_000_000,
            20,
            1_000_000,
            255,
        );

        let slab = SlabState::new(header);
        assert_eq!(slab.header.seqno, 0);
        assert_eq!(slab.quote_cache.seqno_snapshot, 0);
    }
}
