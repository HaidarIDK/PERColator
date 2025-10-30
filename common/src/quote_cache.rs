//! Quote cache - router-readable best bid/ask levels

/// Single price level in the book
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct QuoteLevel {
    /// Price (1e6 scale, e.g., 50_000_000_000 = $50,000)
    pub px: i64,
    /// Available quantity at this level (1e6 scale)
    pub avail_qty: i64,
}

impl Default for QuoteLevel {
    fn default() -> Self {
        Self { px: 0, avail_qty: 0 }
    }
}

/// Quote cache - constantly updated summary of best levels
/// Router reads this directly without CPI
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct QuoteCache {
    /// Snapshot of header.seqno when cache was last written
    pub seqno_snapshot: u32,
    /// Padding
    pub _padding: u32,
    /// Best 4 bid levels (sorted descending by price)
    pub best_bids: [QuoteLevel; 4],
    /// Best 4 ask levels (sorted ascending by price)
    pub best_asks: [QuoteLevel; 4],
}

impl QuoteCache {
    pub const LEN: usize = core::mem::size_of::<Self>();

    /// Create empty quote cache
    pub fn new() -> Self {
        Self {
            seqno_snapshot: 0,
            _padding: 0,
            best_bids: [QuoteLevel::default(); 4],
            best_asks: [QuoteLevel::default(); 4],
        }
    }

    /// Update cache from book state
    pub fn update(&mut self, seqno: u32, bids: &[QuoteLevel], asks: &[QuoteLevel]) {
        self.seqno_snapshot = seqno;

        // Copy up to 4 best levels
        for i in 0..4 {
            if i < bids.len() {
                self.best_bids[i] = bids[i];
            } else {
                self.best_bids[i] = QuoteLevel::default();
            }

            if i < asks.len() {
                self.best_asks[i] = asks[i];
            } else {
                self.best_asks[i] = QuoteLevel::default();
            }
        }
    }

    /// Get total available quantity across all bid levels
    pub fn total_bid_qty(&self) -> i64 {
        self.best_bids.iter().map(|l| l.avail_qty).sum()
    }

    /// Get total available quantity across all ask levels
    pub fn total_ask_qty(&self) -> i64 {
        self.best_asks.iter().map(|l| l.avail_qty).sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quote_cache_creation() {
        let cache = QuoteCache::new();
        assert_eq!(cache.seqno_snapshot, 0);
        assert_eq!(cache.total_bid_qty(), 0);
        assert_eq!(cache.total_ask_qty(), 0);
    }

    #[test]
    fn test_quote_cache_update() {
        let mut cache = QuoteCache::new();

        let bids = [
            QuoteLevel { px: 50_000_000_000, avail_qty: 1_000_000 },
            QuoteLevel { px: 49_999_000_000, avail_qty: 2_000_000 },
        ];
        let asks = [
            QuoteLevel { px: 50_001_000_000, avail_qty: 1_500_000 },
        ];

        cache.update(1, &bids, &asks);

        assert_eq!(cache.seqno_snapshot, 1);
        assert_eq!(cache.best_bids[0].px, 50_000_000_000);
        assert_eq!(cache.best_bids[0].avail_qty, 1_000_000);
        assert_eq!(cache.best_asks[0].px, 50_001_000_000);
        assert_eq!(cache.total_bid_qty(), 3_000_000);
        assert_eq!(cache.total_ask_qty(), 1_500_000);
    }
}
