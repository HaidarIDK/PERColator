//! TopUpInsurance instruction - manually top up insurance fund

use crate::state::SlabRegistry;
use percolator_common::*;
use pinocchio::{
    account_info::AccountInfo,
    msg,
    pubkey::Pubkey,
};

/// Process top_up_insurance instruction
///
/// Manually tops up the insurance fund. Only callable by insurance authority.
/// Useful for bootstrapping or emergency funding.
///
/// # Security Checks
/// - Verifies insurance_authority is signer
/// - Verifies insurance_authority matches registry.insurance_authority
///
/// # Arguments
/// * `registry_account` - The registry account (writable)
/// * `insurance_authority` - The insurance authority (signer, writable for sending funds)
/// * `amount` - Amount to deposit (lamports)
pub fn process_topup_insurance(
    registry_account: &AccountInfo,
    insurance_authority: &AccountInfo,
    amount: u128,
) -> Result<(), PercolatorError> {
    // SECURITY: Verify insurance_authority is signer
    if !insurance_authority.is_signer() {
        msg!("Error: Insurance authority must be a signer");
        return Err(PercolatorError::Unauthorized);
    }

    // Borrow registry data mutably
    let registry = unsafe { borrow_account_data_mut::<SlabRegistry>(registry_account)? };

    // SECURITY: Verify insurance_authority matches
    let insurance_authority_pubkey_array = insurance_authority.key();
    if &registry.insurance_authority != insurance_authority_pubkey_array {
        msg!("Error: Invalid insurance authority");
        return Err(PercolatorError::Unauthorized);
    }

    // Top up the insurance vault
    registry.insurance_state.top_up(amount);

    // TODO: Transfer lamports from insurance_authority to insurance vault PDA
    // For now, just update the state (actual transfer requires vault PDA implementation)

    msg!("Insurance top-up successful");
    Ok(())
}

// Exclude test module from BPF builds
#[cfg(all(test, not(target_os = "solana")))]
#[path = "topup_insurance_test.rs"]
mod topup_insurance_test;
