//! Router Release Instruction
//!
//! Releases reserved collateral from an LP seat back into a portfolio's
//! free collateral. This allows LPs to unlock unused capital.

use crate::state::{Portfolio, RouterLpSeat};
use pinocchio::{
    account_info::AccountInfo,
    program_error::ProgramError,
    pubkey::Pubkey,
    ProgramResult,
};

/// Release collateral from LP seat back to portfolio
///
/// # Arguments
/// * `portfolio_account` - Portfolio account info
/// * `portfolio` - Mutable reference to portfolio state
/// * `seat_account` - LP seat account info
/// * `seat` - Mutable reference to seat state
/// * `base_amount_q64` - Base asset amount to release (Q64 fixed-point)
/// * `quote_amount_q64` - Quote asset amount to release (Q64 fixed-point)
///
/// # Returns
/// * `Ok(())` on success
/// * `Err(ProgramError)` on validation failure or insufficient reserves
pub fn process_router_release(
    portfolio_account: &AccountInfo,
    portfolio: &mut Portfolio,
    seat_account: &AccountInfo,
    seat: &mut RouterLpSeat,
    base_amount_q64: u128,
    quote_amount_q64: u128,
) -> ProgramResult {
    // Verify portfolio owns this seat
    if seat.portfolio != *portfolio_account.key() {
        return Err(ProgramError::InvalidAccountData);
    }

    // Release collateral using FORMALLY VERIFIED logic
    // Properties LP4-LP5: Collateral conservation, no overflow/underflow
    // See: crates/model_safety/src/lp_operations.rs for Kani proofs
    crate::state::model_bridge::release_verified(
        portfolio,
        seat,
        base_amount_q64,
        quote_amount_q64,
    )
    .map_err(|e| match e {
        "Insufficient reserves" => ProgramError::InsufficientFunds,
        _ => ProgramError::ArithmeticOverflow,
    })?;

    Ok(())
}

