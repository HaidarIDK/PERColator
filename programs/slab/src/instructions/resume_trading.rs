//! ResumeTrading instruction
//!
//! Allows LP owner to resume trading activity on the slab after a halt.

use crate::state::SlabState;
use percolator_common::PercolatorError;
use pinocchio::{msg, pubkey::Pubkey};

/// Process resume_trading instruction
///
/// Resumes trading activity after a halt. Only the LP owner can call this.
///
/// # Arguments
/// * `slab` - The slab state account (mut)
/// * `authority` - The LP owner (must be signer)
///
/// # Returns
/// * Ok(()) on success
///
/// # Errors
/// * Unauthorized - If caller is not the LP owner
pub fn process_resume_trading(
    slab: &mut SlabState,
    authority: &Pubkey,
) -> Result<(), PercolatorError> {
    // Verify authority is the LP owner
    if &slab.header.lp_owner != authority {
        msg!("Error: Only LP owner can resume trading");
        return Err(PercolatorError::Unauthorized);
    }

    // Resume trading
    slab.header.resume_trading();
    msg!("Trading resumed");

    Ok(())
}
