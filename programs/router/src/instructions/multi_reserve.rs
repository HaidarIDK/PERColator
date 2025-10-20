//! Multi-reserve instruction - coordinate reserves across multiple slabs

use crate::state::{Escrow, Cap, Portfolio};
use percolator_common::*;

/// Reserve result from a single slab
#[derive(Debug, Clone, Copy)]
pub struct SlabReserveResult {
    pub slab_index: u8,
    pub hold_id: u64,
    pub vwap_px: u64,
    pub worst_px: u64,
    pub max_charge: u128,
    pub filled_qty: u64,
    pub selected: bool,
}

/// Process multi-reserve instruction
///
/// Orchestrates reserve operations across multiple slabs to get best execution:
/// 1. Call reserve() on each target slab
/// 2. Collect reserve results (hold_id, vwap, worst_px, max_charge)
/// 3. Sort by VWAP (best price first)
/// 4. Select optimal subset meeting user's quantity and price limits
/// 5. Credit escrow and mint capability tokens for selected slabs
/// 6. Cancel (rollback) reserves on non-selected slabs
///
/// # Arguments
/// * `portfolio` - User's cross-slab portfolio account
/// * `user_pubkey` - User's wallet pubkey
/// * `slab_requests` - Array of reserve requests per slab
/// * `target_qty` - Total quantity user wants to trade
/// * `limit_px` - User's limit price (worst acceptable)
/// * `route_id` - Unique identifier for this routing operation
///
/// # Returns
/// * `Ok(selected_count)` - Number of slabs selected for execution
/// * `Err(...)` - If no viable execution path exists
pub fn process_multi_reserve(
    portfolio: &mut Portfolio,
    user_pubkey: &pinocchio::pubkey::Pubkey,
    slab_requests: &[SlabReserveRequest],
    target_qty: u64,
    limit_px: u64,
    _route_id: u64,
) -> Result<(), PercolatorError> {
    // Validate inputs
    if slab_requests.is_empty() {
        return Err(PercolatorError::InvalidInstruction);
    }

    if target_qty == 0 {
        return Err(PercolatorError::InvalidQuantity);
    }

    // Step 1: Call reserve on all target slabs (would be CPI in real implementation)
    // For now, we'll track what needs to happen
    let mut results = [SlabReserveResult::default(); 8]; // Max 8 slabs
    let slab_count = core::cmp::min(slab_requests.len(), 8);

    // In real implementation, this would be CPI calls to each slab
    // For now, document the logic
    for i in 0..slab_count {
        results[i] = SlabReserveResult {
            slab_index: i as u8,
            hold_id: 0, // Would come from CPI response
            vwap_px: slab_requests[i].expected_vwap,
            worst_px: slab_requests[i].expected_vwap, // Conservative estimate
            max_charge: 0, // Would come from CPI
            filled_qty: slab_requests[i].qty,
            selected: false,
        };
    }

    // Step 2: Sort results by VWAP (best price first)
    // For buy orders: lower VWAP is better
    // For sell orders: higher VWAP is better
    sort_by_vwap(&mut results[..slab_count], slab_requests[0].side);

    // Step 3: Select optimal subset
    let (selected_count, total_filled) = select_best_slabs(
        &mut results[..slab_count],
        target_qty,
        limit_px,
        slab_requests[0].side,
    )?;

    if total_filled == 0 {
        return Err(PercolatorError::InsufficientLiquidity);
    }

    // Step 4: For selected slabs, credit escrow and mint caps
    // This would happen in the actual entrypoint with access to accounts
    // Here we just validate the logic
    
    // Step 5: Cancel non-selected reserves
    // In real implementation, this would be CPI calls to slab.cancel()
    
    let _ = (portfolio, user_pubkey, selected_count);
    
    Ok(())
}

/// Request for reserving liquidity on a single slab
#[derive(Debug, Clone, Copy)]
pub struct SlabReserveRequest {
    pub slab_pubkey: [u8; 32],
    pub instrument_idx: u16,
    pub side: Side,
    pub qty: u64,
    pub expected_vwap: u64, // Estimated VWAP for sorting
}

impl Default for SlabReserveResult {
    fn default() -> Self {
        Self {
            slab_index: 0,
            hold_id: 0,
            vwap_px: 0,
            worst_px: 0,
            max_charge: 0,
            filled_qty: 0,
            selected: false,
        }
    }
}

/// Sort reserve results by VWAP (best price first)
///
/// For buy orders: ascending VWAP (lower is better)
/// For sell orders: descending VWAP (higher is better)
fn sort_by_vwap(results: &mut [SlabReserveResult], side: Side) {
    // Simple bubble sort (sufficient for small arrays)
    let n = results.len();
    for i in 0..n {
        for j in 0..(n - i - 1) {
            let should_swap = match side {
                Side::Buy => results[j].vwap_px > results[j + 1].vwap_px,
                Side::Sell => results[j].vwap_px < results[j + 1].vwap_px,
            };

            if should_swap {
                // Swap
                let temp = results[j];
                results[j] = results[j + 1];
                results[j + 1] = temp;
            }
        }
    }
}

