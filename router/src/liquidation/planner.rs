//! Reduce-only liquidation planner

use crate::instructions::SlabSplit;
use crate::liquidation::oracle::{calculate_price_band, validate_oracle_alignment};
use crate::state::{Portfolio, SlabRegistry};
use percolator_common::*;
use pinocchio::{msg, pubkey::Pubkey};

/// Maximum splits for a single liquidation (v0 limit for stack safety)
pub const MAX_LIQUIDATION_SPLITS: usize = 8;

/// Liquidation plan containing splits and expected results
#[derive(Debug, Clone)]
pub struct LiquidationPlan {
    /// Slab splits for execution
    pub splits: [SlabSplit; MAX_LIQUIDATION_SPLITS],
    /// Number of valid splits
    pub split_count: usize,
    /// Expected position reduction (sum of all split quantities)
    pub expected_reduction: i64,
    /// Price band lower bound (for sells)
    pub band_px_low: i64,
    /// Price band upper bound (for buys)
    pub band_px_high: i64,
}

impl LiquidationPlan {
    /// Create empty plan
    pub fn new() -> Self {
        Self {
            splits: [SlabSplit {
                slab_id: Pubkey::default(),
                qty: 0,
                side: 0,
                limit_px: 0,
            }; MAX_LIQUIDATION_SPLITS],
            split_count: 0,
            expected_reduction: 0,
            band_px_low: 0,
            band_px_high: 0,
        }
    }

    /// Add a split to the plan
    pub fn add_split(&mut self, split: SlabSplit) -> Result<(), PercolatorError> {
        if self.split_count >= MAX_LIQUIDATION_SPLITS {
            msg!("Error: Plan exceeds MAX_LIQUIDATION_SPLITS");
            return Err(PercolatorError::PoolFull);
        }

        self.splits[self.split_count] = split;
        self.split_count += 1;

        // Update expected reduction
        self.expected_reduction += split.qty;

        Ok(())
    }

    /// Get active splits slice
    pub fn get_splits(&self) -> &[SlabSplit] {
        &self.splits[..self.split_count]
    }
}

/// Oracle price information
#[derive(Debug, Clone, Copy)]
pub struct OraclePrice {
    /// Instrument index
    pub instrument_idx: u16,
    /// Price (1e6 scale)
    pub price: i64,
}

/// Slab information for planning
#[derive(Debug, Clone, Copy)]
pub struct SlabInfo {
    /// Slab program ID
    pub slab_id: Pubkey,
    /// Slab index in registry
    pub slab_idx: u16,
    /// Instrument index
    pub instrument_idx: u16,
    /// Mark price from slab (1e6 scale)
    pub mark_price: i64,
}

