//! WithdrawInsurance instruction - withdraw surplus from insurance fund

use crate::state::SlabRegistry;
use percolator_common::*;
use pinocchio::{
    account_info::AccountInfo,
    msg,
    pubkey::Pubkey,
};

/// Process withdraw_insurance instruction
///
/// Withdraws surplus from the insurance fund. Only callable by insurance authority.
/// Cannot withdraw if there is uncovered bad debt.
///
/// # Security Checks
/// - Verifies insurance_authority is signer
/// - Verifies insurance_authority matches registry.insurance_authority
/// - Ensures no uncovered bad debt exists
/// - Ensures sufficient vault balance
///
/// # Arguments
/// * `registry_account` - The registry account (writable)
/// * `insurance_authority` - The insurance authority (signer, writable for receiving funds)
/// * `amount` - Amount to withdraw (lamports)
pub fn process_withdraw_insurance(
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

    // Attempt withdrawal (internally checks uncovered_bad_debt and balance)
    registry.insurance_state.withdraw_surplus(amount)
        .map_err(|_| {
            msg!("Error: Cannot withdraw - either uncovered bad debt exists or insufficient balance");
            PercolatorError::InsufficientFunds
        })?;

    // TODO: Transfer lamports from insurance vault PDA to insurance_authority
    // For now, just update the state (actual transfer requires vault PDA implementation)

    msg!("Insurance withdrawal successful");
    Ok(())
}

// Exclude test module from BPF builds
#[cfg(all(test, not(target_os = "solana")))]
#[path = "withdraw_insurance_test.rs"]
mod withdraw_insurance_test;