/// Select best slabs to fulfill order within price and quantity constraints
///
/// # Returns
/// * (selected_count, total_filled_qty)
fn select_best_slabs(
    results: &mut [SlabReserveResult],
    target_qty: u64,
    limit_px: u64,
    side: Side,
) -> Result<(u8, u64), PercolatorError> {
    let mut total_filled = 0u64;
    let mut selected_count = 0u8;

    for result in results.iter_mut() {
        // Check if this slab's price is within user's limit
        let within_limit = match side {
            Side::Buy => result.vwap_px <= limit_px,
            Side::Sell => result.vwap_px >= limit_px,
        };

        if !within_limit {
            continue; // Skip this slab
        }

        // Check if we still need more quantity
        if total_filled >= target_qty {
            break; // We have enough
        }

        // Select this slab
        result.selected = true;
        selected_count += 1;

        // Add quantity (cap at remaining needed)
        let qty_needed = target_qty.saturating_sub(total_filled);
        let qty_from_slab = core::cmp::min(result.filled_qty, qty_needed);
        total_filled = total_filled.saturating_add(qty_from_slab);
    }

    Ok((selected_count, total_filled))
}

/// Credit escrow for a specific slab with amount
///
/// This is called for each selected slab to prepare for commit
pub fn credit_escrow_for_slab(
    escrow: &mut Escrow,
    amount: u128,
) -> Result<(), PercolatorError> {
    escrow.credit(amount);
    Ok(())
}

