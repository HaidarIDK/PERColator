//! Router Reserve Instruction
//!
//! Locks collateral from a portfolio's free collateral into an LP seat's
//! reserved amounts. This is the first step in providing liquidity.

use crate::state::{Portfolio, RouterLpSeat};
use pinocchio::{
    account_info::AccountInfo,
    program_error::ProgramError,
    pubkey::Pubkey,
    ProgramResult,
};

/// Reserve collateral from portfolio into LP seat
///
/// # Arguments
/// * `portfolio_account` - Portfolio account info
/// * `portfolio` - Mutable reference to portfolio state
/// * `seat_account` - LP seat account info
/// * `seat` - Mutable reference to seat state
/// * `base_amount_q64` - Base asset amount to reserve (Q64 fixed-point)
/// * `quote_amount_q64` - Quote asset amount to reserve (Q64 fixed-point)
///
/// # Returns
/// * `Ok(())` on success
/// * `Err(ProgramError)` on validation failure or insufficient collateral
pub fn process_router_reserve(
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

    // Verify seat is not frozen
    if seat.is_frozen() {
        return Err(ProgramError::InvalidAccountData);
    }

    // Reserve collateral using FORMALLY VERIFIED logic
    // Properties LP4-LP5: Collateral conservation, no overflow/underflow
    // See: crates/model_safety/src/lp_operations.rs for Kani proofs
    crate::state::model_bridge::reserve_verified(
        portfolio,
        seat,
        base_amount_q64,
        quote_amount_q64,
    )
    .map_err(|e| match e {
        "Insufficient collateral" => ProgramError::InsufficientFunds,
        _ => ProgramError::ArithmeticOverflow,
    })?;

    Ok(())
}

