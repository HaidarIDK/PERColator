//! Initialize receipt PDA instruction

use percolator_common::*;
use pinocchio::{account_info::AccountInfo, msg, pubkey::Pubkey, ProgramResult};

/// Initialize receipt PDA for a slab/user pair
///
/// This creates the receipt PDA that will be used to return fill results from commit_fill.
/// Must be called before ExecuteCrossSlab.
///
/// # Arguments
/// * `receipt_account` - Receipt PDA (to be created, derived from ["receipt", slab, user])
/// * `slab_account` - Slab account (for PDA derivation)
/// * `user_account` - User account (for PDA derivation)
/// * `payer_account` - Payer for rent (signer)
/// * `system_program` - System program for account creation
///
/// # PDA Seeds
/// * ["receipt", slab_pubkey, user_pubkey] owned by slab program
pub fn process_initialize_receipt(
    program_id: &Pubkey,
    receipt_account: &AccountInfo,
    slab_account: &AccountInfo,
    user_account: &AccountInfo,
    payer_account: &AccountInfo,
    system_program: &AccountInfo,
) -> ProgramResult {
    // Derive expected receipt PDA
    let (expected_receipt, bump) = pinocchio::pubkey::find_program_address(
        &[
            b"receipt",
            slab_account.key().as_ref(),
            user_account.key().as_ref(),
        ],
        program_id,
    );

    // Verify the passed receipt matches the derived PDA
    if receipt_account.key() != &expected_receipt {
        msg!("Error: Receipt PDA mismatch");
        return Err(PercolatorError::InvalidAccount.into());
    }

    // Check if already initialized
    if receipt_account.lamports() > 0 {
        msg!("Receipt PDA already initialized");
        return Ok(());
    }

    // Calculate rent
    use pinocchio::sysvars::{rent::Rent, Sysvar};
    let rent = Rent::get().map_err(|_| PercolatorError::InvalidAccount)?;
    let receipt_size = core::mem::size_of::<FillReceipt>();
    let rent_lamports = rent.minimum_balance(receipt_size);

    // Create PDA account using Pinocchio's CPI
    use pinocchio::{
        instruction::{AccountMeta, Instruction, Seed, Signer},
        program::invoke_signed,
    };

    // Build create_account instruction manually
    // Instruction layout: discriminator (4 bytes) + lamports (8) + space (8) + owner (32)
    let mut instruction_data = [0u8; 52];
    instruction_data[0..4].copy_from_slice(&[0, 0, 0, 0]); // CreateAccount discriminator
    instruction_data[4..12].copy_from_slice(&rent_lamports.to_le_bytes());
    instruction_data[12..20].copy_from_slice(&(receipt_size as u64).to_le_bytes());
    instruction_data[20..52].copy_from_slice(program_id.as_ref());

    let account_metas = [
        AccountMeta::writable_signer(payer_account.key()),
        AccountMeta::writable(receipt_account.key()),
    ];

    let create_ix = Instruction {
        program_id: system_program.key(),
        accounts: &account_metas,
        data: &instruction_data,
    };

    let bump_seed = [bump];
    let signer_seeds: &[Seed] = &[
        Seed::from(b"receipt" as &[u8]),
        Seed::from(slab_account.key().as_ref()),
        Seed::from(user_account.key().as_ref()),
        Seed::from(&bump_seed[..]),
    ];

    invoke_signed(
        &create_ix,
        &[payer_account, receipt_account, system_program],
        &[Signer::from(signer_seeds)],
    )
    .map_err(|_| PercolatorError::InvalidAccount)?;

    msg!("Receipt PDA initialized successfully");
    Ok(())
}
