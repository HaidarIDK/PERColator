//! Multi-commit instruction - coordinate commits across multiple slabs

use crate::state::{Cap, UserPortfolio, Escrow};
use percolator_common::*;

/// Commit result from a single slab
#[derive(Debug, Clone, Copy)]
pub struct SlabCommitResult {
    pub slab_index: u8,
    pub hold_id: u64,
    pub success: bool,
    pub fills_count: u32,
    pub total_notional: u128,
    pub position_qty: i64,  // Net position change
}

impl Default for SlabCommitResult {
    fn default() -> Self {
        Self {
            slab_index: 0,
            hold_id: 0,
            success: false,
            fills_count: 0,
            total_notional: 0,
            position_qty: 0,
        }
    }
}

/// Process multi-commit instruction
///
/// Orchestrates commit operations across multiple slabs with atomic semantics:
/// 1. Validate all capabilities are valid and not expired
/// 2. Call commit() on each reserved slab (in order)
/// 3. If ANY commit fails → rollback ALL (cancel remaining + refund escrow)
/// 4. If ALL succeed → update portfolio, burn all caps
///
/// # Arguments
/// * `portfolio` - User's cross-slab portfolio account
/// * `caps` - Array of capability tokens (one per slab)
/// * `escrows` - Array of escrow accounts (one per slab)
/// * `commit_requests` - Array of commit requests
/// * `current_ts` - Current timestamp for expiry checks
///
/// # Returns
/// * `Ok(total_fills)` - Total number of trades executed across all slabs
/// * `Err(...)` - If commits fail and rollback is triggered
pub fn process_multi_commit(
    portfolio: &mut UserPortfolio,
    caps: &mut [Cap],
    escrows: &mut [Escrow],
    commit_requests: &[SlabCommitRequest],
    current_ts: u64,
) -> Result<u32, PercolatorError> {
    // Validate inputs
    if commit_requests.is_empty() {
        return Err(PercolatorError::InvalidInstruction);
    }

    let slab_count = core::cmp::min(commit_requests.len(), 8); // Max 8 slabs

    // Step 1: Validate all capabilities before starting any commits
    for i in 0..slab_count {
        validate_cap(&caps[i], current_ts)?;
    }

    // Step 2: Execute commits on all slabs
    let mut results = [SlabCommitResult::default(); 8];
    let mut total_fills = 0u32;
    let mut all_success = true;

    for i in 0..slab_count {
        // In real implementation, this would be CPI to slab.commit()
        // For now, document the logic
        results[i] = SlabCommitResult {
            slab_index: i as u8,
            hold_id: commit_requests[i].hold_id,
            success: true, // Would come from CPI
            fills_count: 0, // Would come from CPI
            total_notional: 0, // Would come from CPI
            position_qty: 0, // Would come from CPI
        };

        if !results[i].success {
            all_success = false;
            break; // Stop on first failure
        }

        total_fills = total_fills.saturating_add(results[i].fills_count);
    }

    // Step 3: Handle result
    if all_success {
        // Success path: update portfolio and burn caps
        for i in 0..slab_count {
            // Update portfolio exposures
            update_portfolio_exposure(
                portfolio,
                commit_requests[i].slab_pubkey,
                commit_requests[i].instrument_idx,
                results[i].position_qty,
            )?;

            // Burn capability
            caps[i].burned = true;
        }

        Ok(total_fills)
    } else {
        // Failure path: rollback everything
        rollback_commits(
            caps,
            escrows,
            commit_requests,
            &results,
            slab_count,
        )?;

        Err(PercolatorError::CommitFailed)
    }
}

/// Commit request for a single slab
#[derive(Debug, Clone, Copy)]
pub struct SlabCommitRequest {
    pub slab_pubkey: [u8; 32],
    pub instrument_idx: u16,
    pub hold_id: u64,
}

/// Validate capability token before commit
fn validate_cap(cap: &Cap, current_ts: u64) -> Result<(), PercolatorError> {
    // Check if already burned
    if cap.burned {
        return Err(PercolatorError::CapabilityExpired);
    }

    // Check expiry
    if current_ts > cap.expiry_ts {
        return Err(PercolatorError::CapabilityExpired);
    }

    // Check if any remaining amount
    if cap.remaining == 0 {
        return Err(PercolatorError::InsufficientFunds);
    }

    Ok(())
}

/// Update portfolio with new position exposure
fn update_portfolio_exposure(
    portfolio: &mut UserPortfolio,
    slab_pubkey: [u8; 32],
    instrument_idx: u16,
    position_delta: i64,
) -> Result<(), PercolatorError> {
    // In real implementation, this would update portfolio.exposures
    // For now, just validate the logic

    if position_delta == 0 {
        return Ok(()); // No change
    }

    // Find existing exposure or add new one
    // This is a simplified version - real implementation would use the exposures map
    
    // Update IM/MM calculations
    recalculate_portfolio_margin(portfolio)?;

    Ok(())
}

/// Recalculate portfolio initial and maintenance margin
fn recalculate_portfolio_margin(portfolio: &mut UserPortfolio) -> Result<(), PercolatorError> {
    // In real implementation, this would:
    // 1. Iterate through all exposures
    // 2. Calculate IM/MM for each position
    // 3. Apply netting benefits for offsetting positions
    // 4. Update portfolio.im and portfolio.mm

    // For now, just ensure non-negative
    if portfolio.im > i128::MAX as u128 {
        return Err(PercolatorError::InvalidMargin);
    }

    Ok(())
}

