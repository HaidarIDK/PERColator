//! Liquidate instruction - coordinate liquidation across slabs

use crate::state::UserPortfolio;
use percolator_common::*;

/// Liquidation result from a single slab
#[derive(Debug, Clone, Copy)]
pub struct SlabLiquidationResult {
    pub slab_index: u8,
    pub closed_qty: i64,
    pub realized_pnl: i128,
    pub liquidation_fee: u128,
}

impl Default for SlabLiquidationResult {
    fn default() -> Self {
        Self {
            slab_index: 0,
            closed_qty: 0,
            realized_pnl: 0,
            liquidation_fee: 0,
        }
    }
}

/// Process liquidation instruction
///
/// Coordinates liquidation of underwater positions across multiple slabs:
/// 1. Verify account is underwater (equity < maintenance margin)
/// 2. Calculate deficit that needs to be covered
/// 3. Attempt cross-slab position offsetting (if beneficial)
/// 4. Distribute remaining deficit to slabs for forced closure
/// 5. Reward liquidator with a percentage of liquidation
/// 6. Update portfolio state
///
/// # Arguments
/// * `liquidatee_portfolio` - Portfolio of account being liquidated
/// * `liquidator_portfolio` - Portfolio of liquidator (receives reward)
/// * `max_debt` - Maximum debt liquidator is willing to cover
/// * `liquidation_fee_bps` - Liquidation fee in basis points (e.g., 500 = 5%)
///
/// # Returns
/// * `Ok(total_closed_notional)` - Total notional value of positions closed
/// * `Err(NotLiquidatable)` - If account is not underwater
/// * `Err(...)` - Other errors
pub fn process_liquidate(
    liquidatee_portfolio: &mut UserPortfolio,
    liquidator_portfolio: &mut UserPortfolio,
    max_debt: u128,
    liquidation_fee_bps: u16,
) -> Result<u128, PercolatorError> {
    // Step 1: Verify account is liquidatable
    if !is_liquidatable(liquidatee_portfolio) {
        return Err(PercolatorError::NotLiquidatable);
    }

    // Step 2: Calculate deficit
    let deficit = calculate_deficit(liquidatee_portfolio);
    
    if deficit == 0 {
        return Err(PercolatorError::NotLiquidatable);
    }

    // Step 3: Cap deficit at liquidator's max debt willing to cover
    let target_deficit = core::cmp::min(deficit, max_debt);

    // Step 4: Attempt cross-slab offsetting (Phase 2 feature)
    // If liquidatee has long BTC on Slab A and short BTC on Slab B,
    // we can offset these positions before forced liquidation
    // This is more capital efficient than liquidating both separately
    
    // For now, skip to forced liquidation

    // Step 5: Distribute deficit to slabs for forced closure
    let mut results = [SlabLiquidationResult::default(); 8];
    let slab_count = 0; // Would come from portfolio exposures
    
    let total_closed = execute_forced_liquidation(
        liquidatee_portfolio,
        &mut results,
        slab_count,
        target_deficit,
    )?;

    // Step 6: Calculate and transfer liquidation reward
    let liquidation_reward = calculate_liquidation_reward(
        total_closed,
        liquidation_fee_bps,
    );

    // Transfer reward from liquidatee to liquidator
    liquidatee_portfolio.equity = liquidatee_portfolio.equity
        .saturating_sub(liquidation_reward as i128);
    liquidator_portfolio.equity = liquidator_portfolio.equity
        .saturating_add(liquidation_reward as i128);

    // Step 7: Update portfolio margin requirements
    recalculate_margin(liquidatee_portfolio)?;

    Ok(total_closed)
}

/// Check if account is eligible for liquidation
///
/// Account is liquidatable if: equity < maintenance margin
fn is_liquidatable(portfolio: &UserPortfolio) -> bool {
    portfolio.equity < portfolio.mm as i128
}

