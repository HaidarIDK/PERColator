//! Slab state - v1 orderbook implementation

use super::{BookArea, SlabHeader, QuoteCache, Side};

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

    /// Validate price against price bands (Scenario 17: Crossing protection)
    ///
    /// Checks if order price is within acceptable range from best bid/ask.
    /// Prevents fat-finger errors and extreme price deviations.
    ///
    /// # Arguments
    /// * `side` - Order side (Buy or Sell)
    /// * `price` - Order price to validate
    ///
    /// # Returns
    /// * Ok(()) if price is within bands or bands are disabled
    /// * Err("Price outside allowed band") if price violates band
    pub fn validate_price_band(&self, side: Side, price: i64) -> Result<(), &'static str> {
        // Skip if price bands are disabled
        if self.header.price_band_bps == 0 {
            return Ok(());
        }

        use Side::*;
        match side {
            Buy => {
                // For buy orders, check against best ask
                if self.book.num_asks > 0 {
                    let best_ask = self.book.asks[0].price;
                    // Max buy price = best_ask * (1 + band_bps/10000)
                    let max_price = (best_ask as i128 * (10_000 + self.header.price_band_bps as i128) / 10_000) as i64;
                    if price > max_price {
                        return Err("Buy price exceeds price band above best ask");
                    }
                }
            }
            Sell => {
                // For sell orders, check against best bid
                if self.book.num_bids > 0 {
                    let best_bid = self.book.bids[0].price;
                    // Min sell price = best_bid * (1 - band_bps/10000)
                    let min_price = (best_bid as i128 * (10_000 - self.header.price_band_bps as i128) / 10_000) as i64;
                    if price < min_price {
                        return Err("Sell price below price band under best bid");
                    }
                }
            }
        }

        Ok(())
    }

    /// Validate price against oracle price bands (Scenario 37: Oracle bands)
    ///
    /// Checks if order price is within acceptable range from oracle/mark price.
    /// Acts as circuit breaker when market deviates too far from fair value.
    ///
    /// # Arguments
    /// * `price` - Order price to validate
    ///
    /// # Returns
    /// * Ok(()) if price is within oracle bands or bands are disabled
    /// * Err("Price outside oracle band") if price violates band
    pub fn validate_oracle_band(&self, price: i64) -> Result<(), &'static str> {
        // Skip if oracle bands are disabled
        if self.header.oracle_band_bps == 0 {
            return Ok(());
        }

        let oracle_price = self.header.mark_px;

        // Calculate allowed range: oracle_price ± (oracle_price * band_bps / 10000)
        let band_amount = (oracle_price as i128 * self.header.oracle_band_bps as i128 / 10_000) as i64;
        let min_price = oracle_price - band_amount;
        let max_price = oracle_price + band_amount;

        if price < min_price || price > max_price {
            return Err("Price outside oracle price band");
        }

        Ok(())
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
