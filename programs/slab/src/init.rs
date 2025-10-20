//! Account initialization helpers for Slab program

use crate::state::{SlabState, SlabHeader};
use percolator_common::*;

/// Initialize a new SlabState account
///
/// This should be called after allocating a 10MB account for the slab.
/// Sets up all headers, pools, and initial state.
///
/// # Arguments
/// * `slab` - Mutable reference to allocated SlabState (must be zeroed)
/// * `authority` - Program authority pubkey
/// * `oracle` - Oracle program ID
/// * `router` - Router program ID
/// * `imr` - Initial margin ratio in basis points (e.g., 500 = 5%)
/// * `mmr` - Maintenance margin ratio in basis points (e.g., 250 = 2.5%)
/// * `maker_fee` - Maker fee in basis points (can be negative for rebates)
/// * `taker_fee` - Taker fee in basis points
/// * `batch_ms` - Batch window duration in milliseconds
/// * `freeze_levels` - Number of top price levels to freeze
pub fn initialize_slab(
    slab: &mut SlabState,
    authority: pinocchio::pubkey::Pubkey,
    oracle: pinocchio::pubkey::Pubkey,
    router: pinocchio::pubkey::Pubkey,
    imr: u16,
    mmr: u16,
    maker_fee: i16,
    taker_fee: i16,
    batch_ms: u64,
    freeze_levels: u16,
) -> Result<(), PercolatorError> {
    // Validate parameters
    if imr == 0 || mmr == 0 {
        return Err(PercolatorError::InvalidRiskParams);
    }

    if mmr > imr {
        return Err(PercolatorError::InvalidRiskParams);
    }

    if taker_fee < 0 {
        return Err(PercolatorError::InvalidRiskParams);
    }

    // Initialize header
    slab.header = SlabHeader::new(
        authority,
        oracle,
        router,
        imr as u64,
        mmr as u64,
        maker_fee as i64,
        taker_fee as u64,
        batch_ms,
        0, // bump (not used in header, will be removed)
    );
    
    // Set freeze_levels separately (not in SlabHeader::new signature)
    slab.header.freeze_levels = freeze_levels;

    // Initialize instrument count
    slab.instrument_count = 0;

    // All pools are already zero-initialized by allocation
    // No need to explicitly initialize Pool structures

    Ok(())
}

/// Get the required account size for a SlabState
pub const fn get_slab_account_size() -> usize {
    core::mem::size_of::<SlabState>()
}

/// Calculate minimum rent-exempt balance for slab account
///
/// On Solana, accounts must maintain a minimum balance to be rent-exempt.
/// This function calculates that amount for a 10MB slab account.
///
/// Formula: lamports = (account_size + 128) * lamports_per_byte_year * 2
/// where lamports_per_byte_year is typically ~3,480 lamports
#[cfg(target_os = "solana")]
pub fn calculate_slab_rent() -> u64 {
    // Solana rent calculation
    // Size + 128 bytes overhead, * 3480 lamports/byte/year * 2 years
    let size = get_slab_account_size();
    ((size + 128) as u64) * 3_480 * 2
}

#[cfg(not(target_os = "solana"))]
pub fn calculate_slab_rent() -> u64 {
    // For testing, use approximate calculation
    let size = get_slab_account_size();
    ((size + 128) as u64) * 3_480 * 2
}

/// Validate slab initialization parameters
pub fn validate_init_params(
    imr: u16,
    mmr: u16,
    maker_fee: i16,
    taker_fee: i16,
    batch_ms: u64,
) -> Result<(), PercolatorError> {
    // IMR must be positive
    if imr == 0 {
        return Err(PercolatorError::InvalidRiskParams);
    }

    // MMR must be positive
    if mmr == 0 {
        return Err(PercolatorError::InvalidRiskParams);
    }

    // MMR must be less than IMR
    if mmr > imr {
        return Err(PercolatorError::InvalidRiskParams);
    }

    // Taker fee must be non-negative
    if taker_fee < 0 {
        return Err(PercolatorError::InvalidRiskParams);
    }

    // Maker fee can be negative (rebate) but within reasonable bounds
    // Cap at -50 bps (0.5%) rebate and +100 bps (1%) fee
    if maker_fee < -50 || maker_fee > 100 {
        return Err(PercolatorError::InvalidRiskParams);
    }

    // Batch window must be at least 10ms
    if batch_ms < 10 {
        return Err(PercolatorError::InvalidRiskParams);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_slab_account_size() {
        let size = get_slab_account_size();
        // Should be ~7MB (actual size based on struct layout)
        assert!(size > 7_000_000);
        assert!(size < 8_000_000);
    }

    #[test]
    fn test_calculate_slab_rent() {
        let rent = calculate_slab_rent();
        // Should be positive
        assert!(rent > 0);
        // Slab is ~7MB, so rent should be substantial
        // On Solana, large accounts have high rent requirements
    }

    #[test]
    fn test_validate_init_params_valid() {
        assert!(validate_init_params(
            500,  // 5% IMR
            250,  // 2.5% MMR
            -5,   // -0.05% maker rebate
            20,   // 0.2% taker fee
            100,  // 100ms batch
        ).is_ok());
    }

    #[test]
    fn test_validate_init_params_mmr_exceeds_imr() {
        assert!(validate_init_params(
            250,  // IMR
            500,  // MMR > IMR (invalid)
            -5,
            20,
            100,
        ).is_err());
    }

    #[test]
    fn test_validate_init_params_zero_imr() {
        assert!(validate_init_params(
            0,    // Zero IMR (invalid)
            250,
            -5,
            20,
            100,
        ).is_err());
    }

    #[test]
    fn test_validate_init_params_negative_taker_fee() {
        assert!(validate_init_params(
            500,
            250,
            -5,
            -10,  // Negative taker fee (invalid)
            100,
        ).is_err());
    }

    #[test]
    fn test_validate_init_params_batch_too_short() {
        assert!(validate_init_params(
            500,
            250,
            -5,
            20,
            5,    // 5ms batch (too short)
        ).is_err());
    }
}

