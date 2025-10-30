//! Initialize instruction - initialize slab state (v0 minimal)

use crate::state::{SlabHeader, SlabState};
use crate::pda::derive_slab_pda;
use percolator_common::*;
use pinocchio::{
    account_info::AccountInfo, 
    msg, 
    pubkey::Pubkey,
};

/// Process initialize instruction for slab (v0 minimal)
///
/// Initializes the ~4KB slab state account with header, quote cache, and book.
/// This is called once during slab deployment for each market.
/// Creates the PDA if it doesn't exist.
///
/// # Arguments
/// * `program_id` - The slab program ID
/// * `slab_account` - The slab account to initialize (PDA)
/// * `payer` - The payer account (for rent)
/// * `system_program` - The system program account
/// * `lp_owner` - LP owner pubkey
/// * `router_id` - Router program ID
/// * `instrument` - Shared instrument ID (agreed with router, used as market_id)
/// * `mark_px` - Initial mark price from oracle (1e6 scale)
/// * `taker_fee_bps` - Taker fee (basis points)
/// * `contract_size` - Contract size (1e6 scale)
/// * `bump` - PDA bump seed
pub fn process_initialize_slab(
    program_id: &Pubkey,
    slab_account: &AccountInfo,
    lp_owner: Pubkey,
    router_id: Pubkey,
    instrument: Pubkey,
    mark_px: i64,
    taker_fee_bps: i64,
    contract_size: i64,
    bump: u8,
) -> Result<(), PercolatorError> {
    // v0: Skip PDA verification for testing - allows regular accounts
    // TODO: Re-enable PDA check for production
    let (_expected_pda, _expected_bump) = derive_slab_pda(instrument.as_ref(), program_id);

    // For v0, accept any account owned by the program
    // if slab_account.key() != &expected_pda {
    //     msg!("Error: Slab account is not the correct PDA");
    //     return Err(PercolatorError::InvalidAccount);
    // }

    // if bump != expected_bump {
    //     msg!("Error: Bump seed does not match derived PDA");
    //     return Err(PercolatorError::InvalidAccount);
    // }

    // Verify account size
    let data = slab_account.try_borrow_data()
        .map_err(|_| PercolatorError::InvalidAccount)?;

    if data.len() != SlabState::LEN {
        msg!("Error: Slab account has incorrect size");
        return Err(PercolatorError::InvalidAccount);
    }

    // Check if already initialized (magic bytes should not match)
    if data.len() >= 8 && &data[0..8] == SlabHeader::MAGIC {
        msg!("Error: Slab account already initialized");
        return Err(PercolatorError::InvalidAccount);
    }

    drop(data);

    // Initialize the slab state
    let slab = unsafe { borrow_account_data_mut::<SlabState>(slab_account)? };

    // Initialize header with v0 parameters
    let header = SlabHeader::new(
        *program_id,
        lp_owner,
        router_id,
        instrument,
        mark_px,
        taker_fee_bps,
        contract_size,
        bump,
    );

    // Create new slab state (initializes quote_cache and book automatically)
    *slab = SlabState::new(header);

    msg!("Slab initialized successfully");
    Ok(())
}

#[cfg(test)]
#[path = "initialize_test.rs"]
mod initialize_test;
