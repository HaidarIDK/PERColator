//! Initialize portfolio instruction

use crate::pda::derive_portfolio_pda;
use crate::state::Portfolio;
use percolator_common::*;
use pinocchio::{account_info::AccountInfo, msg, pubkey::Pubkey};

/// Process initialize portfolio instruction
///
/// Initializes a user's portfolio account for cross-margin tracking.
///
/// # Arguments
/// * `program_id` - The router program ID
/// * `portfolio_account` - The portfolio account to initialize (must be PDA)
/// * `user` - The user pubkey
pub fn process_initialize_portfolio(
    program_id: &Pubkey,
    portfolio_account: &AccountInfo,
    user: &Pubkey,
) -> Result<(), PercolatorError> {
    // Derive and verify portfolio PDA
    let (expected_pda, bump) = derive_portfolio_pda(user, program_id);

    if portfolio_account.key() != &expected_pda {
        msg!("Error: Portfolio account is not the correct PDA");
        return Err(PercolatorError::InvalidAccount);
    }

    // Verify account size
    let data = portfolio_account.try_borrow_data()
        .map_err(|_| PercolatorError::InvalidAccount)?;

    if data.len() != Portfolio::LEN {
        msg!("Error: Portfolio account has incorrect size");
        return Err(PercolatorError::InvalidAccount);
    }

    // Check if already initialized (first bytes should be zero)
    if data.len() >= 32 && data[0] != 0 {
        msg!("Error: Portfolio account may already be initialized");
        return Err(PercolatorError::InvalidAccount);
    }

    drop(data);

    // Initialize the portfolio in-place (avoids stack overflow)
    let portfolio = unsafe { borrow_account_data_mut::<Portfolio>(portfolio_account)? };

    portfolio.initialize_in_place(*program_id, *user, bump);

    msg!("Portfolio initialized successfully");
    Ok(())
}
