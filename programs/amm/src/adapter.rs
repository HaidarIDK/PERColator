//! LP Adapter instructions for AMM
//!
//! Implements the adapter-core interface to allow Router→AMM CPI for LP operations.

use adapter_core::*;
use crate::{AmmState, math};
use percolator_common::PercolatorError;
use pinocchio::{account_info::AccountInfo, msg, ProgramResult};

/// Process liquidity operation via adapter pattern
///
/// This is the CPI endpoint for the router to manage LP liquidity.
///
/// # Arguments
/// * `accounts` - [amm_account, router_signer]
/// * `intent` - The liquidity operation to perform
/// * `guard` - Risk guards for the operation
///
/// # Returns
/// * `LiquidityResult` - LP share delta, exposure delta, fee credits, PnL delta
pub fn process_adapter_liquidity(
    accounts: &[AccountInfo],
    intent: &LiquidityIntent,
    guard: &RiskGuard,
) -> Result<LiquidityResult, PercolatorError> {
    let [amm_account, router_signer] = accounts else {
        return Err(PercolatorError::InvalidAccount);
    };

    // Verify router signer
    if !router_signer.is_signer() {
        msg!("Error: Router must be signer");
        return Err(PercolatorError::Unauthorized);
    };

    // Get mutable AMM state
    let data = amm_account.try_borrow_mut_data()
        .map_err(|_| PercolatorError::InvalidAccount)?;

    if data.len() != AmmState::LEN {
        msg!("Error: AMM account has incorrect size");
        return Err(PercolatorError::InvalidAccount);
    }

    let amm = unsafe { &mut *(data.as_ptr() as *mut AmmState) };

    // Verify router authority
    if &amm.header.router_id != router_signer.key() {
        msg!("Error: Invalid router signer");
        return Err(PercolatorError::Unauthorized);
    }

    // Process the liquidity operation
    match intent {
        LiquidityIntent::AmmAdd {
            lower_px_q64,
            upper_px_q64,
            quote_notional_q64,
            curve_id: _,
            fee_bps: _,
        } => {
            process_amm_add(amm, *lower_px_q64, *upper_px_q64, *quote_notional_q64, guard)
        }
        LiquidityIntent::Remove { selector } => {
            process_remove(amm, selector, guard)
        }
        LiquidityIntent::ObAdd { .. } => {
            msg!("Error: AMM does not support orderbook orders");
            Err(PercolatorError::InvalidInstruction)
        }
        LiquidityIntent::Hook { .. } => {
            msg!("Error: AMM does not support custom hooks");
            Err(PercolatorError::InvalidInstruction)
        }
        LiquidityIntent::Modify { .. } => {
            msg!("Error: AMM does not support modify (use Remove + AmmAdd)");
            Err(PercolatorError::InvalidInstruction)
        }
    }
}

/// Add liquidity to AMM
///
/// # Simplified Implementation (v0)
/// For now, we implement a basic version that:
/// - Ignores price range (uses full curve)
/// - Mints shares proportional to quote notional vs total reserves
/// - Updates reserves by adding base + quote proportionally
fn process_amm_add(
    amm: &mut AmmState,
    _lower_px_q64: u128,
    _upper_px_q64: u128,
    quote_notional_q64: u128,
    _guard: &RiskGuard,
) -> Result<LiquidityResult, PercolatorError> {
    // Convert Q64 to i64 scale (divide by 2^64)
    let quote_notional = (quote_notional_q64 >> 64) as i64;

    if quote_notional <= 0 {
        msg!("Error: Quote notional must be positive");
        return Err(PercolatorError::InvalidQuantity);
    }

    // Calculate current spot price: p = y/x
    let spot_px = amm.spot_price();
    if spot_px == 0 {
        msg!("Error: AMM has zero spot price");
        return Err(PercolatorError::InvalidAccount);
    }

    // Calculate base amount needed for balanced liquidity add
    // base = quote / price
    let base_amount = (quote_notional as i128 * math::SCALE as i128 / spot_px as i128) as i64;

    // Calculate LP shares to mint
    // shares = sqrt(base * quote) for new liquidity
    // For simplicity: shares ≈ quote_notional (1:1 for now)
    let shares_minted = quote_notional as u128;

    // Update reserves
    amm.pool.x_reserve = amm.pool.x_reserve.saturating_add(base_amount);
    amm.pool.y_reserve = amm.pool.y_reserve.saturating_add(quote_notional);

    // Synthesize new quote cache
    amm.synthesize_quote_cache();

    // Increment seqno
    amm.header.increment_seqno();

    msg!("AMM liquidity added successfully");

    // Return result
    Ok(LiquidityResult {
        lp_shares_delta: shares_minted as i128, // Positive = mint
        exposure_delta: Exposure {
            base_q64: (base_amount as i128) << 64,
            quote_q64: (quote_notional as i128) << 64,
        },
        maker_fee_credits: 0,
        realized_pnl_delta: 0,
    })
}

/// Remove liquidity from AMM
fn process_remove(
    amm: &mut AmmState,
    selector: &RemoveSel,
    _guard: &RiskGuard,
) -> Result<LiquidityResult, PercolatorError> {
    match selector {
        RemoveSel::AmmByShares { shares } => {
            process_amm_remove_shares(amm, *shares)
        }
        RemoveSel::ObByIds { .. } | RemoveSel::ObAll => {
            msg!("Error: AMM does not support orderbook removal");
            Err(PercolatorError::InvalidInstruction)
        }
    }
}

/// Remove AMM liquidity by burning shares
fn process_amm_remove_shares(
    amm: &mut AmmState,
    shares: u128,
) -> Result<LiquidityResult, PercolatorError> {
    if shares == 0 {
        msg!("Error: Must burn at least 1 share");
        return Err(PercolatorError::InvalidQuantity);
    }

    // For simplicity, shares map 1:1 to quote notional
    // In production, would calculate: (shares / total_shares) * reserves
    let quote_returned = shares as i64;

    if quote_returned > amm.pool.y_reserve {
        msg!("Error: Insufficient liquidity to burn shares");
        return Err(PercolatorError::InsufficientLiquidity);
    }

    // Calculate base to return proportionally
    let spot_px = amm.spot_price();
    let base_returned = if spot_px > 0 {
        (quote_returned as i128 * math::SCALE as i128 / spot_px as i128) as i64
    } else {
        0
    };

    // Update reserves (subtract)
    amm.pool.x_reserve = amm.pool.x_reserve.saturating_sub(base_returned);
    amm.pool.y_reserve = amm.pool.y_reserve.saturating_sub(quote_returned);

    // Prevent draining below minimum
    if amm.pool.x_reserve < amm.pool.min_liquidity || amm.pool.y_reserve < amm.pool.min_liquidity {
        msg!("Error: Cannot drain pool below minimum liquidity");
        return Err(PercolatorError::InsufficientLiquidity);
    }

    // Synthesize new quote cache
    amm.synthesize_quote_cache();

    // Increment seqno
    amm.header.increment_seqno();

    msg!("AMM liquidity removed successfully");

    // Return result (negative deltas = burn/remove)
    Ok(LiquidityResult {
        lp_shares_delta: -(shares as i128), // Negative = burn
        exposure_delta: Exposure {
            base_q64: -(base_returned as i128) << 64,
            quote_q64: -(quote_returned as i128) << 64,
        },
        maker_fee_credits: 0,
        realized_pnl_delta: 0,
    })
}