/// Calculate deficit that needs to be covered
///
/// Deficit = maintenance_margin - equity
fn calculate_deficit(portfolio: &UserPortfolio) -> u128 {
    if portfolio.equity >= portfolio.mm as i128 {
        return 0;
    }

    let deficit_i128 = (portfolio.mm as i128).saturating_sub(portfolio.equity);
    
    // Convert to u128, clamping at 0 for safety
    if deficit_i128 < 0 {
        0
    } else {
        deficit_i128 as u128
    }
}

/// Execute forced liquidation across slabs
///
/// This calls each slab to close positions until deficit is covered
fn execute_forced_liquidation(
    portfolio: &mut UserPortfolio,
    results: &mut [SlabLiquidationResult],
    slab_count: usize,
    target_deficit: u128,
) -> Result<u128, PercolatorError> {
    let mut covered_so_far = 0u128;

    for i in 0..slab_count {
        if covered_so_far >= target_deficit {
            break; // Deficit covered
        }

        // In real implementation, this would be CPI to slab.liquidation_call()
        // For now, document the logic
        
        results[i] = SlabLiquidationResult {
            slab_index: i as u8,
            closed_qty: 0, // Would come from CPI
            realized_pnl: 0, // Would come from CPI
            liquidation_fee: 0, // Would come from CPI
        };

        // Update covered amount (would use actual results from CPI)
        // covered_so_far += results[i].liquidation_fee as u128;
    }

    Ok(covered_so_far)
}

/// Calculate liquidation reward for liquidator
///
/// Reward = total_closed_notional * (liquidation_fee_bps / 10000)
fn calculate_liquidation_reward(
    total_closed_notional: u128,
    liquidation_fee_bps: u16,
) -> u128 {
    (total_closed_notional * liquidation_fee_bps as u128) / 10_000
}

/// Recalculate margin requirements after liquidation
fn recalculate_margin(portfolio: &mut UserPortfolio) -> Result<(), PercolatorError> {
    // In real implementation, this would:
    // 1. Iterate through remaining exposures
    // 2. Calculate new IM/MM based on reduced positions
    // 3. Update portfolio.im and portfolio.mm
    
    // For now, ensure non-negative
    if portfolio.im > i128::MAX as u128 {
        return Err(PercolatorError::InvalidMargin);
    }

    // Calculate free collateral
    portfolio.free_collateral = portfolio.equity.saturating_sub(portfolio.im as i128);

    Ok(())
}

