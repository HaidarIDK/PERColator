//! Router Liquidity Instruction
//!
//! Processes liquidity operations (add/remove/modify) by coordinating with
//! a matcher adapter via CPI. Updates seat exposure, LP shares, and venue PnL.

use alloc::vec::Vec;

use crate::state::{Portfolio, RouterLpSeat, VenuePnl};
use adapter_core::{LiquidityIntent, LiquidityResult, RemoveSel, RiskGuard};
use pinocchio::{
    account_info::AccountInfo,
    instruction::{AccountMeta, Instruction},
    program::invoke,
    program_error::ProgramError,
    pubkey::Pubkey,
    ProgramResult,
};

/// Process liquidity operation via matcher adapter
///
/// # Arguments
/// * `portfolio_account` - Portfolio account info
/// * `portfolio` - Mutable reference to portfolio state
/// * `seat_account` - LP seat account info
/// * `seat` - Mutable reference to seat state
/// * `venue_pnl_account` - Venue PnL account info
/// * `venue_pnl` - Mutable reference to venue PnL state
/// * `matcher_program` - Matcher adapter program account
/// * `guard` - Risk guard parameters (slippage, fees, oracle bounds)
/// * `intent` - Liquidity operation intent (add/remove/modify)
///
/// # Returns
/// * `Ok(())` on success
/// * `Err(ProgramError)` on validation failure or matcher rejection
///
/// # Note
/// This instruction will invoke the matcher adapter via CPI to execute the
/// liquidity operation and return a normalized result. For now, this is
/// a simplified version that applies deltas directly.
pub fn process_router_liquidity(
    portfolio_account: &AccountInfo,
    portfolio: &mut Portfolio,
    seat_account: &AccountInfo,
    seat: &mut RouterLpSeat,
    venue_pnl_account: &AccountInfo,
    venue_pnl: &mut VenuePnl,
    _matcher_program: &AccountInfo,
    _guard: RiskGuard,
    _intent: LiquidityIntent,
) -> ProgramResult {
    // Verify portfolio owns this seat
    if seat.portfolio != *portfolio_account.key() {
        return Err(ProgramError::InvalidAccountData);
    }

    // Verify seat is not frozen
    if seat.is_frozen() {
        return Err(ProgramError::InvalidAccountData);
    }

    // Verify venue_pnl matches the seat's matcher
    if venue_pnl.matcher_state != seat.matcher_state {
        return Err(ProgramError::InvalidAccountData);
    }

    // Build CPI instruction data for matcher adapter
    let instruction_data = build_adapter_instruction_data(&_intent, &_guard)?;

    // Build accounts for CPI
    let account_metas = vec![
        AccountMeta::writable(seat_account.key()), // matcher_state (AMM state)
        AccountMeta::readonly_signer(portfolio_account.key()), // router signer
    ];

    // Build instruction
    let instruction = Instruction {
        program_id: _matcher_program.key(),
        accounts: &account_metas,
        data: &instruction_data,
    };

    // Invoke matcher adapter via CPI
    let account_infos = &[
        seat_account,
        portfolio_account,
    ];

    invoke(&instruction, account_infos)?;

    // Read LiquidityResult from return_data
    let result = read_liquidity_result_from_return_data()?;

    // Apply LP shares delta using FORMALLY VERIFIED logic
    // Property LP1: No overflow or underflow in shares arithmetic
    // See: crates/model_safety/src/lp_operations.rs for Kani proofs
    seat.lp_shares = crate::state::model_bridge::apply_shares_delta_verified(
        seat.lp_shares,
        result.lp_shares_delta,
    )
    .map_err(|_| ProgramError::ArithmeticOverflow)?;

    // Apply exposure delta
    seat.exposure.base_q64 = seat
        .exposure
        .base_q64
        .checked_add(result.exposure_delta.base_q64)
        .ok_or(ProgramError::ArithmeticOverflow)?;

    seat.exposure.quote_q64 = seat
        .exposure
        .quote_q64
        .checked_add(result.exposure_delta.quote_q64)
        .ok_or(ProgramError::ArithmeticOverflow)?;

    // Apply venue PnL deltas using FORMALLY VERIFIED logic
    // Properties LP2-LP3: Overflow-safe PnL accounting, conserves net PnL
    // Note: venue_fees_delta is 0 for LP operations (placing/canceling orders)
    // Venue fees are charged when takers execute against LP orders, tracked via commit_fill
    crate::state::model_bridge::apply_venue_pnl_deltas_verified(
        venue_pnl,
        result.maker_fee_credits,
        0, // No venue fees on LP operations
        result.realized_pnl_delta,
    )
    .map_err(|_| ProgramError::ArithmeticOverflow)?;

    // Verify seat credit discipline (exposure within reserved limits)
    // This uses the haircut values from the seat's risk class
    // For now, using conservative 10% haircuts
    let haircut_base_bps = 1000; // 10%
    let haircut_quote_bps = 1000; // 10%

    if !seat.check_limits(haircut_base_bps, haircut_quote_bps) {
        return Err(ProgramError::Custom(0x1001)); // Seat credit limit exceeded
    }

    Ok(())
}

