//! Health calculation for portfolios

use anyhow::{Context, Result};
use std::collections::HashMap;

/// LP bucket type
#[derive(Debug, Clone)]
pub enum LpBucketType {
    Slab {
        reserved_base: u128,
        reserved_quote: u128,
        open_order_count: u16,
    },
    Amm {
        lp_shares: u64,
        share_price_cached: i64,
        last_update_ts: u64,
    },
}

/// LP bucket information for keeper monitoring
#[derive(Debug, Clone)]
pub struct LpBucket {
    pub venue_id: [u8; 32], // Pubkey bytes
    pub bucket_type: LpBucketType,
    pub im: u128,
    pub mm: u128,
}

/// Portfolio state (simplified mirror of on-chain state)
#[derive(Debug, Clone)]
pub struct Portfolio {
    pub equity: i128,
    pub im: u128,
    pub mm: u128,
    pub exposures: Vec<(u16, u16, i64)>, // (slab_idx, instrument_idx, qty)
    pub exposure_count: u16,
    pub lp_buckets: Vec<LpBucket>, // LP positions
}

/// Calculate health: equity - MM
///
/// Returns health value where:
/// - health < 0: Below MM (hard liquidation)
/// - 0 <= health < buffer: Pre-liquidation zone
/// - health >= buffer: Healthy
pub fn calculate_health(
    portfolio: &Portfolio,
    oracle_prices: &HashMap<u16, i64>,
) -> i128 {
    let equity = calculate_equity(portfolio, oracle_prices);
    let mm = portfolio.mm as i128;

    equity - mm
}

/// Calculate equity including unrealized PnL
///
/// Equity = base_equity + sum(position_pnl)
/// where position_pnl = qty * (current_price - entry_price) / 1e6
///
/// For v0, we simplify by using mark-to-market:
/// Equity = base_equity + sum(qty * current_price) / 1e6
pub fn calculate_equity(
    portfolio: &Portfolio,
    oracle_prices: &HashMap<u16, i64>,
) -> i128 {
    let mut equity = portfolio.equity;

    // Add unrealized PnL for each exposure
    for i in 0..portfolio.exposure_count as usize {
        if i >= portfolio.exposures.len() {
            break;
        }

        let (_slab_idx, instrument_idx, qty) = portfolio.exposures[i];

        // Get oracle price for instrument
        let price = oracle_prices.get(&instrument_idx).copied().unwrap_or(0);

        // Calculate notional value (simplified: qty * price / 1e6)
        // In production, this would account for entry price
        let notional = (qty as i128 * price as i128) / 1_000_000;

        equity += notional;
    }

    equity
}

/// Calculate maintenance margin requirement
///
/// MM = sum(abs(exposure) * price * mm_factor) / 1e6
///
/// For v0, we use the portfolio's stored MM value
pub fn calculate_mm(
    portfolio: &Portfolio,
    _oracle_prices: &HashMap<u16, i64>,
) -> u128 {
    // For v0, use pre-calculated MM from portfolio
    portfolio.mm
}

/// Determine if portfolio needs LP liquidation
///
/// Returns true if:
/// - Principal liquidation alone would be insufficient (equity still < MM)
/// - Portfolio has active LP buckets
pub fn needs_lp_liquidation(portfolio: &Portfolio) -> bool {
    // Check if portfolio has any LP positions
    if portfolio.lp_buckets.is_empty() {
        return false;
    }

    // If equity is below MM and we have LP buckets, we may need LP liquidation
    // This is a simplified check - production would estimate principal liquidation proceeds
    portfolio.equity < (portfolio.mm as i128)
}