/// Rollback all commits on partial failure
///
/// This is called when one or more commits fail. We need to:
/// 1. Cancel remaining holds (those not yet attempted)
/// 2. Refund escrow for all slabs
/// 3. Burn capabilities
fn rollback_commits(
    caps: &mut [Cap],
    escrows: &mut [Escrow],
    requests: &[SlabCommitRequest],
    results: &[SlabCommitResult],
    count: usize,
) -> Result<(), PercolatorError> {
    for i in 0..count {
        // If this slab succeeded, we can't undo it (trades executed)
        // In a real system, this would be handled by the slab having rollback capability
        // or by using a two-phase commit protocol
        
        // For now, document that:
        // - Successful commits are final (trades executed)
        // - Failed/not-attempted commits are canceled
        // - Escrow is refunded for non-executed trades
        
        if !results[i].success {
            // Refund escrow
            // In real implementation: escrow.credit(cap.remaining)
            
            // Cancel hold
            // In real implementation: CPI to slab.cancel(hold_id)
        }

        // Burn capability
        caps[i].burned = true;
    }

    Ok(())
}

/// Burn capability and refund remaining escrow
pub fn burn_cap_and_refund(
    cap: &mut Cap,
    escrow: &mut Escrow,
) -> Result<(), PercolatorError> {
    if cap.burned {
        return Ok(()); // Already burned
    }

    // Refund any remaining amount
    if cap.remaining > 0 {
        escrow.credit(cap.remaining)?;
    }

    // Mark as burned
    cap.burned = true;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_cap_success() {
        let cap = Cap {
            scope_user: pinocchio::pubkey::Pubkey::default(),
            scope_slab: pinocchio::pubkey::Pubkey::default(),
            mint: pinocchio::pubkey::Pubkey::default(),
            amount_max: 10_000,
            remaining: 10_000,
            expiry_ts: 2_000_000,
            nonce: 1,
            burned: false,
        };

        assert!(validate_cap(&cap, 1_000_000).is_ok());
    }

    #[test]
    fn test_validate_cap_expired() {
        let cap = Cap {
            scope_user: pinocchio::pubkey::Pubkey::default(),
            scope_slab: pinocchio::pubkey::Pubkey::default(),
            mint: pinocchio::pubkey::Pubkey::default(),
            amount_max: 10_000,
            remaining: 10_000,
            expiry_ts: 1_000_000,
            nonce: 1,
            burned: false,
        };

        assert!(validate_cap(&cap, 2_000_000).is_err());
    }

    #[test]
    fn test_validate_cap_burned() {
        let cap = Cap {
            scope_user: pinocchio::pubkey::Pubkey::default(),
            scope_slab: pinocchio::pubkey::Pubkey::default(),
            mint: pinocchio::pubkey::Pubkey::default(),
            amount_max: 10_000,
            remaining: 10_000,
            expiry_ts: 2_000_000,
            nonce: 1,
            burned: true,
        };

        assert!(validate_cap(&cap, 1_000_000).is_err());
    }

    #[test]
    fn test_validate_cap_no_remaining() {
        let cap = Cap {
            scope_user: pinocchio::pubkey::Pubkey::default(),
            scope_slab: pinocchio::pubkey::Pubkey::default(),
            mint: pinocchio::pubkey::Pubkey::default(),
            amount_max: 10_000,
            remaining: 0,
            expiry_ts: 2_000_000,
            nonce: 1,
            burned: false,
        };

        assert!(validate_cap(&cap, 1_000_000).is_err());
    }

    #[test]
    fn test_burn_cap_and_refund() {
        let mut cap = Cap {
            scope_user: pinocchio::pubkey::Pubkey::default(),
            scope_slab: pinocchio::pubkey::Pubkey::default(),
            mint: pinocchio::pubkey::Pubkey::default(),
            amount_max: 10_000,
            remaining: 3_000,
            expiry_ts: 2_000_000,
            nonce: 1,
            burned: false,
        };

        let mut escrow = Escrow {
            balance: 5_000,
            nonce: 0,
            frozen: false,
        };

        burn_cap_and_refund(&mut cap, &mut escrow).unwrap();

        assert!(cap.burned);
        assert_eq!(escrow.balance, 8_000); // 5000 + 3000 refunded
    }

    #[test]
    fn test_burn_cap_and_refund_idempotent() {
        let mut cap = Cap {
            scope_user: pinocchio::pubkey::Pubkey::default(),
            scope_slab: pinocchio::pubkey::Pubkey::default(),
            mint: pinocchio::pubkey::Pubkey::default(),
            amount_max: 10_000,
            remaining: 3_000,
            expiry_ts: 2_000_000,
            nonce: 1,
            burned: true, // Already burned
        };

        let mut escrow = Escrow {
            balance: 5_000,
            nonce: 0,
            frozen: false,
        };

        // Should succeed but not change anything
        burn_cap_and_refund(&mut cap, &mut escrow).unwrap();

        assert!(cap.burned);
        assert_eq!(escrow.balance, 5_000); // Unchanged
    }

    #[test]
    fn test_update_portfolio_exposure_zero_delta() {
        let mut portfolio = UserPortfolio {
            user: pinocchio::pubkey::Pubkey::default(),
            equity: 100_000,
            im: 10_000,
            mm: 5_000,
            free_collateral: 90_000,
            last_mark_ts: 0,
        };

        let result = update_portfolio_exposure(
            &mut portfolio,
            [0u8; 32],
            0,
            0, // No change
        );

        assert!(result.is_ok());
    }

    #[test]
    fn test_recalculate_portfolio_margin() {
        let mut portfolio = UserPortfolio {
            user: pinocchio::pubkey::Pubkey::default(),
            equity: 100_000,
            im: 10_000,
            mm: 5_000,
            free_collateral: 90_000,
            last_mark_ts: 0,
        };

        let result = recalculate_portfolio_margin(&mut portfolio);
        assert!(result.is_ok());
    }
}
