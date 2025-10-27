//! Program Derived Address (PDA) helpers for Router program
//!
//! PDAs are deterministic addresses derived from seeds and the program ID.
//! They allow the program to own and control accounts without needing a private key.

use pinocchio::pubkey::{find_program_address, Pubkey};

/// Seed prefix for vault accounts (one per mint)
pub const VAULT_SEED: &[u8] = b"vault";

/// Seed prefix for escrow accounts (per user, slab, mint)
pub const ESCROW_SEED: &[u8] = b"escrow";

/// Seed prefix for capability token accounts
pub const CAP_SEED: &[u8] = b"cap";

/// Seed prefix for portfolio accounts (per user)
pub const PORTFOLIO_SEED: &[u8] = b"portfolio";

/// Seed prefix for slab registry
pub const REGISTRY_SEED: &[u8] = b"registry";

/// Seed prefix for router authority (used for CPI signing)
pub const AUTHORITY_SEED: &[u8] = b"authority";

/// Seed prefix for router signer PDA (used for matcher CPIs)
pub const ROUTER_SIGNER_SEED: &[u8] = b"router_signer";

/// Derive router authority PDA
///
/// This PDA is used as the router's signing authority for CPIs to slabs.
/// Slabs should be initialized with this PDA as their router_id.
///
/// # Arguments
/// * `program_id` - The router program ID
///
/// # Returns
/// * `(Pubkey, u8)` - The derived PDA and its bump seed
pub fn derive_authority_pda(program_id: &Pubkey) -> (Pubkey, u8) {
    find_program_address(&[AUTHORITY_SEED], program_id)
}

/// Derive router signer PDA for matcher CPIs
///
/// This PDA is used as a signer for all Router → Matcher CPIs.
/// Matchers verify this PDA's derivation to authenticate the Router.
///
/// # Arguments
/// * `program_id` - The router program ID
///
/// # Returns
/// * `(Pubkey, u8)` - The derived PDA and its bump seed
pub fn derive_router_signer_pda(program_id: &Pubkey) -> (Pubkey, u8) {
    find_program_address(&[ROUTER_SIGNER_SEED], program_id)
}

/// Derive vault PDA for a given mint
///
/// Vault stores collateral for a specific mint (e.g., USDC, SOL)
///
/// # Arguments
/// * `mint` - The mint pubkey for which to derive the vault
/// * `program_id` - The router program ID
///
/// # Returns
/// * `(Pubkey, u8)` - The derived PDA and its bump seed
pub fn derive_vault_pda(mint: &Pubkey, program_id: &Pubkey) -> (Pubkey, u8) {
    find_program_address(&[VAULT_SEED, mint.as_ref()], program_id)
}

/// Derive escrow PDA for a user on a specific slab with a specific mint
///
/// Escrow holds user funds pledged to a specific slab
///
/// # Arguments
/// * `user` - The user's pubkey
/// * `slab` - The slab program's pubkey
/// * `mint` - The mint pubkey
/// * `program_id` - The router program ID
///
/// # Returns
/// * `(Pubkey, u8)` - The derived PDA and its bump seed
pub fn derive_escrow_pda(
    user: &Pubkey,
    slab: &Pubkey,
    mint: &Pubkey,
    program_id: &Pubkey,
) -> (Pubkey, u8) {
    find_program_address(
        &[ESCROW_SEED, user.as_ref(), slab.as_ref(), mint.as_ref()],
        program_id,
    )
}

/// Derive capability token PDA
///
/// Capability tokens authorize scoped debits from escrows
///
/// # Arguments
/// * `user` - The user's pubkey
/// * `slab` - The slab program's pubkey
/// * `mint` - The mint pubkey
/// * `nonce` - Unique nonce to allow multiple concurrent caps
/// * `program_id` - The router program ID
///
/// # Returns
/// * `(Pubkey, u8)` - The derived PDA and its bump seed
pub fn derive_cap_pda(
    user: &Pubkey,
    slab: &Pubkey,
    mint: &Pubkey,
    nonce: u64,
    program_id: &Pubkey,
) -> (Pubkey, u8) {
    find_program_address(
        &[
            CAP_SEED,
            user.as_ref(),
            slab.as_ref(),
            mint.as_ref(),
            &nonce.to_le_bytes(),
        ],
        program_id,
    )
}

/// Derive portfolio PDA for a user
///
/// Portfolio aggregates user's positions and margin across all slabs
///
/// # Arguments
/// * `user` - The user's pubkey
/// * `program_id` - The router program ID
///
/// # Returns
/// * `(Pubkey, u8)` - The derived PDA and its bump seed
pub fn derive_portfolio_pda(user: &Pubkey, program_id: &Pubkey) -> (Pubkey, u8) {
    find_program_address(&[PORTFOLIO_SEED, user.as_ref()], program_id)
}

