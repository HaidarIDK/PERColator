//! Program Derived Address (PDA) helpers for Slab program
//!
//! PDAs are deterministic addresses derived from seeds and the program ID.
//! They allow the program to own and control accounts without needing a private key.

use pinocchio::pubkey::{create_program_address, find_program_address, Pubkey};

/// Seed prefix for slab state accounts
pub const SLAB_SEED: &[u8] = b"slab";

/// Seed prefix for slab authority (PDA that signs for the slab)
pub const AUTHORITY_SEED: &[u8] = b"authority";

/// Derive slab state PDA
///
/// The slab state is the main 10MB account storing all orderbook data
///
/// # Arguments
/// * `market_id` - Unique identifier for this market/slab
/// * `program_id` - The slab program ID
///
/// # Returns
/// * `(Pubkey, u8)` - The derived PDA and its bump seed
pub fn derive_slab_pda(market_id: &[u8], program_id: &Pubkey) -> (Pubkey, u8) {
    find_program_address(&[SLAB_SEED, market_id], program_id)
}

/// Derive slab authority PDA
///
/// The authority PDA can sign on behalf of the slab for CPIs
///
/// # Arguments
/// * `slab` - The slab state account pubkey
/// * `program_id` - The slab program ID
///
/// # Returns
/// * `(Pubkey, u8)` - The derived PDA and its bump seed
pub fn derive_authority_pda(slab: &Pubkey, program_id: &Pubkey) -> (Pubkey, u8) {
    find_program_address(&[AUTHORITY_SEED, slab.as_ref()], program_id)
}

/// Verify that a given pubkey matches the expected slab PDA
///
/// # Arguments
/// * `pubkey` - The pubkey to verify
/// * `market_id` - The market ID used to derive the PDA
/// * `bump` - The bump seed
/// * `program_id` - The slab program ID
///
/// # Returns
/// * `bool` - True if the pubkey matches the derived PDA
pub fn verify_slab_pda(
    pubkey: &Pubkey,
    market_id: &[u8],
    bump: u8,
    program_id: &Pubkey,
) -> bool {
    let derived = create_program_address(
        &[SLAB_SEED, market_id, &[bump]],
        program_id,
    );

    match derived {
        Ok(derived_pubkey) => &derived_pubkey == pubkey,
        Err(_) => false,
    }
}

/// Verify that a given pubkey matches the expected authority PDA
///
/// # Arguments
/// * `pubkey` - The pubkey to verify
/// * `slab` - The slab state account pubkey
/// * `bump` - The bump seed
/// * `program_id` - The slab program ID
///
/// # Returns
/// * `bool` - True if the pubkey matches the derived PDA
pub fn verify_authority_pda(
    pubkey: &Pubkey,
    slab: &Pubkey,
    bump: u8,
    program_id: &Pubkey,
) -> bool {
    let derived = create_program_address(
        &[AUTHORITY_SEED, slab.as_ref(), &[bump]],
        program_id,
    );

    match derived {
        Ok(derived_pubkey) => &derived_pubkey == pubkey,
        Err(_) => false,
    }
}

#[cfg(test)]
mod tests {
    #[cfg(target_os = "solana")]
    use super::*;

    // Note: PDA tests only run on Solana target due to syscall requirements
    #[test]
    #[cfg(target_os = "solana")]
    fn test_slab_pda_derivation() {
        let program_id = Pubkey::default();
        let market_id = b"BTC-PERP";

        let (pda1, bump1) = derive_slab_pda(market_id, &program_id);
        let (pda2, bump2) = derive_slab_pda(market_id, &program_id);

        // Same inputs should produce same output
        assert_eq!(pda1, pda2);
        assert_eq!(bump1, bump2);
    }

    #[test]
    #[cfg(target_os = "solana")]
    fn test_slab_pda_different_markets() {
        let program_id = Pubkey::default();
        let market_id1 = b"BTC-PERP";
        let market_id2 = b"ETH-PERP";

        let (pda1, _) = derive_slab_pda(market_id1, &program_id);
        let (pda2, _) = derive_slab_pda(market_id2, &program_id);

        // Different market IDs should produce different PDAs
        assert_ne!(pda1, pda2);
    }

    #[test]
    #[cfg(target_os = "solana")]
    fn test_authority_pda_derivation() {
        let program_id = Pubkey::default();
        let slab = Pubkey::default();

        let (pda1, bump1) = derive_authority_pda(&slab, &program_id);
        let (pda2, bump2) = derive_authority_pda(&slab, &program_id);

        // Same inputs should produce same output
        assert_eq!(pda1, pda2);
        assert_eq!(bump1, bump2);
    }

    #[test]
    #[cfg(target_os = "solana")]
    fn test_verify_slab_pda() {
        let program_id = Pubkey::default();
        let market_id = b"BTC-PERP";

        let (pda, bump) = derive_slab_pda(market_id, &program_id);

        // Verification should succeed with correct parameters
        assert!(verify_slab_pda(&pda, market_id, bump, &program_id));

        // Verification should fail with wrong bump
        assert!(!verify_slab_pda(&pda, market_id, bump.wrapping_add(1), &program_id));
    }

    #[test]
    #[cfg(target_os = "solana")]
    fn test_verify_authority_pda() {
        let program_id = Pubkey::default();
        let slab = Pubkey::default();

        let (pda, bump) = derive_authority_pda(&slab, &program_id);

        // Verification should succeed with correct parameters
        assert!(verify_authority_pda(&pda, &slab, bump, &program_id));

        // Verification should fail with wrong bump
        assert!(!verify_authority_pda(&pda, &slab, bump.wrapping_add(1), &program_id));
    }

    #[test]
    #[cfg(target_os = "solana")]
    fn test_slab_pda_is_on_curve() {
        let program_id = Pubkey::default();
        let market_id = b"BTC-PERP";

        let (pda, _bump) = derive_slab_pda(market_id, &program_id);

        // PDAs should always be off-curve (valid for Solana)
        // This is guaranteed by find_program_address
        // We just check that derivation succeeds
        assert_ne!(pda, Pubkey::default());
    }
}
