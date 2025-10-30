//! AMM state - constant product automated market maker

use percolator_common::{SlabHeader, QuoteCache};

/// AMM pool state - uses same header/cache layout as orderbook slab
/// Layout: SlabHeader (200B) + QuoteCache (136B) + AmmData (variable)
#[repr(C)]
pub struct AmmState {
    /// Standard slab header (magic, version, seqno, mark_px, etc.)
    pub header: SlabHeader,

    /// Router-readable quote cache (synthesized from curve)
    pub quote_cache: QuoteCache,

    /// AMM-specific pool data
    pub pool: AmmPool,
}

/// AMM pool reserves and parameters
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct AmmPool {
    /// Base reserve (x in x·y=k) - instrument contracts, scaled by SCALE
    pub x_reserve: i64,

    /// Quote reserve (y in x·y=k) - collateral/USDC, scaled by SCALE
    pub y_reserve: i64,

    /// Fee in basis points (e.g., 5 = 0.05%)
    pub fee_bps: i64,

    /// Minimum liquidity floor (prevents draining pool completely)
    pub min_liquidity: i64,

    /// Padding for future use
    pub _padding: [u64; 4],
}

impl AmmState {
    pub const LEN: usize = core::mem::size_of::<Self>();

    /// Create new AMM state
    pub fn new(header: SlabHeader, x_reserve: i64, y_reserve: i64, fee_bps: i64) -> Self {
        Self {
            header,
            quote_cache: QuoteCache::new(),
            pool: AmmPool {
                x_reserve,
                y_reserve,
                fee_bps,
                min_liquidity: 1000, // 0.001 contracts minimum
                _padding: [0; 4],
            },
        }
    }

    /// Get spot price: p = y/x (scaled)
    pub fn spot_price(&self) -> i64 {
        if self.pool.x_reserve == 0 {
            return 0;
        }
        // p = y/x * SCALE (reserves are scaled, so y/x cancels scale, multiply by SCALE to restore)
        (self.pool.y_reserve as i128 * crate::math::SCALE as i128 / self.pool.x_reserve as i128) as i64
    }