/// Derive slab registry PDA
///
/// Registry maintains list of approved slabs
///
/// # Arguments
/// * `program_id` - The router program ID
///
/// # Returns
/// * `(Pubkey, u8)` - The derived PDA and its bump seed
pub fn derive_registry_pda(program_id: &Pubkey) -> (Pubkey, u8) {
    find_program_address(&[REGISTRY_SEED], program_id)
}

/// Derive LP seat PDA for adapter pattern
///
/// LP seat tracks liquidity provision for a specific (router × matcher × portfolio × context).
/// Seats provide isolation between different LPs on the same matcher.
///
/// # Arguments
/// * `router_id` - The router program ID (for cross-program authentication)
/// * `matcher_state` - The matcher state account
/// * `portfolio` - The portfolio account providing liquidity
/// * `context_id` - Context ID to allow multiple seats per portfolio × matcher
/// * `program_id` - The router program ID (used for derivation)
///
/// # Returns
/// * `(Pubkey, u8)` - The derived PDA and its bump seed
pub fn derive_lp_seat_pda(
    router_id: &Pubkey,
    matcher_state: &Pubkey,
    portfolio: &Pubkey,
    context_id: u32,
    program_id: &Pubkey,
) -> (Pubkey, u8) {
    find_program_address(
        &[
            b"lp_seat",
            router_id.as_ref(),
            matcher_state.as_ref(),
            portfolio.as_ref(),
            &context_id.to_le_bytes(),
        ],
        program_id,
    )
}

#[cfg(test)]
mod tests {
    #[cfg(target_os = "solana")]
    use super::*;

    // Note: PDA tests only run on Solana target due to syscall requirements
    #[test]
    #[cfg(target_os = "solana")]
    fn test_vault_pda_derivation() {
        let program_id = Pubkey::default();
        let mint = Pubkey::default();

        let (pda1, bump1) = derive_vault_pda(&mint, &program_id);
        let (pda2, bump2) = derive_vault_pda(&mint, &program_id);

        // Same inputs should produce same output
        assert_eq!(pda1, pda2);
        assert_eq!(bump1, bump2);
    }

    #[test]
    #[cfg(target_os = "solana")]
    fn test_escrow_pda_derivation() {
        let program_id = Pubkey::default();
        let user = Pubkey::default();
        let slab = Pubkey::default();
        let mint = Pubkey::default();

        let (pda1, bump1) = derive_escrow_pda(&user, &slab, &mint, &program_id);
        let (pda2, bump2) = derive_escrow_pda(&user, &slab, &mint, &program_id);

        // Same inputs should produce same output
        assert_eq!(pda1, pda2);
        assert_eq!(bump1, bump2);
    }

    #[test]
    #[cfg(target_os = "solana")]
    fn test_cap_pda_unique_nonces() {
        let program_id = Pubkey::default();
        let user = Pubkey::default();
        let slab = Pubkey::default();
        let mint = Pubkey::default();

        let (pda1, _) = derive_cap_pda(&user, &slab, &mint, 0, &program_id);
        let (pda2, _) = derive_cap_pda(&user, &slab, &mint, 1, &program_id);

        // Different nonces should produce different PDAs
        assert_ne!(pda1, pda2);
    }

    #[test]
    #[cfg(target_os = "solana")]
    fn test_portfolio_pda_derivation() {
        let program_id = Pubkey::default();
        let user = Pubkey::default();

        let (pda1, bump1) = derive_portfolio_pda(&user, &program_id);
        let (pda2, bump2) = derive_portfolio_pda(&user, &program_id);

        // Same inputs should produce same output
        assert_eq!(pda1, pda2);
        assert_eq!(bump1, bump2);
    }

    #[test]
    #[cfg(target_os = "solana")]
    fn test_registry_pda_derivation() {
        let program_id = Pubkey::default();

        let (pda1, bump1) = derive_registry_pda(&program_id);
        let (pda2, bump2) = derive_registry_pda(&program_id);

        // Same program ID should produce same output
        assert_eq!(pda1, pda2);
        assert_eq!(bump1, bump2);
    }

    #[test]
    #[cfg(target_os = "solana")]
    fn test_lp_seat_pda_derivation() {
        let program_id = Pubkey::default();
        let router_id = Pubkey::default();
        let matcher_state = Pubkey::default();
        let portfolio = Pubkey::default();
        let context_id = 0u32;

        let (pda1, bump1) = derive_lp_seat_pda(&router_id, &matcher_state, &portfolio, context_id, &program_id);
        let (pda2, bump2) = derive_lp_seat_pda(&router_id, &matcher_state, &portfolio, context_id, &program_id);

        // Same inputs should produce same output
        assert_eq!(pda1, pda2);
        assert_eq!(bump1, bump2);

        // Different context_id should produce different PDA
        let (pda3, _) = derive_lp_seat_pda(&router_id, &matcher_state, &portfolio, 1u32, &program_id);
        assert_ne!(pda1, pda3);
    }
}
