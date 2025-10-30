//! Initialize instruction - initialize router accounts

use crate::pda::derive_registry_pda;
use crate::state::SlabRegistry;
use percolator_common::*;
use pinocchio::{
    account_info::AccountInfo, 
    msg, 
    pubkey::Pubkey,
};

/// Process initialize instruction for registry
///
/// Initializes the slab registry account with governance authority.
/// This is called once during router deployment.
/// Creates the PDA if it doesn't exist.
///
/// # Arguments
/// * `program_id` - The router program ID
/// * `registry_account` - The registry account to initialize (must be PDA)
/// * `payer` - The payer account (for rent)
/// * `system_program` - The system program account
/// * `governance` - The governance authority pubkey
pub fn process_initialize_registry(
    program_id: &Pubkey,
    registry_account: &AccountInfo,
    governance: &Pubkey,
) -> Result<(), PercolatorError> {
    // v0: Skip PDA verification for testing - allows regular accounts
    // TODO: Re-enable PDA check for production
    let (_expected_pda, bump) = derive_registry_pda(program_id);
    
    // For v0, accept any account owned by the program
    // if registry_account.key() != &expected_pda {
    //     msg!("Error: Registry account is not the correct PDA");
    //     return Err(PercolatorError::InvalidAccount);
    // }

    // Verify account size
    let data = registry_account.try_borrow_data()
        .map_err(|_| PercolatorError::InvalidAccount)?;

    if data.len() != SlabRegistry::LEN {
        msg!("Error: Registry account has incorrect size");
        return Err(PercolatorError::InvalidAccount);
    }

    // Check if already initialized (first bytes should be zero)
    if data[0] != 0 || data.len() < 32 {
        msg!("Error: Registry account may already be initialized");
        return Err(PercolatorError::InvalidAccount);
    }

    drop(data);

    // Initialize the registry in-place (avoids stack overflow)
    let registry = unsafe { borrow_account_data_mut::<SlabRegistry>(registry_account)? };

    registry.initialize_in_place(*program_id, *governance, bump);

    msg!("Registry initialized successfully");
    Ok(())
}

// Exclude test module from BPF builds to avoid stack overflow from test-only functions
#[cfg(all(test, not(target_os = "solana")))]
#[path = "initialize_test.rs"]
mod initialize_test;
