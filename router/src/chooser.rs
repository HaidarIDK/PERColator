//! Router chooser - selects best execution path across multiple slabs
//!
//! The chooser compares quotes from orderbook slabs and AMM slabs,
//! calculates VWAP for the desired quantity, and selects the optimal
//! execution path (single slab or split across multiple slabs).

use percolator_common::QuoteCache;
use pinocchio::pubkey::Pubkey;

/// Quote from a single slab
#[derive(Debug, Clone, Copy)]
pub struct SlabQuote {
    /// Slab account pubkey
    pub slab_id: Pubkey,
    /// Best achievable VWAP for the quantity (scaled by 1e6)
    pub vwap_px: i64,
    /// Maximum quantity available at this VWAP (scaled by 1e6)
    pub max_qty: i64,
    /// Slab type indicator (0 = orderbook, 1 = AMM)
    pub slab_type: u8,
}

/// Result of chooser logic
#[derive(Debug, Clone)]
pub struct ChooseResult {
    /// Selected slab ID
    pub slab_id: Pubkey,
    /// Execution price (VWAP)
    pub vwap_px: i64,
    /// Quantity to execute
    pub qty: i64,
}

/// Calculate VWAP for buying a given quantity from a slab's QuoteCache
///
/// # Arguments
/// * `cache` - QuoteCache from the slab
/// * `qty` - Desired quantity to buy (scaled by 1e6)
///
/// # Returns
/// * VWAP in scaled units (1e6), or None if insufficient liquidity
pub fn calculate_buy_vwap(cache: &QuoteCache, qty: i64) -> Option<(i64, i64)> {
    if qty <= 0 {
        return None;
    }

    let mut remaining_qty = qty;
    let mut total_cost: i128 = 0;
    let mut filled_qty: i64 = 0;

    // Walk through ask levels (we're buying, so we take from asks)
    for level in &cache.best_asks {
        if level.px == 0 || level.avail_qty == 0 {
            break; // No more levels
        }

        let qty_at_level = remaining_qty.min(level.avail_qty);
        total_cost += qty_at_level as i128 * level.px as i128;
        filled_qty += qty_at_level;
        remaining_qty -= qty_at_level;

        if remaining_qty == 0 {
            break;
        }
    }

    if filled_qty == 0 {
        return None;
    }

    // VWAP = total_cost / filled_qty
    let vwap = (total_cost / filled_qty as i128) as i64;
    Some((vwap, filled_qty))
}

/// Calculate VWAP for selling a given quantity to a slab's QuoteCache
///
/// # Arguments
/// * `cache` - QuoteCache from the slab
/// * `qty` - Desired quantity to sell (scaled by 1e6)
///
/// # Returns
/// * VWAP in scaled units (1e6), or None if insufficient liquidity
pub fn calculate_sell_vwap(cache: &QuoteCache, qty: i64) -> Option<(i64, i64)> {
    if qty <= 0 {
        return None;
    }

    let mut remaining_qty = qty;
    let mut total_proceeds: i128 = 0;
    let mut filled_qty: i64 = 0;

    // Walk through bid levels (we're selling, so we take from bids)
    for level in &cache.best_bids {
        if level.px == 0 || level.avail_qty == 0 {
            break; // No more levels
        }

        let qty_at_level = remaining_qty.min(level.avail_qty);
        total_proceeds += qty_at_level as i128 * level.px as i128;
        filled_qty += qty_at_level;
        remaining_qty -= qty_at_level;

        if remaining_qty == 0 {
            break;
        }
    }

    if filled_qty == 0 {
        return None;
    }

    // VWAP = total_proceeds / filled_qty
    let vwap = (total_proceeds / filled_qty as i128) as i64;
    Some((vwap, filled_qty))
}

/// Choose best slab for buying
///
/// # Arguments
/// * `quotes` - Array of SlabQuote from different slabs
/// * `qty` - Desired quantity to buy
///
/// # Returns
/// * Index of best slab (lowest VWAP), or None if no slab can fill
pub fn choose_best_buy(quotes: &[SlabQuote], qty: i64) -> Option<usize> {
    let mut best_idx: Option<usize> = None;
    let mut best_vwap = i64::MAX;

    for (idx, quote) in quotes.iter().enumerate() {
        // Skip if slab can't fill the quantity
        if quote.max_qty < qty {
            continue;
        }

        // Choose slab with lowest VWAP
        if quote.vwap_px < best_vwap {
            best_vwap = quote.vwap_px;
            best_idx = Some(idx);
        }
    }

    best_idx
}

/// Choose best slab for selling
///
/// # Arguments
/// * `quotes` - Array of SlabQuote from different slabs
/// * `qty` - Desired quantity to sell
///
/// # Returns
/// * Index of best slab (highest VWAP), or None if no slab can fill
pub fn choose_best_sell(quotes: &[SlabQuote], qty: i64) -> Option<usize> {
    let mut best_idx: Option<usize> = None;
    let mut best_vwap = i64::MIN;

    for (idx, quote) in quotes.iter().enumerate() {
        // Skip if slab can't fill the quantity
        if quote.max_qty < qty {
            continue;
        }

        // Choose slab with highest VWAP
        if quote.vwap_px > best_vwap {
            best_vwap = quote.vwap_px;
            best_idx = Some(idx);
        }
    }

    best_idx
}

