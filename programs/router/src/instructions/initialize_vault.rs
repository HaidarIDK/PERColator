//! Initialize vault instruction - create the vault PDA for a specific mint

use crate::pda::derive_vault_pda;
use crate::state::Vault;
use percolator_common::*;
use pinocchio::{
    account_info::AccountInfo,
    msg,
    pubkey::Pubkey,
    sysvars::{rent::Rent, Sysvar},
    ProgramResult,
};

/// Process initialize vault instruction
///
/// Creates and initializes the vault PDA for a specific mint (e.g., SOL).
/// The vault stores collateral and tracks pledges to escrows.
///
/// # Security Checks
/// - Verifies payer is a signer
/// - Verifies vault is the correct PDA for the mint
/// - Prevents double initialization
/// - Validates account size
///
/// # Arguments
/// * `program_id` - The router program ID
/// * `vault_account` - The vault PDA account to initialize
/// * `mint_account` - The mint account (e.g., native SOL mint)
/// * `payer` - Account paying for rent
/// * `system_program` - The System Program
pub fn process_initialize_vault(
    program_id: &Pubkey,
    vault_account: &AccountInfo,
    mint_account: &AccountInfo,
    payer: &AccountInfo,
    system_program: &AccountInfo,
) -> ProgramResult {
    // SECURITY: Verify payer is signer
    if !payer.is_signer() {
        msg!("Error: Payer must be a signer");
        return Err(PercolatorError::Unauthorized.into());
    }

    // Derive expected vault PDA
    let (expected_vault, bump) = derive_vault_pda(mint_account.key(), program_id);

    // SECURITY: Verify vault address matches expected PDA
    if vault_account.key() != &expected_vault {
        msg!("Error: Invalid vault PDA");
        return Err(PercolatorError::InvalidAccount.into());
    }

    // Check if vault already exists
    if vault_account.data_len() > 0 {
        msg!("Error: Vault already initialized");
        return Err(PercolatorError::AlreadyInitialized.into());
    }

    // Calculate rent
    let vault_size = Vault::LEN;
    let rent = Rent::get()?;
    let rent_lamports = rent.minimum_balance(vault_size);

    // Create the vault PDA using CPI to System Program
    // Instruction: CreateAccount with PDA
    let create_account_ix_data = {
        let mut data = [0u8; 52];
        // System Program CreateAccount discriminator = 0
        data[0..4].copy_from_slice(&0u32.to_le_bytes());
        // lamports
        data[4..12].copy_from_slice(&rent_lamports.to_le_bytes());
        // space
        data[12..20].copy_from_slice(&(vault_size as u64).to_le_bytes());
        // owner (program_id)
        data[20..52].copy_from_slice(program_id.as_ref());
        data
    };

    // Build seeds for PDA signing
    use pinocchio::instruction::{Seed, Signer};
    use crate::pda::VAULT_SEED;

    let bump_bytes = [bump];
    let seeds = &[
        Seed::from(VAULT_SEED),
        Seed::from(mint_account.key().as_ref()),
        Seed::from(&bump_bytes[..]),
    ];
    let signer = Signer::from(seeds);

    // CPI to create account
    invoke_signed(
        &Instruction {
            program_id: system_program.key(),
            accounts: &[
                AccountMeta {
                    pubkey: payer.key(),
                    is_signer: true,
                    is_writable: true,
                },
                AccountMeta {
                    pubkey: vault_account.key(),
                    is_signer: true, // MUST be true for CreateAccount, even for PDAs
                    is_writable: true,
                },
            ],
            data: &create_account_ix_data,
        },
        &[payer, vault_account, system_program],
        &[signer],
    )?;

    // Initialize vault state
    let vault = unsafe { borrow_account_data_mut::<Vault>(vault_account)? };

    vault.router_id = *program_id;
    vault.mint = *mint_account.key();
    vault.token_account = Pubkey::default(); // Not used for native SOL
    vault.balance = 0;
    vault.total_pledged = 0;
    vault.bump = bump;
    vault._padding = [0; 7];

    msg!("Vault initialized successfully");
    Ok(())
}

// Note: Using invoke_signed from pinocchio
use pinocchio::instruction::{AccountMeta, Instruction};
use pinocchio::program::invoke_signed;