    /// Synthesize QuoteCache from AMM curve
    /// Generates 4 bid and 4 ask levels by sampling the curve at different quantities
    pub fn synthesize_quote_cache(&mut self) {
        use crate::math::{quote_buy, quote_sell};
        use percolator_common::QuoteLevel;

        let spot = self.spot_price();
        if spot == 0 {
            self.quote_cache = QuoteCache::new();
            return;
        }

        // Sample quantities: 1%, 2%, 5%, 10% of reserves (scaled)
        let sample_fractions = [10_000, 20_000, 50_000, 100_000]; // in basis points (out of 1M)

        let mut bids = [QuoteLevel::default(); 4];
        let mut asks = [QuoteLevel::default(); 4];

        // Generate ask levels (buying from AMM = selling to user)
        for (i, &fraction_bps) in sample_fractions.iter().enumerate() {
            let qty = (self.pool.x_reserve as i128 * fraction_bps as i128 / 1_000_000) as i64;
            if qty > 0 {
                if let Ok(result) = quote_buy(
                    self.pool.x_reserve,
                    self.pool.y_reserve,
                    self.pool.fee_bps,
                    qty,
                    self.pool.min_liquidity,
                ) {
                    asks[i] = QuoteLevel {
                        px: result.vwap_px,
                        avail_qty: qty,
                    };
                }
            }
        }

        // Generate bid levels (selling to AMM = buying from user)
        for (i, &fraction_bps) in sample_fractions.iter().enumerate() {
            let qty = (self.pool.x_reserve as i128 * fraction_bps as i128 / 1_000_000) as i64;
            if qty > 0 {
                if let Ok(result) = quote_sell(
                    self.pool.x_reserve,
                    self.pool.y_reserve,
                    self.pool.fee_bps,
                    qty,
                    self.pool.min_liquidity,
                ) {
                    bids[i] = QuoteLevel {
                        px: result.vwap_px,
                        avail_qty: qty,
                    };
                }
            }
        }

        // Update quote cache (bids descending by price, asks ascending by price)
        self.quote_cache.update(self.header.seqno, &bids, &asks);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pinocchio::pubkey::Pubkey;

    #[test]
    fn test_amm_state_size() {
        use core::mem::size_of;
        let size = size_of::<AmmState>();

        // Should be reasonable size: Header(200) + QuoteCache(136) + AmmPool
        assert!(size < 10000, "AmmState is {} bytes, seems too large", size);
        assert!(size > 300, "AmmState is {} bytes, seems too small", size);

        println!("AmmState size: {} bytes", size);
        println!("  SlabHeader: {} bytes", size_of::<SlabHeader>());
        println!("  QuoteCache: {} bytes", size_of::<QuoteCache>());
        println!("  AmmPool: {} bytes", size_of::<AmmPool>());
    }

    #[test]
    fn test_spot_price() {
        let header = SlabHeader::new(
            Pubkey::default(),
            Pubkey::default(),
            Pubkey::default(),
            Pubkey::default(),
            60_000_000_000,
            5,
            1_000_000,
            255,
        );

        // x=1000 contracts (scaled), y=60M quote units (scaled)
        // spot = (y/x) * SCALE = (60M/1000) * SCALE = 60k * SCALE
        let amm = AmmState::new(header, 1000 * 1_000_000, 60_000_000 * 1_000_000, 5);
        let spot = amm.spot_price();

        assert_eq!(spot, 60_000 * 1_000_000, "Spot price should be 60k scaled");
    }

    #[test]
    fn test_quote_cache_synthesis() {
        let header = SlabHeader::new(
            Pubkey::default(),
            Pubkey::default(),
            Pubkey::default(),
            Pubkey::default(),
            60_000_000_000,
            5,
            1_000_000,
            255,
        );

        let mut amm = AmmState::new(header, 1000 * 1_000_000, 60_000_000 * 1_000_000, 5);
        amm.synthesize_quote_cache();

        // Should have 4 bid and 4 ask levels
        let cache = &amm.quote_cache;

        // Check that levels are populated (non-zero)
        for i in 0..4 {
            assert!(cache.best_bids[i].px > 0, "Bid {} should have price", i);
            assert!(cache.best_bids[i].avail_qty > 0, "Bid {} should have quantity", i);
            assert!(cache.best_asks[i].px > 0, "Ask {} should have price", i);
            assert!(cache.best_asks[i].avail_qty > 0, "Ask {} should have quantity", i);
        }

        // Bids should be below spot, asks should be above spot
        let spot = amm.spot_price();
        for i in 0..4 {
            assert!(cache.best_bids[i].px < spot, "Bid price should be below spot");
            assert!(cache.best_asks[i].px > spot, "Ask price should be above spot");
        }
    }

    #[test]
    fn test_quote_cache_ordering() {
        let header = SlabHeader::new(
            Pubkey::default(),
            Pubkey::default(),
            Pubkey::default(),
            Pubkey::default(),
            60_000_000_000,
            5,
            1_000_000,
            255,
        );

        let mut amm = AmmState::new(header, 1000 * 1_000_000, 60_000_000 * 1_000_000, 5);
        amm.synthesize_quote_cache();

        let cache = &amm.quote_cache;

        // Bids should be descending (best bid first)
        for i in 0..3 {
            assert!(
                cache.best_bids[i].px >= cache.best_bids[i + 1].px,
                "Bids should be in descending order"
            );
        }

        // Asks should be ascending (best ask first)
        for i in 0..3 {
            assert!(
                cache.best_asks[i].px <= cache.best_asks[i + 1].px,
                "Asks should be in ascending order"
            );
        }
    }

    #[test]
    fn test_quote_cache_quantities_increasing() {
        let header = SlabHeader::new(
            Pubkey::default(),
            Pubkey::default(),
            Pubkey::default(),
            Pubkey::default(),
            60_000_000_000,
            5,
            1_000_000,
            255,
        );

        let mut amm = AmmState::new(header, 1000 * 1_000_000, 60_000_000 * 1_000_000, 5);
        amm.synthesize_quote_cache();

        let cache = &amm.quote_cache;

        // Quantities should increase (1%, 2%, 5%, 10% of reserves)
        for i in 0..3 {
            assert!(
                cache.best_bids[i].avail_qty < cache.best_bids[i + 1].avail_qty,
                "Bid quantities should increase"
            );
            assert!(
                cache.best_asks[i].avail_qty < cache.best_asks[i + 1].avail_qty,
                "Ask quantities should increase"
            );
        }
    }

    #[test]
    fn test_quote_cache_price_impact() {
        let header = SlabHeader::new(
            Pubkey::default(),
            Pubkey::default(),
            Pubkey::default(),
            Pubkey::default(),
            60_000_000_000,
            5,
            1_000_000,
            255,
        );

        let mut amm = AmmState::new(header, 1000 * 1_000_000, 60_000_000 * 1_000_000, 5);
        amm.synthesize_quote_cache();

        let cache = &amm.quote_cache;

        // Price impact should increase with quantity
        // For asks (buys), prices should increase
        for i in 0..3 {
            assert!(
                cache.best_asks[i].px < cache.best_asks[i + 1].px,
                "Ask prices should increase with size (higher price impact)"
            );
        }

        // For bids (sells), prices should decrease
        for i in 0..3 {
            assert!(
                cache.best_bids[i].px > cache.best_bids[i + 1].px,
                "Bid prices should decrease with size (worse execution)"
            );
        }
    }

    #[test]
    fn test_quote_cache_seqno() {
        let header = SlabHeader::new(
            Pubkey::default(),
            Pubkey::default(),
            Pubkey::default(),
            Pubkey::default(),
            60_000_000_000,
            5,
            1_000_000,
            255,
        );

        let mut amm = AmmState::new(header, 1000 * 1_000_000, 60_000_000 * 1_000_000, 5);
        amm.synthesize_quote_cache();

        // QuoteCache should capture the seqno from header
        assert_eq!(amm.quote_cache.seqno_snapshot, amm.header.seqno);
    }

    #[test]
    fn test_quote_cache_with_zero_reserves() {
        let header = SlabHeader::new(
            Pubkey::default(),
            Pubkey::default(),
            Pubkey::default(),
            Pubkey::default(),
            60_000_000_000,
            5,
            1_000_000,
            255,
        );

        let mut amm = AmmState::new(header, 0, 60_000_000 * 1_000_000, 5);
        amm.synthesize_quote_cache();

        // Should handle zero reserves gracefully (all levels should be zero)
        let cache = &amm.quote_cache;
        for i in 0..4 {
            assert_eq!(cache.best_bids[i].px, 0);
            assert_eq!(cache.best_asks[i].px, 0);
        }
    }

    #[test]
    fn test_multiple_syntheses() {
        let header = SlabHeader::new(
            Pubkey::default(),
            Pubkey::default(),
            Pubkey::default(),
            Pubkey::default(),
            60_000_000_000,
            5,
            1_000_000,
            255,
        );

        let mut amm = AmmState::new(header, 1000 * 1_000_000, 60_000_000 * 1_000_000, 5);

        // First synthesis
        amm.synthesize_quote_cache();
        let cache1 = amm.quote_cache;

        // Modify reserves
        amm.pool.x_reserve = 900 * 1_000_000;
        amm.pool.y_reserve = 66_666_666 * 1_000_000;

        // Second synthesis
        amm.synthesize_quote_cache();
        let cache2 = amm.quote_cache;

        // Prices should have changed (spot price changed from 60k to ~74k)
        assert!(cache2.best_asks[0].px > cache1.best_asks[0].px, "Prices should update");
    }

    #[test]
    fn test_spot_price_zero_x() {
        let header = SlabHeader::new(
            Pubkey::default(),
            Pubkey::default(),
            Pubkey::default(),
            Pubkey::default(),
            60_000_000_000,
            5,
            1_000_000,
            255,
        );

        let amm = AmmState::new(header, 0, 60_000_000 * 1_000_000, 5);
        let spot = amm.spot_price();

        // Should return 0 instead of panicking
        assert_eq!(spot, 0);
    }

    #[test]
    fn test_different_reserve_scales() {
        let header = SlabHeader::new(
            Pubkey::default(),
            Pubkey::default(),
            Pubkey::default(),
            Pubkey::default(),
            60_000_000_000,
            5,
            1_000_000,
            255,
        );

        // Small pool
        let small_amm = AmmState::new(header, 10 * 1_000_000, 600_000 * 1_000_000, 5);
        let small_spot = small_amm.spot_price();

        // Large pool (100x)
        let large_amm = AmmState::new(header, 1000 * 1_000_000, 60_000_000 * 1_000_000, 5);
        let large_spot = large_amm.spot_price();

        // Spot prices should be the same (y/x ratio is the same)
        assert_eq!(small_spot, large_spot, "Spot price should be scale-independent");
    }
}