/// Plan reduce-only liquidation execution
///
/// This function analyzes the portfolio's exposures and plans how to
/// reduce them across available slabs using reduce-only orders.
///
/// # Arguments
/// * `portfolio` - User's portfolio with exposures
/// * `registry` - Slab registry with liquidation parameters
/// * `oracle_prices` - Array of oracle prices per instrument
/// * `oracle_count` - Number of valid oracle prices
/// * `slab_infos` - Array of slab information
/// * `slab_count` - Number of valid slabs
/// * `is_preliq` - Whether this is pre-liquidation (tighter band)
///
/// # Returns
/// * `LiquidationPlan` with splits ready for execution
///
/// # Algorithm
/// 1. Determine price band based on mode (pre-liq vs hard liq)
/// 2. For each exposure in portfolio:
///    - If qty > 0 (long), plan sell orders
///    - If qty < 0 (short), plan buy orders
/// 3. Filter slabs by oracle alignment
/// 4. Apply per-slab caps
/// 5. Set limit prices within band
pub fn plan_reduce_only(
    portfolio: &Portfolio,
    registry: &SlabRegistry,
    oracle_prices: &[OraclePrice],
    oracle_count: usize,
    slab_infos: &[SlabInfo],
    slab_count: usize,
    is_preliq: bool,
) -> Result<LiquidationPlan, PercolatorError> {
    msg!("Planner: Starting reduce-only planning");

    let mut plan = LiquidationPlan::new();

    // If no exposures, return empty plan
    if portfolio.exposure_count == 0 {
        msg!("Planner: No exposures to liquidate");
        return Ok(plan);
    }

    // Determine price band based on mode
    let band_bps = if is_preliq {
        registry.preliq_band_bps
    } else {
        registry.liq_band_bps
    };

    msg!("Planner: Determined price band based on mode");

    // Process each exposure in the portfolio
    for i in 0..portfolio.exposure_count as usize {
        let (exp_slab_idx, exp_instrument_idx, qty) = portfolio.exposures[i];

        if qty == 0 {
            continue; // Skip zero exposures
        }

        msg!("Planner: Processing portfolio exposure");

        // Find oracle price for this instrument
        let oracle_price = find_oracle_price(oracle_prices, oracle_count, exp_instrument_idx);
        if oracle_price == 0 {
            msg!("Planner: No oracle price available for instrument");
            continue; // Skip if no oracle price
        }

        // Calculate price band
        let (band_low, band_high) = calculate_price_band(oracle_price, band_bps);
        plan.band_px_low = band_low;
        plan.band_px_high = band_high;

        msg!("Planner: Calculated price band for liquidation");

        // Determine side and limit price
        let (side, limit_px) = if qty > 0 {
            // Long position: need to sell (reduce-only)
            // Use lower band as limit (willing to sell at discount)
            (1u8, band_low) // side=1 is sell
        } else {
            // Short position: need to buy (reduce-only)
            // Use upper band as limit (willing to buy at premium)
            (0u8, band_high) // side=0 is buy
        };

        let qty_to_reduce = qty.abs();

        // Find aligned slabs for this instrument
        for j in 0..slab_count {
            let slab_info = &slab_infos[j];

            // Match by slab index and instrument
            if slab_info.slab_idx != exp_slab_idx || slab_info.instrument_idx != exp_instrument_idx {
                continue;
            }

            // Check oracle alignment
            if !validate_oracle_alignment(
                slab_info.mark_price,
                oracle_price,
                registry.oracle_tolerance_bps,
            ) {
                msg!("Planner: Skipping misaligned slab");
                continue; // Skip misaligned slabs
            }

            // Apply per-slab cap
            let capped_qty = qty_to_reduce.min(registry.router_cap_per_slab as i64);

            msg!("Planner: Adding split to liquidation plan");

            // Add split to plan
            plan.add_split(SlabSplit {
                slab_id: slab_info.slab_id,
                qty: capped_qty,
                side,
                limit_px,
            })?;

            // For v0, we only plan one split per exposure
            // In production, we could split across multiple slabs
            break;
        }
    }

    msg!("Planner: Liquidation plan completed");

    Ok(plan)
}

/// Find oracle price for a given instrument
fn find_oracle_price(
    oracle_prices: &[OraclePrice],
    count: usize,
    instrument_idx: u16,
) -> i64 {
    for i in 0..count.min(oracle_prices.len()) {
        if oracle_prices[i].instrument_idx == instrument_idx {
            return oracle_prices[i].price;
        }
    }
    0 // Not found
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_liquidation_plan_new() {
        let plan = LiquidationPlan::new();
        assert_eq!(plan.split_count, 0);
        assert_eq!(plan.expected_reduction, 0);
    }

    #[test]
    fn test_liquidation_plan_add_split() {
        let mut plan = LiquidationPlan::new();

        let split = SlabSplit {
            slab_id: Pubkey::default(),
            qty: 100,
            side: 1,
            limit_px: 1_000_000,
        };

        plan.add_split(split).unwrap();

        assert_eq!(plan.split_count, 1);
        assert_eq!(plan.expected_reduction, 100);
        assert_eq!(plan.get_splits().len(), 1);
    }

    #[test]
    fn test_find_oracle_price_found() {
        let oracles = [
            OraclePrice { instrument_idx: 0, price: 1_000_000 },
            OraclePrice { instrument_idx: 1, price: 2_000_000 },
            OraclePrice { instrument_idx: 2, price: 0 },
        ];

        let price = find_oracle_price(&oracles, 2, 1);
        assert_eq!(price, 2_000_000);
    }

    #[test]
    fn test_find_oracle_price_not_found() {
        let oracles = [
            OraclePrice { instrument_idx: 0, price: 1_000_000 },
            OraclePrice { instrument_idx: 1, price: 2_000_000 },
            OraclePrice { instrument_idx: 2, price: 0 },
        ];

        let price = find_oracle_price(&oracles, 2, 5);
        assert_eq!(price, 0);
    }

    #[test]
    fn test_find_oracle_price_empty() {
        let oracles = [OraclePrice { instrument_idx: 0, price: 0 }];

        let price = find_oracle_price(&oracles, 0, 0);
        assert_eq!(price, 0);
    }
}
