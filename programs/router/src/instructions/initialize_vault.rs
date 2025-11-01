//! Initialize vault instruction

use crate::state::Vault;
use percolator_common::*;
use pinocchio::{
    account_info::AccountInfo,
    msg,
    pubkey::Pubkey,
};

/// Process initialize vault instruction
///
/// Initializes a collateral vault account for holding protocol funds.
/// The vault account must be created externally before calling this instruction.
///
/// # Security Checks
/// - Verifies vault account is owned by the router program
/// - Verifies authority is a signer
/// - Prevents double initialization
/// - Validates account size matches Vault::LEN
///
/// # Arguments
/// * `program_id` - The router program ID
/// * `vault_account` - The vault account to initialize
/// * `authority` - Authority that can initialize (must be signer)
/// * `mint` - The mint pubkey for this vault (e.g., native SOL = Pubkey::default())
pub fn process_initialize_vault(
    program_id: &Pubkey,
    vault_account: &AccountInfo,
    authority: &AccountInfo,
    mint: &Pubkey,
) -> Result<(), PercolatorError> {
    // SECURITY: Verify authority is signer
    if !authority.is_signer() {
        msg!("Error: Authority must be a signer");
        return Err(PercolatorError::Unauthorized);
    }

    // SECURITY: Verify account ownership
    if vault_account.owner() != program_id {
        msg!("Error: Vault account has incorrect owner");
        return Err(PercolatorError::InvalidAccountOwner);
    }

    // SECURITY: Verify account size
    let mut data = vault_account.try_borrow_mut_data()
        .map_err(|_| PercolatorError::InvalidAccount)?;

    if data.len() < Vault::LEN {
        msg!("Error: Vault account too small");
        return Err(PercolatorError::InvalidAccount);
    }

    // SECURITY: Check if already initialized (router_id field should be zero)
    let mut is_initialized = false;
    for i in 0..32 {
        if data[i] != 0 {
            is_initialized = true;
            break;
        }
    }

    if is_initialized {
        msg!("Error: Vault account is already initialized");
        return Err(PercolatorError::AlreadyInitialized);
    }

    // Initialize vault data
    // Vault layout: router_id (32) + mint (32) + token_account (32) + balance (16) + total_pledged (16) + bump (1) + padding (7) = 136 bytes
    let vault_data = Vault {
        router_id: *program_id,
        mint: *mint,
        token_account: Pubkey::default(), // Not used for native SOL
        balance: 0,
        total_pledged: 0,
        bump: 0, // For regular accounts (not PDAs), bump is 0
        _padding: [0; 7],
    };

    // Write vault data to account
    // SAFETY: We've verified the account size and ownership above
    unsafe {
        let vault_ptr = data.as_mut_ptr() as *mut Vault;
        core::ptr::write(vault_ptr, vault_data);
    }

    msg!("Vault initialized successfully");
    Ok(())
}
