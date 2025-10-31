//! HaltTrading instruction
//!
//! Allows LP owner to halt all trading activity on the slab.
//! When halted, PlaceOrder and CommitFill instructions will be rejected.

use crate::state::SlabState;
use percolator_common::PercolatorError;
use pinocchio::{msg, pubkey::Pubkey};

/// Process halt_trading instruction
///
/// Halts all trading activity. Only the LP owner can call this.
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
pub fn process_halt_trading(
    slab: &mut SlabState,
    authority: &Pubkey,
) -> Result<(), PercolatorError> {
    // Verify authority is the LP owner
    if &slab.header.lp_owner != authority {
        msg!("Error: Only LP owner can halt trading");
        return Err(PercolatorError::Unauthorized);
    }

    // Halt trading
    slab.header.halt_trading();
    msg!("Trading halted");

    Ok(())
}
