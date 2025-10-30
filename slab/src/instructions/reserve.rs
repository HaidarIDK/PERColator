//! Reserve instruction - lock liquidity for trading (v0 simplified)

use percolator_common::*;
use pinocchio::{account_info::AccountInfo, msg, pubkey::Pubkey};

/// Process reserve instruction (v0 simplified version)
///
/// For v0 POC, this just logs the reserve request and returns success.
/// In production, this would:
/// 1. Validate the slab account
/// 2. Check user has sufficient margin
/// 3. Create a reservation entry in the slab
/// 4. Lock the liquidity
/// 5. Return a hold_id
///
/// # Arguments
/// * `program_id` - The slab program ID
/// * `slab_account` - The slab account
/// * `user_account` - The user's wallet (signer)
/// * `instruction_data` - Serialized reserve parameters
pub fn process_reserve(
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
    // Format: [account_idx(4), instrument_idx(2), side(1), qty(8), limit_px(8), ttl_ms(8), commitment_hash(32), route_id(8)]
    if instruction_data.len() < 71 {
        msg!("Error: Invalid instruction data length");
        return Err(PercolatorError::InvalidAccount);
    }

    // For v0 POC, just log and return success
    msg!("Reserve instruction received (v0 POC mode)");
    msg!("Reservation created successfully (mock)");

    // In production, would:
    // - Parse all parameters
    // - Validate margin requirements
    // - Create reservation in slab state
    // - Emit event with hold_id
    
    Ok(())
}

