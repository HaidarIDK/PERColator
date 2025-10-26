//! RegisterSlab instruction - register a new slab in the registry

use crate::state::SlabRegistry;
use percolator_common::*;
use pinocchio::{
    account_info::AccountInfo,
    msg,
    pubkey::Pubkey,
    sysvars::{clock::Clock, Sysvar},
};

/// Process register_slab instruction
///
/// Registers a new slab in the registry. Only callable by governance authority.
///
/// # Security Checks
/// - Verifies governance is signer
/// - Verifies governance matches registry.governance
/// - Validates slab_id and oracle_id are not default/zero
/// - Checks registry has capacity for more slabs
///
/// # Arguments
/// * `registry_account` - The registry account (writable)
/// * `governance` - The governance authority (signer)
/// * `slab_id` - Pubkey of the slab program account
/// * `version_hash` - 32-byte hash of slab version
/// * `oracle_id` - Pubkey of the price oracle
/// * `imr` - Initial margin ratio (bps)
/// * `mmr` - Maintenance margin ratio (bps)
/// * `maker_fee_cap` - Maximum maker fee (bps)
/// * `taker_fee_cap` - Maximum taker fee (bps)
/// * `latency_sla_ms` - Latency SLA in milliseconds
/// * `max_exposure` - Maximum position exposure
pub fn process_register_slab(
    registry_account: &AccountInfo,
    governance: &AccountInfo,
    slab_id: Pubkey,
    version_hash: [u8; 32],
    oracle_id: Pubkey,
    imr: u64,
    mmr: u64,
    maker_fee_cap: u64,
    taker_fee_cap: u64,
    latency_sla_ms: u64,
    max_exposure: u128,
) -> Result<(), PercolatorError> {
    // SECURITY: Verify governance is signer
    if !governance.is_signer() {
        msg!("Error: Governance must be a signer");
        return Err(PercolatorError::Unauthorized);
    }

    // Borrow registry data mutably
    let registry = unsafe { borrow_account_data_mut::<SlabRegistry>(registry_account)? };

    // SECURITY: Verify governance authority matches
    let governance_pubkey_array = governance.key();
    if &registry.governance != governance_pubkey_array {
        msg!("Error: Invalid governance authority");
        return Err(PercolatorError::Unauthorized);
    }

    // SECURITY: Validate slab_id is not default
    if slab_id == Pubkey::default() {
        msg!("Error: Invalid slab_id");
        return Err(PercolatorError::InvalidAccount);
    }

    // SECURITY: Validate oracle_id is not default
    if oracle_id == Pubkey::default() {
        msg!("Error: Invalid oracle_id");
        return Err(PercolatorError::InvalidAccount);
    }

    // Get current timestamp from Clock sysvar
    let current_ts = Clock::get()
        .map(|clock| clock.unix_timestamp as u64)
        .unwrap_or(0);

    // Register the slab
    let _slab_index = registry
        .register_slab(
            slab_id,
            version_hash,
            oracle_id,
            imr,
            mmr,
            maker_fee_cap,
            taker_fee_cap,
            latency_sla_ms,
            max_exposure,
            current_ts,
        )
        .map_err(|_| {
            msg!("Error: Failed to register slab");
            PercolatorError::InvalidInstruction
        })?;

    msg!("Slab registered successfully");
    Ok(())
}

// Exclude test module from BPF builds
#[cfg(all(test, not(target_os = "solana")))]
#[path = "register_slab_test.rs"]
mod register_slab_test;