// Removed: apply_shares_delta() - now using formally verified version from model_bridge

/// Build instruction data for adapter CPI
///
/// Format: [discriminator=2][intent_disc][intent_data][risk_guard(8)]
fn build_adapter_instruction_data(
    intent: &LiquidityIntent,
    guard: &RiskGuard,
) -> Result<Vec<u8>, ProgramError> {
    let mut data = vec![2u8]; // Discriminator 2 for adapter_liquidity

    match intent {
        LiquidityIntent::AmmAdd {
            lower_px_q64,
            upper_px_q64,
            quote_notional_q64,
            curve_id,
            fee_bps,
        } => {
            data.push(0); // Intent discriminator for AmmAdd
            data.extend_from_slice(&lower_px_q64.to_le_bytes());
            data.extend_from_slice(&upper_px_q64.to_le_bytes());
            data.extend_from_slice(&quote_notional_q64.to_le_bytes());
            data.extend_from_slice(&curve_id.to_le_bytes());
            data.extend_from_slice(&fee_bps.to_le_bytes());
        }
        LiquidityIntent::Remove { selector } => {
            data.push(1); // Intent discriminator for Remove
            match selector {
                RemoveSel::AmmByShares { shares } => {
                    data.push(0); // Selector discriminator for AmmByShares
                    data.extend_from_slice(&shares.to_le_bytes());
                }
                RemoveSel::ObByIds { ids } => {
                    data.push(1); // Selector discriminator for ObByIds
                    data.extend_from_slice(&(ids.len() as u32).to_le_bytes());
                    for id in ids {
                        data.extend_from_slice(&(*id as u64).to_le_bytes());
                    }
                }
                RemoveSel::ObAll => {
                    data.push(2); // Selector discriminator for ObAll
                    // No additional data needed
                }
            }
        }
        LiquidityIntent::ObAdd {
            orders,
            post_only,
            reduce_only,
        } => {
            data.push(2); // Intent discriminator for ObAdd
            data.extend_from_slice(&(orders.len() as u32).to_le_bytes());

            // Serialize each order: side(1) + px_q64(16) + qty_q64(16) + tif_slots(4)
            for order in orders {
                let side_byte = match order.side {
                    adapter_core::Side::Bid => 0u8,
                    adapter_core::Side::Ask => 1u8,
                };
                data.push(side_byte);
                data.extend_from_slice(&order.px_q64.to_le_bytes());
                data.extend_from_slice(&order.qty_q64.to_le_bytes());
                data.extend_from_slice(&order.tif_slots.to_le_bytes());
            }

            data.push(*post_only as u8);
            data.push(*reduce_only as u8);
        }
        _ => {
            return Err(ProgramError::InvalidInstructionData);
        }
    }

    // Append RiskGuard (8 bytes)
    data.extend_from_slice(&guard.max_slippage_bps.to_le_bytes());
    data.extend_from_slice(&guard.max_fee_bps.to_le_bytes());
    data.extend_from_slice(&guard.oracle_bound_bps.to_le_bytes());
    data.extend_from_slice(&[0u8; 2]); // padding

    Ok(data)
}


/// Read LiquidityResult from return_data
///
/// Reads the 80-byte result from CPI return_data and deserializes it
fn read_liquidity_result_from_return_data() -> Result<adapter_core::LiquidityResult, ProgramError> {
    // Get return_data (program_id, data)
    let (_program_id, return_data) = unsafe {
        let mut buf = [0u8; 1024];
        let mut program_id_buf = [0u8; 32];
        let len = pinocchio::syscalls::sol_get_return_data(
            buf.as_mut_ptr(),
            buf.len() as u64,
            &mut program_id_buf as *mut [u8; 32],
        );

        if len == 0 {
            return Err(ProgramError::InvalidAccountData);
        }

        let program_id = pinocchio::pubkey::Pubkey::from(program_id_buf);
        (program_id, buf[..len as usize].to_vec())
    };

    // Verify return_data is 80 bytes (LiquidityResult size)
    if return_data.len() != 80 {
        return Err(ProgramError::InvalidAccountData);
    }

    // Deserialize (same format as slab serialization)
    let lp_shares_delta = i128::from_le_bytes(return_data[0..16].try_into().unwrap());
    let base_q64 = i128::from_le_bytes(return_data[16..32].try_into().unwrap());
    let quote_q64 = i128::from_le_bytes(return_data[32..48].try_into().unwrap());
    let maker_fee_credits = i128::from_le_bytes(return_data[48..64].try_into().unwrap());
    let realized_pnl_delta = i128::from_le_bytes(return_data[64..80].try_into().unwrap());

    Ok(adapter_core::LiquidityResult {
        lp_shares_delta,
        exposure_delta: adapter_core::Exposure {
            base_q64,
            quote_q64,
        },
        maker_fee_credits,
        realized_pnl_delta,
    })
}
