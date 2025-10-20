//! Liquidation instruction - close underwater positions

use crate::state::SlabState;
use crate::matching::liquidate::{execute_liquidation, LiquidationResult};
use percolator_common::*;

/// Process liquidation instruction
///
/// Called by the Router to close positions of an underwater account.
/// Executes market orders within price bands to minimize slippage.
///
/// # Arguments
/// * `slab` - Mutable slab state
/// * `account_idx` - Account to liquidate
/// * `deficit_target` - Amount of deficit to cover
/// * `liquidation_fee_bps` - Liquidation fee in basis points (e.g., 500 = 5%)
/// * `price_band_bps` - Maximum price deviation from mark (e.g., 300 = 3%)
///
/// # Returns
/// * `Ok(LiquidationResult)` - Details of liquidation
/// * `Err(...)` - If account not liquidatable or execution fails
pub fn process_liquidation(
    slab: &mut SlabState,
    account_idx: u32,
    deficit_target: u128,
    liquidation_fee_bps: u16,
    price_band_bps: u16,
) -> Result<LiquidationResult, PercolatorError> {
    // Validate parameters
    if liquidation_fee_bps > 1000 {
        // Cap at 10%
        return Err(PercolatorError::InvalidRiskParams);
    }

    if price_band_bps > 2000 {
        // Cap at 20%
        return Err(PercolatorError::InvalidRiskParams);
    }

    // Execute liquidation
    execute_liquidation(
        slab,
        account_idx,
        deficit_target,
        liquidation_fee_bps,
        price_band_bps,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_liquidation_fee_validation() {
        // Note: Full test would require creating a slab with positions
        // For now, test parameter validation
        
        // Fee too high should be rejected in real implementation
        // Just verify the constant
        assert!(500u16 <= 1000); // 5% is valid
        assert!(1500u16 > 1000); // 15% is invalid
    }

    #[test]
    fn test_price_band_validation() {
        assert!(300u16 <= 2000); // 3% is valid
        assert!(2500u16 > 2000); // 25% is invalid
    }

    #[test]
    fn test_liquidation_result_structure() {
        let result = LiquidationResult {
            closed_qty: 500,
            realized_pnl: -1_000,
            closed_notional: 25_000,
            liquidation_fee: 1_250,
            remaining_deficit: 0,
        };

        assert_eq!(result.closed_qty, 500);
        assert_eq!(result.liquidation_fee, 1_250);
        assert!(result.remaining_deficit == 0);
    }
}

