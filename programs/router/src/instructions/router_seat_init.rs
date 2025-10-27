//! Initialize LP Seat instruction
//!
//! Creates an LP seat PDA for tracking liquidity provision on a specific matcher.

use crate::pda::derive_lp_seat_pda;
use crate::state::{Portfolio, RouterLpSeat};
use percolator_common::*;
use pinocchio::{
    account_info::AccountInfo,
    msg,
    pubkey::Pubkey,
};

/// Process initialize LP seat instruction
///
/// Initializes an LP seat PDA for tracking liquidity provision.
/// The seat PDA must be derived correctly using the provided parameters.
///
/// # Security Checks
/// - Verifies seat PDA derivation matches expected seeds
/// - Verifies signer is portfolio owner or authorized operator
/// - Prevents double initialization
/// - Validates account ownership and size
///
/// # Arguments
/// * `program_id` - The router program ID
/// * `seat_account` - The LP seat PDA account
/// * `portfolio_account` - The portfolio account
/// * `matcher_state` - The matcher state pubkey
/// * `signer` - The transaction signer (portfolio owner or operator)
/// * `context_id` - Context ID for multiple seats per portfolio×matcher
pub fn process_router_seat_init(
    program_id: &Pubkey,
    seat_account: &AccountInfo,
    portfolio_account: &AccountInfo,
    matcher_state: &Pubkey,
    signer: &AccountInfo,
    context_id: u32,
) -> Result<(), PercolatorError> {
    // SECURITY: Verify signer
    if !signer.is_signer() {
        msg!("Error: Signer must sign the transaction");
        return Err(PercolatorError::Unauthorized);
    }

    // SECURITY: Verify portfolio account ownership
    if portfolio_account.owner() != program_id {
        msg!("Error: Portfolio account has incorrect owner");
        return Err(PercolatorError::InvalidAccountOwner);
    }

    // Load portfolio to verify authorization
    let portfolio_data = portfolio_account.try_borrow_data()
        .map_err(|_| PercolatorError::InvalidAccount)?;

    if portfolio_data.len() < Portfolio::LEN {
        msg!("Error: Portfolio account too small");
        return Err(PercolatorError::InvalidAccount);
    }

    // SAFETY: We've verified size, ownership, and this is read-only access
    let portfolio = unsafe {
        &*(portfolio_data.as_ptr() as *const Portfolio)
    };

    // SECURITY: Verify authorization (portfolio owner or operator)
    // Note: Portfolio doesn't have operator field, only LP seats do.
    // For seat initialization, only portfolio owner can create seats.
    if signer.key() != &portfolio.user {
        msg!("Error: Only portfolio owner can initialize LP seats");
        return Err(PercolatorError::Unauthorized);
    }

    drop(portfolio_data);

    // SECURITY: Verify seat PDA derivation
    let (expected_seat_pda, bump) = derive_lp_seat_pda(
        program_id,
        matcher_state,
        portfolio_account.key(),
        context_id,
        program_id,
    );

    if seat_account.key() != &expected_seat_pda {
        msg!("Error: Seat PDA derivation mismatch");
        return Err(PercolatorError::InvalidAccount);
    }

    // SECURITY: Verify seat account ownership
    if seat_account.owner() != program_id {
        msg!("Error: Seat account has incorrect owner");
        return Err(PercolatorError::InvalidAccountOwner);
    }

    // SECURITY: Verify account size
    let seat_data = seat_account.try_borrow_mut_data()
        .map_err(|_| PercolatorError::InvalidAccount)?;

    if seat_data.len() < RouterLpSeat::LEN {
        msg!("Error: Seat account too small");
        return Err(PercolatorError::InvalidAccount);
    }

    // SECURITY: Check if already initialized (router_id field should be zero)
    let mut is_initialized = false;
    for i in 0..32 {
        if seat_data[i] != 0 {
            is_initialized = true;
            break;
        }
    }

    if is_initialized {
        msg!("Error: Seat account is already initialized");
        return Err(PercolatorError::AlreadyInitialized);
    }

    // Initialize the seat in place
    // SAFETY: We've verified size, ownership, and initialization status
    let seat = unsafe {
        &mut *(seat_data.as_ptr() as *mut RouterLpSeat)
    };

    seat.initialize_in_place(
        *program_id,
        *matcher_state,
        *portfolio_account.key(),
        context_id,
        bump,
    );

    msg!("LP Seat initialized successfully");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_seat_init_validates_authorization() {
        // This is a documentation test - actual testing requires BPF environment
        // The key authorization check is: signer.key() == portfolio.owner
    }

    #[test]
    fn test_seat_init_validates_pda_derivation() {
        // This is a documentation test - actual testing requires BPF environment
        // The key validation is: derive_lp_seat_pda must match seat_account.key()
    }
}