/// Get quotes from multiple slabs (test helper only, requires alloc)
///
/// # Arguments
/// * `caches` - Array of (slab_id, QuoteCache, slab_type) tuples
/// * `qty` - Desired quantity
/// * `is_buy` - True for buy, false for sell
///
/// # Returns
/// * Array of SlabQuote
#[cfg(not(target_os = "solana"))]
pub fn get_slab_quotes(
    caches: &[(Pubkey, QuoteCache, u8)],
    qty: i64,
    is_buy: bool,
) -> Vec<SlabQuote> {
    let mut quotes = Vec::new();

    for (slab_id, cache, slab_type) in caches {
        let result = if is_buy {
            calculate_buy_vwap(cache, qty)
        } else {
            calculate_sell_vwap(cache, qty)
        };

        if let Some((vwap, max_qty)) = result {
            quotes.push(SlabQuote {
                slab_id: *slab_id,
                vwap_px: vwap,
                max_qty,
                slab_type: *slab_type,
            });
        }
    }

    quotes
}

#[cfg(test)]
mod tests {
    use super::*;
    use percolator_common::QuoteLevel;

    fn make_quote_level(px: i64, qty: i64) -> QuoteLevel {
        QuoteLevel {
            px,
            avail_qty: qty,
        }
    }

    #[test]
    fn test_calculate_buy_vwap_single_level() {
        let mut cache = QuoteCache::new();
        cache.best_asks[0] = make_quote_level(60_000_000_000, 10_000_000);

        let (vwap, filled) = calculate_buy_vwap(&cache, 5_000_000).unwrap();
        assert_eq!(vwap, 60_000_000_000);
        assert_eq!(filled, 5_000_000);
    }

    #[test]
    fn test_calculate_buy_vwap_multiple_levels() {
        let mut cache = QuoteCache::new();
        cache.best_asks[0] = make_quote_level(60_000_000_000, 5_000_000);
        cache.best_asks[1] = make_quote_level(61_000_000_000, 5_000_000);

        // Buy 8 units: 5 @ 60k, 3 @ 61k
        // Total cost = 5*60k + 3*61k = 300k + 183k = 483k
        // VWAP = 483k / 8 = 60,375
        let (vwap, filled) = calculate_buy_vwap(&cache, 8_000_000).unwrap();
        assert_eq!(filled, 8_000_000);
        // Expected: (5*60_000 + 3*61_000) / 8 = 60_375 * 1_000_000
        assert_eq!(vwap, 60_375_000_000);
    }

    #[test]
    fn test_calculate_sell_vwap_single_level() {
        let mut cache = QuoteCache::new();
        cache.best_bids[0] = make_quote_level(59_000_000_000, 10_000_000);

        let (vwap, filled) = calculate_sell_vwap(&cache, 5_000_000).unwrap();
        assert_eq!(vwap, 59_000_000_000);
        assert_eq!(filled, 5_000_000);
    }

    #[test]
    fn test_calculate_sell_vwap_multiple_levels() {
        let mut cache = QuoteCache::new();
        cache.best_bids[0] = make_quote_level(59_000_000_000, 5_000_000);
        cache.best_bids[1] = make_quote_level(58_000_000_000, 5_000_000);

        // Sell 8 units: 5 @ 59k, 3 @ 58k
        // Total proceeds = 5*59k + 3*58k = 295k + 174k = 469k
        // VWAP = 469k / 8 = 58,625
        let (vwap, filled) = calculate_sell_vwap(&cache, 8_000_000).unwrap();
        assert_eq!(filled, 8_000_000);
        // Expected: (5*59_000 + 3*58_000) / 8 = 58_625 * 1_000_000
        assert_eq!(vwap, 58_625_000_000);
    }

    #[test]
    fn test_choose_best_buy() {
        let quotes = vec![
            SlabQuote {
                slab_id: Pubkey::from([1; 32]),
                vwap_px: 60_500_000_000,
                max_qty: 10_000_000,
                slab_type: 1, // AMM
            },
            SlabQuote {
                slab_id: Pubkey::from([2; 32]),
                vwap_px: 60_000_000_000,
                max_qty: 10_000_000,
                slab_type: 0, // OB
            },
        ];

        // Should choose orderbook (lower VWAP for buy)
        let best = choose_best_buy(&quotes, 5_000_000).unwrap();
        assert_eq!(best, 1);
    }

    #[test]
    fn test_choose_best_sell() {
        let quotes = vec![
            SlabQuote {
                slab_id: Pubkey::from([1; 32]),
                vwap_px: 59_500_000_000,
                max_qty: 10_000_000,
                slab_type: 1, // AMM
            },
            SlabQuote {
                slab_id: Pubkey::from([2; 32]),
                vwap_px: 60_000_000_000,
                max_qty: 10_000_000,
                slab_type: 0, // OB
            },
        ];

        // Should choose orderbook (higher VWAP for sell)
        let best = choose_best_sell(&quotes, 5_000_000).unwrap();
        assert_eq!(best, 1);
    }

    #[test]
    fn test_insufficient_liquidity() {
        let quotes = vec![
            SlabQuote {
                slab_id: Pubkey::from([1; 32]),
                vwap_px: 60_000_000_000,
                max_qty: 5_000_000,
                slab_type: 0,
            },
        ];

        // Try to buy more than available
        let best = choose_best_buy(&quotes, 10_000_000);
        assert!(best.is_none());
    }
}