/// Attempt cross-slab position offsetting
///
/// If user has offsetting positions (e.g., long on Slab A, short on Slab B),
/// we can net these positions before forced liquidation for better capital efficiency
///
/// Returns: (offset_achieved, remaining_deficit)
pub fn attempt_cross_slab_offset(
    portfolio: &UserPortfolio,
    deficit: u128,
    grace_window_ms: u64,
    current_ts: u64,
) -> Result<(u128, u128), PercolatorError> {
    // Check if still within grace window
    let liquidation_start_ts = portfolio.last_mark_ts;
    if current_ts > liquidation_start_ts.saturating_add(grace_window_ms) {
        // Grace window expired, proceed to forced liquidation
        return Ok((0, deficit));
    }

    // In real implementation, this would:
    // 1. Identify offsetting positions across slabs
    // 2. Calculate potential savings from netting
    // 3. Execute offset trades if beneficial
    // 4. Return amount offset and remaining deficit

    // For now, return no offset achieved
    Ok((0, deficit))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_liquidatable_underwater() {
        let portfolio = UserPortfolio {
            user: pinocchio::pubkey::Pubkey::default(),
            equity: 8_000,  // Below MM
            im: 15_000,
            mm: 10_000,
            free_collateral: -2_000,
            last_mark_ts: 0,
        };

        assert!(is_liquidatable(&portfolio));
    }

    #[test]
    fn test_is_liquidatable_healthy() {
        let portfolio = UserPortfolio {
            user: pinocchio::pubkey::Pubkey::default(),
            equity: 12_000,  // Above MM
            im: 15_000,
            mm: 10_000,
            free_collateral: 2_000,
            last_mark_ts: 0,
        };

        assert!(!is_liquidatable(&portfolio));
    }

    #[test]
    fn test_calculate_deficit() {
        let portfolio = UserPortfolio {
            user: pinocchio::pubkey::Pubkey::default(),
            equity: 8_000,  // MM is 10_000, deficit = 2_000
            im: 15_000,
            mm: 10_000,
            free_collateral: -2_000,
            last_mark_ts: 0,
        };

        assert_eq!(calculate_deficit(&portfolio), 2_000);
    }

    #[test]
    fn test_calculate_deficit_healthy() {
        let portfolio = UserPortfolio {
            user: pinocchio::pubkey::Pubkey::default(),
            equity: 12_000,
            im: 15_000,
            mm: 10_000,
            free_collateral: 2_000,
            last_mark_ts: 0,
        };

        assert_eq!(calculate_deficit(&portfolio), 0);
    }

    #[test]
    fn test_calculate_liquidation_reward() {
        // Close $10,000 notional with 5% fee
        let reward = calculate_liquidation_reward(10_000, 500);
        assert_eq!(reward, 500); // 5% of 10,000
    }

    #[test]
    fn test_calculate_liquidation_reward_zero() {
        let reward = calculate_liquidation_reward(0, 500);
        assert_eq!(reward, 0);
    }

    #[test]
    fn test_process_liquidate_not_liquidatable() {
        let mut liquidatee = UserPortfolio {
            user: pinocchio::pubkey::Pubkey::default(),
            equity: 12_000,  // Healthy
            im: 15_000,
            mm: 10_000,
            free_collateral: 2_000,
            last_mark_ts: 0,
        };

        let mut liquidator = UserPortfolio {
            user: pinocchio::pubkey::Pubkey::default(),
            equity: 50_000,
            im: 0,
            mm: 0,
            free_collateral: 50_000,
            last_mark_ts: 0,
        };

        let result = process_liquidate(
            &mut liquidatee,
            &mut liquidator,
            10_000,
            500,
        );

        assert!(result.is_err());
    }

    #[test]
    fn test_attempt_cross_slab_offset_grace_window_active() {
        let portfolio = UserPortfolio {
            user: pinocchio::pubkey::Pubkey::default(),
            equity: 8_000,
            im: 15_000,
            mm: 10_000,
            free_collateral: -2_000,
            last_mark_ts: 1_000_000,
        };

        let result = attempt_cross_slab_offset(
            &portfolio,
            2_000,
            60_000,  // 1 minute grace
            1_030_000,  // 30 seconds later
        );

        assert!(result.is_ok());
        let (offset, remaining) = result.unwrap();
        assert_eq!(offset, 0);  // No offset achieved (not implemented)
        assert_eq!(remaining, 2_000);
    }

    #[test]
    fn test_attempt_cross_slab_offset_grace_window_expired() {
        let portfolio = UserPortfolio {
            user: pinocchio::pubkey::Pubkey::default(),
            equity: 8_000,
            im: 15_000,
            mm: 10_000,
            free_collateral: -2_000,
            last_mark_ts: 1_000_000,
        };

        let result = attempt_cross_slab_offset(
            &portfolio,
            2_000,
            60_000,  // 1 minute grace
            1_070_000,  // 70 seconds later (expired)
        );

        assert!(result.is_ok());
        let (offset, remaining) = result.unwrap();
        assert_eq!(offset, 0);
        assert_eq!(remaining, 2_000);
    }

    #[test]
    fn test_recalculate_margin() {
        let mut portfolio = UserPortfolio {
            user: pinocchio::pubkey::Pubkey::default(),
            equity: 12_000,
            im: 15_000,
            mm: 10_000,
            free_collateral: 0, // Will be recalculated
            last_mark_ts: 0,
        };

        recalculate_margin(&mut portfolio).unwrap();
        
        // Free collateral should be equity - im = 12000 - 15000 = -3000
        assert_eq!(portfolio.free_collateral, -3_000);
    }
}
