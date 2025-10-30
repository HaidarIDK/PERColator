//! Commit instruction - execute a reservation (v0 simplified)

use percolator_common::*;
use pinocchio::{account_info::AccountInfo, msg, pubkey::Pubkey};

/// Process commit instruction (v0 simplified version)
///
/// For v0 POC, this just logs the commit and returns success.
/// In production, this would:
/// 1. Verify the commitment reveal matches the hash
/// 2. Find the reservation by hold_id
/// 3. Execute the trade
/// 4. Update positions
/// 5. Clear the reservation
///
/// # Arguments
/// * `program_id` - The slab program ID
/// * `slab_account` - The slab account
/// * `user_account` - The user's wallet (signer)
/// * `instruction_data` - Serialized commit parameters
pub fn process_commit(
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
    // Format: [hold_id(8), commitment_reveal(32)]
    if instruction_data.len() < 40 {
        msg!("Error: Invalid instruction data length");
        return Err(PercolatorError::InvalidAccount);
    }

    // For v0 POC, just log and return success
    msg!("Commit instruction received (v0 POC mode)");
    msg!("Trade committed successfully (mock)");

    // In production, would:
    // - Verify commitment reveal
    // - Find reservation
    // - Execute trade
    // - Update slab state
    // - Emit fill event
    
    Ok(())
}