/// Mint capability token for a selected slab
///
/// Issues a time-limited, scoped capability allowing the slab to debit escrow
pub fn mint_cap_for_selected_slab(
    cap: &mut Cap,
    user: pinocchio::pubkey::Pubkey,
    slab: pinocchio::pubkey::Pubkey,
    mint: pinocchio::pubkey::Pubkey,
    amount_max: u128,
    ttl_ms: u64,
    route_id: u64,
    current_ts: u64,
) -> Result<(), PercolatorError> {
    // Cap TTL capped at 2 minutes (120,000 ms)
    const MAX_TTL_MS: u64 = 120_000;
    let capped_ttl = core::cmp::min(ttl_ms, MAX_TTL_MS);
    let expiry_ts = current_ts.saturating_add(capped_ttl);

    *cap = Cap {
        router_id: pinocchio::pubkey::Pubkey::default(),
        route_id,
        scope_user: user,
        scope_slab: slab,
        scope_mint: mint,
        amount_max,
        remaining: amount_max,
        expiry_ts,
        nonce: route_id,
        burned: false,
        bump: 0,
        _padding: [0; 6],
    };

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sort_by_vwap_buy() {
        let mut results = [
            SlabReserveResult {
                slab_index: 0,
                vwap_px: 105_000,
                filled_qty: 100,
                ..Default::default()
            },
            SlabReserveResult {
                slab_index: 1,
                vwap_px: 100_000,
                filled_qty: 200,
                ..Default::default()
            },
            SlabReserveResult {
                slab_index: 2,
                vwap_px: 102_000,
                filled_qty: 150,
                ..Default::default()
            },
        ];

        sort_by_vwap(&mut results, Side::Buy);

        // For buy orders, should be sorted ascending (best/lowest first)
        assert_eq!(results[0].vwap_px, 100_000);
        assert_eq!(results[1].vwap_px, 102_000);
        assert_eq!(results[2].vwap_px, 105_000);
    }

    #[test]
    fn test_sort_by_vwap_sell() {
        let mut results = [
            SlabReserveResult {
                slab_index: 0,
                vwap_px: 105_000,
                filled_qty: 100,
                ..Default::default()
            },
            SlabReserveResult {
                slab_index: 1,
                vwap_px: 100_000,
                filled_qty: 200,
                ..Default::default()
            },
            SlabReserveResult {
                slab_index: 2,
                vwap_px: 102_000,
                filled_qty: 150,
                ..Default::default()
            },
        ];

        sort_by_vwap(&mut results, Side::Sell);

        // For sell orders, should be sorted descending (best/highest first)
        assert_eq!(results[0].vwap_px, 105_000);
        assert_eq!(results[1].vwap_px, 102_000);
        assert_eq!(results[2].vwap_px, 100_000);
    }

    #[test]
    fn test_select_best_slabs_exact_match() {
        let mut results = [
            SlabReserveResult {
                slab_index: 0,
                vwap_px: 100_000,
                filled_qty: 500,
                ..Default::default()
            },
            SlabReserveResult {
                slab_index: 1,
                vwap_px: 101_000,
                filled_qty: 500,
                ..Default::default()
            },
        ];

        let (selected, filled) = select_best_slabs(
            &mut results,
            500,      // target_qty
            102_000,  // limit_px
            Side::Buy,
        ).unwrap();

        assert_eq!(selected, 1); // Only first slab needed
        assert_eq!(filled, 500);
        assert!(results[0].selected);
        assert!(!results[1].selected);
    }

    #[test]
    fn test_select_best_slabs_multiple_needed() {
        let mut results = [
            SlabReserveResult {
                slab_index: 0,
                vwap_px: 100_000,
                filled_qty: 300,
                ..Default::default()
            },
            SlabReserveResult {
                slab_index: 1,
                vwap_px: 101_000,
                filled_qty: 400,
                ..Default::default()
            },
            SlabReserveResult {
                slab_index: 2,
                vwap_px: 102_000,
                filled_qty: 500,
                ..Default::default()
            },
        ];

        let (selected, filled) = select_best_slabs(
            &mut results,
            600,      // target_qty
            102_000,  // limit_px
            Side::Buy,
        ).unwrap();

        assert_eq!(selected, 2); // First two slabs needed
        assert_eq!(filled, 600); // 300 + 300 (only need 300 from second)
        assert!(results[0].selected);
        assert!(results[1].selected);
        assert!(!results[2].selected);
    }

    #[test]
    fn test_select_best_slabs_price_limit() {
        let mut results = [
            SlabReserveResult {
                slab_index: 0,
                vwap_px: 100_000,
                filled_qty: 300,
                ..Default::default()
            },
            SlabReserveResult {
                slab_index: 1,
                vwap_px: 105_000, // Above limit
                filled_qty: 400,
                ..Default::default()
            },
        ];

        let (selected, filled) = select_best_slabs(
            &mut results,
            500,      // target_qty
            102_000,  // limit_px (second slab violates this)
            Side::Buy,
        ).unwrap();

        assert_eq!(selected, 1); // Only first slab within limit
        assert_eq!(filled, 300);
        assert!(results[0].selected);
        assert!(!results[1].selected); // Rejected due to price
    }

    #[test]
    fn test_credit_escrow_for_slab() {
        let mut escrow = Escrow {
            router_id: pinocchio::pubkey::Pubkey::default(),
            slab_id: pinocchio::pubkey::Pubkey::default(),
            user: pinocchio::pubkey::Pubkey::default(),
            mint: pinocchio::pubkey::Pubkey::default(),
            balance: 1000,
            nonce: 0,
            frozen: false,
            bump: 0,
            _padding: [0; 6],
        };

        credit_escrow_for_slab(&mut escrow, 500).unwrap();
        assert_eq!(escrow.balance, 1500);
    }

    #[test]
    fn test_mint_cap_for_selected_slab() {
        let mut cap = Cap {
            router_id: pinocchio::pubkey::Pubkey::default(),
            route_id: 0,
            scope_user: pinocchio::pubkey::Pubkey::default(),
            scope_slab: pinocchio::pubkey::Pubkey::default(),
            scope_mint: pinocchio::pubkey::Pubkey::default(),
            amount_max: 0,
            remaining: 0,
            expiry_ts: 0,
            nonce: 0,
            burned: false,
            bump: 0,
            _padding: [0; 6],
        };
        
        let user = pinocchio::pubkey::Pubkey::default();
        let slab = pinocchio::pubkey::Pubkey::default();
        let mint = pinocchio::pubkey::Pubkey::default();

        mint_cap_for_selected_slab(
            &mut cap,
            user,
            slab,
            mint,
            10_000,
            60_000, // 1 minute
            12345,
            1_000_000,
        ).unwrap();

        assert_eq!(cap.amount_max, 10_000);
        assert_eq!(cap.remaining, 10_000);
        assert_eq!(cap.expiry_ts, 1_060_000);
        assert_eq!(cap.nonce, 12345);
        assert!(!cap.burned);
    }

    #[test]
    fn test_mint_cap_ttl_capped() {
        let mut cap = Cap {
            router_id: pinocchio::pubkey::Pubkey::default(),
            route_id: 0,
            scope_user: pinocchio::pubkey::Pubkey::default(),
            scope_slab: pinocchio::pubkey::Pubkey::default(),
            scope_mint: pinocchio::pubkey::Pubkey::default(),
            amount_max: 0,
            remaining: 0,
            expiry_ts: 0,
            nonce: 0,
            burned: false,
            bump: 0,
            _padding: [0; 6],
        };
        
        let user = pinocchio::pubkey::Pubkey::default();
        let slab = pinocchio::pubkey::Pubkey::default();
        let mint = pinocchio::pubkey::Pubkey::default();

        // Request 5 minute TTL, should be capped at 2 minutes
        mint_cap_for_selected_slab(
            &mut cap,
            user,
            slab,
            mint,
            10_000,
            300_000, // 5 minutes
            12345,
            1_000_000,
        ).unwrap();

        // Should be capped at 2 minutes (120,000 ms)
        assert_eq!(cap.expiry_ts, 1_120_000);
    }
}