/// Get LP liquidation priority
///
/// Returns buckets in liquidation priority order:
/// 1. Slab LP (higher priority - easier to unwind)
/// 2. AMM LP (lower priority - last resort due to staleness concerns)
pub fn get_lp_liquidation_priority(portfolio: &Portfolio) -> (Vec<&LpBucket>, Vec<&LpBucket>) {
    let mut slab_buckets = Vec::new();
    let mut amm_buckets = Vec::new();

    for bucket in &portfolio.lp_buckets {
        match bucket.bucket_type {
            LpBucketType::Slab { .. } => slab_buckets.push(bucket),
            LpBucketType::Amm { .. } => amm_buckets.push(bucket),
        }
    }

    (slab_buckets, amm_buckets)
}

/// Check if AMM LP bucket has stale price
///
/// Returns true if the AMM price update is older than max_staleness_secs
pub fn is_amm_price_stale(
    bucket: &LpBucket,
    current_timestamp: u64,
    max_staleness_secs: u64,
) -> bool {
    if let LpBucketType::Amm { last_update_ts, .. } = bucket.bucket_type {
        let age = current_timestamp.saturating_sub(last_update_ts);
        age > max_staleness_secs
    } else {
        false
    }
}

/// Parse portfolio from account data
///
/// Portfolio struct layout (repr(C)):
/// - router_id: Pubkey (32)
/// - user: Pubkey (32)
/// - equity: i128 (16)
/// - im: u128 (16)
/// - mm: u128 (16)
/// - free_collateral: i128 (16)
/// - last_mark_ts: u64 (8)
/// - exposure_count: u16 (2)
/// - bump: u8 (1)
/// - _padding: [u8; 5] (5)
/// - health: i128 (16)
/// - last_liquidation_ts: u64 (8)
/// - cooldown_seconds: u64 (8)
/// - _padding2: [u8; 8] (8)
/// - principal: i128 (16)
/// - pnl: i128 (16)
/// - vested_pnl: i128 (16)
/// - last_slot: u64 (8)
/// - pnl_index_checkpoint: i128 (16)
/// - _padding4: [u8; 8] (8)
/// - exposures: [(u16, u16, i64); 256] (256 * 10 = 2560)
/// - funding_offsets: [i128; 256] (256 * 16 = 4096)
/// - lp_buckets: [LpBucket; 32] (32 * bucket_size)
/// - lp_bucket_count: u16 (2)
/// - _padding3: [u8; 6] (6)
pub fn parse_portfolio(data: &[u8]) -> Result<Portfolio> {
    // Minimum required size: header fields before exposures array
    const MIN_SIZE: usize = 32 + 32 + 16 + 16 + 16 + 16 + 8 + 2 + 1 + 5 + 16 + 8 + 8 + 8 + 16 + 16 + 16 + 8 + 16 + 8;

    if data.len() < MIN_SIZE {
        anyhow::bail!("Portfolio account data too small: {} < {}", data.len(), MIN_SIZE);
    }

    let mut offset = 0;

    // Skip router_id (32) and user (32)
    offset += 64;

    // Read equity (i128 at offset 64)
    let equity = i128::from_le_bytes(
        data[offset..offset + 16]
            .try_into()
            .context("Failed to read equity")?
    );
    offset += 16;

    // Read im (u128 at offset 80)
    let im = u128::from_le_bytes(
        data[offset..offset + 16]
            .try_into()
            .context("Failed to read im")?
    );
    offset += 16;

    // Read mm (u128 at offset 96)
    let mm = u128::from_le_bytes(
        data[offset..offset + 16]
            .try_into()
            .context("Failed to read mm")?
    );
    offset += 16;

    // Skip free_collateral (16), last_mark_ts (8)
    offset += 24;

    // Read exposure_count (u16 at offset 136)
    let exposure_count = u16::from_le_bytes(
        data[offset..offset + 2]
            .try_into()
            .context("Failed to read exposure_count")?
    );
    offset += 2;

    // Skip bump (1), _padding (5), health (16), last_liquidation_ts (8),
    // cooldown_seconds (8), _padding2 (8), principal (16), pnl (16),
    // vested_pnl (16), last_slot (8), pnl_index_checkpoint (16), _padding4 (8)
    offset += 1 + 5 + 16 + 8 + 8 + 8 + 16 + 16 + 16 + 8 + 16 + 8;  // = 130 bytes

    // Now at offset 268 - start of exposures array
    // exposures: [(u16, u16, i64); 256] - each entry is 10 bytes
    let mut exposures = Vec::new();
    for i in 0..(exposure_count as usize).min(256) {
        let expo_offset = offset + (i * 10);
        if expo_offset + 10 > data.len() {
            break;
        }

        let slab_idx = u16::from_le_bytes([data[expo_offset], data[expo_offset + 1]]);
        let instrument_idx = u16::from_le_bytes([data[expo_offset + 2], data[expo_offset + 3]]);
        let qty = i64::from_le_bytes(
            data[expo_offset + 4..expo_offset + 12]
                .try_into()
                .context("Failed to read exposure qty")?
        );

        if qty != 0 {
            exposures.push((slab_idx, instrument_idx, qty));
        }
    }
    offset += 256 * 10;  // Skip all exposures

    // Skip funding_offsets: [i128; 256] (256 * 16 = 4096)
    offset += 256 * 16;

    // Now at lp_buckets array
    // Parse LP buckets (simplified for v0 - just get count)
    // LpBucket is large (~200+ bytes each), so for v0 we'll just get the count
    // and skip detailed parsing

    // Fast-forward to lp_bucket_count field
    // Each LpBucket is sizeof(LpBucket), typically ~256 bytes
    // We need to skip 32 buckets to get to lp_bucket_count
    const LP_BUCKET_SIZE: usize = 256;  // Approximate
    offset += 32 * LP_BUCKET_SIZE;

    let lp_bucket_count = if offset + 2 <= data.len() {
        u16::from_le_bytes([data[offset], data[offset + 1]])
    } else {
        0
    };

    // For v0, we don't parse individual LP buckets in the keeper
    // The keeper just checks if lp_bucket_count > 0 to know if LP liquidation is needed
    // TODO: Add full LP bucket parsing for detailed monitoring

    Ok(Portfolio {
        equity,
        im,
        mm,
        exposures,
        exposure_count,
        lp_buckets: Vec::new(),  // TODO: Parse actual LP buckets
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_health_below_mm() {
        let portfolio = Portfolio {
            equity: 95_000_000, // $95
            im: 110_000_000,
            mm: 100_000_000,    // $100
            exposures: vec![],
            exposure_count: 0,
            lp_buckets: vec![],
        };

        let oracle_prices = HashMap::new();
        let health = calculate_health(&portfolio, &oracle_prices);

        // Health = 95 - 100 = -5
        assert_eq!(health, -5_000_000);
    }

    #[test]
    fn test_calculate_health_in_preliq_zone() {
        let portfolio = Portfolio {
            equity: 105_000_000, // $105
            im: 110_000_000,
            mm: 100_000_000,     // $100
            exposures: vec![],
            exposure_count: 0,
            lp_buckets: vec![],
        };

        let oracle_prices = HashMap::new();
        let health = calculate_health(&portfolio, &oracle_prices);

        // Health = 105 - 100 = 5
        assert_eq!(health, 5_000_000);

        // Should be in preliq zone if buffer is $10
        let buffer = 10_000_000;
        assert!(health > 0 && health < buffer);
    }

    #[test]
    fn test_calculate_equity_with_positions() {
        let mut portfolio = Portfolio {
            equity: 100_000_000, // $100 base
            im: 110_000_000,
            mm: 100_000_000,
            exposures: vec![
                (0, 0, 10_000_000),  // Long 10 units at instrument 0
                (1, 1, -5_000_000),  // Short 5 units at instrument 1
            ],
            exposure_count: 2,
            lp_buckets: vec![],
        };

        let mut oracle_prices = HashMap::new();
        oracle_prices.insert(0, 50_000_000);  // $50 per unit
        oracle_prices.insert(1, 100_000_000); // $100 per unit

        let equity = calculate_equity(&portfolio, &oracle_prices);

        // Base equity: $100
        // Long position: 10 * $50 / 1e6 = $500
        // Short position: -5 * $100 / 1e6 = -$500
        // Total: $100 + $500 - $500 = $100
        assert_eq!(equity, 100_000_000);
    }

    #[test]
    fn test_calculate_equity_no_positions() {
        let portfolio = Portfolio {
            equity: 100_000_000,
            im: 110_000_000,
            mm: 100_000_000,
            exposures: vec![],
            exposure_count: 0,
            lp_buckets: vec![],
        };

        let oracle_prices = HashMap::new();
        let equity = calculate_equity(&portfolio, &oracle_prices);

        assert_eq!(equity, 100_000_000);
    }

    #[test]
    fn test_calculate_mm() {
        let portfolio = Portfolio {
            equity: 100_000_000,
            im: 110_000_000,
            mm: 90_000_000,
            exposures: vec![],
            exposure_count: 0,
            lp_buckets: vec![],
        };

        let oracle_prices = HashMap::new();
        let mm = calculate_mm(&portfolio, &oracle_prices);

        assert_eq!(mm, 90_000_000);
    }

    #[test]
    fn test_needs_lp_liquidation_no_buckets() {
        let portfolio = Portfolio {
            equity: 95_000_000, // Below MM
            im: 110_000_000,
            mm: 100_000_000,
            exposures: vec![],
            exposure_count: 0,
            lp_buckets: vec![], // No LP positions
        };

        assert!(!needs_lp_liquidation(&portfolio));
    }

    #[test]
    fn test_needs_lp_liquidation_with_buckets() {
        let slab_bucket = LpBucket {
            venue_id: [0u8; 32],
            bucket_type: LpBucketType::Slab {
                reserved_base: 50_000_000,
                reserved_quote: 50_000_000,
                open_order_count: 5,
            },
            im: 10_000_000,
            mm: 8_000_000,
        };

        let portfolio = Portfolio {
            equity: 95_000_000, // Below MM
            im: 110_000_000,
            mm: 100_000_000,
            exposures: vec![],
            exposure_count: 0,
            lp_buckets: vec![slab_bucket],
        };

        assert!(needs_lp_liquidation(&portfolio));
    }

    #[test]
    fn test_get_lp_liquidation_priority() {
        let slab_bucket = LpBucket {
            venue_id: [1u8; 32],
            bucket_type: LpBucketType::Slab {
                reserved_base: 50_000_000,
                reserved_quote: 50_000_000,
                open_order_count: 5,
            },
            im: 10_000_000,
            mm: 8_000_000,
        };

        let amm_bucket = LpBucket {
            venue_id: [2u8; 32],
            bucket_type: LpBucketType::Amm {
                lp_shares: 1000,
                share_price_cached: 100_000_000,
                last_update_ts: 1234567890,
            },
            im: 20_000_000,
            mm: 15_000_000,
        };

        let portfolio = Portfolio {
            equity: 95_000_000,
            im: 110_000_000,
            mm: 100_000_000,
            exposures: vec![],
            exposure_count: 0,
            lp_buckets: vec![slab_bucket, amm_bucket],
        };

        let (slab_buckets, amm_buckets) = get_lp_liquidation_priority(&portfolio);

        assert_eq!(slab_buckets.len(), 1);
        assert_eq!(amm_buckets.len(), 1);
    }

    #[test]
    fn test_is_amm_price_stale() {
        let amm_bucket = LpBucket {
            venue_id: [0u8; 32],
            bucket_type: LpBucketType::Amm {
                lp_shares: 1000,
                share_price_cached: 100_000_000,
                last_update_ts: 1000,
            },
            im: 20_000_000,
            mm: 15_000_000,
        };

        // Price is 100 seconds old, max staleness is 60 seconds
        assert!(is_amm_price_stale(&amm_bucket, 1100, 60));

        // Price is 50 seconds old, max staleness is 60 seconds
        assert!(!is_amm_price_stale(&amm_bucket, 1050, 60));
    }
}
