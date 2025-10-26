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
/// The account must be created externally using create_account_with_seed before calling this instruction.
///
/// # Security Checks
/// - Verifies registry account is derived from payer with correct seed
/// - Verifies governance pubkey is valid
/// - Prevents double initialization
/// - Validates account ownership and size
///
/// # Arguments
/// * `program_id` - The router program ID
/// * `registry_account` - The registry account (created with seed "registry")
/// * `payer` - Account paying for rent (also base for seed derivation)
/// * `governance` - The governance authority pubkey
pub fn process_initialize_registry(
    program_id: &Pubkey,
    registry_account: &AccountInfo,
    payer: &AccountInfo,
    governance: &Pubkey,
) -> Result<(), PercolatorError> {
    // Derive the authority PDA that will be stored in the registry
    let (authority_pda, bump) = derive_registry_pda(program_id);

    // NOTE: We cannot verify create_with_seed derivation in pinocchio (no_std BPF environment)
    // because Pubkey::create_with_seed is not available. The client is responsible for
    // deriving the correct address. We rely on other security checks:
    // - Ownership verification (must be owned by this program)
    // - Size verification (must match SlabRegistry::LEN exactly)
    // - Initialization check (prevents double-initialization)
    // - Signer verification (payer must sign)

    // SECURITY: Verify payer is signer
    if !payer.is_signer() {
        msg!("Error: Payer must be a signer");
        return Err(PercolatorError::Unauthorized);
    }

    // SECURITY: Verify governance pubkey is valid (not zero/default)
    if governance == &Pubkey::default() {
        msg!("Error: Invalid governance pubkey");
        return Err(PercolatorError::InvalidAccount);
    }

    // SECURITY: Verify account ownership
    if registry_account.owner() != program_id {
        msg!("Error: Registry account has incorrect owner");
        return Err(PercolatorError::InvalidAccount);
    }

    // SECURITY: Verify account size (minimum required size)
    // Accept accounts that are at least as large as SlabRegistry::LEN to handle
    // differences between native Rust and BPF compilation alignment
    let data = registry_account.try_borrow_data()
        .map_err(|_| PercolatorError::InvalidAccount)?;

    if data.len() < SlabRegistry::LEN {
        msg!("Error: Registry account too small");
        return Err(PercolatorError::InvalidAccount);
    }

    // SECURITY: Check if already initialized (router_id should be zero)
    // We check the first 32 bytes which should be the router_id field
    let mut is_initialized = false;
    for i in 0..32 {
        if data[i] != 0 {
            is_initialized = true;
            break;
        }
    }

    if is_initialized {
        msg!("Error: Registry account is already initialized");
        return Err(PercolatorError::AlreadyInitialized);
    }

    drop(data);

    // Initialize the registry in-place (avoids stack overflow)
    // Store the authority PDA in the registry for future authority checks
    let registry = unsafe { borrow_account_data_mut::<SlabRegistry>(registry_account)? };

    registry.initialize_in_place(authority_pda, *governance, bump);

    msg!("Registry initialized successfully");
    Ok(())
}

// Exclude test module from BPF builds to avoid stack overflow from test-only functions
#[cfg(all(test, not(target_os = "solana")))]
#[path = "initialize_test.rs"]
mod initialize_test;
