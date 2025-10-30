//! Cancel instruction - cancel a reservation (v0 simplified)

use percolator_common::*;
use pinocchio::{account_info::AccountInfo, msg, pubkey::Pubkey};

/// Process cancel instruction (v0 simplified version)
///
/// For v0 POC, this just logs the cancellation and returns success.
/// In production, this would:
/// 1. Find the reservation by hold_id
/// 2. Verify user owns it
/// 3. Release locked liquidity
/// 4. Clear the reservation
///
/// # Arguments
/// * `program_id` - The slab program ID
/// * `slab_account` - The slab account
/// * `user_account` - The user's wallet (signer)
/// * `instruction_data` - Serialized cancel parameters (hold_id)
pub fn process_cancel(
    _program_id: &Pubkey,
    slab_account: &AccountInfo,
    user_account: &AccountInfo,
    instruction_data: &[u8],
) -> Result<(), PercolatorError> {
    // Verify slab account is writable
    if !slab_account.is_writable() {
        msg!("Error: Slab account must be writable");
        return Err(PercolatorError::InvalidAccount);
    }

    // Verify user is signer
    if !user_account.is_signer() {
        msg!("Error: User must be signer");
        return Err(PercolatorError::Unauthorized);
    }

    // Parse instruction data
    // Format: [hold_id(8)]
    if instruction_data.len() < 8 {
        msg!("Error: Invalid instruction data length");
        return Err(PercolatorError::InvalidAccount);
    }

    // For v0 POC, just log and return success
    msg!("Cancel instruction received (v0 POC mode)");
    msg!("Reservation cancelled successfully (mock)");

    // In production, would:
    // - Find and verify reservation
    // - Release locked liquidity
    // - Clear reservation entry
    // - Emit cancel event
    
    Ok(())
}

